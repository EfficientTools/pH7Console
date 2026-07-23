mod pty;

pub use pty::{PtyEventSinks, TerminalSnapshot};

use crate::shell_integration::ShellIntegration;
use portable_pty::PtySize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::ipc::{Channel, Response};
use uuid::Uuid;

use self::pty::PtySession;

const MAX_TERMINAL_SESSIONS: usize = 16;
const MAX_SESSION_TITLE_CHARS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub title: String,
    pub working_directory: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip)]
    pub environment_vars: HashMap<String, String>,
    pub shell: String,
    pub pty_size: (u16, u16), // cols, rows
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    pub id: String,
    pub session_id: String,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub working_directory: String,
}

pub struct TerminalManager {
    sessions: HashMap<String, TerminalSession>,
    pty_sessions: HashMap<String, PtySession>,
    command_history: Vec<CommandExecution>,
    pty_event_sinks: PtyEventSinks,
    shell_integration: Option<ShellIntegration>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self::with_event_sinks(PtyEventSinks::default())
    }

    pub fn with_event_sinks(pty_event_sinks: PtyEventSinks) -> Self {
        Self::with_runtime_integrations(pty_event_sinks, None)
    }

    pub fn with_runtime_integrations(
        pty_event_sinks: PtyEventSinks,
        shell_integration: Option<ShellIntegration>,
    ) -> Self {
        Self {
            sessions: HashMap::new(),
            pty_sessions: HashMap::new(),
            command_history: Vec::new(),
            pty_event_sinks,
            shell_integration,
        }
    }

    pub fn create_session(
        &mut self,
        title: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.create_session_at(title, None)
    }

    pub fn create_session_at(
        &mut self,
        title: Option<String>,
        working_directory: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.sessions.len() >= MAX_TERMINAL_SESSIONS {
            return Err(format!(
                "At most {MAX_TERMINAL_SESSIONS} terminal sessions can be open at once"
            )
            .into());
        }
        let (session_id, session, pty_session) =
            self.build_session(title, working_directory, None, (80, 24))?;
        self.sessions.insert(session_id.clone(), session);
        self.pty_sessions.insert(session_id.clone(), pty_session);
        Ok(session_id)
    }

    /// Spawn and validate a replacement before retiring the old PTY. This is
    /// safe even at the tab limit and guarantees a failed shell launch leaves
    /// the existing tab intact.
    pub fn restart_session(
        &mut self,
        session_id: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let previous = self
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("Terminal session not found: {session_id}"))?;
        let (replacement_id, replacement, replacement_pty) = self.build_session(
            Some(previous.title.clone()),
            Some(&previous.working_directory),
            Some(previous.shell.clone()),
            previous.pty_size,
        )?;

        let previous_pty = self.pty_sessions.remove(session_id);
        self.sessions.remove(session_id);
        self.sessions.insert(replacement_id.clone(), replacement);
        self.pty_sessions
            .insert(replacement_id.clone(), replacement_pty);
        drop(previous_pty);
        Ok(replacement_id)
    }

    fn build_session(
        &self,
        title: Option<String>,
        working_directory: Option<&str>,
        shell: Option<String>,
        pty_size: (u16, u16),
    ) -> Result<(String, TerminalSession, PtySession), Box<dyn std::error::Error>> {
        let session_id = Uuid::new_v4().to_string();
        let working_directory = working_directory
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
            .canonicalize()
            .map_err(|error| format!("Terminal workspace is unavailable: {error}"))?;
        if !working_directory.is_dir() {
            return Err("Terminal workspace is not a directory".into());
        }
        let working_directory = working_directory.to_string_lossy().to_string();

        // Get default shell
        let shell = shell.unwrap_or_else(|| {
            std::env::var("SHELL")
                .or_else(|_| std::env::var("COMSPEC"))
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        "cmd.exe".to_string()
                    } else {
                        "/bin/bash".to_string()
                    }
                })
        });

        // Get environment variables
        let mut environment_vars = HashMap::new();
        for (key, value) in std::env::vars() {
            environment_vars.insert(key, value);
        }

        let session_title = match title {
            Some(title) => validate_session_title(&title)?,
            None => format!("Terminal {}", &session_id[..8]),
        };

        let session = TerminalSession {
            id: session_id.clone(),
            title: session_title,
            working_directory,
            is_active: true,
            created_at: chrono::Utc::now(),
            environment_vars,
            shell,
            pty_size,
        };

        let pty_session = PtySession::spawn(
            session_id.clone(),
            &session.shell,
            &session.working_directory,
            PtySize {
                rows: session.pty_size.1,
                cols: session.pty_size.0,
                pixel_width: 0,
                pixel_height: 0,
            },
            self.pty_event_sinks.clone(),
            self.shell_integration.as_ref(),
            &session.environment_vars,
        )?;

        Ok((session_id, session, pty_session))
    }

    pub fn set_working_directory(
        &mut self,
        session_id: &str,
        new_path: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let canonical_path = PathBuf::from(new_path).canonicalize()?;
        if !canonical_path.is_dir() {
            return Err("Selected workspace is not a directory".into());
        }

        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Terminal session not found: {session_id}"))?;
        session.working_directory = canonical_path.to_string_lossy().to_string();
        let working_directory = session.working_directory.clone();
        let shell = session.shell.clone();

        if let Some(pty) = self.pty_sessions.get(session_id) {
            let command = shell_change_directory_command(&shell, &working_directory);
            pty.write(command.as_bytes())?;
        }

        Ok(working_directory)
    }

    pub async fn execute_command(
        &mut self,
        session_id: &str,
        command: &str,
    ) -> Result<CommandExecution, Box<dyn std::error::Error + Send + Sync>> {
        self.execute_command_with_history(session_id, command, command)
            .await
    }

    /// Execute a command but store a different command in history (useful for natural language translation)
    pub async fn execute_command_with_history(
        &mut self,
        session_id: &str,
        command_to_execute: &str,
        command_for_history: &str,
    ) -> Result<CommandExecution, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();
        let execution_id = Uuid::new_v4().to_string();

        // Parse command and arguments for execution
        let parts = shell_words::split(command_to_execute)?;
        if parts.is_empty() {
            return Err("Empty command".into());
        }

        let cmd = parts[0].as_str();
        let args: Vec<&str> = parts[1..].iter().map(String::as_str).collect();

        // Handle built-in commands
        if let Some(result) = self.handle_builtin_command(session_id, cmd, &args).await? {
            let duration = start_time.elapsed();
            let execution = CommandExecution {
                id: execution_id,
                session_id: session_id.to_string(),
                command: command_for_history.to_string(), // Store the original command in history
                output: result.0,
                exit_code: Some(result.1),
                duration_ms: duration.as_millis() as u64,
                timestamp: chrono::Utc::now(),
                working_directory: self
                    .sessions
                    .get(session_id)
                    .map(|session| session.working_directory.clone())
                    .unwrap_or_default(),
            };

            // IMPORTANT: Add built-in commands to history too!
            self.command_history.push(execution.clone());

            // Limit history size
            if self.command_history.len() > 1000 {
                self.command_history.remove(0);
            }

            return Ok(execution);
        }

        // Set working directory and environment if session exists
        let (working_dir, env_vars, shell) = if let Some(session) = self.sessions.get(session_id) {
            (
                session.working_directory.clone(),
                session.environment_vars.clone(),
                session.shell.clone(),
            )
        } else {
            (
                std::env::current_dir()?.to_string_lossy().to_string(),
                std::env::vars().collect(),
                default_shell(),
            )
        };

        // Execute command with enhanced error handling
        let output_result = self
            .execute_system_command(command_to_execute, &shell, &working_dir, &env_vars)
            .await;

        let (output, exit_code) = match output_result {
            Ok((stdout, stderr, exit_code)) => {
                if exit_code.unwrap_or(0) == 0 || stderr.is_empty() {
                    // Success or no errors - combine stdout/stderr normally
                    let combined = if stderr.is_empty() {
                        stdout
                    } else if stdout.is_empty() {
                        stderr
                    } else {
                        format!("{}\n{}", stdout, stderr)
                    };
                    (combined, exit_code)
                } else {
                    // Error case - enhance the error message
                    let enhanced_error =
                        self.enhance_error_message(command_to_execute, &stderr, exit_code);
                    let combined = if stdout.is_empty() {
                        enhanced_error
                    } else {
                        format!("{}\n\n{}", stdout, enhanced_error)
                    };
                    (combined, exit_code)
                }
            }
            Err(e) => {
                let enhanced_error =
                    self.enhance_error_message(command_to_execute, &e.to_string(), Some(1));
                (enhanced_error, Some(1))
            }
        };

        let duration = start_time.elapsed();

        // Update working directory if command was 'cd'
        if cmd == "cd" && exit_code == Some(0) {
            self.update_session_directory(session_id, &args);
        }

        let execution = CommandExecution {
            id: execution_id,
            session_id: session_id.to_string(),
            command: command_for_history.to_string(), // Store the original command in history
            output,
            exit_code,
            duration_ms: duration.as_millis() as u64,
            timestamp: chrono::Utc::now(),
            working_directory: self
                .sessions
                .get(session_id)
                .map(|session| session.working_directory.clone())
                .unwrap_or_default(),
        };

        self.command_history.push(execution.clone());

        // Limit history size
        if self.command_history.len() > 1000 {
            self.command_history.remove(0);
        }

        Ok(execution)
    }

    /// Handle built-in terminal commands
    async fn handle_builtin_command(
        &mut self,
        session_id: &str,
        cmd: &str,
        args: &[&str],
    ) -> Result<Option<(String, i32)>, Box<dyn std::error::Error + Send + Sync>> {
        match cmd {
            "cd" => {
                let target_dir = if args.is_empty() {
                    // Go to home directory
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
                } else {
                    let path = args[0];
                    let expanded_path = if path.starts_with('~') {
                        // Expand ~ to home directory
                        if let Some(home) = dirs::home_dir() {
                            if path == "~" {
                                home
                            } else {
                                home.join(&path[2..]) // Skip "~/"
                            }
                        } else {
                            PathBuf::from(path)
                        }
                    } else if path.starts_with('/') {
                        // Absolute path
                        PathBuf::from(path)
                    } else {
                        // Relative path - resolve from current working directory
                        if let Some(session) = self.sessions.get(session_id) {
                            let current_dir = PathBuf::from(&session.working_directory);
                            current_dir.join(path)
                        } else {
                            PathBuf::from(path)
                        }
                    };

                    // Try to canonicalize the path to resolve .. and . components
                    match expanded_path.canonicalize() {
                        Ok(canonical) => canonical,
                        Err(_) => {
                            // If canonicalize fails, try to resolve manually
                            let mut components = Vec::new();
                            for component in expanded_path.components() {
                                match component {
                                    std::path::Component::ParentDir => {
                                        components.pop();
                                    }
                                    std::path::Component::CurDir => {
                                        // Skip current directory
                                    }
                                    _ => {
                                        components.push(component.as_os_str());
                                    }
                                }
                            }
                            let mut result = PathBuf::new();
                            for component in components {
                                result.push(component);
                            }
                            result
                        }
                    }
                };

                if target_dir.exists() && target_dir.is_dir() {
                    if let Some(session) = self.sessions.get_mut(session_id) {
                        session.working_directory = target_dir.to_string_lossy().to_string();
                    }
                    Ok(Some((
                        format!("📁 Changed directory to {}", target_dir.display()),
                        0,
                    )))
                } else {
                    // Enhanced error message with suggestions
                    let suggestion = if !target_dir.exists() {
                        let parent = target_dir.parent();
                        let suggestions = if let Some(parent_dir) = parent {
                            if parent_dir.exists() {
                                // List similar directories in parent
                                if let Ok(entries) = std::fs::read_dir(parent_dir) {
                                    let similar_dirs: Vec<String> = entries
                                        .filter_map(|entry| entry.ok())
                                        .filter(|entry| entry.path().is_dir())
                                        .map(|entry| {
                                            entry.file_name().to_string_lossy().to_string()
                                        })
                                        .filter(|name| {
                                            if let Some(target_name) = target_dir.file_name() {
                                                let target_str =
                                                    target_name.to_string_lossy().to_lowercase();
                                                let name_lower = name.to_lowercase();
                                                name_lower.starts_with(
                                                    &target_str
                                                        [..std::cmp::min(3, target_str.len())],
                                                )
                                            } else {
                                                false
                                            }
                                        })
                                        .take(3)
                                        .collect();

                                    if !similar_dirs.is_empty() {
                                        format!("\n💡 Did you mean: {}", similar_dirs.join(", "))
                                    } else {
                                        "\n💡 Try using 'ls' to see available directories or check the path spelling".to_string()
                                    }
                                } else {
                                    "\n💡 Try using 'ls' to see available directories or check the path spelling".to_string()
                                }
                            } else {
                                "\n💡 Parent directory doesn't exist. Check the full path."
                                    .to_string()
                            }
                        } else {
                            "\n💡 Try using 'ls' to see available directories or use an absolute path starting with /".to_string()
                        };
                        suggestions
                    } else {
                        "\n💡 The path exists but is not a directory".to_string()
                    };
                    Ok(Some((
                        format!(
                            "❌ Directory '{}' not found{}",
                            target_dir.display(),
                            suggestion
                        ),
                        1,
                    )))
                }
            }
            "pwd" => {
                if let Some(session) = self.sessions.get(session_id) {
                    Ok(Some((session.working_directory.clone(), 0)))
                } else {
                    Ok(Some((
                        std::env::current_dir()?.to_string_lossy().to_string(),
                        0,
                    )))
                }
            }
            "history" => {
                let history_output = self
                    .command_history
                    .iter()
                    .enumerate()
                    .map(|(i, cmd)| format!("{:4} {}", i + 1, cmd.command))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(Some((history_output, 0)))
            }
            "clear" => {
                Ok(Some(("\x1b[2J\x1b[H".to_string(), 0))) // ANSI clear screen
            }
            "exit" => {
                if let Some(session) = self.sessions.get_mut(session_id) {
                    session.is_active = false;
                }
                Ok(Some(("Session ended".to_string(), 0)))
            }
            _ => Ok(None), // Not a built-in command
        }
    }

    /// Execute system command with enhanced features
    async fn execute_system_command(
        &self,
        command_text: &str,
        shell: &str,
        working_dir: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(String, String, Option<i32>), Box<dyn std::error::Error + Send + Sync>> {
        let mut command = tokio::process::Command::new(shell);
        if cfg!(windows) {
            if shell.to_ascii_lowercase().contains("powershell") {
                command.args(["-NoProfile", "-Command", command_text]);
            } else {
                command.args(["/C", command_text]);
            }
        } else {
            command.args(["-lc", command_text]);
        }
        command.current_dir(working_dir);

        // Set environment variables
        for (key, value) in env_vars {
            command.env(key, value);
        }

        // Execute with timeout and better error handling
        let output =
            tokio::time::timeout(std::time::Duration::from_secs(300), command.output()).await?;

        let output = output?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();

        Ok((stdout, stderr, exit_code))
    }

    /// Enhance error messages with user-friendly explanations and suggestions
    fn enhance_error_message(&self, command: &str, stderr: &str, exit_code: Option<i32>) -> String {
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        let base_cmd = cmd_parts.first().unwrap_or(&"unknown");

        // If stderr is empty but exit code indicates error, provide generic help
        if stderr.trim().is_empty() && exit_code.unwrap_or(0) != 0 {
            return match base_cmd {
                &"ls" | &"dir" => "❌ Unable to list directory contents\n💡 Check if the directory exists or if you have permission to access it".to_string(),
                &"cat" | &"less" | &"more" => "❌ Unable to read file\n💡 Check if the file exists and you have read permissions".to_string(),
                &"mkdir" => "❌ Unable to create directory\n💡 Check if the parent directory exists and you have write permissions".to_string(),
                &"rm" | &"rmdir" => "❌ Unable to remove file/directory\n💡 Check if the item exists and you have write permissions".to_string(),
                &"cp" | &"mv" => "❌ Unable to copy/move file\n💡 Check if source exists and destination is writable".to_string(),
                _ => format!("❌ Command '{}' failed\n💡 Try running with --help for usage information", base_cmd),
            };
        }

        let error_lower = stderr.to_lowercase();

        // Enhanced error patterns with helpful suggestions
        if error_lower.contains("no such file or directory") || error_lower.contains("not found") {
            if error_lower.contains("command not found") {
                format!("❌ Command '{}' not found\n💡 Try:\n  • Check spelling: did you mean a similar command?\n  • Install the command if it's a package\n  • Use 'which {}' to see if it's in PATH", base_cmd, base_cmd)
            } else {
                format!("❌ File or directory not found\n{}\n💡 Try:\n  • Use 'ls' to see available files\n  • Check the path spelling\n  • Use absolute path starting with /", stderr.trim())
            }
        } else if error_lower.contains("permission denied") {
            format!("❌ Permission denied\n{}\n💡 Try:\n  • Use 'sudo' for administrator privileges\n  • Check file permissions with 'ls -la'\n  • Make sure you own the file/directory", stderr.trim())
        } else if error_lower.contains("directory not empty") {
            format!("❌ Directory not empty\n{}\n💡 Try:\n  • Use 'rm -rf' to remove directory and contents\n  • Remove contents first, then the directory", stderr.trim())
        } else if error_lower.contains("already exists") {
            format!("❌ File/directory already exists\n{}\n💡 Try:\n  • Use a different name\n  • Remove existing file first\n  • Use --force flag if available", stderr.trim())
        } else if error_lower.contains("disk")
            && (error_lower.contains("full") || error_lower.contains("space"))
        {
            format!("❌ Insufficient disk space\n{}\n💡 Try:\n  • Free up space by removing unnecessary files\n  • Use 'df -h' to check disk usage\n  • Clean temporary files", stderr.trim())
        } else if error_lower.contains("connection")
            && (error_lower.contains("refused") || error_lower.contains("timeout"))
        {
            format!("❌ Network connection issue\n{}\n💡 Try:\n  • Check your internet connection\n  • Verify the server/URL is correct\n  • Check if firewall is blocking the connection", stderr.trim())
        } else if !stderr.trim().is_empty() {
            // For other errors, just format them nicely
            format!("❌ Error:\n{}", stderr.trim())
        } else {
            format!(
                "❌ Command failed with exit code {}",
                exit_code.unwrap_or(-1)
            )
        }
    }

    /// Update session working directory
    fn update_session_directory(&mut self, session_id: &str, args: &[&str]) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            if !args.is_empty() {
                let new_dir = PathBuf::from(&session.working_directory).join(args[0]);
                if let Ok(canonical) = new_dir.canonicalize() {
                    session.working_directory = canonical.to_string_lossy().to_string();
                }
            }
        }
    }

    pub fn get_session(&self, session_id: &str) -> Option<&TerminalSession> {
        self.sessions.get(session_id)
    }

    pub fn get_all_sessions(&self) -> Vec<&TerminalSession> {
        self.sessions.values().collect()
    }

    pub fn get_command_history(&self, limit: Option<usize>) -> Vec<&CommandExecution> {
        let history = &self.command_history;
        match limit {
            Some(n) => history.iter().rev().take(n).collect(),
            None => history.iter().rev().collect(),
        }
    }

    /// Clear the bounded process-local history cache. Durable history, when
    /// enabled, is cleared independently by the command layer so this remains
    /// available in the memory-only privacy fallback.
    pub fn clear_command_history(&mut self) {
        self.command_history.clear();
    }

    pub fn get_smart_context(&self, session_id: &str) -> String {
        let mut context = String::new();

        if let Some(session) = self.sessions.get(session_id) {
            context.push_str(&format!(
                "Working Directory: {}\n",
                session.working_directory
            ));
            context.push_str(&format!("Shell: {}\n", session.shell));

            // Filesystem enrichment is deliberately performed by the command
            // layer after this lock is released. A slow network volume must
            // never delay PTY input, resize, or output attachment.
        }

        // Add recent command history for context
        let recent_commands: Vec<String> = self
            .command_history
            .iter()
            .rev()
            .filter(|command| command.session_id == session_id)
            .take(5)
            .map(|cmd| format!("{} (exit: {:?})", cmd.command, cmd.exit_code))
            .collect();

        if !recent_commands.is_empty() {
            context.push_str("Recent Commands:\n");
            context.push_str(&recent_commands.join("\n"));
        }

        context
    }

    /// Get session-specific command history
    pub fn get_session_history(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Vec<&CommandExecution> {
        let history = self
            .command_history
            .iter()
            .rev()
            .filter(|command| command.session_id == session_id);
        match limit {
            Some(limit) => history.take(limit).collect(),
            None => history.collect(),
        }
    }

    /// Update session title
    pub fn update_session_title(&mut self, session_id: &str, title: String) -> Result<(), String> {
        let title = validate_session_title(&title).map_err(|error| error.to_string())?;
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.title = title;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Close session
    pub fn close_session(&mut self, session_id: &str) -> Result<(), String> {
        if let Some(pty) = self.pty_sessions.remove(session_id) {
            pty.terminate()?;
        }

        if let Some(mut session) = self.sessions.remove(session_id) {
            session.is_active = false;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Resize terminal
    pub fn resize_terminal(
        &mut self,
        session_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), String> {
        if let Some(pty) = self.pty_sessions.get(session_id) {
            pty.resize(rows, cols)?;
        } else {
            return Err("Terminal process not found".to_string());
        }

        if let Some(session) = self.sessions.get_mut(session_id) {
            session.pty_size = (cols, rows);
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Write raw UTF-8 input to the persistent shell attached to a session.
    pub fn write_to_terminal(&self, session_id: &str, data: &str) -> Result<(), String> {
        self.write_bytes_to_terminal(session_id, data.as_bytes())
    }

    pub fn write_bytes_to_terminal(&self, session_id: &str, data: &[u8]) -> Result<(), String> {
        self.pty_sessions
            .get(session_id)
            .ok_or_else(|| "Terminal process not found".to_string())?
            .write(data)
    }

    /// Send a program and argument vector through the interactive shell while
    /// quoting every value for that shell. Callers never concatenate paths.
    pub fn write_shell_command(
        &self,
        session_id: &str,
        program: &str,
        arguments: &[String],
    ) -> Result<(), String> {
        let shell = &self
            .sessions
            .get(session_id)
            .ok_or_else(|| "Session not found".to_string())?
            .shell;
        let mut command = shell_command_prefix(shell, program);
        for argument in arguments {
            command.push(' ');
            command.push_str(&shell_quote_argument(shell, argument));
        }
        command.push('\r');
        self.write_to_terminal(session_id, &command)
    }

    /// Return bounded scrollback plus the sequence used for race-free replay.
    pub fn get_terminal_snapshot(&self, session_id: &str) -> Result<TerminalSnapshot, String> {
        self.pty_sessions
            .get(session_id)
            .ok_or_else(|| "Terminal process not found".to_string())?
            .snapshot()
    }

    pub fn attach_output_channel(
        &self,
        session_id: &str,
        channel: Channel<Response>,
    ) -> Result<u64, String> {
        self.pty_sessions
            .get(session_id)
            .ok_or_else(|| "Terminal process not found".to_string())?
            .attach_output_channel(channel)
    }

    pub fn detach_output_channel(
        &self,
        session_id: &str,
        subscriber_id: u64,
    ) -> Result<(), String> {
        self.pty_sessions
            .get(session_id)
            .ok_or_else(|| "Terminal process not found".to_string())?
            .detach_output_channel(subscriber_id)
    }

    /// Synchronize metadata from a trusted OSC 7 path emitted by the shell.
    /// This never executes shell text.
    pub fn sync_working_directory(
        &mut self,
        session_id: &str,
        working_directory: &str,
    ) -> Result<String, String> {
        let path = PathBuf::from(working_directory);
        if !path.is_absolute() || !path.is_dir() {
            return Err("Shell reported an invalid working directory".to_string());
        }

        let path = path
            .canonicalize()
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        session.working_directory = path.clone();
        Ok(path)
    }

    /// Get system information
    pub fn get_system_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();

        info.insert("os".to_string(), std::env::consts::OS.to_string());
        info.insert("arch".to_string(), std::env::consts::ARCH.to_string());

        if let Ok(hostname) = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")) {
            info.insert("hostname".to_string(), hostname);
        }

        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            info.insert("user".to_string(), user);
        }

        info.insert(
            "shell".to_string(),
            std::env::var("SHELL")
                .or_else(|_| std::env::var("COMSPEC"))
                .unwrap_or_else(|_| "unknown".to_string()),
        );

        info
    }

    /// Get command suggestions based on current context
    pub fn get_context_suggestions(&self, session_id: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        if let Some(session) = self.sessions.get(session_id) {
            let work_dir = PathBuf::from(&session.working_directory);

            // Suggest based on files in current directory
            if work_dir.join("package.json").exists() {
                suggestions.extend(vec![
                    "npm install".to_string(),
                    "npm run dev".to_string(),
                    "npm test".to_string(),
                    "npm run build".to_string(),
                ]);
            }

            if work_dir.join("Cargo.toml").exists() {
                suggestions.extend(vec![
                    "cargo build".to_string(),
                    "cargo test".to_string(),
                    "cargo run".to_string(),
                    "cargo check".to_string(),
                ]);
            }

            if work_dir.join(".git").exists() {
                suggestions.extend(vec![
                    "git status".to_string(),
                    "git add .".to_string(),
                    "git commit".to_string(),
                    "git push".to_string(),
                ]);
            }

            // Always include basic commands
            suggestions.extend(vec![
                "ls -la".to_string(),
                "pwd".to_string(),
                "cd ..".to_string(),
            ]);
        }

        suggestions
    }

    /// Get file and directory completions for a given partial path
    pub fn get_path_completions(&self, session_id: &str, partial_path: &str) -> Vec<String> {
        let mut completions = Vec::new();

        let (search_dir, prefix) = if partial_path.is_empty() {
            // No path provided, search current directory
            if let Some(session) = self.sessions.get(session_id) {
                (PathBuf::from(&session.working_directory), String::new())
            } else {
                (
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    String::new(),
                )
            }
        } else if partial_path.ends_with('/') {
            // Path ends with /, search in that directory
            let path = self.expand_path(session_id, partial_path);
            (path, String::new())
        } else {
            // Partial filename, search in parent directory
            let path_buf = PathBuf::from(partial_path);
            if let Some(parent) = path_buf.parent() {
                let expanded_parent = self.expand_path(session_id, &parent.to_string_lossy());
                let prefix = path_buf
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                (expanded_parent, prefix)
            } else {
                // No parent, search current directory
                if let Some(session) = self.sessions.get(session_id) {
                    (
                        PathBuf::from(&session.working_directory),
                        partial_path.to_string(),
                    )
                } else {
                    (
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                        partial_path.to_string(),
                    )
                }
            }
        };

        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files unless prefix starts with .
                if name.starts_with('.') && !prefix.starts_with('.') {
                    continue;
                }

                // Check if name starts with prefix (case-insensitive)
                if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    if entry.path().is_dir() {
                        completions.push(format!("{}/", name));
                    } else {
                        completions.push(name);
                    }
                }
            }
        }

        completions.sort();
        completions
    }

    /// Expand path relative to session working directory
    fn expand_path(&self, session_id: &str, path: &str) -> PathBuf {
        if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                if path == "~" {
                    home
                } else {
                    home.join(&path[2..]) // Skip "~/"
                }
            } else {
                PathBuf::from(path)
            }
        } else if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            if let Some(session) = self.sessions.get(session_id) {
                PathBuf::from(&session.working_directory).join(path)
            } else {
                PathBuf::from(path)
            }
        }
    }

    /// Get command history for arrow key navigation
    pub fn get_command_history_for_navigation(&self, session_id: &str) -> Vec<String> {
        self.command_history
            .iter()
            .rev()
            .filter(|command| command.session_id == session_id)
            .map(|cmd| cmd.command.clone())
            .collect()
    }

    /// Search command history
    pub fn search_command_history(&self, pattern: &str) -> Vec<String> {
        self.command_history
            .iter()
            .rev()
            .filter(|cmd| cmd.command.to_lowercase().contains(&pattern.to_lowercase()))
            .map(|cmd| cmd.command.clone())
            .take(10) // Limit to 10 results
            .collect()
    }

    /// Store a command in history without executing it (for natural language commands)
    pub fn store_command_in_history(
        &mut self,
        session_id: &str,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create a minimal command execution entry for history storage
        let execution = CommandExecution {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            command: command.to_string(),
            output: String::new(), // Empty output since this is just for history tracking
            exit_code: None,
            duration_ms: 0, // No actual execution time
            timestamp: chrono::Utc::now(),
            working_directory: self
                .sessions
                .get(session_id)
                .map(|session| session.working_directory.clone())
                .unwrap_or_default(),
        };

        self.command_history.push(execution);

        // Keep only the last 1000 commands
        if self.command_history.len() > 1000 {
            self.command_history.remove(0);
        }

        Ok(())
    }

    /// Add a completed interactive-shell command captured by OSC shell
    /// integration. This keeps the in-memory UI cache aligned with durable
    /// history without placing SQLite on the PTY reader thread.
    pub fn record_command_execution(&mut self, execution: CommandExecution) {
        self.command_history.push(execution);
        if self.command_history.len() > 1_000 {
            self.command_history.remove(0);
        }
    }
}

fn validate_session_title(title: &str) -> Result<String, Box<dyn std::error::Error>> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Terminal title cannot be empty".into());
    }
    if title.chars().count() > MAX_SESSION_TITLE_CHARS {
        return Err(format!("Terminal title exceeds {MAX_SESSION_TITLE_CHARS} characters").into());
    }
    if title.chars().any(|character| {
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
    }) {
        return Err("Terminal title contains unsafe control characters".into());
    }
    Ok(title.to_owned())
}

fn shell_change_directory_command(shell: &str, path: &str) -> String {
    let shell_name = shell.to_ascii_lowercase();
    if cfg!(windows) && shell_name.contains("powershell") {
        return format!("Set-Location -LiteralPath '{}'\r", path.replace('\'', "''"));
    }
    if cfg!(windows) {
        // Double quotes cannot occur in Windows file names.
        return format!("cd /d \"{}\"\r", path);
    }

    // POSIX shells interpret a single quote literally when we close the
    // quoted segment, insert an escaped quote, and reopen it.
    format!("cd -- '{}'\r", path.replace('\'', "'\\''"))
}

fn shell_command_prefix(shell: &str, program: &str) -> String {
    if cfg!(windows) && shell.to_ascii_lowercase().contains("powershell") {
        return format!("& {}", shell_quote_argument(shell, program));
    }
    shell_quote_argument(shell, program)
}

fn shell_quote_argument(shell: &str, value: &str) -> String {
    if cfg!(windows) && shell.to_ascii_lowercase().contains("powershell") {
        return format!("'{}'", value.replace('\'', "''"));
    }
    if cfg!(windows) {
        // Windows file names cannot contain double quotes. Percent signs are
        // doubled so cmd.exe does not expand them as environment variables.
        return format!("\"{}\"", value.replace('%', "%%"));
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn default_shell() -> String {
    std::env::var("SHELL")
        .or_else(|_| std::env::var("COMSPEC"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else if cfg!(target_os = "macos") {
                "/bin/zsh".to_string()
            } else {
                "/bin/bash".to_string()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::{
        shell_change_directory_command, validate_session_title, PtyEventSinks, TerminalManager,
    };
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    #[test]
    fn clear_command_history_removes_the_process_local_cache() {
        let mut manager = TerminalManager::new();
        manager
            .store_command_in_history("session-a", "echo private")
            .expect("store command");
        assert_eq!(manager.get_command_history(None).len(), 1);

        manager.clear_command_history();

        assert!(manager.get_command_history(None).is_empty());
        assert!(manager
            .get_command_history_for_navigation("session-a")
            .is_empty());
        assert!(manager.search_command_history("private").is_empty());
    }

    #[test]
    fn terminal_titles_reject_controls_and_visual_reordering() {
        assert!(validate_session_title("build\ncomplete").is_err());
        assert!(validate_session_title("safe\u{202e}txt").is_err());
        assert!(validate_session_title(&"x".repeat(129)).is_err());
        assert_eq!(
            validate_session_title("  Project Shell  ").unwrap(),
            "Project Shell"
        );
    }

    fn receive_until(
        receiver: &mpsc::Receiver<super::pty::TerminalOutputEvent>,
        marker: &str,
    ) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut output = Vec::new();
        while Instant::now() < deadline {
            if let Ok(event) = receiver.recv_timeout(Duration::from_millis(100)) {
                output.extend(BASE64.decode(event.data_base64).unwrap());
                let text = String::from_utf8_lossy(&output);
                if text.contains(marker) {
                    return text.into_owned();
                }
            }
        }
        panic!(
            "PTY output did not contain marker {marker:?}: {}",
            String::from_utf8_lossy(&output)
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn workspace_change_shell_quotes_paths() {
        assert_eq!(
            shell_change_directory_command("/bin/zsh", "/tmp/alice's project"),
            "cd -- '/tmp/alice'\\''s project'\r"
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn persistent_pty_streams_output_and_preserves_shell_state() {
        let (sender, receiver) = mpsc::channel();
        let sinks = PtyEventSinks::new(
            move |event| {
                let _ = sender.send(event);
            },
            |_| {},
        );
        let mut manager = TerminalManager::with_event_sinks(sinks);
        let session_id = manager
            .create_session(Some("PTY test".to_string()))
            .unwrap();

        manager
            .write_to_terminal(
                &session_id,
                "export PH7_PTY_STATE='persistent'; printf '__PH7_PTY__%s__\\n' \"$PH7_PTY_STATE\"\r",
            )
            .unwrap();
        let output = receive_until(&receiver, "__PH7_PTY__persistent__");
        assert!(output.contains("__PH7_PTY__persistent__"));

        manager.resize_terminal(&session_id, 120, 40).unwrap();
        let snapshot = manager.get_terminal_snapshot(&session_id).unwrap();
        assert!(snapshot.is_running);
        assert!(snapshot.last_sequence > 0);

        manager.close_session(&session_id).unwrap();
    }

    #[test]
    #[cfg(not(windows))]
    fn ctrl_c_interrupts_a_foreground_job() {
        let (sender, receiver) = mpsc::channel();
        let sinks = PtyEventSinks::new(
            move |event| {
                let _ = sender.send(event);
            },
            |_| {},
        );
        let mut manager = TerminalManager::with_event_sinks(sinks);
        let session_id = manager
            .create_session(Some("signal test".to_string()))
            .unwrap();

        manager
            .write_to_terminal(
                &session_id,
                "printf '__PH7_SLEEPING__\\n'; sleep 10; printf '__PH7_AFTER_SLEEP__\\n'\r",
            )
            .unwrap();
        receive_until(&receiver, "__PH7_SLEEPING__");
        manager
            .write_bytes_to_terminal(&session_id, &[0x03])
            .unwrap();
        manager
            .write_to_terminal(&session_id, "printf '__PH7_INTERRUPT_OK__\\n'\r")
            .unwrap();

        let output = receive_until(&receiver, "__PH7_INTERRUPT_OK__");
        assert!(output.contains("__PH7_INTERRUPT_OK__"));
        manager.close_session(&session_id).unwrap();
    }

    #[test]
    #[cfg(not(windows))]
    fn sessions_stream_without_cross_session_blocking() {
        let (sender, receiver) = mpsc::channel();
        let sinks = PtyEventSinks::new(
            move |event| {
                let _ = sender.send(event);
            },
            |_| {},
        );
        let mut manager = TerminalManager::with_event_sinks(sinks);
        let slow = manager.create_session(Some("slow".to_string())).unwrap();
        let fast = manager.create_session(Some("fast".to_string())).unwrap();

        manager.write_to_terminal(&slow, "sleep 2\r").unwrap();
        let started = Instant::now();
        manager
            .write_to_terminal(&fast, "printf '__PH7_FAST_SESSION__\\n'\r")
            .unwrap();
        receive_until(&receiver, "__PH7_FAST_SESSION__");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "one session was blocked by another"
        );

        manager.close_session(&slow).unwrap();
        manager.close_session(&fast).unwrap();
    }

    #[tokio::test]
    async fn executes_commands_with_shell_syntax() {
        let mut manager = TerminalManager::new();
        let session_id = manager.create_session(Some("test".to_string())).unwrap();

        let execution = manager
            .execute_command(&session_id, "printf 'hello world' | tr 'a-z' 'A-Z'")
            .await
            .unwrap();

        assert_eq!(execution.exit_code, Some(0));
        assert_eq!(execution.output, "HELLO WORLD");
    }

    #[test]
    fn selects_a_workspace_with_spaces_in_its_path() {
        let mut manager = TerminalManager::new();
        let session_id = manager.create_session(Some("test".to_string())).unwrap();
        let workspace =
            std::env::temp_dir().join(format!("ph7console workspace {}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&workspace).unwrap();

        let selected = manager
            .set_working_directory(&session_id, workspace.to_str().unwrap())
            .unwrap();

        assert_eq!(
            selected,
            workspace.canonicalize().unwrap().to_string_lossy()
        );
        std::fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn creates_a_shell_directly_in_the_requested_workspace() {
        let workspace =
            std::env::temp_dir().join(format!("ph7console restart {}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&workspace).unwrap();
        let mut manager = TerminalManager::new();

        let session_id = manager
            .create_session_at(Some("Recovered shell".to_string()), workspace.to_str())
            .unwrap();
        let session = manager.get_session(&session_id).unwrap();

        assert_eq!(
            session.working_directory,
            workspace.canonicalize().unwrap().to_string_lossy()
        );
        manager.close_session(&session_id).unwrap();
        std::fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn restarts_a_shell_without_losing_its_workspace_or_tab_metadata() {
        let workspace =
            std::env::temp_dir().join(format!("ph7console recovery {}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&workspace).unwrap();
        let mut manager = TerminalManager::new();
        let previous_id = manager
            .create_session_at(Some("Development".to_string()), workspace.to_str())
            .unwrap();
        manager.resize_terminal(&previous_id, 132, 42).unwrap();

        let replacement_id = manager.restart_session(&previous_id).unwrap();
        let replacement = manager.get_session(&replacement_id).unwrap();

        assert_ne!(replacement_id, previous_id);
        assert!(manager.get_session(&previous_id).is_none());
        assert_eq!(replacement.title, "Development");
        assert_eq!(replacement.pty_size, (132, 42));
        assert_eq!(
            replacement.working_directory,
            workspace.canonicalize().unwrap().to_string_lossy()
        );
        manager.close_session(&replacement_id).unwrap();
        std::fs::remove_dir_all(workspace).unwrap();
    }
}
