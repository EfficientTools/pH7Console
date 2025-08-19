use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub title: String,
    pub working_directory: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub environment_vars: HashMap<String, String>,
    pub shell: String,
    pub pty_size: (u16, u16), // cols, rows
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
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            command_history: Vec::new(),
        }
    }

    pub fn create_session(&mut self, title: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
        let session_id = Uuid::new_v4().to_string();
        let working_directory = std::env::current_dir()?.to_string_lossy().to_string();
        
        // Get default shell
        let shell = std::env::var("SHELL")
            .or_else(|_| std::env::var("COMSPEC"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "cmd.exe".to_string()
                } else {
                    "/bin/bash".to_string()
                }
            });

        // Get environment variables
        let mut environment_vars = HashMap::new();
        for (key, value) in std::env::vars() {
            environment_vars.insert(key, value);
        }
        
        let session = TerminalSession {
            id: session_id.clone(),
            title: title.unwrap_or_else(|| format!("Terminal {}", session_id[..8].to_string())),
            working_directory,
            is_active: true,
            created_at: chrono::Utc::now(),
            environment_vars,
            shell,
            pty_size: (80, 24), // Default terminal size
        };
        
        self.sessions.insert(session_id.clone(), session);
        Ok(session_id)
    }

    pub async fn execute_command(
        &mut self,
        session_id: &str,
        command: &str,
    ) -> Result<CommandExecution, Box<dyn std::error::Error + Send + Sync>> {
        self.execute_command_with_history(session_id, command, command).await
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
        let parts: Vec<&str> = command_to_execute.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".into());
        }

        let cmd = parts[0];
        let args = &parts[1..];
        
        // Handle built-in commands
        if let Some(result) = self.handle_builtin_command(session_id, cmd, args).await? {
            let duration = start_time.elapsed();
            let execution = CommandExecution {
                id: execution_id,
                command: command_for_history.to_string(), // Store the original command in history
                output: result.0,
                exit_code: Some(result.1),
                duration_ms: duration.as_millis() as u64,
                timestamp: chrono::Utc::now(),
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
        let (working_dir, env_vars) = if let Some(session) = self.sessions.get(session_id) {
            (session.working_directory.clone(), session.environment_vars.clone())
        } else {
            (std::env::current_dir()?.to_string_lossy().to_string(), std::env::vars().collect())
        };
        
        // Execute command with enhanced error handling
        let output_result = self.execute_system_command(cmd, args, &working_dir, &env_vars).await;
        
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
                    let enhanced_error = self.enhance_error_message(command_to_execute, &stderr, exit_code);
                    let combined = if stdout.is_empty() {
                        enhanced_error
                    } else {
                        format!("{}\n\n{}", stdout, enhanced_error)
                    };
                    (combined, exit_code)
                }
            },
            Err(e) => {
                let enhanced_error = self.enhance_error_message(command_to_execute, &e.to_string(), Some(1));
                (enhanced_error, Some(1))
            }
        };
        
        let duration = start_time.elapsed();
        
        // Update working directory if command was 'cd'
        if cmd == "cd" && exit_code == Some(0) {
            self.update_session_directory(session_id, args);
        }
        
        let execution = CommandExecution {
            id: execution_id,
            command: command_for_history.to_string(), // Store the original command in history
            output,
            exit_code,
            duration_ms: duration.as_millis() as u64,
            timestamp: chrono::Utc::now(),
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
                                    },
                                    std::path::Component::CurDir => {
                                        // Skip current directory
                                    },
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
                    Ok(Some((format!("📁 Changed directory to {}", target_dir.display()), 0)))
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
                                        .map(|entry| entry.file_name().to_string_lossy().to_string())
                                        .filter(|name| {
                                            if let Some(target_name) = target_dir.file_name() {
                                                let target_str = target_name.to_string_lossy().to_lowercase();
                                                let name_lower = name.to_lowercase();
                                                name_lower.starts_with(&target_str[..std::cmp::min(3, target_str.len())])
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
                                "\n💡 Parent directory doesn't exist. Check the full path.".to_string()
                            }
                        } else {
                            "\n💡 Try using 'ls' to see available directories or use an absolute path starting with /".to_string()
                        };
                        suggestions
                    } else {
                        "\n💡 The path exists but is not a directory".to_string()
                    };
                    Ok(Some((format!("❌ Directory '{}' not found{}", target_dir.display(), suggestion), 1)))
                }
            },
            "pwd" => {
                if let Some(session) = self.sessions.get(session_id) {
                    Ok(Some((session.working_directory.clone(), 0)))
                } else {
                    Ok(Some((std::env::current_dir()?.to_string_lossy().to_string(), 0)))
                }
            },
            "history" => {
                let history_output = self.command_history
                    .iter()
                    .enumerate()
                    .map(|(i, cmd)| format!("{:4} {}", i + 1, cmd.command))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(Some((history_output, 0)))
            },
            "clear" => {
                Ok(Some(("\x1b[2J\x1b[H".to_string(), 0))) // ANSI clear screen
            },
            "exit" => {
                if let Some(session) = self.sessions.get_mut(session_id) {
                    session.is_active = false;
                }
                Ok(Some(("Session ended".to_string(), 0)))
            },
            _ => Ok(None) // Not a built-in command
        }
    }

    /// Execute system command with enhanced features
    async fn execute_system_command(
        &self,
        cmd: &str,
        args: &[&str],
        working_dir: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(String, String, Option<i32>), Box<dyn std::error::Error + Send + Sync>> {
        let mut command = tokio::process::Command::new(cmd);
        command.args(args);
        command.current_dir(working_dir);
        
        // Set environment variables
        for (key, value) in env_vars {
            command.env(key, value);
        }
        
        // Execute with timeout and better error handling
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(30), // 30 second timeout
            command.output()
        ).await?;
        
        let output = output?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();
        
        Ok((stdout, stderr, exit_code))
    }

    /// Enhance error messages with user-friendly explanations and suggestions
    fn enhance_error_message(&self, command: &str, stderr: &str, exit_code: Option<i32>) -> String {
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        let base_cmd = cmd_parts.get(0).unwrap_or(&"unknown");
        
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
        } else if error_lower.contains("disk") && (error_lower.contains("full") || error_lower.contains("space")) {
            format!("❌ Insufficient disk space\n{}\n💡 Try:\n  • Free up space by removing unnecessary files\n  • Use 'df -h' to check disk usage\n  • Clean temporary files", stderr.trim())
        } else if error_lower.contains("connection") && (error_lower.contains("refused") || error_lower.contains("timeout")) {
            format!("❌ Network connection issue\n{}\n💡 Try:\n  • Check your internet connection\n  • Verify the server/URL is correct\n  • Check if firewall is blocking the connection", stderr.trim())
        } else if !stderr.trim().is_empty() {
            // For other errors, just format them nicely
            format!("❌ Error:\n{}", stderr.trim())
        } else {
            format!("❌ Command failed with exit code {}", exit_code.unwrap_or(-1))
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

    pub fn get_smart_context(&self, session_id: &str) -> String {
        let mut context = String::new();
        
        if let Some(session) = self.sessions.get(session_id) {
            context.push_str(&format!("Working Directory: {}\n", session.working_directory));
            context.push_str(&format!("Shell: {}\n", session.shell));
            
            // Add file type context
            if let Ok(entries) = std::fs::read_dir(&session.working_directory) {
                let mut file_types = Vec::new();
                for entry in entries.take(20) { // Limit to avoid performance issues
                    if let Ok(entry) = entry {
                        if let Some(ext) = entry.path().extension() {
                            if let Some(ext_str) = ext.to_str() {
                                file_types.push(ext_str.to_string());
                            }
                        }
                    }
                }
                
                if !file_types.is_empty() {
                    file_types.sort();
                    file_types.dedup();
                    context.push_str(&format!("File Types: {}\n", file_types.join(", ")));
                }
            }
            
            // Check for common project files
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
            
            for (file, tech) in &project_indicators {
                let file_path = PathBuf::from(&session.working_directory).join(file);
                if file_path.exists() {
                    context.push_str(&format!("Project Type: {}\n", tech));
                }
            }
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

    /// Get session-specific command history
    pub fn get_session_history(&self, session_id: &str, limit: Option<usize>) -> Vec<&CommandExecution> {
        // For now, return global history. In a full implementation, 
        // we'd track per-session history
        self.get_command_history(limit)
    }

    /// Update session title
    pub fn update_session_title(&mut self, session_id: &str, title: String) -> Result<(), String> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.title = title;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Close session
    pub fn close_session(&mut self, session_id: &str) -> Result<(), String> {
        if let Some(mut session) = self.sessions.remove(session_id) {
            session.is_active = false;
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Resize terminal
    pub fn resize_terminal(&mut self, session_id: &str, rows: u16, cols: u16) -> Result<(), String> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.pty_size = (cols, rows);
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Get system information
    pub fn get_system_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        
        info.insert("os".to_string(), std::env::consts::OS.to_string());
        info.insert("arch".to_string(), std::env::consts::ARCH.to_string());
        
        if let Ok(hostname) = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME")) {
            info.insert("hostname".to_string(), hostname);
        }
        
        if let Ok(user) = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME")) {
            info.insert("user".to_string(), user);
        }
        
        info.insert("shell".to_string(), 
            std::env::var("SHELL")
                .or_else(|_| std::env::var("COMSPEC"))
                .unwrap_or_else(|_| "unknown".to_string())
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
                (std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")), String::new())
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
                let prefix = path_buf.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                (expanded_parent, prefix)
            } else {
                // No parent, search current directory
                if let Some(session) = self.sessions.get(session_id) {
                    (PathBuf::from(&session.working_directory), partial_path.to_string())
                } else {
                    (std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")), partial_path.to_string())
                }
            }
        };

        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
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
    pub fn get_command_history_for_navigation(&self, _session_id: &str) -> Vec<String> {
        // Return commands in reverse chronological order (most recent first)
        // Note: Currently using global history, but could be filtered by session in the future
        self.command_history
            .iter()
            .rev()
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
    pub fn store_command_in_history(&mut self, _session_id: &str, command: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create a minimal command execution entry for history storage
        let execution = CommandExecution {
            id: uuid::Uuid::new_v4().to_string(),
            command: command.to_string(),
            output: String::new(), // Empty output since this is just for history tracking
            exit_code: Some(0), // Mark as successful since it's just being stored
            duration_ms: 0, // No actual execution time
            timestamp: chrono::Utc::now(),
        };

        self.command_history.push(execution);
        
        // Keep only the last 1000 commands
        if self.command_history.len() > 1000 {
            self.command_history.remove(0);
        }
        
        Ok(())
    }
}
