use crate::ai::AIResponse;
use crate::history::{redact_sensitive, CommandFinish, CommandRecord, CommandStart, HistorySearch};
use crate::terminal::{CommandExecution, TerminalSession, TerminalSnapshot};
use crate::voice::{self, VoiceEvent};
use crate::{ai, AppState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::{Channel, Response};
use tauri::State;

#[tauri::command]
pub fn get_voice_input_status(locale: Option<String>) -> Result<VoiceEvent, String> {
    voice::status(locale)
}

#[tauri::command]
pub fn request_voice_input_access() -> Result<(), String> {
    voice::request_access();
    Ok(())
}

#[tauri::command]
pub fn start_voice_input(locale: Option<String>) -> Result<(), String> {
    voice::start(locale)
}

#[tauri::command]
pub fn stop_voice_input() -> Result<(), String> {
    voice::stop();
    Ok(())
}

#[tauri::command]
pub async fn create_terminal(
    state: State<'_, AppState>,
    title: Option<String>,
    working_directory: Option<String>,
) -> Result<String, String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;

    terminal_manager
        .create_session_at(title, working_directory.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restart_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<TerminalSession, String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    let replacement_id = terminal_manager
        .restart_session(&session_id)
        .map_err(|error| error.to_string())?;
    terminal_manager
        .get_session(&replacement_id)
        .cloned()
        .ok_or_else(|| "Replacement terminal session was not registered".to_string())
}

#[tauri::command]
pub async fn execute_command(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<CommandExecution, String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("Command cannot be empty".to_string());
    }

    // This compatibility command now writes the user's text literally to the
    // persistent PTY. Natural-language conversion is a separate preview-only
    // action; unfamiliar CLIs are never silently replaced or auto-executed.
    let started = std::time::Instant::now();
    let terminal_manager = state.inner().terminal_manager.lock().await;
    let working_directory = terminal_manager
        .get_session(&session_id)
        .ok_or_else(|| "Terminal session not found".to_string())?
        .working_directory
        .clone();
    terminal_manager.write_to_terminal(&session_id, &format!("{command}\r"))?;
    Ok(CommandExecution {
        id: uuid::Uuid::new_v4().to_string(),
        session_id,
        command: command.to_string(),
        output: String::new(),
        exit_code: None,
        duration_ms: started.elapsed().as_millis() as u64,
        timestamp: chrono::Utc::now(),
        working_directory,
    })
}

/// Detect if a command is natural language vs a regular shell command
fn is_natural_language_command(command: &str) -> bool {
    let cmd_lower = command.to_lowercase().trim().to_string();

    // Check for obvious shell commands first (including single-word commands)
    if cmd_lower.starts_with("ls")
        || cmd_lower.starts_with("cd ")
        || cmd_lower.starts_with("pwd")
        || cmd_lower.starts_with("git ")
        || cmd_lower.starts_with("npm ")
        || cmd_lower.starts_with("cargo ")
        || cmd_lower.starts_with("mkdir ")
        || cmd_lower.starts_with("touch ")
        || cmd_lower.starts_with("rm ")
        || cmd_lower.starts_with("cp ")
        || cmd_lower.starts_with("mv ")
        || cmd_lower.starts_with("find ")
        || cmd_lower.starts_with("grep ")
        || cmd_lower.starts_with("cat ")
        || cmd_lower.starts_with("echo ")
        || cmd_lower.starts_with("sudo ")
        || cmd_lower.starts_with("./")
        || cmd_lower.starts_with("../")
        || cmd_lower.starts_with("man ")
        || cmd_lower.starts_with("which ")
        || cmd_lower.starts_with("ps ")
        || cmd_lower.starts_with("top")
        || cmd_lower.starts_with("htop")
        || cmd_lower.starts_with("df ")
        || cmd_lower.starts_with("open ")
        || cmd_lower == "open"
        || cmd_lower.starts_with("vim ")
        || cmd_lower.starts_with("nano ")
        || cmd_lower.starts_with("emacs ")
        || cmd_lower.starts_with("code ")
        || cmd_lower.starts_with("ssh ")
        || cmd_lower.starts_with("scp ")
        || cmd_lower.starts_with("curl ")
        || cmd_lower.starts_with("wget ")
        || cmd_lower.starts_with("brew ")
        || cmd_lower.starts_with("pip ")
        || cmd_lower.starts_with("python ")
        || cmd_lower.starts_with("node ")
        || cmd_lower.starts_with("java ")
        || cmd_lower.starts_with("rustc ")
        || cmd_lower.starts_with("gcc ")
        || cmd_lower.starts_with("clang ")
        || cmd_lower.starts_with("tar ")
        || cmd_lower.starts_with("unzip ")
        || cmd_lower.starts_with("zip ")
        || cmd_lower == "pwd"
        || cmd_lower == "ls"
        || cmd_lower == "clear"
        || cmd_lower == "exit"
        || cmd_lower == "history"
        || cmd_lower == "top"
        || cmd_lower == "htop"
        || cmd_lower == "whoami"
        || cmd_lower.starts_with("/")
        || cmd_lower.starts_with("~")
    {
        return false;
    }

    // Highly specific natural language patterns that we want to catch
    let high_confidence_patterns = [
        "go home",
        "go to home",
        "go home directory",
        "go to home directory",
        "go to parent",
        "go to parent directory",
        "go up",
        "go back",
        "show files",
        "list files",
        "show me files",
        "display files",
        "what files",
        "what's here",
        "what is here",
        "where am i",
        "current directory",
        "present working directory",
        "create file",
        "make file",
        "new file",
        "add file",
        "create folder",
        "make folder",
        "make directory",
        "create directory",
        "git status",
        "check git",
        "git state",
        "repository status",
        "install package",
        "add package",
        "npm install",
        "run project",
        "start project",
        "build project",
    ];

    // Check for exact matches or substring matches of high confidence patterns
    for pattern in &high_confidence_patterns {
        if cmd_lower == *pattern || cmd_lower.contains(pattern) {
            return true;
        }
    }

    // Check for natural language sentence structure patterns
    let natural_patterns = [
        "go to",
        "navigate to",
        "change to",
        "move to",
        "switch to",
        "show me",
        "list",
        "display",
        "what",
        "where",
        "how",
        "create",
        "make",
        "build",
        "install",
        "run",
        "find",
        "search for",
        "look for",
        "locate",
        "home directory",
        "parent directory",
        "current directory",
        "files",
        "folder",
        "directory",
        "status",
        "help",
        "explain",
    ];

    let pattern_matches = natural_patterns
        .iter()
        .filter(|&&pattern| cmd_lower.contains(pattern))
        .count();

    // If it contains natural language patterns
    if pattern_matches > 0 {
        return true;
    }

    // Check for sentence-like structure (contains common English words)
    let english_words = [
        "the", "a", "an", "to", "in", "on", "at", "for", "with", "by", "my", "me", "i",
    ];
    let word_count = english_words
        .iter()
        .filter(|&&word| cmd_lower.contains(word))
        .count();

    // If it contains multiple English words and is longer than typical commands, likely natural language
    if word_count >= 1 && cmd_lower.len() > 10 {
        return true;
    }

    // Additional check: if it doesn't start with a known command and contains spaces, likely natural language
    let words: Vec<&str> = cmd_lower.split_whitespace().collect();
    if words.len() > 1 {
        let first_word = words[0];
        // Comprehensive list of Unix/macOS/Linux commands
        let unix_commands = [
            // Core Unix commands
            "ls",
            "cd",
            "pwd",
            "mkdir",
            "rmdir",
            "rm",
            "cp",
            "mv",
            "ln",
            "find",
            "grep",
            "cat",
            "less",
            "more",
            "head",
            "tail",
            "sort",
            "uniq",
            "wc",
            "chmod",
            "chown",
            "ps",
            "top",
            "kill",
            "jobs",
            "bg",
            "fg",
            "nohup",
            "ssh",
            "scp",
            "rsync",
            "tar",
            "gzip",
            "gunzip",
            "zip",
            "unzip",
            "curl",
            "wget",
            "ping",
            "traceroute",
            "netstat",
            "ifconfig",
            "iptables",
            "sudo",
            "su",
            "whoami",
            "id",
            "groups",
            "history",
            "alias",
            "which",
            "whereis",
            "locate",
            "man",
            "info",
            "help",
            "clear",
            "reset",
            "exit",
            "logout",
            // macOS specific commands
            "open",
            "say",
            "osascript",
            "pbcopy",
            "pbpaste",
            "sw_vers",
            "system_profiler",
            "diskutil",
            "hdiutil",
            "mdls",
            "mdfind",
            "spotlight",
            "launchctl",
            "scutil",
            "networksetup",
            "security",
            "keychain",
            // Development tools
            "git",
            "npm",
            "yarn",
            "pnpm",
            "cargo",
            "python",
            "python3",
            "node",
            "java",
            "javac",
            "rustc",
            "gcc",
            "clang",
            "g++",
            "make",
            "cmake",
            "autoconf",
            "automake",
            "libtool",
            "pkg-config",
            // Package managers
            "brew",
            "pip",
            "pip3",
            "pipx",
            "conda",
            "apt",
            "yum",
            "dnf",
            "pacman",
            "snap",
            "flatpak",
            // Text editors
            "vim",
            "vi",
            "nvim",
            "nano",
            "emacs",
            "code",
            "subl",
            "atom",
            "pico",
            // System monitoring
            "htop",
            "iotop",
            "nettop",
            "activity",
            "fs_usage",
            "dtruss",
            "ktrace",
            "iostat",
            "vmstat",
            // Network tools
            "nc",
            "netcat",
            "telnet",
            "ftp",
            "sftp",
            "rsync",
            "scp",
            "dig",
            "nslookup",
            "host",
            "whois",
            // File operations
            "file",
            "stat",
            "du",
            "df",
            "lsof",
            "fuser",
            "basename",
            "dirname",
            "realpath",
            "readlink",
            // Process control
            "pgrep",
            "pkill",
            "killall",
            "nohup",
            "screen",
            "tmux",
            "at",
            "crontab",
            "watch",
            // Compression
            "compress",
            "uncompress",
            "bzip2",
            "bunzip2",
            "xz",
            "unxz",
            "7z",
            "rar",
            "unrar",
            // Database tools
            "sqlite3",
            "mysql",
            "psql",
            "mongo",
            "redis-cli",
            // Container tools
            "docker",
            "podman",
            "kubectl",
            "helm",
            "docker-compose",
            // Media tools
            "ffmpeg",
            "imagemagick",
            "convert",
            "identify",
            "exiftool",
            // Misc utilities
            "awk",
            "sed",
            "tr",
            "cut",
            "paste",
            "join",
            "tee",
            "xargs",
            "parallel",
            "jq",
            "yq",
            "base64",
            "uuencode",
            "uudecode",
            "hexdump",
            "od",
            "strings",
            "xxd",
        ];

        if !unix_commands.contains(&first_word) && words.len() >= 2 {
            return true;
        }
    }

    false
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPlan {
    pub original_input: String,
    pub command: String,
    pub explanation: String,
    pub confidence: f32,
    pub source: String,
    pub risk_level: CommandRiskLevel,
    pub risk_reasons: Vec<String>,
    pub requires_confirmation: bool,
    pub requires_strong_confirmation: bool,
}

/// Produce a preview-only command plan. This command never writes to a PTY.
#[tauri::command]
pub async fn create_command_plan(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<CommandPlan, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Request cannot be empty".to_string());
    }
    if input.len() > 8 * 1024 {
        return Err("Request exceeds the 8 KiB command-planning limit".to_string());
    }

    let is_natural_language = is_natural_language_command(input);
    let (command, explanation, confidence, source) = if is_natural_language {
        let (context, workspace) = {
            let terminal_manager = state.inner().terminal_manager.lock().await;
            let session = terminal_manager
                .get_session(&session_id)
                .ok_or_else(|| "Terminal session not found".to_string())?;
            (
                terminal_manager.get_smart_context(&session_id),
                PathBuf::from(&session.working_directory),
            )
        };
        let context = enrich_terminal_context(context, workspace).await;

        let model_manager = state.inner().model_manager.lock().await.clone();
        let generation = model_manager
            .generate_command_plan(input, Some(&context))
            .await;
        let command = generation.command.trim().to_string();
        if command.is_empty()
            || command.starts_with('#')
            || command.to_ascii_lowercase().contains("need more")
        {
            return Err("Local intelligence could not produce a safe command preview".to_string());
        }
        (
            command,
            generation.explanation,
            generation.confidence,
            generation.source.as_str().to_string(),
        )
    } else {
        (
            input.to_string(),
            "Literal shell input. No AI translation was applied.".to_string(),
            1.0,
            "literal".to_string(),
        )
    };
    if has_unsafe_terminal_characters(&command) {
        return Err("Command previews must contain printable, single-line text".to_string());
    }

    let (risk_level, risk_reasons) = assess_command_risk(&command);
    let requires_strong_confirmation = matches!(
        risk_level,
        CommandRiskLevel::High | CommandRiskLevel::Critical
    );
    let requires_confirmation = source != "literal" || !matches!(risk_level, CommandRiskLevel::Low);

    Ok(CommandPlan {
        original_input: input.to_string(),
        command,
        explanation,
        confidence,
        source,
        risk_level,
        risk_reasons,
        requires_confirmation,
        requires_strong_confirmation,
    })
}

fn inspect_workspace_context(workspace: &std::path::Path) -> String {
    let mut context = String::new();
    if let Ok(entries) = std::fs::read_dir(workspace) {
        let mut file_types = entries
            .take(20)
            .flatten()
            .filter_map(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(ToOwned::to_owned)
            })
            .collect::<Vec<_>>();
        file_types.sort();
        file_types.dedup();
        if !file_types.is_empty() {
            context.push_str(&format!("File Types: {}\n", file_types.join(", ")));
        }
    }

    let project_indicators = [
        ("package.json", "Node.js"),
        ("Cargo.toml", "Rust"),
        ("pyproject.toml", "Python"),
        ("pom.xml", "Java/Maven"),
        ("build.gradle", "Java/Gradle"),
        (".git", "Git Repository"),
        ("docker-compose.yml", "Docker"),
        ("Dockerfile", "Docker"),
    ];
    for (file, project_type) in project_indicators {
        if workspace.join(file).exists() {
            context.push_str(&format!("Project Type: {project_type}\n"));
        }
    }
    context
}

async fn enrich_terminal_context(mut context: String, workspace: PathBuf) -> String {
    let inspection = tokio::task::spawn_blocking(move || inspect_workspace_context(&workspace));
    if let Ok(Ok(enrichment)) =
        tokio::time::timeout(std::time::Duration::from_millis(750), inspection).await
    {
        context.push_str(&enrichment);
    }
    context
}

fn has_shell_compound_operator(command: &str) -> bool {
    let mut characters = command.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(character) = characters.next() {
        if character == '\\' && !in_single_quote {
            let _ = characters.next();
            continue;
        }
        if character == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if character == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }
        if in_single_quote {
            continue;
        }
        if character == '`' || (character == '$' && characters.peek() == Some(&'(')) {
            return true;
        }
        if !in_double_quote && matches!(character, ';' | '|' | '&') {
            return true;
        }
    }
    false
}

/// Detect output redirection without treating quoted or escaped `>` literals
/// as filesystem writes. This intentionally catches compact forms such as
/// `printf secret>file` and file-descriptor prefixes such as `2>errors.log`.
fn has_shell_output_redirection(command: &str) -> bool {
    let mut characters = command.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(character) = characters.next() {
        if character == '\\' && !in_single_quote {
            let _ = characters.next();
            continue;
        }
        if character == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if character == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }
        if !in_single_quote && !in_double_quote && character == '>' {
            return true;
        }
    }
    false
}

fn assess_command_risk(command: &str) -> (CommandRiskLevel, Vec<String>) {
    let normalized = command
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut reasons = Vec::new();

    let critical_patterns = [
        "rm -rf /",
        "rm -fr /",
        "mkfs",
        "diskutil erase",
        "dd if=",
        "of=/dev/",
        ":(){ :|:& };:",
    ];
    for pattern in critical_patterns {
        if normalized.contains(pattern) {
            reasons.push(format!(
                "Potentially destructive system-wide operation: {pattern}"
            ));
        }
    }
    if !reasons.is_empty() {
        return (CommandRiskLevel::Critical, reasons);
    }

    let high_patterns = [
        ("sudo ", "Requests administrator privileges"),
        ("rm -r", "Recursively deletes files or directories"),
        ("rm -f", "Forces file deletion"),
        (" -delete", "Deletes matched files"),
        ("git reset --hard", "Discards uncommitted changes"),
        ("git clean -f", "Deletes untracked files"),
        ("git push --force", "Rewrites remote history"),
        ("git push -f", "Rewrites remote history"),
        ("terraform destroy", "Destroys managed infrastructure"),
        ("kubectl delete", "Deletes cluster resources"),
        ("npm publish", "Publishes a package externally"),
        ("cargo publish", "Publishes a crate externally"),
        (" | sh", "Executes downloaded or piped shell text"),
        (" | bash", "Executes downloaded or piped shell text"),
        ("chmod -r", "Recursively changes permissions"),
        ("chown -r", "Recursively changes ownership"),
    ];
    for (pattern, reason) in high_patterns {
        if normalized.contains(pattern) {
            reasons.push(reason.to_string());
        }
    }
    let mutating_find_exec = [
        " -exec rm ",
        " -exec mv ",
        " -exec chmod ",
        " -exec chown ",
        " -exec sh ",
        " -exec bash ",
        " -exec zsh ",
        " -exec python ",
        " -exec python3 ",
        " -exec osascript ",
        " -execdir rm ",
        " -execdir sh ",
    ];
    if normalized.starts_with("find ")
        && mutating_find_exec
            .iter()
            .any(|pattern| normalized.contains(pattern))
    {
        reasons.push("Executes a potentially mutating command for matched paths".to_string());
    }
    if has_shell_compound_operator(command) {
        reasons.push("Combines shell operations or performs command substitution".to_string());
    }
    if !reasons.is_empty() {
        return (CommandRiskLevel::High, reasons);
    }

    let medium_patterns = [
        ("git push", "Writes to a remote repository"),
        ("git commit", "Creates repository history"),
        ("curl ", "Performs a network request"),
        ("wget ", "Downloads remote content"),
        ("brew install", "Installs system software"),
        ("npm install", "Installs project dependencies"),
        ("pip install", "Installs Python packages"),
        ("cargo install", "Installs a Rust binary"),
        ("kill ", "Sends a signal to a process"),
        ("pkill ", "Signals processes by name"),
        ("mv ", "Moves or renames files"),
        ("cp ", "Copies files"),
        ("mkdir ", "Creates a directory"),
        ("touch ", "Creates or modifies a file timestamp"),
    ];
    for (pattern, reason) in medium_patterns {
        if normalized.starts_with(pattern) || normalized.contains(pattern) {
            reasons.push(reason.to_string());
        }
    }
    if has_shell_output_redirection(command) {
        reasons.push("Redirects shell output and may create or overwrite a file".to_string());
    }
    if !reasons.is_empty() {
        (CommandRiskLevel::Medium, reasons)
    } else if normalized.starts_with("find ") {
        (
            CommandRiskLevel::Low,
            vec!["Read-only directory search; no mutation flags detected".to_string()],
        )
    } else {
        (
            CommandRiskLevel::Low,
            vec!["No known write or privilege pattern detected".to_string()],
        )
    }
}

#[tauri::command]
pub async fn get_terminal_output(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<CommandExecution>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;

    let history = terminal_manager.get_session_history(&session_id, limit);
    Ok(history.into_iter().cloned().collect())
}

#[tauri::command]
pub async fn ai_suggest_command(
    state: State<'_, AppState>,
    context: String,
    intent: Option<String>,
) -> Result<AIResponse, String> {
    if context.len() > 16 * 1024 || intent.as_ref().is_some_and(|value| value.len() > 2 * 1024) {
        return Err("AI suggestion context exceeds the local processing limit".to_string());
    }
    let model_manager = state.inner().model_manager.lock().await.clone();
    Ok(model_manager
        .suggest_command(intent.as_deref(), &context)
        .await)
}

#[tauri::command]
pub async fn ai_explain_command(
    state: State<'_, AppState>,
    command: String,
) -> Result<AIResponse, String> {
    if command.is_empty() || command.len() > 8 * 1024 {
        return Err("Command exceeds the local explanation limit".to_string());
    }
    let model_manager = state.inner().model_manager.lock().await.clone();
    Ok(model_manager.explain_command(&command).await)
}

#[tauri::command]
pub async fn ai_fix_error(
    state: State<'_, AppState>,
    error_output: String,
    command: String,
    context: Option<String>,
) -> Result<AIResponse, String> {
    if command.is_empty()
        || command.len() > 8 * 1024
        || error_output.len() > 16 * 1024
        || context.as_ref().is_some_and(|value| value.len() > 8 * 1024)
    {
        return Err("Error-analysis input exceeds the local processing limit".to_string());
    }
    let model_manager = state.inner().model_manager.lock().await.clone();
    Ok(model_manager
        .fix_error(&command, &error_output, context.as_deref())
        .await)
}

#[tauri::command]
pub async fn ai_analyze_output(
    state: State<'_, AppState>,
    output: String,
    command: String,
) -> Result<AIResponse, String> {
    if command.is_empty() || command.len() > 8 * 1024 || output.len() > 16 * 1024 {
        return Err("Output-analysis input exceeds the local processing limit".to_string());
    }
    let model_manager = state.inner().model_manager.lock().await.clone();
    Ok(model_manager.analyze_output(&command, &output).await)
}

#[tauri::command]
pub async fn get_smart_completions(
    state: State<'_, AppState>,
    partial_command: String,
    session_id: String,
) -> Result<Vec<String>, String> {
    let context = {
        let terminal_manager = state.inner().terminal_manager.lock().await;
        terminal_manager.get_smart_context(&session_id)
    };
    let model_manager = state.inner().model_manager.lock().await.clone();

    // Get enhanced completions with session context
    let completions = model_manager
        .get_enhanced_completions(&partial_command, &context, &session_id)
        .await;
    Ok(completions)
}

#[tauri::command]
pub async fn ai_translate_natural_language(
    state: State<'_, AppState>,
    natural_language: String,
    context: String,
) -> Result<AIResponse, String> {
    if natural_language.is_empty() || natural_language.len() > 8 * 1024 || context.len() > 16 * 1024
    {
        return Err("Command-planning input exceeds the local processing limit".to_string());
    }
    let model_manager = state.inner().model_manager.lock().await.clone();
    Ok(model_manager
        .process_command_with_ml(&natural_language, Some(&context))
        .await)
}

/// Get user analytics from learning engine
#[tauri::command]
pub async fn get_user_analytics(
    state: State<'_, AppState>,
) -> Result<Option<ai::UserAnalytics>, String> {
    let model_manager = state.inner().model_manager.lock().await.clone();
    Ok(model_manager.get_analytics().await)
}

/// Update feedback for learning
#[tauri::command]
pub async fn update_ai_feedback(
    state: State<'_, AppState>,
    command: String,
    feedback: f32,
) -> Result<(), String> {
    if command.trim().is_empty()
        || command.len() > crate::shell_integration::MAX_RECORDED_COMMAND_BYTES
    {
        return Err("Feedback command exceeds the local learning limit".to_string());
    }
    if !feedback.is_finite() || !(0.0..=1.0).contains(&feedback) {
        return Err("Feedback must be a finite value between 0 and 1".to_string());
    }
    let model_manager = state.inner().model_manager.lock().await.clone();
    model_manager.update_feedback(&command, feedback).await;
    Ok(())
}

/// Close terminal session
#[tauri::command]
pub async fn close_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.close_session(&session_id)
}

/// Update session title
#[tauri::command]
pub async fn update_session_title(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.update_session_title(&session_id, title)
}

/// Resize terminal
#[tauri::command]
pub async fn resize_terminal(
    state: State<'_, AppState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.resize_terminal(&session_id, cols, rows)
}

/// Send keyboard, paste, or control-sequence input to a session's live PTY.
#[tauri::command]
pub async fn write_to_terminal(
    state: State<'_, AppState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.write_to_terminal(&session_id, &data)
}

#[tauri::command]
pub async fn write_bytes_to_terminal(
    state: State<'_, AppState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.write_bytes_to_terminal(&session_id, &data)
}

/// Fetch bounded scrollback so a newly mounted renderer can replay output
/// without losing bytes emitted before its event listener was attached.
#[tauri::command]
pub async fn get_terminal_snapshot(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<TerminalSnapshot, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.get_terminal_snapshot(&session_id)
}

#[tauri::command]
pub async fn attach_terminal_stream(
    state: State<'_, AppState>,
    session_id: String,
    on_event: Channel<Response>,
) -> Result<u64, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.attach_output_channel(&session_id, on_event)
}

#[tauri::command]
pub async fn detach_terminal_stream(
    state: State<'_, AppState>,
    session_id: String,
    subscriber_id: u64,
) -> Result<(), String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.detach_output_channel(&session_id, subscriber_id)
}

#[tauri::command]
pub async fn sync_terminal_working_directory(
    state: State<'_, AppState>,
    session_id: String,
    working_directory: String,
) -> Result<String, String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.sync_working_directory(&session_id, &working_directory)
}

/// Get system information
#[tauri::command]
pub async fn get_system_info(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager.get_system_info())
}

/// Get context-aware command suggestions
#[tauri::command]
pub async fn get_context_suggestions(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<String>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager.get_context_suggestions(&session_id))
}

/// Get all sessions
#[tauri::command]
pub async fn get_all_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::terminal::TerminalSession>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager
        .get_all_sessions()
        .into_iter()
        .cloned()
        .collect())
}

/// Get path completions for Tab autocomplete
#[tauri::command]
pub async fn get_path_completions(
    state: State<'_, AppState>,
    session_id: String,
    partial_path: String,
) -> Result<Vec<String>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager.get_path_completions(&session_id, &partial_path))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPersistenceStatus {
    pub encrypted_persistence: bool,
    pub mode: &'static str,
    pub message: &'static str,
}

/// Report whether history is backed by SQLCipher or the bounded in-memory
/// cache. The error details stay in native logs and never expose Keychain
/// state to web content.
#[tauri::command]
pub fn get_history_persistence_status(state: State<'_, AppState>) -> HistoryPersistenceStatus {
    if state.inner().history_store.is_some() {
        HistoryPersistenceStatus {
            encrypted_persistence: true,
            mode: "encrypted",
            message: "Encrypted on this Mac",
        }
    } else {
        HistoryPersistenceStatus {
            encrypted_persistence: false,
            mode: "memory_only",
            message: "Memory only — cleared when pH7Console closes",
        }
    }
}

/// Remove both the bounded process-local cache and every durable history row.
/// A later completion update for a command cleared while it was running is a
/// harmless no-op.
#[tauri::command]
pub async fn clear_command_history(state: State<'_, AppState>) -> Result<(), String> {
    let history_store = state.inner().history_store.clone();

    // Queue the durable mutation first so commands recorded after the clear
    // request remain newer than it. This enqueue is non-blocking.
    let durable_clear = history_store
        .as_ref()
        .map(|store| store.clear(true))
        .transpose()
        .map_err(|error| error.to_string());

    // Never hold the async terminal lock while waiting for SQLite I/O.
    state
        .inner()
        .terminal_manager
        .lock()
        .await
        .clear_command_history();

    let model_manager = state.inner().model_manager.lock().await.clone();
    model_manager.clear_learning_memory().await;

    durable_clear?;
    let Some(history_store) = history_store else {
        return Ok(());
    };
    tauri::async_runtime::spawn_blocking(move || history_store.flush())
        .await
        .map_err(|error| format!("Encrypted history clear task failed: {error}"))?
        .map_err(|error| error.to_string())
}

/// Get command history for arrow key navigation
#[tauri::command]
pub async fn get_command_history_for_navigation(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<String>, String> {
    if let Some(history_store) = state.inner().history_store.as_ref() {
        match history_store.recent(Some(&session_id), 500) {
            Ok(records) => return Ok(records.into_iter().map(|record| record.command).collect()),
            Err(error) => eprintln!("Persistent command history read failed: {error}"),
        }
    }
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager.get_command_history_for_navigation(&session_id))
}

/// Search command history
#[tauri::command]
pub async fn search_command_history(
    state: State<'_, AppState>,
    pattern: String,
) -> Result<Vec<String>, String> {
    if let Some(history_store) = state.inner().history_store.as_ref() {
        match history_store.search(&HistorySearch::full_text(&pattern).with_limit(50)) {
            Ok(records) => return Ok(records.into_iter().map(|record| record.command).collect()),
            Err(error) => eprintln!("Persistent command history search failed: {error}"),
        }
    }
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager.search_command_history(&pattern))
}

/// Search the complete encrypted history and return records for the History
/// window. SQLite/FTS work runs off the async command executor so a large
/// history can never delay PTY input or output.
#[tauri::command]
pub async fn search_command_history_records(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<CommandExecution>, String> {
    const MAX_HISTORY_QUERY_BYTES: usize = 512;
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    if query.len() > MAX_HISTORY_QUERY_BYTES || has_unsafe_terminal_characters(query) {
        return Err("History search contains unsupported text".to_owned());
    }
    let limit = limit.unwrap_or(100).clamp(1, 500);

    if let Some(history_store) = state.inner().history_store.clone() {
        let query = query.to_owned();
        return tauri::async_runtime::spawn_blocking(move || {
            history_store
                .search(&HistorySearch::full_text(query).with_limit(limit))
                .map(|records| {
                    records
                        .into_iter()
                        .map(history_record_to_execution)
                        .collect()
                })
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| format!("Encrypted history search task failed: {error}"))?;
    }

    let query_lower = query.to_lowercase();
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager
        .get_command_history(None)
        .into_iter()
        .filter(|execution| execution.command.to_lowercase().contains(&query_lower))
        .take(limit)
        .cloned()
        .collect())
}

/// Store a command in history without executing it (for natural language commands)
#[tauri::command]
pub async fn store_command_in_history(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<(), String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager
        .store_command_in_history(&session_id, &command)
        .map_err(|e| e.to_string())
}

fn history_record_to_execution(record: CommandRecord) -> CommandExecution {
    CommandExecution {
        id: record.id,
        session_id: record.session_id,
        command: record.command,
        output: record.output_excerpt.unwrap_or_default(),
        exit_code: record.exit_code,
        duration_ms: record.duration_ms.unwrap_or_default(),
        timestamp: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(record.started_at_ms)
            .unwrap_or_else(chrono::Utc::now),
        working_directory: record.cwd,
    }
}

/// Record the command-completion event emitted by the private shell
/// integration. The PTY parser treats this metadata as inert data and this
/// command redacts it again before either memory or SQLite sees it.
fn has_unsafe_terminal_characters(value: &str) -> bool {
    value.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '\u{061c}'
                    | '\u{200e}'
                    | '\u{200f}'
                    | '\u{2028}'..='\u{202e}'
                    | '\u{2066}'..='\u{2069}'
                    | '\u{feff}'
            )
    })
}

#[tauri::command]
pub async fn record_shell_command(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
    exit_code: i32,
    duration_ms: u64,
) -> Result<CommandExecution, String> {
    if command.len() > crate::shell_integration::MAX_RECORDED_COMMAND_BYTES {
        return Err("Shell command metadata exceeds the safety limit".to_string());
    }
    if has_unsafe_terminal_characters(&command) {
        return Err("Shell command metadata contains unsafe terminal controls".to_string());
    }
    if !(0..=255).contains(&exit_code) {
        return Err("Shell exit status is invalid".to_string());
    }

    let redacted = redact_sensitive(command.trim());
    if redacted.value.is_empty() {
        return Err("Shell command metadata is empty".to_string());
    }
    let duration_ms = duration_ms.min(7 * 24 * 60 * 60 * 1_000);
    let (working_directory, shell) = {
        let terminal_manager = state.inner().terminal_manager.lock().await;
        let session = terminal_manager
            .get_session(&session_id)
            .ok_or_else(|| "Terminal session not found".to_string())?;
        (session.working_directory.clone(), session.shell.clone())
    };

    let id = uuid::Uuid::new_v4().to_string();
    let finished_at_ms = chrono::Utc::now().timestamp_millis();
    let started_at_ms = finished_at_ms.saturating_sub(duration_ms as i64);
    let execution = CommandExecution {
        id: id.clone(),
        session_id: session_id.clone(),
        command: redacted.value.clone(),
        output: String::new(),
        exit_code: Some(exit_code),
        duration_ms,
        timestamp: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(started_at_ms)
            .unwrap_or_else(chrono::Utc::now),
        working_directory: working_directory.clone(),
    };

    if let Some(history_store) = state.inner().history_store.as_ref() {
        let start = CommandStart {
            id: id.clone(),
            session_id: session_id.clone(),
            command: redacted.value.clone(),
            cwd: working_directory.clone(),
            shell: Some(shell.clone()),
            started_at_ms,
        };
        match history_store.record_start(start) {
            Ok(()) => {
                let mut finish = CommandFinish::completed(&id, Some(exit_code), duration_ms);
                finish.finished_at_ms = finished_at_ms;
                if let Err(error) = history_store.record_finish(finish) {
                    eprintln!("Persistent command completion enqueue failed: {error}");
                }
            }
            Err(error) => {
                // A full history queue must never apply backpressure to input
                // or PTY output. The in-memory cache remains available.
                eprintln!("Persistent command history enqueue failed: {error}");
            }
        }
    }

    state
        .inner()
        .terminal_manager
        .lock()
        .await
        .record_command_execution(execution.clone());

    // Learning runs outside the PTY/history path and only receives the
    // already-redacted command plus bounded execution metadata. It never
    // delays the next prompt and never writes command-derived plaintext.
    let model_manager = Arc::clone(&state.inner().model_manager);
    let learning_session_id = session_id;
    let learning_command = redacted.value;
    let learning_context = format!("cwd:{working_directory}\nshell:{shell}");
    tauri::async_runtime::spawn(async move {
        let manager = model_manager.lock().await.clone();
        manager
            .learn_from_completed_command(
                &learning_session_id,
                &learning_command,
                &learning_context,
                exit_code == 0,
                Some(duration_ms),
            )
            .await;
    });
    Ok(execution)
}

#[tauri::command]
pub async fn get_recent_command_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<CommandExecution>, String> {
    let limit = limit.unwrap_or(500).min(500);
    if let Some(history_store) = state.inner().history_store.as_ref() {
        return history_store
            .recent(None, limit)
            .map(|records| {
                // The store returns newest first; the UI keeps chronological
                // order and reverses only at presentation time.
                records
                    .into_iter()
                    .rev()
                    .map(history_record_to_execution)
                    .collect()
            })
            .map_err(|error| error.to_string());
    }

    let terminal_manager = state.inner().terminal_manager.lock().await;
    let mut records = terminal_manager
        .get_command_history(Some(limit))
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    records.reverse();
    Ok(records)
}

/// Validate and clean up frequent directories by removing non-existent ones
#[tauri::command]
pub async fn validate_frequent_directories(
    frequent_dirs: Vec<String>,
    current_working_dir: String,
) -> Result<Vec<String>, String> {
    let mut valid_dirs = Vec::new();

    for dir in frequent_dirs {
        let path = if dir.starts_with('~') {
            // Expand ~ to home directory
            if let Some(home_dir) = dirs::home_dir() {
                dir.replacen("~", home_dir.to_string_lossy().as_ref(), 1)
            } else {
                dir
            }
        } else if !dir.starts_with('/') {
            // Convert relative path to absolute from current working directory
            PathBuf::from(&current_working_dir)
                .join(&dir)
                .to_string_lossy()
                .to_string()
        } else {
            dir
        };

        // Check if directory exists
        if PathBuf::from(&path).is_dir() {
            valid_dirs.push(path);
        }
    }

    Ok(valid_dirs)
}

/// Find the correct path for a given directory name in common locations
#[tauri::command]
pub async fn find_path_in_common_locations(
    target_name: String,
    current_working_dir: String,
) -> Result<Option<String>, String> {
    let search_locations = vec![
        current_working_dir.clone(),
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        "/usr/local".to_string(),
        "/opt".to_string(),
        format!(
            "{}/Desktop",
            dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        ),
        format!(
            "{}/Documents",
            dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        ),
        format!(
            "{}/Downloads",
            dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        ),
    ];

    for location in search_locations {
        let potential_path = PathBuf::from(&location).join(&target_name);
        if potential_path.is_dir() {
            return Ok(Some(potential_path.to_string_lossy().to_string()));
        }

        // Also search one level deep in common directories
        if let Ok(entries) = std::fs::read_dir(&location) {
            for entry in entries.take(50).flatten() {
                // Limit search to prevent performance issues
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let nested_path = entry.path().join(&target_name);
                    if nested_path.is_dir() {
                        return Ok(Some(nested_path.to_string_lossy().to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Validate if a specific path exists and return corrected path
#[tauri::command]
pub async fn validate_and_correct_path(
    path: String,
    current_working_dir: String,
    frequent_directories: Vec<String>,
) -> Result<Option<String>, String> {
    let expanded_path = if path.starts_with('~') {
        if let Some(home_dir) = dirs::home_dir() {
            path.replacen("~", home_dir.to_string_lossy().as_ref(), 1)
        } else {
            path.clone()
        }
    } else if !path.starts_with('/') {
        // Relative path - make it absolute
        PathBuf::from(&current_working_dir)
            .join(&path)
            .to_string_lossy()
            .to_string()
    } else {
        path.clone()
    };

    // Check if the expanded path exists
    if PathBuf::from(&expanded_path).exists() {
        return Ok(Some(expanded_path));
    }

    // If not found, try to find it in frequent directories
    let path_buf = PathBuf::from(&path);
    let path_name = path_buf
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path);

    for freq_dir in frequent_directories {
        let potential_path = PathBuf::from(&freq_dir).join(path_name);
        if potential_path.exists() {
            return Ok(Some(potential_path.to_string_lossy().to_string()));
        }
    }

    // Last resort: search in common locations
    find_path_in_common_locations(path_name.to_string(), current_working_dir).await
}

/// Repository information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoInfo {
    pub is_git_repo: bool,
    pub current_branch: Option<String>,
    pub repo_name: Option<String>,
    pub remote_url: Option<String>,
    pub has_changes: bool,
    pub ahead: i32,
    pub behind: i32,
}

/// Language/runtime information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeInfo {
    pub node_version: Option<String>,
    pub npm_version: Option<String>,
    pub rust_version: Option<String>,
    pub python_version: Option<String>,
    pub git_version: Option<String>,
    pub go_version: Option<String>,
    pub java_version: Option<String>,
    pub project_type: Option<String>, // Detected from project files (package.json, Cargo.toml, etc.)
}

const METADATA_PATH_LIMIT: usize = 16 * 1024;
const METADATA_OUTPUT_LIMIT: usize = 16 * 1024;
const METADATA_COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1_500);

struct MetadataCommandOutput {
    stdout: String,
    stderr: String,
}

impl MetadataCommandOutput {
    fn first_line(&self) -> Option<String> {
        self.stdout
            .lines()
            .chain(self.stderr.lines())
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned)
    }
}

fn validate_metadata_directory(path: &str) -> Result<PathBuf, String> {
    if path.is_empty() || path.len() > METADATA_PATH_LIMIT {
        return Err("Workspace path is empty or too long".to_string());
    }
    let directory = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("Workspace is unavailable: {error}"))?;
    if !directory.is_dir() {
        return Err("Workspace is not a directory".to_string());
    }
    Ok(directory)
}

fn bounded_metadata_text(bytes: &[u8]) -> String {
    let end = bytes.len().min(METADATA_OUTPUT_LIMIT);
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

async fn run_metadata_command(
    program: &str,
    arguments: &[&str],
    working_directory: &std::path::Path,
) -> Option<MetadataCommandOutput> {
    let mut command = tokio::process::Command::new(program);
    command
        .args(arguments)
        .current_dir(working_directory)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .kill_on_drop(true);

    let output = tokio::time::timeout(METADATA_COMMAND_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(MetadataCommandOutput {
        stdout: bounded_metadata_text(&output.stdout),
        stderr: bounded_metadata_text(&output.stderr),
    })
}

/// Get repository information for the current directory
#[tauri::command]
pub async fn get_repo_info(path: String) -> Result<RepoInfo, String> {
    let working_dir = validate_metadata_directory(&path)?;

    let mut repo_info = RepoInfo {
        is_git_repo: false,
        current_branch: None,
        repo_name: None,
        remote_url: None,
        has_changes: false,
        ahead: 0,
        behind: 0,
    };

    let working_dir_text = working_dir.to_string_lossy();
    let Some(git_root) = find_git_root(&working_dir_text) else {
        return Ok(repo_info);
    };
    repo_info.is_git_repo = true;
    repo_info.repo_name = std::path::Path::new(&git_root)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned);

    // Independent repository probes run concurrently and are individually
    // bounded. A stalled Git hook, network mount, or broken executable can no
    // longer make the terminal header—or another native command—hang.
    let (branch, remote, status, divergence) = tokio::join!(
        run_metadata_command("git", &["branch", "--show-current"], &working_dir),
        run_metadata_command("git", &["remote", "get-url", "origin"], &working_dir),
        run_metadata_command(
            "git",
            &["status", "--porcelain=v1", "--untracked-files=no"],
            &working_dir,
        ),
        run_metadata_command(
            "git",
            &["rev-list", "--left-right", "--count", "HEAD...@{u}"],
            &working_dir,
        ),
    );

    repo_info.current_branch = branch.and_then(|output| output.first_line());
    if let Some(remote_url) = remote.and_then(|output| output.first_line()) {
        repo_info.repo_name = extract_repo_name(&remote_url).or(repo_info.repo_name);
        repo_info.remote_url = Some(remote_url);
    }
    repo_info.has_changes = status.is_some_and(|output| !output.stdout.is_empty());
    if let Some((ahead, behind)) =
        divergence.and_then(|output| parse_ahead_behind(output.stdout.trim()))
    {
        repo_info.ahead = ahead;
        repo_info.behind = behind;
    }

    Ok(repo_info)
}

/// Get runtime/language version information
#[tauri::command]
pub async fn get_runtime_info(path: String) -> Result<RuntimeInfo, String> {
    let working_dir = validate_metadata_directory(&path)?;

    let mut runtime_info = RuntimeInfo {
        node_version: None,
        npm_version: None,
        rust_version: None,
        python_version: None,
        git_version: None,
        go_version: None,
        java_version: None,
        project_type: None,
    };

    // Detect project type from files in the directory
    let working_dir_text = working_dir.to_string_lossy();
    runtime_info.project_type = detect_project_type(&working_dir_text);

    let git = run_metadata_command("git", &["--version"], &working_dir);
    match runtime_info.project_type.as_deref() {
        Some("typescript" | "javascript") => {
            let (node, npm, git) = tokio::join!(
                run_metadata_command("node", &["--version"], &working_dir),
                run_metadata_command("npm", &["--version"], &working_dir),
                git,
            );
            runtime_info.node_version = node.and_then(|output| output.first_line());
            runtime_info.npm_version = npm.and_then(|output| output.first_line());
            runtime_info.git_version = git.and_then(|output| output.first_line());
        }
        Some("rust") => {
            let (rust, git) = tokio::join!(
                run_metadata_command("rustc", &["--version"], &working_dir),
                git,
            );
            runtime_info.rust_version = rust.and_then(|output| output.first_line());
            runtime_info.git_version = git.and_then(|output| output.first_line());
        }
        Some("python") => {
            let (python3, git) = tokio::join!(
                run_metadata_command("python3", &["--version"], &working_dir),
                git,
            );
            let python = match python3 {
                Some(output) => Some(output),
                None => run_metadata_command("python", &["--version"], &working_dir).await,
            };
            runtime_info.python_version = python.and_then(|output| output.first_line());
            runtime_info.git_version = git.and_then(|output| output.first_line());
        }
        Some("go") => {
            let (go, git) =
                tokio::join!(run_metadata_command("go", &["version"], &working_dir), git,);
            runtime_info.go_version = go.and_then(|output| {
                output
                    .first_line()?
                    .split_whitespace()
                    .nth(2)
                    .map(ToOwned::to_owned)
            });
            runtime_info.git_version = git.and_then(|output| output.first_line());
        }
        Some("java") => {
            let (java, git) = tokio::join!(
                run_metadata_command("java", &["--version"], &working_dir),
                git,
            );
            let java = match java {
                Some(output) => Some(output),
                None => run_metadata_command("java", &["-version"], &working_dir).await,
            };
            runtime_info.java_version = java.and_then(|output| output.first_line());
            runtime_info.git_version = git.and_then(|output| output.first_line());
        }
        _ => {
            runtime_info.git_version = git.await.and_then(|output| output.first_line());
        }
    }

    Ok(runtime_info)
}

/// Detect project type based on files in the directory
fn detect_project_type(working_dir: &str) -> Option<String> {
    let path = std::path::Path::new(working_dir);

    // Check for common project files
    if path.join("package.json").exists() {
        // Check if it's a TypeScript project
        if path.join("tsconfig.json").exists() || path.join("typescript").exists() {
            return Some("typescript".to_string());
        }
        return Some("javascript".to_string());
    }

    if path.join("Cargo.toml").exists() {
        return Some("rust".to_string());
    }

    if path.join("go.mod").exists() || path.join("go.sum").exists() {
        return Some("go".to_string());
    }

    if path.join("requirements.txt").exists()
        || path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
        || path.join("Pipfile").exists()
    {
        return Some("python".to_string());
    }

    if path.join("pom.xml").exists()
        || path.join("build.gradle").exists()
        || path.join("build.gradle.kts").exists()
    {
        return Some("java".to_string());
    }

    None
}

/// Helper function to find git root directory
fn find_git_root(start_path: &str) -> Option<String> {
    let mut current_path = std::path::Path::new(start_path);

    loop {
        if current_path.join(".git").exists() {
            return Some(current_path.to_string_lossy().to_string());
        }

        if let Some(parent) = current_path.parent() {
            current_path = parent;
        } else {
            break;
        }
    }

    None
}

/// Helper function to extract repository name from remote URL
fn extract_repo_name(remote_url: &str) -> Option<String> {
    if remote_url.is_empty() {
        return None;
    }

    // Handle GitHub URLs (both HTTPS and SSH)
    if let Some(captures) = regex::Regex::new(r"github\.com[:/]([^/]+)/([^/]+?)(?:\.git)?/?$")
        .ok()?
        .captures(remote_url)
    {
        let owner = captures.get(1)?.as_str();
        let repo = captures.get(2)?.as_str();
        return Some(format!("{}/{}", owner, repo));
    }

    // Handle other Git URLs
    if let Some(captures) = regex::Regex::new(r"/([^/]+?)(?:\.git)?/?$")
        .ok()?
        .captures(remote_url)
    {
        return Some(captures.get(1)?.as_str().to_string());
    }

    None
}

/// Helper function to parse ahead/behind count
fn parse_ahead_behind(output: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() >= 2 {
        if let (Ok(ahead), Ok(behind)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
            return Some((ahead, behind));
        }
    }
    None
}

/// Initialize the ML system
#[tauri::command]
pub async fn initialize_ml_system(state: State<'_, AppState>) -> Result<String, String> {
    let mut model_manager = state.inner().model_manager.lock().await;

    match model_manager.load_model().await {
        Ok(_) => Ok("ML system initialized successfully".to_string()),
        Err(e) => Err(format!("Failed to initialize ML system: {}", e)),
    }
}

#[tauri::command]
pub async fn get_local_llm_status(
    state: State<'_, AppState>,
) -> Result<ai::RealLocalLlmStatus, String> {
    let model_manager = state.inner().model_manager.lock().await;
    Ok(model_manager.real_local_llm_status().await)
}

#[tauri::command]
pub async fn cancel_ai_generation(state: State<'_, AppState>) -> Result<bool, String> {
    let model_manager = state.inner().model_manager.lock().await.clone();
    Ok(model_manager.cancel_active_generation())
}

#[derive(Debug, serde::Serialize)]
pub struct DirectoryInfo {
    name: String,
    path: String,
    is_directory: bool,
}

/// Get parent directories for navigation
#[tauri::command]
pub async fn get_parent_directories(current_path: String) -> Result<Vec<DirectoryInfo>, String> {
    use std::path::Path;

    let path = Path::new(&current_path);
    let mut parents = Vec::new();

    // Add parent directories going up the hierarchy
    let mut current = path;
    while let Some(parent) = current.parent() {
        if let Some(name) = parent.file_name() {
            parents.push(DirectoryInfo {
                name: name.to_string_lossy().to_string(),
                path: parent.to_string_lossy().to_string(),
                is_directory: true,
            });
        } else {
            // Root directory
            parents.push(DirectoryInfo {
                name: "/".to_string(),
                path: parent.to_string_lossy().to_string(),
                is_directory: true,
            });
        }
        current = parent;

        // Limit to reasonable number of parent levels
        if parents.len() >= 10 {
            break;
        }
    }

    Ok(parents)
}

/// Get child directories and files for navigation
#[tauri::command]
pub async fn get_child_directories(current_path: String) -> Result<Vec<DirectoryInfo>, String> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(&current_path);
    let mut children = Vec::new();

    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if let Some(name) = entry_path.file_name() {
                    let name_str = name.to_string_lossy().to_string();
                    // Skip hidden files and directories (starting with .)
                    if !name_str.starts_with('.') {
                        children.push(DirectoryInfo {
                            name: name_str,
                            path: entry_path.to_string_lossy().to_string(),
                            is_directory: entry_path.is_dir(),
                        });
                    }
                }
            }
        }
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    }

    // Sort with directories first, then files, both alphabetically
    children.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less, // Directories first
            (false, true) => std::cmp::Ordering::Greater, // Files second
            _ => a.name.cmp(&b.name),                  // Alphabetical within same type
        }
    });

    Ok(children)
}

/// Change current working directory
#[tauri::command]
pub async fn change_directory(
    state: State<'_, AppState>,
    session_id: String,
    new_path: String,
) -> Result<String, String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;

    terminal_manager
        .set_working_directory(&session_id, &new_path)
        .map_err(|error| {
            eprintln!("Workspace access failed: {error}");
            format!("Failed to change directory: {error}")
        })
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Default,
    Editor,
    Terminal,
    System,
    Reveal,
    Properties,
}

fn is_text_file(extension: &str) -> bool {
    matches!(
        extension,
        "txt"
            | "md"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "xml"
            | "html"
            | "css"
            | "scss"
            | "less"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "vue"
            | "svelte"
            | "php"
            | "rb"
            | "py"
            | "go"
            | "rs"
            | "java"
            | "cpp"
            | "c"
            | "h"
            | "swift"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "env"
            | "gitignore"
            | "conf"
            | "cfg"
            | "ini"
            | "sql"
            | "csv"
    )
}

async fn perform_file_action(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
    action: FileAction,
) -> Result<String, String> {
    // Canonicalize before constructing an argument vector. The webview never
    // gets to interpolate a path into shell syntax.
    let path = PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|error| format!("File is unavailable: {error}"))?;
    if !path.is_file() {
        return Err("Selected item is not a file".to_string());
    }

    let path_string = path.to_string_lossy().to_string();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let default_opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };

    let mut commands: Vec<(String, Vec<String>)> = Vec::new();
    match action {
        FileAction::Default => {
            if cfg!(target_os = "macos") && is_text_file(&extension) {
                commands.push(("open".to_string(), vec!["-t".to_string(), path_string]));
            } else {
                commands.push((default_opener.to_string(), vec![path_string]));
            }
        }
        FileAction::Editor => {
            if !is_text_file(&extension) {
                return Err("Selected file is not recognized as text".to_string());
            }
            if cfg!(target_os = "macos") {
                commands.push(("open".to_string(), vec!["-t".to_string(), path_string]));
            } else {
                commands.push(("nano".to_string(), vec![path_string]));
            }
        }
        FileAction::Terminal => {
            let command = match extension.as_str() {
                "sh" | "bash" => ("bash".to_string(), vec![path_string]),
                "zsh" => ("zsh".to_string(), vec![path_string]),
                "fish" => ("fish".to_string(), vec![path_string]),
                "py" => ("python3".to_string(), vec![path_string]),
                "js" => ("node".to_string(), vec![path_string]),
                _ if is_text_file(&extension) => {
                    ("cat".to_string(), vec!["--".to_string(), path_string])
                }
                _ => ("file".to_string(), vec!["--".to_string(), path_string]),
            };
            commands.push(command);
        }
        FileAction::System => {
            commands.push((default_opener.to_string(), vec![path_string]));
        }
        FileAction::Reveal => {
            if cfg!(target_os = "macos") {
                commands.push(("open".to_string(), vec!["-R".to_string(), path_string]));
            } else if cfg!(windows) {
                commands.push((
                    "explorer".to_string(),
                    vec![format!("/select,{path_string}")],
                ));
            } else {
                let parent = path
                    .parent()
                    .map(|value| value.to_string_lossy().to_string())
                    .ok_or_else(|| "Selected file has no parent directory".to_string())?;
                commands.push(("xdg-open".to_string(), vec![parent]));
            }
        }
        FileAction::Properties => {
            commands.push((
                "ls".to_string(),
                vec!["-ld".to_string(), "--".to_string(), path_string.clone()],
            ));
            commands.push(("file".to_string(), vec!["--".to_string(), path_string]));
        }
    }

    let terminal_manager = state.inner().terminal_manager.lock().await;
    for (program, arguments) in commands {
        terminal_manager
            .write_shell_command(&session_id, &program, &arguments)
            .map_err(|error| format!("Failed to perform file action: {error}"))?;
    }
    Ok("Sent safe file action to the active terminal".to_string())
}

/// Safely open a file using its platform default. Kept for compatibility with
/// callers that predate the typed `file_action` command.
#[tauri::command]
pub async fn execute_file(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
) -> Result<String, String> {
    perform_file_action(state, session_id, file_path, FileAction::Default).await
}

/// File actions are represented as data and converted to a safely quoted
/// program/argument vector by the native backend.
#[tauri::command]
pub async fn file_action(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
    action: FileAction,
) -> Result<String, String> {
    perform_file_action(state, session_id, file_path, action).await
}

#[cfg(test)]
mod tests {
    use super::{
        assess_command_risk, bounded_metadata_text, extract_repo_name, has_shell_compound_operator,
        has_shell_output_redirection, has_unsafe_terminal_characters, inspect_workspace_context,
        parse_ahead_behind, CommandRiskLevel, METADATA_OUTPUT_LIMIT,
    };

    #[test]
    fn command_risk_flags_destructive_operations() {
        assert_eq!(
            assess_command_risk("rm -rf /").0,
            CommandRiskLevel::Critical
        );
        assert_eq!(
            assess_command_risk("curl https://example.test/install | sh").0,
            CommandRiskLevel::High
        );
        assert_eq!(
            assess_command_risk("git push origin main").0,
            CommandRiskLevel::Medium
        );
        assert_eq!(
            assess_command_risk("find . -type f -size +100M -print").0,
            CommandRiskLevel::Low
        );
        assert_eq!(
            assess_command_risk("find . -type f -delete").0,
            CommandRiskLevel::High
        );
        assert_eq!(
            assess_command_risk("find . -type f -exec rm -- {} +").0,
            CommandRiskLevel::High
        );
        assert_eq!(
            assess_command_risk("find . -type f | sort").0,
            CommandRiskLevel::High
        );
        assert_eq!(
            assess_command_risk("printf secret>report.txt").0,
            CommandRiskLevel::Medium
        );
        assert_eq!(
            assess_command_risk("printf secret>>report.txt").0,
            CommandRiskLevel::Medium
        );
        assert_eq!(assess_command_risk("git status").0, CommandRiskLevel::Low);
    }

    #[test]
    fn compound_detection_ignores_quoted_literals_but_flags_execution() {
        assert!(!has_shell_compound_operator("grep -r -- 'a|b' ."));
        assert!(!has_shell_compound_operator(
            "printf '%s' '$(not executed)'"
        ));
        assert!(has_shell_compound_operator("git status && git diff"));
        assert!(has_shell_compound_operator("echo $(whoami)"));
        assert!(has_shell_compound_operator("find . | sort"));
    }

    #[test]
    fn output_redirection_detection_ignores_quoted_and_escaped_literals() {
        assert!(has_shell_output_redirection("echo secret>file"));
        assert!(has_shell_output_redirection("echo secret 2>errors.log"));
        assert!(!has_shell_output_redirection("printf '%s' 'a>b'"));
        assert!(!has_shell_output_redirection("printf \"a>b\""));
        assert!(!has_shell_output_redirection("printf a\\>b"));
    }

    #[test]
    fn command_history_rejects_control_and_bidi_characters() {
        assert!(!has_unsafe_terminal_characters("git status --short"));
        assert!(has_unsafe_terminal_characters(
            "echo safe\ntouch /tmp/owned"
        ));
        assert!(has_unsafe_terminal_characters("printf '\u{1b}[6n'"));
        assert!(has_unsafe_terminal_characters("safe\u{202e}txt.exe"));
    }

    #[test]
    fn workspace_metadata_output_is_bounded() {
        let oversized = vec![b'x'; METADATA_OUTPUT_LIMIT + 512];
        assert_eq!(
            bounded_metadata_text(&oversized).len(),
            METADATA_OUTPUT_LIMIT
        );
    }

    #[test]
    fn repository_metadata_parses_common_remotes_and_divergence() {
        assert_eq!(
            extract_repo_name("git@github.com:EfficientTools/pH7Console.git"),
            Some("EfficientTools/pH7Console".to_string())
        );
        assert_eq!(parse_ahead_behind("3\t2"), Some((3, 2)));
        assert_eq!(parse_ahead_behind("unknown"), None);
    }

    #[test]
    fn workspace_context_detects_project_files_with_a_bounded_scan() {
        let workspace = std::env::temp_dir().join(format!(
            "ph7console-context-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(workspace.join("main.rs"), "fn main() {}").unwrap();

        let context = inspect_workspace_context(&workspace);

        assert!(context.contains("File Types: rs, toml"));
        assert!(context.contains("Project Type: Rust"));
        std::fs::remove_dir_all(workspace).unwrap();
    }
}
