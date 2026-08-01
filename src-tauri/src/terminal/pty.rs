use crate::shell_integration::ShellIntegration;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::{Channel, Response};

const DEFAULT_SCROLLBACK_BYTES: usize = 4 * 1024 * 1024;
const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_STREAM_SUBSCRIBERS: usize = 4;
const READ_CHUNK_BYTES: usize = 64 * 1024;
const STREAM_FRAME_VERSION: u8 = 1;
static SUBSCRIBER_ID: AtomicU64 = AtomicU64::new(1);
type StreamSubscribers = Arc<Mutex<Vec<(u64, Channel<Response>)>>>;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputEvent {
    pub session_id: String,
    pub sequence: u64,
    pub data_base64: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalExitEvent {
    pub session_id: String,
    pub exit_code: u32,
    pub signal: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSnapshot {
    pub data_base64: String,
    pub last_sequence: u64,
    pub is_running: bool,
    pub process_id: Option<u32>,
}

#[derive(Clone)]
pub struct PtyEventSinks {
    output: Arc<dyn Fn(TerminalOutputEvent) + Send + Sync>,
    exit: Arc<dyn Fn(TerminalExitEvent) + Send + Sync>,
}

impl PtyEventSinks {
    pub fn new<Output, Exit>(output: Output, exit: Exit) -> Self
    where
        Output: Fn(TerminalOutputEvent) + Send + Sync + 'static,
        Exit: Fn(TerminalExitEvent) + Send + Sync + 'static,
    {
        Self {
            output: Arc::new(output),
            exit: Arc::new(exit),
        }
    }
}

impl Default for PtyEventSinks {
    fn default() -> Self {
        Self::new(|_| {}, |_| {})
    }
}

#[derive(Default)]
struct OutputBuffer {
    chunks: VecDeque<Vec<u8>>,
    bytes: usize,
    sequence: u64,
}

impl OutputBuffer {
    fn push(&mut self, chunk: &[u8], max_bytes: usize) -> u64 {
        self.sequence = self.sequence.saturating_add(1);

        if chunk.len() >= max_bytes {
            self.chunks.clear();
            self.bytes = 0;
            self.chunks
                .push_back(chunk[chunk.len() - max_bytes..].to_vec());
            self.bytes = max_bytes;
            return self.sequence;
        }

        self.chunks.push_back(chunk.to_vec());
        self.bytes += chunk.len();
        while self.bytes > max_bytes {
            if let Some(front) = self.chunks.pop_front() {
                self.bytes = self.bytes.saturating_sub(front.len());
            } else {
                self.bytes = 0;
                break;
            }
        }

        self.sequence
    }

    fn snapshot(&self) -> (Vec<u8>, u64) {
        let mut data = Vec::with_capacity(self.bytes);
        for chunk in &self.chunks {
            data.extend_from_slice(chunk);
        }
        (data, self.sequence)
    }
}

/// Owns the native PTY resources for one independent terminal tab.
///
/// The child is reaped on a dedicated thread, while the master reader streams
/// output independently of input, resize, and every other session.
pub struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    output: Arc<Mutex<OutputBuffer>>,
    subscribers: StreamSubscribers,
    running: Arc<AtomicBool>,
    process_id: Option<u32>,
}

impl PtySession {
    pub fn spawn(
        session_id: String,
        shell: &str,
        working_directory: &str,
        size: PtySize,
        sinks: PtyEventSinks,
        shell_integration: Option<&ShellIntegration>,
        inherited_environment: &HashMap<String, String>,
    ) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(size)?;

        let mut command = CommandBuilder::new(shell);
        let integration_enabled = shell_integration
            .map(|integration| {
                let config = integration.launch_config(shell, inherited_environment);
                let enabled = config.integration_enabled;
                config.apply_to(&mut command);
                enabled
            })
            .unwrap_or(false);
        if !integration_enabled {
            configure_login_shell(&mut command, shell);
        }
        command.cwd(working_directory);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "pH7Console");
        command.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));

        let mut child = pair.slave.spawn_command(command)?;
        let process_id = child.process_id();
        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        drop(pair.slave);

        let output = Arc::new(Mutex::new(OutputBuffer::default()));
        let subscribers = Arc::new(Mutex::new(Vec::<(u64, Channel<Response>)>::new()));
        let running = Arc::new(AtomicBool::new(true));

        let reader_output = output.clone();
        let reader_subscribers = subscribers.clone();
        let reader_session_id = session_id.clone();
        let output_sink = sinks.output.clone();
        std::thread::Builder::new()
            .name(format!("pty-reader-{}", short_id(&session_id)))
            .spawn(move || {
                let mut buffer = vec![0_u8; READ_CHUNK_BYTES];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(read) => {
                            let bytes = &buffer[..read];
                            let sequence = match reader_output.lock() {
                                Ok(mut state) => state.push(bytes, DEFAULT_SCROLLBACK_BYTES),
                                Err(_) => break,
                            };

                            let channels = reader_subscribers
                                .lock()
                                .map(|subscribers| subscribers.clone())
                                .unwrap_or_default();
                            if channels.is_empty() {
                                output_sink(TerminalOutputEvent {
                                    session_id: reader_session_id.clone(),
                                    sequence,
                                    data_base64: BASE64.encode(bytes),
                                });
                            } else {
                                let frame = stream_frame(sequence, bytes);
                                let mut disconnected = Vec::new();
                                for (id, channel) in channels {
                                    if channel.send(Response::new(frame.clone())).is_err() {
                                        disconnected.push(id);
                                    }
                                }
                                if !disconnected.is_empty() {
                                    if let Ok(mut subscribers) = reader_subscribers.lock() {
                                        subscribers.retain(|(id, _)| !disconnected.contains(id));
                                    }
                                }
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            })?;

        let wait_running = running.clone();
        let wait_session_id = session_id;
        let exit_sink = sinks.exit;
        std::thread::Builder::new()
            .name(format!("pty-wait-{}", short_id(&wait_session_id)))
            .spawn(move || {
                let (exit_code, signal) = match child.wait() {
                    Ok(status) => (status.exit_code(), status.signal().map(str::to_owned)),
                    Err(_) => (1, Some("wait failed".to_string())),
                };
                wait_running.store(false, Ordering::Release);
                exit_sink(TerminalExitEvent {
                    session_id: wait_session_id,
                    exit_code,
                    signal,
                });
            })?;

        Ok(Self {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            killer: Mutex::new(killer),
            output,
            subscribers,
            running,
            process_id,
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        if data.len() > MAX_INPUT_BYTES {
            return Err("Terminal input exceeds the 1 MiB safety limit".to_string());
        }
        if !self.is_running() {
            return Err("Terminal process has exited".to_string());
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| "Terminal writer lock is unavailable".to_string())?;
        writer.write_all(data).map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        if rows == 0 || cols == 0 {
            return Err("Terminal dimensions must be non-zero".to_string());
        }
        self.master
            .lock()
            .map_err(|_| "Terminal resize lock is unavailable".to_string())?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| error.to_string())
    }

    pub fn snapshot(&self) -> Result<TerminalSnapshot, String> {
        let (data, last_sequence) = self
            .output
            .lock()
            .map_err(|_| "Terminal output buffer is unavailable".to_string())?
            .snapshot();

        Ok(TerminalSnapshot {
            data_base64: BASE64.encode(data),
            last_sequence,
            is_running: self.is_running(),
            process_id: self.process_id,
        })
    }

    pub fn terminate(&self) -> Result<(), String> {
        if !self.is_running() {
            return Ok(());
        }
        let result = self
            .killer
            .lock()
            .map_err(|_| "Terminal process lock is unavailable".to_string())?
            .kill();
        match result {
            Ok(()) => Ok(()),
            Err(error) if process_already_exited(&error) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    pub fn attach_output_channel(&self, channel: Channel<Response>) -> Result<u64, String> {
        let id = SUBSCRIBER_ID.fetch_add(1, Ordering::Relaxed);
        let mut subscribers = self
            .subscribers
            .lock()
            .map_err(|_| "Terminal stream subscriber lock is unavailable".to_string())?;
        if subscribers.len() >= MAX_STREAM_SUBSCRIBERS {
            return Err("Terminal output has too many attached views".to_string());
        }
        subscribers.push((id, channel));
        Ok(id)
    }

    pub fn detach_output_channel(&self, subscriber_id: u64) -> Result<(), String> {
        let mut subscribers = self
            .subscribers
            .lock()
            .map_err(|_| "Terminal stream subscriber lock is unavailable".to_string())?;
        subscribers.retain(|(id, _)| *id != subscriber_id);
        Ok(())
    }
}

fn process_already_exited(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::NotFound {
        return true;
    }

    #[cfg(unix)]
    if error.raw_os_error() == Some(libc::ESRCH) {
        return true;
    }

    false
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

fn configure_login_shell(command: &mut CommandBuilder, shell: &str) {
    let executable = shell
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(shell)
        .to_ascii_lowercase();

    if executable == "fish" {
        command.arg("--login");
    } else if !cfg!(windows) {
        command.arg("-l");
    }
}

fn short_id(session_id: &str) -> &str {
    session_id.get(..8).unwrap_or(session_id)
}

fn stream_frame(sequence: u64, bytes: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(9 + bytes.len());
    frame.push(STREAM_FRAME_VERSION);
    frame.extend_from_slice(&sequence.to_be_bytes());
    frame.extend_from_slice(bytes);
    frame
}

#[cfg(test)]
mod tests {
    use super::{process_already_exited, OutputBuffer};

    #[test]
    fn scrollback_is_byte_bounded() {
        let mut buffer = OutputBuffer::default();
        buffer.push(b"1234", 6);
        buffer.push(b"5678", 6);

        let (snapshot, sequence) = buffer.snapshot();
        assert_eq!(snapshot, b"5678");
        assert_eq!(sequence, 2);
    }

    #[test]
    fn oversized_chunk_keeps_only_its_tail() {
        let mut buffer = OutputBuffer::default();
        buffer.push(b"abcdefgh", 4);

        let (snapshot, _) = buffer.snapshot();
        assert_eq!(snapshot, b"efgh");
    }

    #[test]
    fn missing_process_is_already_terminated() {
        let missing = std::io::Error::from_raw_os_error(
            #[cfg(unix)]
            libc::ESRCH,
            #[cfg(windows)]
            3,
        );
        assert!(process_already_exited(&missing));

        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert!(!process_already_exited(&denied));
    }
}
