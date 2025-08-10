use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub title: String,
    pub working_directory: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    pub id: String,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct TerminalManager {
    sessions: HashMap<String, TerminalSession>,
    command_history: Vec<CommandExecution>,
    pty_system: Box<dyn portable_pty::PtySystem>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            command_history: Vec::new(),
            pty_system: native_pty_system(),
        }
    }

    pub fn create_session(&mut self, title: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
        let session_id = Uuid::new_v4().to_string();
        let working_directory = std::env::current_dir()?.to_string_lossy().to_string();
        
        let session = TerminalSession {
            id: session_id.clone(),
            title: title.unwrap_or_else(|| format!("Terminal {}", session_id[..8].to_string())),
            working_directory,
            is_active: true,
            created_at: chrono::Utc::now(),
        };
        
        self.sessions.insert(session_id.clone(), session);
        Ok(session_id)
    }

    pub async fn execute_command(
        &mut self,
        session_id: &str,
        command: &str,
    ) -> Result<CommandExecution, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();
        let execution_id = Uuid::new_v4().to_string();
        
        // Parse command and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".into());
        }

        let cmd = parts[0];
        let args = &parts[1..];
        
        // Create PTY for command execution
        let pty_size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };
        
        let mut command_builder = CommandBuilder::new(cmd);
        command_builder.args(args);
        
        // Set working directory if session exists
        if let Some(session) = self.sessions.get(session_id) {
            command_builder.cwd(&session.working_directory);
        }
        
        let mut child = self.pty_system.openpty(pty_size)?;
        let mut process = child.slave.spawn_command(command_builder)?;
        
        // Read output (simplified for demo)
        let mut output = String::new();
        let exit_code = match process.wait() {
            Ok(status) => status.success().then_some(0).or(Some(1)),
            Err(_) => Some(1),
        };
        
        // For demo purposes, simulate some output
        if cmd == "echo" {
            output = args.join(" ");
        } else if cmd == "pwd" {
            output = std::env::current_dir()?.to_string_lossy().to_string();
        } else if cmd == "ls" {
            // Simple ls implementation
            let entries = std::fs::read_dir(".")?
                .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("\n");
            output = entries;
        } else {
            output = format!("Command '{}' executed", command);
        }
        
        let duration = start_time.elapsed();
        
        let execution = CommandExecution {
            id: execution_id,
            command: command.to_string(),
            output,
            exit_code,
            duration_ms: duration.as_millis() as u64,
            timestamp: chrono::Utc::now(),
        };
        
        self.command_history.push(execution.clone());
        Ok(execution)
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

    pub fn get_smart_context(&self, session_id: &str) -> String {
        let mut context = String::new();
        
        if let Some(session) = self.sessions.get(session_id) {
            context.push_str(&format!("Working Directory: {}\n", session.working_directory));
        }
        
        // Add recent command history for context
        let recent_commands: Vec<String> = self.command_history
            .iter()
            .rev()
            .take(5)
            .map(|cmd| format!("{} (exit: {:?})", cmd.command, cmd.exit_code))
            .collect();
        
        if !recent_commands.is_empty() {
            context.push_str("Recent Commands:\n");
            context.push_str(&recent_commands.join("\n"));
        }
        
        context
    }
}
