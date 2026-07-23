//! Durable, privacy-first command history.
//!
//! The store deliberately keeps SQLite work off the terminal I/O path.  Command
//! lifecycle updates are sanitized before being placed on a bounded queue, then
//! committed in batches by a dedicated writer thread.  Reads use a separate
//! connection and WAL mode, so history search cannot stall PTY output.
//!
//! Persistence is fail-closed: every connection is keyed with SQLCipher before
//! its first query. On macOS, the random database key lives in the
//! non-synchronizing, device-only data-protection Keychain. If Keychain access,
//! migration, or decryption fails, callers keep their in-memory shell history
//! and no plaintext database writes are attempted.

use regex::Regex;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Row, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[cfg(target_os = "macos")]
use security_framework::access_control::{ProtectionMode, SecAccessControl};
#[cfg(target_os = "macos")]
use security_framework::passwords::{
    generic_password, set_generic_password_options, PasswordOptions,
};
#[cfg(target_os = "macos")]
use security_framework::random::SecRandom;

const SCHEMA_VERSION: i32 = 2;
const DEFAULT_QUEUE_CAPACITY: usize = 1_024;
const DEFAULT_BATCH_SIZE: usize = 64;
const DEFAULT_PRUNE_EVERY_WRITES: usize = 256;
const DEFAULT_MAX_COMMAND_BYTES: usize = 64 * 1_024;
const DEFAULT_MAX_CWD_BYTES: usize = 16 * 1_024;
const DEFAULT_MAX_SHELL_BYTES: usize = 4 * 1_024;
const MAX_IDENTIFIER_BYTES: usize = 512;
const MAX_SEARCH_RESULTS: usize = 500;
const WRITER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const SQLITE_PLAINTEXT_HEADER: &[u8; 16] = b"SQLite format 3\0";
const HISTORY_KEY_BYTES: usize = 32;
const MIGRATION_STAGING_SUFFIX: &str = ".plaintext-migration";
const MIGRATION_ENCRYPTED_SUFFIX: &str = ".encrypted-migration";

#[cfg(target_os = "macos")]
const HISTORY_KEYCHAIN_SERVICE: &str = "com.efficienttools.ph7console.history-database";
#[cfg(target_os = "macos")]
const HISTORY_KEYCHAIN_ACCOUNT: &str = "sqlcipher-key-v1";
#[cfg(target_os = "macos")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25_300;

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS commands (
    id                TEXT PRIMARY KEY NOT NULL,
    session_id        TEXT NOT NULL,
    command           TEXT NOT NULL,
    cwd               TEXT NOT NULL DEFAULT '',
    shell             TEXT,
    started_at_ms     INTEGER NOT NULL,
    finished_at_ms    INTEGER,
    duration_ms       INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    exit_code         INTEGER,
    status            INTEGER NOT NULL DEFAULT 0 CHECK (status IN (0, 1, 2)),
    output_excerpt    TEXT
);

CREATE INDEX IF NOT EXISTS commands_session_started_idx
    ON commands(session_id, started_at_ms DESC);
CREATE INDEX IF NOT EXISTS commands_started_idx
    ON commands(started_at_ms DESC);
CREATE INDEX IF NOT EXISTS commands_status_idx
    ON commands(status);

PRAGMA user_version = 1;
"#;

const MIGRATION_2: &str = r#"
ALTER TABLE commands
    ADD COLUMN redacted INTEGER NOT NULL DEFAULT 0 CHECK (redacted IN (0, 1));
ALTER TABLE commands
    ADD COLUMN starred INTEGER NOT NULL DEFAULT 0 CHECK (starred IN (0, 1));

CREATE INDEX IF NOT EXISTS commands_starred_started_idx
    ON commands(starred, started_at_ms DESC);

CREATE VIRTUAL TABLE command_search USING fts5(
    command,
    cwd,
    output_excerpt,
    content = 'commands',
    content_rowid = 'rowid',
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER commands_search_insert AFTER INSERT ON commands BEGIN
    INSERT INTO command_search(rowid, command, cwd, output_excerpt)
    VALUES (new.rowid, new.command, new.cwd, new.output_excerpt);
END;

CREATE TRIGGER commands_search_delete AFTER DELETE ON commands BEGIN
    INSERT INTO command_search(command_search, rowid, command, cwd, output_excerpt)
    VALUES ('delete', old.rowid, old.command, old.cwd, old.output_excerpt);
END;

CREATE TRIGGER commands_search_update AFTER UPDATE ON commands BEGIN
    INSERT INTO command_search(command_search, rowid, command, cwd, output_excerpt)
    VALUES ('delete', old.rowid, old.command, old.cwd, old.output_excerpt);
    INSERT INTO command_search(rowid, command, cwd, output_excerpt)
    VALUES (new.rowid, new.command, new.cwd, new.output_excerpt);
END;

INSERT INTO command_search(command_search) VALUES ('rebuild');
PRAGMA user_version = 2;
"#;

/// Controls automatic and explicit history pruning.
///
/// When `preserve_starred` is true, protected rows do not count toward
/// `max_records`; the database may therefore contain more rows than that cap.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RetentionPolicy {
    /// Maximum number of non-running records to retain. `None` means unlimited.
    pub max_records: Option<u64>,
    /// Delete completed/interrupted records older than this many days.
    pub max_age_days: Option<u32>,
    /// Never remove starred records during retention pruning.
    pub preserve_starred: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_records: Some(25_000),
            max_age_days: Some(180),
            preserve_starred: true,
        }
    }
}

impl RetentionPolicy {
    /// A policy useful for callers that implement retention elsewhere.
    pub const fn unlimited() -> Self {
        Self {
            max_records: None,
            max_age_days: None,
            preserve_starred: true,
        }
    }
}

/// A 256-bit SQLCipher key. The value is intentionally omitted from every
/// formatter and is overwritten when its owner is dropped.
pub struct HistoryEncryptionKey([u8; HISTORY_KEY_BYTES]);

impl HistoryEncryptionKey {
    /// Construct a key supplied by a test or another platform-specific secure
    /// storage provider. Production macOS startup uses [`HistoryStore::open_with_keychain`].
    pub fn from_bytes(bytes: [u8; HISTORY_KEY_BYTES]) -> Self {
        Self(bytes)
    }

    fn expose(&self) -> &[u8; HISTORY_KEY_BYTES] {
        &self.0
    }
}

impl fmt::Debug for HistoryEncryptionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HistoryEncryptionKey(<redacted>)")
    }
}

impl Drop for HistoryEncryptionKey {
    fn drop(&mut self) {
        for byte in &mut self.0 {
            // SAFETY: `byte` is a valid, uniquely borrowed byte in this value.
            // Volatile writes keep the best-effort wipe from being optimized
            // away when the key leaves Rust-owned memory.
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

/// Runtime configuration for [`HistoryStore`].
#[derive(Debug, Clone)]
pub struct HistoryConfig {
    pub database_path: PathBuf,
    pub queue_capacity: usize,
    pub batch_size: usize,
    pub prune_every_writes: usize,
    pub retention: RetentionPolicy,
    /// Output is more likely than commands to contain private data, so it is not
    /// persisted by default. When enabled, it is still redacted and bounded.
    pub store_output_excerpts: bool,
    pub max_output_excerpt_bytes: usize,
    pub max_command_bytes: usize,
    pub max_cwd_bytes: usize,
}

impl HistoryConfig {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            batch_size: DEFAULT_BATCH_SIZE,
            prune_every_writes: DEFAULT_PRUNE_EVERY_WRITES,
            retention: RetentionPolicy::default(),
            store_output_excerpts: false,
            max_output_excerpt_bytes: 4 * 1_024,
            max_command_bytes: DEFAULT_MAX_COMMAND_BYTES,
            max_cwd_bytes: DEFAULT_MAX_CWD_BYTES,
        }
    }
}

#[derive(Debug)]
pub enum HistoryError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Security(String),
    InvalidInput(&'static str),
    UnsupportedSchema(i32),
    AlreadyOpen,
    QueueFull,
    Closed,
    Worker(String),
    Synchronization(&'static str),
}

impl fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "history database error: {error}"),
            Self::Io(error) => write!(formatter, "history filesystem error: {error}"),
            Self::Security(message) => write!(formatter, "history security error: {message}"),
            Self::InvalidInput(message) => write!(formatter, "invalid history input: {message}"),
            Self::UnsupportedSchema(version) => write!(
                formatter,
                "history schema {version} is newer than supported version {SCHEMA_VERSION}"
            ),
            Self::AlreadyOpen => write!(
                formatter,
                "encrypted history is already open in another pH7Console process"
            ),
            Self::QueueFull => write!(formatter, "history writer queue is full"),
            Self::Closed => write!(formatter, "history store is closed"),
            Self::Worker(message) => write!(formatter, "history writer error: {message}"),
            Self::Synchronization(message) => {
                write!(formatter, "history synchronization error: {message}")
            }
        }
    }
}

impl std::error::Error for HistoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for HistoryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<std::io::Error> for HistoryError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub type HistoryResult<T> = Result<T, HistoryError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Running,
    Completed,
    Interrupted,
}

impl CommandStatus {
    fn as_db(self) -> i64 {
        match self {
            Self::Running => 0,
            Self::Completed => 1,
            Self::Interrupted => 2,
        }
    }

    fn from_db(value: i64) -> Self {
        match value {
            0 => Self::Running,
            1 => Self::Completed,
            _ => Self::Interrupted,
        }
    }
}

/// A command lifecycle start event. Sensitive fields are intentionally hidden
/// from `Debug`; sanitization happens synchronously in [`HistoryStore::record_start`].
#[derive(Clone)]
pub struct CommandStart {
    pub id: String,
    pub session_id: String,
    pub command: String,
    pub cwd: String,
    pub shell: Option<String>,
    pub started_at_ms: i64,
}

impl CommandStart {
    pub fn new(
        session_id: impl Into<String>,
        command: impl Into<String>,
        cwd: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            command: command.into(),
            cwd: cwd.into(),
            shell: None,
            started_at_ms: unix_timestamp_ms(),
        }
    }

    pub fn with_shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = Some(shell.into());
        self
    }
}

impl fmt::Debug for CommandStart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandStart")
            .field("id", &self.id)
            .field("session_id", &self.session_id)
            .field("command", &"<sensitive>")
            .field("cwd", &"<private path>")
            .field("shell", &self.shell)
            .field("started_at_ms", &self.started_at_ms)
            .finish()
    }
}

/// A command lifecycle completion event.
#[derive(Clone)]
pub struct CommandFinish {
    pub id: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub output_excerpt: Option<String>,
    pub finished_at_ms: i64,
    pub status: CommandStatus,
}

impl CommandFinish {
    pub fn completed(id: impl Into<String>, exit_code: Option<i32>, duration_ms: u64) -> Self {
        Self {
            id: id.into(),
            exit_code,
            duration_ms,
            output_excerpt: None,
            finished_at_ms: unix_timestamp_ms(),
            status: CommandStatus::Completed,
        }
    }

    pub fn interrupted(id: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            id: id.into(),
            exit_code: None,
            duration_ms,
            output_excerpt: None,
            finished_at_ms: unix_timestamp_ms(),
            status: CommandStatus::Interrupted,
        }
    }

    pub fn with_output_excerpt(mut self, output: impl Into<String>) -> Self {
        self.output_excerpt = Some(output.into());
        self
    }
}

impl fmt::Debug for CommandFinish {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandFinish")
            .field("id", &self.id)
            .field("exit_code", &self.exit_code)
            .field("duration_ms", &self.duration_ms)
            .field(
                "output_excerpt",
                &self.output_excerpt.as_ref().map(|_| "<sensitive>"),
            )
            .field("finished_at_ms", &self.finished_at_ms)
            .field("status", &self.status)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandRecord {
    pub id: String,
    pub session_id: String,
    pub command: String,
    pub cwd: String,
    pub shell: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub status: CommandStatus,
    pub output_excerpt: Option<String>,
    pub redacted: bool,
    pub starred: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    Prefix,
    FullText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistorySearch {
    pub query: String,
    pub session_id: Option<String>,
    pub mode: SearchMode,
    pub limit: usize,
}

impl HistorySearch {
    pub fn prefix(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            session_id: None,
            mode: SearchMode::Prefix,
            limit: 100,
        }
    }

    pub fn full_text(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            session_id: None,
            mode: SearchMode::FullText,
            limit: 100,
        }
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// The result of sanitizing text before persistence.
#[derive(Clone, PartialEq, Eq)]
pub struct RedactedText {
    pub value: String,
    pub was_redacted: bool,
}

impl fmt::Debug for RedactedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedactedText")
            .field("value", &"<sensitive text>")
            .field("was_redacted", &self.was_redacted)
            .finish()
    }
}

struct RedactionRule {
    expression: Regex,
    replacement: &'static str,
}

/// Redact common credential shapes while retaining enough command structure to
/// keep search and local learning useful. This is intentionally conservative:
/// no redactor can prove arbitrary text contains no secret.
pub fn redact_sensitive(input: &str) -> RedactedText {
    let mut value = input.to_owned();
    let mut was_redacted = false;

    for rule in redaction_rules() {
        let replacement = rule
            .expression
            .replace_all(&value, rule.replacement)
            .into_owned();
        if replacement != value {
            value = replacement;
            was_redacted = true;
        }
    }

    RedactedText {
        value,
        was_redacted,
    }
}

fn redaction_rules() -> &'static [RedactionRule] {
    static RULES: OnceLock<Vec<RedactionRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            RedactionRule {
                expression: Regex::new(
                    r"(?is)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?-----END [A-Z0-9 ]*PRIVATE KEY-----",
                )
                .expect("valid private-key redaction regex"),
                replacement: "[REDACTED PRIVATE KEY]",
            },
            RedactionRule {
                expression: Regex::new(
                    r#"(?i)([\"'](?:token|secret|password|passwd|api[-_]?key|private[-_]?key|access[-_]?key|client[-_]?secret|authorization|credential)[\"']\s*:\s*)(?:\"[^\"]*\"|'[^']*'|[^\s,;}]+)"#,
                )
                .expect("valid structured-value redaction regex"),
                replacement: "$1[REDACTED]",
            },
            RedactionRule {
                expression: Regex::new(
                    r#"(?i)(\b(?:[a-z0-9_]*(?:token|secret|password|passwd|api_?key|apikey|private_?key|access_?key|credential)[a-z0-9_]*|authorization)\s*=\s*)(?:\"[^\"]*\"|'[^']*'|[^\s,;]+)"#,
                )
                .expect("valid assignment redaction regex"),
                replacement: "$1[REDACTED]",
            },
            RedactionRule {
                expression: Regex::new(
                    r#"(?i)(--(?:token|secret|password|passwd|api[-_]?key|private[-_]?key|access[-_]?key|client[-_]?secret|authorization|credential)(?:=|\s+))(?:\"[^\"]*\"|'[^']*'|[^\s,;]+)"#,
                )
                .expect("valid option redaction regex"),
                replacement: "$1[REDACTED]",
            },
            RedactionRule {
                expression: Regex::new(r"(?i)(\b(?:bearer|basic)\s+)[A-Za-z0-9._~+/=-]+")
                    .expect("valid authorization redaction regex"),
                replacement: "$1[REDACTED]",
            },
            RedactionRule {
                expression: Regex::new(
                    r"(?i)([a-z][a-z0-9+.-]*://[^/\s:@]+:)[^@\s/]+(@)",
                )
                .expect("valid URL credential redaction regex"),
                replacement: "$1[REDACTED]$2",
            },
            RedactionRule {
                expression: Regex::new(
                    r"\b(?:sk-(?:proj-)?[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[baprs]-[A-Za-z0-9-]{16,}|AKIA[0-9A-Z]{16})\b",
                )
                .expect("valid known-token redaction regex"),
                replacement: "[REDACTED TOKEN]",
            },
            RedactionRule {
                expression: Regex::new(
                    r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b",
                )
                .expect("valid JWT redaction regex"),
                replacement: "[REDACTED JWT]",
            },
        ]
    })
}

#[derive(Clone)]
struct StoredCommandStart {
    id: String,
    session_id: String,
    command: String,
    cwd: String,
    shell: Option<String>,
    started_at_ms: i64,
    redacted: bool,
}

#[derive(Clone)]
struct StoredCommandFinish {
    id: String,
    exit_code: Option<i32>,
    duration_ms: i64,
    output_excerpt: Option<String>,
    finished_at_ms: i64,
    status: CommandStatus,
    redacted: bool,
}

enum Mutation {
    Start(StoredCommandStart),
    Finish(StoredCommandFinish),
    SetStarred { id: String, starred: bool },
    DeleteSession { session_id: String },
    Clear { include_starred: bool },
}

enum WriterMessage {
    Mutation(Mutation),
    Prune {
        policy: RetentionPolicy,
        reply: mpsc::Sender<Result<usize, String>>,
    },
    Flush {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Shutdown {
        reply: mpsc::Sender<Result<(), String>>,
    },
}

struct HistoryInner {
    path: PathBuf,
    config: HistoryConfig,
    /// Held for the lifetime of the store. This prevents a second app process
    /// from racing first-run Keychain initialization or plaintext migration.
    _process_lock: File,
    sender: Mutex<Option<SyncSender<WriterMessage>>>,
    read_connection: Mutex<Connection>,
    worker: Mutex<Option<JoinHandle<()>>>,
    last_background_error: Arc<Mutex<Option<String>>>,
}

impl Drop for HistoryInner {
    fn drop(&mut self) {
        let sender = match self.sender.get_mut() {
            Ok(slot) => slot.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };

        if let Some(sender) = sender {
            let (reply, response) = mpsc::channel();
            if sender.send(WriterMessage::Shutdown { reply }).is_ok() {
                let _ = response.recv_timeout(WRITER_SHUTDOWN_TIMEOUT);
            }
            drop(sender);
        }

        let worker = match self.worker.get_mut() {
            Ok(slot) => slot.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
}

/// Thread-safe handle to durable command history.
#[derive(Clone)]
pub struct HistoryStore {
    inner: Arc<HistoryInner>,
}

impl fmt::Debug for HistoryStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HistoryStore")
            .field("database_path", &self.inner.path)
            .finish_non_exhaustive()
    }
}

impl HistoryStore {
    /// Open encrypted history using an explicitly supplied key. This is the
    /// injection point used by tests and non-macOS secure-storage adapters.
    /// The key is applied before any query and wiped from Rust-owned memory
    /// when this function returns.
    pub fn open(config: HistoryConfig, key: HistoryEncryptionKey) -> HistoryResult<Self> {
        let (config, process_lock) = prepare_open(config)?;
        Self::open_locked(config, key, process_lock)
    }

    /// Open encrypted history with a random key in the native macOS
    /// data-protection Keychain. The item is device-only and explicitly not
    /// synchronized through iCloud.
    #[cfg(target_os = "macos")]
    pub fn open_with_keychain(config: HistoryConfig) -> HistoryResult<Self> {
        let (config, process_lock) = prepare_open(config)?;
        let key = load_or_create_keychain_key(&config.database_path)?;
        Self::open_locked(config, key, process_lock)
    }

    fn open_locked(
        config: HistoryConfig,
        key: HistoryEncryptionKey,
        process_lock: File,
    ) -> HistoryResult<Self> {
        recover_interrupted_plaintext_migration(&config.database_path)?;
        migrate_plaintext_database(&config.database_path, &key)?;

        let writer_flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut writer_connection =
            open_encrypted_connection(&config.database_path, writer_flags, &key)?;
        configure_writer_connection(&writer_connection)?;
        migrate(&mut writer_connection)?;
        recover_interrupted_commands(&writer_connection, unix_timestamp_ms())?;
        apply_retention(
            &mut writer_connection,
            config.retention,
            unix_timestamp_ms(),
        )?;
        set_private_database_permissions(&config.database_path)?;

        let read_connection = open_encrypted_connection(
            &config.database_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            &key,
        )?;
        configure_read_connection(&read_connection)?;

        // Only remove a pre-encryption database after both encrypted
        // connections have authenticated and the schema migration succeeded.
        remove_plaintext_migration_artifacts(&config.database_path)?;

        let (sender, receiver) = mpsc::sync_channel(config.queue_capacity);
        let last_background_error = Arc::new(Mutex::new(None));
        let worker_error = Arc::clone(&last_background_error);
        let worker_config = config.clone();
        let worker = thread::Builder::new()
            .name("ph7-history-writer".to_owned())
            .spawn(move || writer_loop(writer_connection, receiver, worker_config, worker_error))?;

        Ok(Self {
            inner: Arc::new(HistoryInner {
                path: config.database_path.clone(),
                config,
                _process_lock: process_lock,
                sender: Mutex::new(Some(sender)),
                read_connection: Mutex::new(read_connection),
                worker: Mutex::new(Some(worker)),
                last_background_error,
            }),
        })
    }
}

fn prepare_open(mut config: HistoryConfig) -> HistoryResult<(HistoryConfig, File)> {
    if config.database_path.as_os_str().is_empty() {
        return Err(HistoryError::InvalidInput("database path is empty"));
    }

    config.queue_capacity = config.queue_capacity.max(1);
    config.batch_size = config.batch_size.clamp(1, 1_024);
    config.prune_every_writes = config.prune_every_writes.max(1);
    config.max_command_bytes = config.max_command_bytes.max(1);
    config.max_cwd_bytes = config.max_cwd_bytes.max(1);

    prepare_database_directory(&config.database_path)?;

    let process_lock = acquire_process_lock(&config.database_path)?;
    Ok((config, process_lock))
}

impl HistoryStore {
    pub fn database_path(&self) -> &Path {
        &self.inner.path
    }

    /// Sanitize and enqueue a command start without blocking on SQLite.
    pub fn record_start(&self, start: CommandStart) -> HistoryResult<()> {
        validate_identifier(&start.id, "command id")?;
        validate_identifier(&start.session_id, "session id")?;
        if start.command.trim().is_empty() {
            return Err(HistoryError::InvalidInput("command is empty"));
        }

        let command = redact_sensitive(start.command.trim());
        let cwd = redact_sensitive(&start.cwd);
        let (shell, shell_redacted) = start.shell.map_or((None, false), |value| {
            let redacted = redact_sensitive(&value);
            (
                Some(truncate_utf8(&redacted.value, DEFAULT_MAX_SHELL_BYTES)),
                redacted.was_redacted,
            )
        });
        let stored = StoredCommandStart {
            id: start.id,
            session_id: start.session_id,
            command: truncate_utf8(&command.value, self.inner.config.max_command_bytes),
            cwd: truncate_utf8(&cwd.value, self.inner.config.max_cwd_bytes),
            shell,
            started_at_ms: if start.started_at_ms > 0 {
                start.started_at_ms
            } else {
                unix_timestamp_ms()
            },
            redacted: command.was_redacted || cwd.was_redacted || shell_redacted,
        };

        self.enqueue(Mutation::Start(stored))
    }

    /// Sanitize and enqueue a command completion without blocking on SQLite.
    pub fn record_finish(&self, finish: CommandFinish) -> HistoryResult<()> {
        validate_identifier(&finish.id, "command id")?;
        if finish.status == CommandStatus::Running {
            return Err(HistoryError::InvalidInput(
                "a finish event cannot have running status",
            ));
        }

        let (output_excerpt, output_redacted) = if self.inner.config.store_output_excerpts {
            finish.output_excerpt.map_or((None, false), |output| {
                let redacted = redact_sensitive(&output);
                (
                    Some(truncate_utf8(
                        &redacted.value,
                        self.inner.config.max_output_excerpt_bytes,
                    )),
                    redacted.was_redacted,
                )
            })
        } else {
            (None, false)
        };

        let stored = StoredCommandFinish {
            id: finish.id,
            exit_code: finish.exit_code,
            duration_ms: finish.duration_ms.min(i64::MAX as u64) as i64,
            output_excerpt,
            finished_at_ms: if finish.finished_at_ms > 0 {
                finish.finished_at_ms
            } else {
                unix_timestamp_ms()
            },
            status: finish.status,
            redacted: output_redacted,
        };

        self.enqueue(Mutation::Finish(stored))
    }

    pub fn set_starred(&self, id: impl Into<String>, starred: bool) -> HistoryResult<()> {
        let id = id.into();
        validate_identifier(&id, "command id")?;
        self.enqueue(Mutation::SetStarred { id, starred })
    }

    pub fn delete_session(&self, session_id: impl Into<String>) -> HistoryResult<()> {
        let session_id = session_id.into();
        validate_identifier(&session_id, "session id")?;
        self.enqueue(Mutation::DeleteSession { session_id })
    }

    /// Clear history. Starred records can be preserved.
    pub fn clear(&self, include_starred: bool) -> HistoryResult<()> {
        self.enqueue(Mutation::Clear { include_starred })
    }

    /// Wait until every previously enqueued mutation is committed.
    pub fn flush(&self) -> HistoryResult<()> {
        let (reply, response) = mpsc::channel();
        self.send_control(WriterMessage::Flush { reply })?;
        match response.recv() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(message)) => Err(HistoryError::Worker(message)),
            Err(_) => Err(HistoryError::Closed),
        }
    }

    /// Apply a retention policy immediately and return the number of rows removed.
    pub fn enforce_retention(&self, policy: RetentionPolicy) -> HistoryResult<usize> {
        let (reply, response) = mpsc::channel();
        self.send_control(WriterMessage::Prune { policy, reply })?;
        match response.recv() {
            Ok(Ok(deleted)) => Ok(deleted),
            Ok(Err(message)) => Err(HistoryError::Worker(message)),
            Err(_) => Err(HistoryError::Closed),
        }
    }

    pub fn get(&self, id: &str) -> HistoryResult<Option<CommandRecord>> {
        validate_identifier(id, "command id")?;
        let connection = self.read_connection()?;
        connection
            .query_row(
                &format!("SELECT {RECORD_COLUMNS} FROM commands c WHERE c.id = ?1 LIMIT 1"),
                params![id],
                row_to_record,
            )
            .optional()
            .map_err(HistoryError::from)
    }

    /// Most-recent-first records, optionally scoped to one terminal session.
    pub fn recent(
        &self,
        session_id: Option<&str>,
        limit: usize,
    ) -> HistoryResult<Vec<CommandRecord>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        if let Some(session_id) = session_id {
            validate_identifier(session_id, "session id")?;
        }

        let limit = limit.min(MAX_SEARCH_RESULTS) as i64;
        let connection = self.read_connection()?;
        if let Some(session_id) = session_id {
            collect_records(
                &connection,
                &format!(
                    "SELECT {RECORD_COLUMNS} FROM commands c \
                     WHERE c.session_id = ?1 ORDER BY c.started_at_ms DESC, c.rowid DESC LIMIT ?2"
                ),
                params![session_id, limit],
            )
        } else {
            collect_records(
                &connection,
                &format!(
                    "SELECT {RECORD_COLUMNS} FROM commands c \
                     ORDER BY c.started_at_ms DESC, c.rowid DESC LIMIT ?1"
                ),
                params![limit],
            )
        }
    }

    /// Prefix search preserves shell punctuation; full-text search uses FTS5
    /// token-prefix matching and BM25 relevance ordering.
    pub fn search(&self, search: &HistorySearch) -> HistoryResult<Vec<CommandRecord>> {
        if search.limit == 0 || search.query.trim().is_empty() {
            return Ok(Vec::new());
        }
        if let Some(session_id) = search.session_id.as_deref() {
            validate_identifier(session_id, "session id")?;
        }

        let limit = search.limit.min(MAX_SEARCH_RESULTS) as i64;
        let connection = self.read_connection()?;
        match search.mode {
            SearchMode::Prefix => {
                let pattern = format!("{}%", escape_like(search.query.trim()));
                if let Some(session_id) = search.session_id.as_deref() {
                    collect_records(
                        &connection,
                        &format!(
                            "SELECT {RECORD_COLUMNS} FROM commands c \
                             WHERE c.command COLLATE NOCASE LIKE ?1 ESCAPE '\\' \
                               AND c.session_id = ?2 \
                             ORDER BY c.started_at_ms DESC, c.rowid DESC LIMIT ?3"
                        ),
                        params![pattern, session_id, limit],
                    )
                } else {
                    collect_records(
                        &connection,
                        &format!(
                            "SELECT {RECORD_COLUMNS} FROM commands c \
                             WHERE c.command COLLATE NOCASE LIKE ?1 ESCAPE '\\' \
                             ORDER BY c.started_at_ms DESC, c.rowid DESC LIMIT ?2"
                        ),
                        params![pattern, limit],
                    )
                }
            }
            SearchMode::FullText => {
                let Some(fts_query) = build_fts_query(search.query.trim()) else {
                    return Ok(Vec::new());
                };
                if let Some(session_id) = search.session_id.as_deref() {
                    collect_records(
                        &connection,
                        &format!(
                            "SELECT {RECORD_COLUMNS} \
                             FROM command_search JOIN commands c ON c.rowid = command_search.rowid \
                             WHERE command_search MATCH ?1 AND c.session_id = ?2 \
                             ORDER BY bm25(command_search, 10.0, 1.0, 0.5), \
                                      c.started_at_ms DESC LIMIT ?3"
                        ),
                        params![fts_query, session_id, limit],
                    )
                } else {
                    collect_records(
                        &connection,
                        &format!(
                            "SELECT {RECORD_COLUMNS} \
                             FROM command_search JOIN commands c ON c.rowid = command_search.rowid \
                             WHERE command_search MATCH ?1 \
                             ORDER BY bm25(command_search, 10.0, 1.0, 0.5), \
                                      c.started_at_ms DESC LIMIT ?2"
                        ),
                        params![fts_query, limit],
                    )
                }
            }
        }
    }

    pub fn schema_version(&self) -> HistoryResult<i32> {
        let connection = self.read_connection()?;
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(HistoryError::from)
    }

    /// Lightweight SQLite consistency check suitable for diagnostics.
    pub fn quick_check(&self) -> HistoryResult<()> {
        let connection = self.read_connection()?;
        let result: String = connection.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;
        if result == "ok" {
            Ok(())
        } else {
            Err(HistoryError::Worker(format!(
                "SQLite quick_check reported: {result}"
            )))
        }
    }

    pub fn last_background_error(&self) -> Option<String> {
        match self.inner.last_background_error.lock() {
            Ok(value) => value.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    fn enqueue(&self, mutation: Mutation) -> HistoryResult<()> {
        let sender = self.sender()?;
        match sender.try_send(WriterMessage::Mutation(mutation)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(HistoryError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(HistoryError::Closed),
        }
    }

    fn send_control(&self, message: WriterMessage) -> HistoryResult<()> {
        let sender = self.sender()?;
        sender.send(message).map_err(|_| HistoryError::Closed)
    }

    fn sender(&self) -> HistoryResult<SyncSender<WriterMessage>> {
        let sender = self
            .inner
            .sender
            .lock()
            .map_err(|_| HistoryError::Synchronization("writer sender lock poisoned"))?;
        sender.as_ref().cloned().ok_or(HistoryError::Closed)
    }

    fn read_connection(&self) -> HistoryResult<MutexGuard<'_, Connection>> {
        self.inner
            .read_connection
            .lock()
            .map_err(|_| HistoryError::Synchronization("read connection lock poisoned"))
    }
}

const RECORD_COLUMNS: &str = "\
    c.id, c.session_id, c.command, c.cwd, c.shell, c.started_at_ms, \
    c.finished_at_ms, c.duration_ms, c.exit_code, c.status, \
    c.output_excerpt, c.redacted, c.starred";

fn row_to_record(row: &Row<'_>) -> rusqlite::Result<CommandRecord> {
    let duration_ms: Option<i64> = row.get(7)?;
    Ok(CommandRecord {
        id: row.get(0)?,
        session_id: row.get(1)?,
        command: row.get(2)?,
        cwd: row.get(3)?,
        shell: row.get(4)?,
        started_at_ms: row.get(5)?,
        finished_at_ms: row.get(6)?,
        duration_ms: duration_ms.map(|value| value.max(0) as u64),
        exit_code: row.get(8)?,
        status: CommandStatus::from_db(row.get(9)?),
        output_excerpt: row.get(10)?,
        redacted: row.get::<_, i64>(11)? != 0,
        starred: row.get::<_, i64>(12)? != 0,
    })
}

fn collect_records<P>(
    connection: &Connection,
    sql: &str,
    parameters: P,
) -> HistoryResult<Vec<CommandRecord>>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare_cached(sql)?;
    let records = statement
        .query_map(parameters, row_to_record)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

fn writer_loop(
    mut connection: Connection,
    receiver: Receiver<WriterMessage>,
    config: HistoryConfig,
    last_background_error: Arc<Mutex<Option<String>>>,
) {
    let mut pending: Vec<Mutation> = Vec::with_capacity(config.batch_size);
    let mut writes_since_prune = 0usize;
    let mut unreported_error: Option<String> = None;
    let mut should_exit = false;

    while !should_exit {
        let first = match receiver.recv() {
            Ok(message) => message,
            Err(_) => break,
        };
        let mut messages = Vec::with_capacity(config.batch_size);
        messages.push(first);
        while messages.len() < config.batch_size {
            match receiver.try_recv() {
                Ok(message) => messages.push(message),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        for message in messages {
            match message {
                WriterMessage::Mutation(mutation) => pending.push(mutation),
                WriterMessage::Prune { policy, reply } => {
                    flush_pending(
                        &mut connection,
                        &mut pending,
                        &mut writes_since_prune,
                        &config,
                        &last_background_error,
                        &mut unreported_error,
                    );
                    let result = apply_retention(&mut connection, policy, unix_timestamp_ms())
                        .map_err(|error| error.to_string());
                    if let Err(message) = &result {
                        remember_worker_error(
                            message.clone(),
                            &last_background_error,
                            &mut unreported_error,
                        );
                    }
                    let _ = reply.send(result);
                }
                WriterMessage::Flush { reply } => {
                    flush_pending(
                        &mut connection,
                        &mut pending,
                        &mut writes_since_prune,
                        &config,
                        &last_background_error,
                        &mut unreported_error,
                    );
                    let result = match unreported_error.take() {
                        Some(message) => Err(message),
                        None => Ok(()),
                    };
                    let _ = reply.send(result);
                }
                WriterMessage::Shutdown { reply } => {
                    flush_pending(
                        &mut connection,
                        &mut pending,
                        &mut writes_since_prune,
                        &config,
                        &last_background_error,
                        &mut unreported_error,
                    );
                    let _ = connection.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
                    let result = match unreported_error.take() {
                        Some(message) => Err(message),
                        None => Ok(()),
                    };
                    let _ = reply.send(result);
                    should_exit = true;
                    break;
                }
            }
        }

        flush_pending(
            &mut connection,
            &mut pending,
            &mut writes_since_prune,
            &config,
            &last_background_error,
            &mut unreported_error,
        );
    }
}

fn flush_pending(
    connection: &mut Connection,
    pending: &mut Vec<Mutation>,
    writes_since_prune: &mut usize,
    config: &HistoryConfig,
    last_background_error: &Arc<Mutex<Option<String>>>,
    unreported_error: &mut Option<String>,
) {
    if pending.is_empty() {
        return;
    }

    let count = pending.len();
    if apply_mutation_batch(connection, pending).is_err() {
        // A malformed mutation must not make unrelated command history disappear.
        // The batch transaction has rolled back, so retry each row independently.
        let mut first_individual_error = None;
        for mutation in pending.iter() {
            if let Err(individual_error) = apply_mutation(connection, mutation) {
                if first_individual_error.is_none() {
                    first_individual_error = Some(individual_error.to_string());
                }
            }
        }
        if let Some(message) = first_individual_error {
            remember_worker_error(message, last_background_error, unreported_error);
        }
    }
    pending.clear();

    *writes_since_prune = writes_since_prune.saturating_add(count);
    if *writes_since_prune >= config.prune_every_writes {
        if let Err(error) = apply_retention(connection, config.retention, unix_timestamp_ms()) {
            remember_worker_error(error.to_string(), last_background_error, unreported_error);
        }
        *writes_since_prune = 0;
    }
}

fn remember_worker_error(
    message: String,
    last_background_error: &Arc<Mutex<Option<String>>>,
    unreported_error: &mut Option<String>,
) {
    *unreported_error = Some(message.clone());
    match last_background_error.lock() {
        Ok(mut last_error) => *last_error = Some(message),
        Err(poisoned) => *poisoned.into_inner() = Some(message),
    }
}

fn apply_mutation_batch(
    connection: &mut Connection,
    mutations: &[Mutation],
) -> rusqlite::Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    for mutation in mutations {
        apply_mutation_on(&transaction, mutation)?;
    }
    transaction.commit()
}

fn apply_mutation(connection: &Connection, mutation: &Mutation) -> rusqlite::Result<()> {
    apply_mutation_on(connection, mutation)
}

fn apply_mutation_on(connection: &Connection, mutation: &Mutation) -> rusqlite::Result<()> {
    match mutation {
        Mutation::Start(start) => {
            connection.execute(
                "INSERT INTO commands (
                    id, session_id, command, cwd, shell, started_at_ms, status, redacted
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    start.id,
                    start.session_id,
                    start.command,
                    start.cwd,
                    start.shell,
                    start.started_at_ms,
                    bool_as_i64(start.redacted),
                ],
            )?;
        }
        Mutation::Finish(finish) => {
            connection.execute(
                "UPDATE commands SET
                    finished_at_ms = ?2,
                    duration_ms = ?3,
                    exit_code = ?4,
                    status = ?5,
                    output_excerpt = ?6,
                    redacted = CASE WHEN ?7 = 1 THEN 1 ELSE redacted END
                 WHERE id = ?1",
                params![
                    finish.id,
                    finish.finished_at_ms,
                    finish.duration_ms,
                    finish.exit_code,
                    finish.status.as_db(),
                    finish.output_excerpt,
                    bool_as_i64(finish.redacted),
                ],
            )?;
        }
        Mutation::SetStarred { id, starred } => {
            connection.execute(
                "UPDATE commands SET starred = ?2 WHERE id = ?1",
                params![id, bool_as_i64(*starred)],
            )?;
        }
        Mutation::DeleteSession { session_id } => {
            connection.execute(
                "DELETE FROM commands WHERE session_id = ?1",
                params![session_id],
            )?;
        }
        Mutation::Clear { include_starred } => {
            if *include_starred {
                connection.execute("DELETE FROM commands", [])?;
            } else {
                connection.execute("DELETE FROM commands WHERE starred = 0", [])?;
            }
        }
    }
    Ok(())
}

fn append_path_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn acquire_process_lock(database_path: &Path) -> HistoryResult<File> {
    let lock_path = append_path_suffix(database_path, ".lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    set_private_file_permissions(&lock_path)?;

    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        // SAFETY: `lock` owns a valid descriptor for the lifetime of the
        // returned File. `flock` changes only the descriptor's advisory lock.
        let result = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::AlreadyExists
            ) {
                return Err(HistoryError::AlreadyOpen);
            }
            return Err(HistoryError::Io(error));
        }
    }

    Ok(lock)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatabaseFormat {
    Missing,
    Empty,
    PlaintextSqlite,
    EncryptedOrUnknown,
}

fn database_format(path: &Path) -> HistoryResult<DatabaseFormat> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DatabaseFormat::Missing)
        }
        Err(error) => return Err(HistoryError::Io(error)),
    };
    if metadata.len() == 0 {
        return Ok(DatabaseFormat::Empty);
    }

    let mut file = File::open(path)?;
    let mut header = [0_u8; SQLITE_PLAINTEXT_HEADER.len()];
    let read = file.read(&mut header)?;
    if read == SQLITE_PLAINTEXT_HEADER.len() && &header == SQLITE_PLAINTEXT_HEADER {
        Ok(DatabaseFormat::PlaintextSqlite)
    } else {
        Ok(DatabaseFormat::EncryptedOrUnknown)
    }
}

fn sqlite_error(code: i32, context: &'static str) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(context.to_owned()))
}

fn apply_sqlcipher_key(connection: &Connection, key: &HistoryEncryptionKey) -> HistoryResult<()> {
    // SAFETY: the connection is freshly opened and remains alive for the call;
    // the key points to exactly 32 initialized bytes. SQLCipher copies/derives
    // its own key material synchronously.
    let result = unsafe {
        rusqlite::ffi::sqlite3_key(
            connection.handle(),
            key.expose().as_ptr().cast(),
            HISTORY_KEY_BYTES as i32,
        )
    };
    if result == rusqlite::ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(HistoryError::Sqlite(sqlite_error(
            result,
            "SQLCipher rejected the history key",
        )))
    }
}

fn apply_sqlcipher_key_to_attached(
    connection: &Connection,
    database_name: &CString,
    key: &HistoryEncryptionKey,
) -> HistoryResult<()> {
    // SAFETY: `database_name` is NUL-terminated and identifies the freshly
    // attached database. The connection and key stay alive for the call.
    let result = unsafe {
        rusqlite::ffi::sqlite3_key_v2(
            connection.handle(),
            database_name.as_ptr(),
            key.expose().as_ptr().cast(),
            HISTORY_KEY_BYTES as i32,
        )
    };
    if result == rusqlite::ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(HistoryError::Sqlite(sqlite_error(
            result,
            "SQLCipher rejected the migration key",
        )))
    }
}

fn configure_cipher_connection(connection: &Connection) -> HistoryResult<()> {
    // These are the first SQL statements after sqlite3_key. A schema read then
    // authenticates the first encrypted page, so a wrong or missing key fails
    // before migrations, WAL setup, or any write can occur.
    connection.pragma_update(None, "cipher_plaintext_header_size", 0_i64)?;
    connection.pragma_update(None, "cipher_memory_security", "ON")?;
    let cipher_version: String = connection
        .pragma_query_value(None, "cipher_version", |row| row.get(0))
        .map_err(HistoryError::from)?;
    if cipher_version.trim().is_empty() {
        return Err(HistoryError::Security(
            "the bundled database library does not provide SQLCipher".to_owned(),
        ));
    }
    #[cfg(target_os = "macos")]
    {
        let cipher_provider: String = connection
            .pragma_query_value(None, "cipher_provider", |row| row.get(0))
            .map_err(HistoryError::from)?;
        if cipher_provider != "commoncrypto" {
            return Err(HistoryError::Security(format!(
                "the macOS history database is using an unexpected SQLCipher provider: {cipher_provider}"
            )));
        }
    }
    let _: i64 =
        connection.query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))?;
    Ok(())
}

fn open_encrypted_connection(
    path: &Path,
    flags: OpenFlags,
    key: &HistoryEncryptionKey,
) -> HistoryResult<Connection> {
    let connection = Connection::open_with_flags(path, flags)?;
    apply_sqlcipher_key(&connection, key)?;
    configure_cipher_connection(&connection)?;
    Ok(connection)
}

fn recover_interrupted_plaintext_migration(database_path: &Path) -> HistoryResult<()> {
    let staging_path = append_path_suffix(database_path, MIGRATION_STAGING_SUFFIX);
    if !staging_path.exists() {
        return Ok(());
    }

    match database_format(database_path)? {
        DatabaseFormat::Missing | DatabaseFormat::Empty => {
            if database_path.exists() {
                fs::remove_file(database_path)?;
            }
            fs::rename(&staging_path, database_path)?;
            sync_parent_directory(database_path)?;
            Ok(())
        }
        DatabaseFormat::PlaintextSqlite => Err(HistoryError::Security(
            "two plaintext history migration sources exist; refusing to overwrite either"
                .to_owned(),
        )),
        DatabaseFormat::EncryptedOrUnknown => Ok(()),
    }
}

/// Convert a database written by pre-encryption builds. SQLCipher's supported
/// `sqlcipher_export()` path copies tables, FTS virtual tables, triggers, and
/// indexes into a newly keyed database. The original remains in place until
/// the encrypted copy has passed `quick_check` and can be atomically swapped.
fn migrate_plaintext_database(
    database_path: &Path,
    key: &HistoryEncryptionKey,
) -> HistoryResult<()> {
    if database_format(database_path)? != DatabaseFormat::PlaintextSqlite {
        return Ok(());
    }

    let encrypted_path = append_path_suffix(database_path, MIGRATION_ENCRYPTED_SUFFIX);
    remove_database_files(&encrypted_path)?;

    {
        let source = Connection::open_with_flags(
            database_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        source.busy_timeout(Duration::from_secs(5))?;
        let check: String = source.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;
        if check != "ok" {
            return Err(HistoryError::Security(format!(
                "plaintext history migration aborted because quick_check reported: {check}"
            )));
        }

        // Fold any plaintext WAL into the source before export. Refuse to
        // proceed if another process holds a lock rather than risking data loss.
        let _: (i64, i64, i64) =
            source.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
        let journal_mode: String =
            source.query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))?;
        if !journal_mode.eq_ignore_ascii_case("delete") {
            return Err(HistoryError::Security(
                "could not checkpoint plaintext history before encryption".to_owned(),
            ));
        }

        let encrypted_path_text = encrypted_path.to_string_lossy().into_owned();
        source
            .execute(
                "ATTACH DATABASE ?1 AS encrypted",
                params![encrypted_path_text],
            )
            .map_err(|error| {
                HistoryError::Security(format!(
                    "could not attach encrypted history migration target: {error}"
                ))
            })?;
        let encrypted_name = CString::new("encrypted").expect("static database name has no NUL");
        apply_sqlcipher_key_to_attached(&source, &encrypted_name, key)?;
        source
            .execute_batch(
                "PRAGMA encrypted.cipher_plaintext_header_size = 0;
                 PRAGMA encrypted.cipher_memory_security = ON;",
            )
            .map_err(|error| {
                HistoryError::Security(format!(
                    "could not configure encrypted history migration target: {error}"
                ))
            })?;

        let _: Option<String> = source
            .query_row("SELECT sqlcipher_export('encrypted')", [], |row| row.get(0))
            .map_err(|error| {
                HistoryError::Security(format!("could not export encrypted history: {error}"))
            })?;
        let user_version: i32 =
            source.pragma_query_value(None, "user_version", |row| row.get(0))?;
        source.pragma_update(Some("encrypted"), "user_version", user_version)?;
        source.execute_batch("DETACH DATABASE encrypted;")?;
    }

    set_private_database_permissions(&encrypted_path)?;
    {
        let encrypted = open_encrypted_connection(
            &encrypted_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            key,
        )?;
        let check: String = encrypted.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;
        if check != "ok" {
            return Err(HistoryError::Security(format!(
                "encrypted history migration failed quick_check: {check}"
            )));
        }
    }
    File::open(&encrypted_path)?.sync_all()?;

    let staging_path = append_path_suffix(database_path, MIGRATION_STAGING_SUFFIX);
    if staging_path.exists() {
        return Err(HistoryError::Security(
            "a plaintext history migration is already pending".to_owned(),
        ));
    }
    fs::rename(database_path, &staging_path)?;
    if let Err(error) = fs::rename(&encrypted_path, database_path) {
        let _ = fs::rename(&staging_path, database_path);
        return Err(HistoryError::Io(error));
    }
    sync_parent_directory(database_path)?;
    Ok(())
}

fn remove_plaintext_migration_artifacts(database_path: &Path) -> HistoryResult<()> {
    remove_database_files(&append_path_suffix(database_path, MIGRATION_STAGING_SUFFIX))?;
    remove_database_files(&append_path_suffix(
        database_path,
        MIGRATION_ENCRYPTED_SUFFIX,
    ))?;
    Ok(())
}

fn remove_database_files(path: &Path) -> HistoryResult<()> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let candidate = append_path_suffix(path, suffix);
        match fs::remove_file(candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(HistoryError::Io(error)),
        }
    }
    Ok(())
}

fn sync_parent_directory(path: &Path) -> HistoryResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn keychain_lookup_options() -> PasswordOptions {
    let mut options =
        PasswordOptions::new_generic_password(HISTORY_KEYCHAIN_SERVICE, HISTORY_KEYCHAIN_ACCOUNT);
    options.set_access_synchronized(Some(false));
    options.use_protected_keychain();
    options
}

#[cfg(target_os = "macos")]
fn decode_keychain_key(mut bytes: Vec<u8>) -> HistoryResult<HistoryEncryptionKey> {
    if bytes.len() != HISTORY_KEY_BYTES {
        bytes.fill(0);
        return Err(HistoryError::Security(
            "the history Keychain item has an invalid length".to_owned(),
        ));
    }
    let mut key = [0_u8; HISTORY_KEY_BYTES];
    key.copy_from_slice(&bytes);
    bytes.fill(0);
    Ok(HistoryEncryptionKey::from_bytes(key))
}

#[cfg(target_os = "macos")]
fn generate_and_store_keychain_key() -> HistoryResult<HistoryEncryptionKey> {
    let mut key = HistoryEncryptionKey::from_bytes([0_u8; HISTORY_KEY_BYTES]);
    SecRandom::default()
        .copy_bytes(&mut key.0)
        .map_err(|error| {
            HistoryError::Security(format!("secure random generation failed: {error}"))
        })?;

    let access_control = SecAccessControl::create_with_protection(
        Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
        0,
    )
    .map_err(|error| HistoryError::Security(format!("Keychain access control failed: {error}")))?;
    let mut options = keychain_lookup_options();
    options.set_access_control(access_control);
    options.set_label("pH7Console encrypted history key");
    options.set_description("Encrypts local command history; never synchronized");
    set_generic_password_options(key.expose(), options).map_err(|error| {
        HistoryError::Security(format!("saving the history key failed: {error}"))
    })?;

    // Re-read the item rather than trusting the generated buffer, so a future
    // change to Keychain update semantics cannot desynchronize key and DB.
    drop(key);
    let stored = generic_password(keychain_lookup_options()).map_err(|error| {
        HistoryError::Security(format!("reading the saved history key failed: {error}"))
    })?;
    decode_keychain_key(stored)
}

#[cfg(target_os = "macos")]
fn load_or_create_keychain_key(database_path: &Path) -> HistoryResult<HistoryEncryptionKey> {
    let format = database_format(database_path)?;
    match generic_password(keychain_lookup_options()) {
        Ok(bytes) => {
            // A missing/empty DB represents a new install or an explicit data
            // reset. Rotate the old Keychain item instead of silently reusing a
            // key that can outlive an app uninstall.
            if matches!(format, DatabaseFormat::Missing | DatabaseFormat::Empty) {
                let mut bytes = bytes;
                bytes.fill(0);
                generate_and_store_keychain_key()
            } else {
                decode_keychain_key(bytes)
            }
        }
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => {
            if format == DatabaseFormat::EncryptedOrUnknown {
                Err(HistoryError::Security(
                    "the encrypted history key is missing; refusing to overwrite the database"
                        .to_owned(),
                ))
            } else {
                generate_and_store_keychain_key()
            }
        }
        Err(error) => Err(HistoryError::Security(format!(
            "reading the history key from Keychain failed: {error}"
        ))),
    }
}

fn prepare_database_directory(path: &Path) -> HistoryResult<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    let existed = parent.exists();
    fs::create_dir_all(parent)?;
    if !existed {
        set_private_directory_permissions(parent)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> HistoryResult<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> HistoryResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_database_permissions(path: &Path) -> HistoryResult<()> {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = append_path_suffix(path, suffix);
        if candidate.exists() {
            set_private_file_permissions(&candidate)?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_private_database_permissions(_path: &Path) -> HistoryResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> HistoryResult<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> HistoryResult<()> {
    Ok(())
}

fn configure_writer_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    connection.pragma_update(None, "secure_delete", "ON")?;
    let _: String = connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    let _: i64 = connection.query_row("PRAGMA wal_autocheckpoint = 1000", [], |row| row.get(0))?;
    let _: i64 =
        connection.query_row("PRAGMA journal_size_limit = 16777216", [], |row| row.get(0))?;
    Ok(())
}

fn configure_read_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(Duration::from_secs(2))?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    connection.pragma_update(None, "query_only", "ON")?;
    Ok(())
}

fn migrate(connection: &mut Connection) -> HistoryResult<()> {
    let mut version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(HistoryError::UnsupportedSchema(version));
    }

    if version < 1 {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(MIGRATION_1)?;
        transaction.commit()?;
        version = 1;
    }

    if version < 2 {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(MIGRATION_2)?;
        transaction.commit()?;
    }

    Ok(())
}

fn recover_interrupted_commands(connection: &Connection, now_ms: i64) -> rusqlite::Result<usize> {
    connection.execute(
        "UPDATE commands SET
            status = 2,
            finished_at_ms = COALESCE(finished_at_ms, ?1),
            duration_ms = COALESCE(duration_ms, MAX(0, ?1 - started_at_ms))
         WHERE status = 0",
        params![now_ms],
    )
}

fn apply_retention(
    connection: &mut Connection,
    policy: RetentionPolicy,
    now_ms: i64,
) -> rusqlite::Result<usize> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut deleted = 0usize;
    let preserve_starred = bool_as_i64(policy.preserve_starred);

    if let Some(max_age_days) = policy.max_age_days {
        let max_age_ms = i64::from(max_age_days).saturating_mul(86_400_000);
        let cutoff = now_ms.saturating_sub(max_age_ms);
        deleted += transaction.execute(
            "DELETE FROM commands
             WHERE status <> 0 AND started_at_ms < ?1
               AND (?2 = 0 OR starred = 0)",
            params![cutoff, preserve_starred],
        )?;
    }

    if let Some(max_records) = policy.max_records {
        let offset = max_records.min(i64::MAX as u64) as i64;
        deleted += transaction.execute(
            "DELETE FROM commands WHERE rowid IN (
                SELECT rowid FROM commands
                WHERE status <> 0 AND (?1 = 0 OR starred = 0)
                ORDER BY started_at_ms DESC, rowid DESC
                LIMIT -1 OFFSET ?2
             )",
            params![preserve_starred, offset],
        )?;
    }

    transaction.commit()?;
    Ok(deleted)
}

fn validate_identifier(value: &str, name: &'static str) -> HistoryResult<()> {
    if value.trim().is_empty() {
        return Err(HistoryError::InvalidInput(name));
    }
    if value.len() > MAX_IDENTIFIER_BYTES {
        return Err(HistoryError::InvalidInput(name));
    }
    Ok(())
}

fn bool_as_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn unix_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn truncate_utf8(input: &str, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input.to_owned();
    }
    if max_bytes == 0 {
        return String::new();
    }

    let mut boundary = max_bytes.min(input.len());
    while boundary > 0 && !input.is_char_boundary(boundary) {
        boundary -= 1;
    }
    input[..boundary].to_owned()
}

fn escape_like(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' | '%' | '_' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn build_fts_query(input: &str) -> Option<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in input.chars() {
        if character.is_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if tokens.is_empty() {
        return None;
    }

    Some(
        tokens
            .into_iter()
            .map(|token| format!("\"{token}\"*"))
            .collect::<Vec<_>>()
            .join(" AND "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDatabase {
        path: PathBuf,
    }

    impl TestDatabase {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("ph7-history-{name}-{}.sqlite3", Uuid::new_v4()));
            Self { path }
        }

        fn config(&self) -> HistoryConfig {
            let mut config = HistoryConfig::new(&self.path);
            config.retention = RetentionPolicy::unlimited();
            config.prune_every_writes = usize::MAX;
            config.store_output_excerpts = true;
            config
        }

        fn key(&self) -> HistoryEncryptionKey {
            HistoryEncryptionKey::from_bytes([0x42; HISTORY_KEY_BYTES])
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            for suffix in [
                "",
                "-wal",
                "-shm",
                ".lock",
                MIGRATION_STAGING_SUFFIX,
                MIGRATION_ENCRYPTED_SUFFIX,
            ] {
                let _ = fs::remove_file(format!("{}{}", self.path.display(), suffix));
            }
        }
    }

    #[test]
    fn bundles_the_reviewed_sqlcipher_security_baseline() {
        let database = TestDatabase::new("cipher-version");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");
        let connection = store.read_connection().expect("lock read connection");
        let cipher_version: String = connection
            .pragma_query_value(None, "cipher_version", |row| row.get(0))
            .expect("read SQLCipher version");
        let sqlite_version: String = connection
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .expect("read SQLite version");

        assert!(
            cipher_version.starts_with("4.14.0"),
            "unexpected SQLCipher security baseline: {cipher_version}"
        );
        assert_eq!(sqlite_version, "3.51.3");
        #[cfg(target_os = "macos")]
        {
            let cipher_provider: String = connection
                .pragma_query_value(None, "cipher_provider", |row| row.get(0))
                .expect("read SQLCipher provider");
            assert_eq!(cipher_provider, "commoncrypto");
        }
    }

    #[test]
    fn redacts_likely_secrets_before_persistence() {
        let database = TestDatabase::new("redaction");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");
        // Assemble the synthetic credential at runtime so public-repository
        // scanners never mistake the fixture for live key material.
        let secret = ["sk", "proj", "1234567890abcdefghijklmnop"].join("-");
        let start = CommandStart::new(
            "session-a",
            format!("curl --api-key {secret} https://user:hunter2@example.com"),
            "/private/workspace",
        );
        let id = start.id.clone();
        store.record_start(start).expect("record start");
        store
            .record_finish(
                CommandFinish::completed(&id, Some(0), 20).with_output_excerpt(
                    "Authorization: Bearer eyJabcdefghijk.abcdefghijk.abcdefghijk",
                ),
            )
            .expect("record finish");
        store.flush().expect("flush history");

        let record = store.get(&id).expect("read record").expect("record exists");
        assert!(record.redacted);
        assert!(!record.command.contains(secret.as_str()));
        assert!(!record.command.contains("hunter2"));
        assert!(record.command.contains("[REDACTED]"));
        assert!(!record
            .output_excerpt
            .as_deref()
            .unwrap_or_default()
            .contains("eyJabcdefghijk"));

        let connection = Connection::open(&database.path).expect("open raw database");
        assert!(connection
            .query_row("SELECT count(*) FROM commands", [], |row| row
                .get::<_, i64>(0))
            .is_err());
    }

    #[test]
    fn supports_per_session_prefix_and_full_text_search() {
        let database = TestDatabase::new("search");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");

        for (session, command) in [
            ("session-a", "git status --short"),
            ("session-a", "cargo test terminal_reconnect"),
            ("session-b", "git status --branch"),
        ] {
            let start = CommandStart::new(session, command, "/workspace/ph7");
            let id = start.id.clone();
            store.record_start(start).expect("record start");
            store
                .record_finish(CommandFinish::completed(id, Some(0), 10))
                .expect("record finish");
        }
        store.flush().expect("flush history");

        let prefix = store
            .search(&HistorySearch::prefix("git sta").for_session("session-a"))
            .expect("prefix search");
        assert_eq!(prefix.len(), 1);
        assert_eq!(prefix[0].command, "git status --short");

        let full_text = store
            .search(&HistorySearch::full_text("terminal reconnect"))
            .expect("full-text search");
        assert_eq!(full_text.len(), 1);
        assert_eq!(full_text[0].session_id, "session-a");
    }

    #[test]
    fn completion_updates_the_start_record() {
        let database = TestDatabase::new("lifecycle");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");
        let start = CommandStart::new("session-a", "false", "/workspace");
        let id = start.id.clone();
        store.record_start(start).expect("record start");
        store
            .record_finish(CommandFinish::completed(&id, Some(1), 42))
            .expect("record finish");
        store.flush().expect("flush history");

        let record = store.get(&id).expect("get record").expect("record exists");
        assert_eq!(record.status, CommandStatus::Completed);
        assert_eq!(record.exit_code, Some(1));
        assert_eq!(record.duration_ms, Some(42));
        assert!(record.finished_at_ms.is_some());
    }

    #[test]
    fn transactional_migration_preserves_rows_and_rebuilds_search() {
        let database = TestDatabase::new("migration");
        {
            let connection = Connection::open(&database.path).expect("open legacy database");
            connection
                .execute_batch(MIGRATION_1)
                .expect("create version-one schema");
            connection
                .execute(
                    "INSERT INTO commands (
                        id, session_id, command, cwd, started_at_ms, status
                     ) VALUES ('legacy-id', 'legacy-session', 'git legacy branch', '/old', 1, 1)",
                    [],
                )
                .expect("insert legacy row");
        }

        let store = HistoryStore::open(database.config(), database.key()).expect("migrate history");
        assert_eq!(
            store.schema_version().expect("schema version"),
            SCHEMA_VERSION
        );
        store.quick_check().expect("database consistency");
        let matches = store
            .search(&HistorySearch::full_text("legacy branch"))
            .expect("search migrated content");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "legacy-id");
        let encrypted_bytes = fs::read(&database.path).expect("read encrypted migration result");
        assert!(!encrypted_bytes
            .windows(b"git legacy branch".len())
            .any(|window| window == b"git legacy branch"));
        assert!(!append_path_suffix(&database.path, MIGRATION_STAGING_SUFFIX).exists());
    }

    #[test]
    fn unfinished_records_are_recovered_after_restart() {
        let database = TestDatabase::new("recovery");
        let id;
        {
            let store =
                HistoryStore::open(database.config(), database.key()).expect("open history");
            let start = CommandStart::new("session-a", "long-running-task", "/workspace");
            id = start.id.clone();
            store.record_start(start).expect("record start");
            store.flush().expect("flush start");
        }

        let reopened =
            HistoryStore::open(database.config(), database.key()).expect("reopen history");
        let record = reopened
            .get(&id)
            .expect("get recovered record")
            .expect("recovered record exists");
        assert_eq!(record.status, CommandStatus::Interrupted);
        assert!(record.finished_at_ms.is_some());
        reopened.quick_check().expect("database consistency");
    }

    #[test]
    fn retention_preserves_starred_records() {
        let database = TestDatabase::new("retention");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");
        let now = unix_timestamp_ms();
        let mut ids = Vec::new();

        for index in 0..3 {
            let mut start =
                CommandStart::new("session-a", format!("echo history-{index}"), "/workspace");
            start.started_at_ms = now - i64::from(3 - index) * 1_000;
            ids.push(start.id.clone());
            store.record_start(start).expect("record start");
            store
                .record_finish(CommandFinish::completed(&ids[index as usize], Some(0), 1))
                .expect("record finish");
        }
        store.set_starred(&ids[0], true).expect("star oldest");
        store.flush().expect("flush history");

        let removed = store
            .enforce_retention(RetentionPolicy {
                max_records: Some(1),
                max_age_days: None,
                preserve_starred: true,
            })
            .expect("prune history");
        assert_eq!(removed, 1);
        assert!(store.get(&ids[0]).expect("read star").is_some());
        assert!(store.get(&ids[2]).expect("read newest").is_some());
    }

    #[test]
    fn output_is_not_persisted_unless_explicitly_enabled() {
        let database = TestDatabase::new("no-output");
        let mut config = database.config();
        config.store_output_excerpts = false;
        let store = HistoryStore::open(config, database.key()).expect("open history");
        let start = CommandStart::new("session-a", "echo ok", "/workspace");
        let id = start.id.clone();
        store.record_start(start).expect("record start");
        store
            .record_finish(
                CommandFinish::completed(&id, Some(0), 1).with_output_excerpt("private output"),
            )
            .expect("record finish");
        store.flush().expect("flush history");

        let record = store.get(&id).expect("get record").expect("record exists");
        assert_eq!(record.output_excerpt, None);
    }

    #[test]
    fn database_fts_and_wal_never_expose_command_or_cwd_plaintext() {
        let database = TestDatabase::new("encrypted-pages");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");
        let command = "printf ph7_sqlcipher_command_sentinel_918273645";
        let cwd = "/private/ph7_sqlcipher_cwd_sentinel_564738291";
        let start = CommandStart::new("session-encrypted", command, cwd);
        let id = start.id.clone();
        store.record_start(start).expect("record start");
        store
            .record_finish(CommandFinish::completed(&id, Some(0), 3))
            .expect("record finish");
        store.flush().expect("flush encrypted history");

        let matches = store
            .search(&HistorySearch::full_text("sqlcipher command sentinel"))
            .expect("search encrypted FTS");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].cwd, cwd);

        for suffix in ["", "-wal", "-shm"] {
            let path = append_path_suffix(&database.path, suffix);
            if !path.exists() {
                continue;
            }
            let bytes = fs::read(&path).expect("read encrypted database component");
            for sentinel in [command.as_bytes(), cwd.as_bytes()] {
                assert!(
                    !bytes
                        .windows(sentinel.len())
                        .any(|window| window == sentinel),
                    "plaintext leaked into {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn wrong_key_open_fails_without_modifying_existing_history() {
        let database = TestDatabase::new("wrong-key");
        {
            let store =
                HistoryStore::open(database.config(), database.key()).expect("open history");
            let start = CommandStart::new("session-a", "echo encrypted", "/workspace/private");
            store.record_start(start).expect("record start");
            store.flush().expect("flush history");
        }
        let before = fs::read(&database.path).expect("read encrypted history before wrong key");

        let error = HistoryStore::open(
            database.config(),
            HistoryEncryptionKey::from_bytes([0x24; HISTORY_KEY_BYTES]),
        )
        .expect_err("wrong key must fail");
        assert!(matches!(error, HistoryError::Sqlite(_)));
        let after = fs::read(&database.path).expect("read encrypted history after wrong key");
        assert_eq!(before, after);
    }

    #[test]
    fn second_process_fails_closed_while_encrypted_store_is_open() {
        let database = TestDatabase::new("process-lock");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");
        let second = HistoryStore::open(database.config(), database.key())
            .expect_err("second process must not race key or migration state");
        assert!(matches!(second, HistoryError::AlreadyOpen));

        let start = CommandStart::new("session-a", "echo still-available", "/workspace");
        let id = start.id.clone();
        store
            .record_start(start)
            .expect("first store remains usable");
        store.flush().expect("flush first store");
        assert!(store.get(&id).expect("read first store").is_some());
    }

    #[test]
    fn clear_removes_running_completed_starred_and_fts_entries() {
        let database = TestDatabase::new("clear");
        let store = HistoryStore::open(database.config(), database.key()).expect("open history");
        let completed = CommandStart::new("session-a", "echo erase-me-unique", "/workspace");
        let completed_id = completed.id.clone();
        store
            .record_start(completed)
            .expect("record completed start");
        store
            .record_finish(CommandFinish::completed(&completed_id, Some(0), 1))
            .expect("record finish");
        store
            .set_starred(&completed_id, true)
            .expect("star completed record");
        store
            .record_start(CommandStart::new(
                "session-a",
                "sleep erase-running-unique",
                "/workspace",
            ))
            .expect("record running start");
        store.flush().expect("flush history");

        store.clear(true).expect("enqueue clear");
        store.flush().expect("flush clear");
        assert!(store.recent(None, 10).expect("recent history").is_empty());
        assert!(store
            .search(&HistorySearch::full_text("erase unique"))
            .expect("search cleared FTS")
            .is_empty());
    }
}
