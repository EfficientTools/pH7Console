use crate::{AppState, ai};
use crate::ai::{AIResponse};
use crate::terminal::CommandExecution;
use tauri::State;
use std::path::PathBuf;

#[tauri::command]
pub async fn create_terminal(
    state: State<'_, AppState>,
    title: Option<String>
) -> Result<String, String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    
    terminal_manager.create_session(title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_command(
    state: State<'_, AppState>,
    session_id: String,
    command: String
) -> Result<CommandExecution, String> {
    let _start_time = std::time::Instant::now();
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    
    // Detect if this is a natural language command and translate it first
    let actual_command = if is_natural_language_command(&command) {
        println!("🔍 Detected natural language command: '{}'", command);
        
        // Get the model manager to translate
        let model_manager = state.inner().model_manager.lock().await;
        
        // Check if model is loaded
        if !model_manager.is_model_loaded() {
            println!("⚠️ Model not loaded yet, attempting to load...");
            // Try to load the model if not already loaded
            drop(model_manager); // Release the lock
            let mut model_manager = state.inner().model_manager.lock().await;
            if let Err(e) = model_manager.load_model().await {
                println!("❌ Failed to load model: {}", e);
                // Fall back to original command
                command.clone()
            } else {
                println!("✅ Model loaded successfully!");
                let context = terminal_manager.get_smart_context(&session_id);
                let translation_result = model_manager.process_command_with_ml(&command, Some(&context)).await;
                
                if translation_result.confidence > 0.6 {
                    let translated_cmd = translation_result.text.clone();
                    println!("✅ Translated to: '{}' (confidence: {:.1}%)", translated_cmd, translation_result.confidence * 100.0);
                    
                    // Remove the 🤖 marker if present for execution
                    translated_cmd.replace("🤖 ", "")
                } else {
                    println!("⚠️ Low confidence translation, executing original command");
                    command.clone()
                }
            }
        } else {
            let context = terminal_manager.get_smart_context(&session_id);
            
            // Translate natural language to command
            let translation_result = model_manager.process_command_with_ml(&command, Some(&context)).await;
            
            if translation_result.confidence > 0.6 {
                let translated_cmd = translation_result.text.clone();
                println!("✅ Translated to: '{}' (confidence: {:.1}%)", translated_cmd, translation_result.confidence * 100.0);
                
                // Remove the 🤖 marker if present for execution
                translated_cmd.replace("🤖 ", "")
            } else {
                println!("⚠️ Low confidence translation, executing original command");
                command.clone()
            }
        }
    } else {
        println!("📝 Regular shell command: '{}'", command);
        command.clone()
    };
    
    // Execute the command - use special method for natural language to preserve original in history
    let result = if is_natural_language_command(&command) && actual_command != command {
        // For natural language commands, execute the translated command but store original in history
        terminal_manager.execute_command_with_history(&session_id, &actual_command, &command)
            .await
            .map_err(|e| e.to_string())
    } else {
        // For regular commands, use normal execution
        terminal_manager.execute_command(&session_id, &actual_command)
            .await
            .map_err(|e| e.to_string())
    };

    // Learn from this command execution
    if let Ok(execution) = &result {
        let model_manager = state.inner().model_manager.lock().await;
        let context = terminal_manager.get_smart_context(&session_id);
        let success = execution.exit_code.unwrap_or(0) == 0;
        
        // Enhanced learning with session context
        model_manager.learn_from_command(
            &command, // Use original command for learning
            &execution.output,
            &context,
            success,
            Some(execution.duration_ms),
        ).await;
        
        // Track session workflow for pattern recognition
        model_manager.track_session_workflow(&session_id, &command).await;
    }

    result
}

/// Detect if a command is natural language vs a regular shell command
fn is_natural_language_command(command: &str) -> bool {
    let cmd_lower = command.to_lowercase().trim().to_string();
    
    // Check for obvious shell commands first (including single-word commands)
    if cmd_lower.starts_with("ls") || cmd_lower.starts_with("cd ") || cmd_lower.starts_with("pwd") ||
       cmd_lower.starts_with("git ") || cmd_lower.starts_with("npm ") || cmd_lower.starts_with("cargo ") ||
       cmd_lower.starts_with("mkdir ") || cmd_lower.starts_with("touch ") || cmd_lower.starts_with("rm ") ||
       cmd_lower.starts_with("cp ") || cmd_lower.starts_with("mv ") || cmd_lower.starts_with("find ") ||
       cmd_lower.starts_with("grep ") || cmd_lower.starts_with("cat ") || cmd_lower.starts_with("echo ") ||
       cmd_lower.starts_with("sudo ") || cmd_lower.starts_with("./") || cmd_lower.starts_with("../") ||
       cmd_lower.starts_with("man ") || cmd_lower.starts_with("which ") || cmd_lower.starts_with("ps ") ||
       cmd_lower.starts_with("top") || cmd_lower.starts_with("htop") || cmd_lower.starts_with("df ") ||
       cmd_lower.starts_with("open ") || cmd_lower == "open" || cmd_lower.starts_with("vim ") || 
       cmd_lower.starts_with("nano ") || cmd_lower.starts_with("emacs ") || cmd_lower.starts_with("code ") ||
       cmd_lower.starts_with("ssh ") || cmd_lower.starts_with("scp ") || cmd_lower.starts_with("curl ") ||
       cmd_lower.starts_with("wget ") || cmd_lower.starts_with("brew ") || cmd_lower.starts_with("pip ") ||
       cmd_lower.starts_with("python ") || cmd_lower.starts_with("node ") || cmd_lower.starts_with("java ") ||
       cmd_lower.starts_with("rustc ") || cmd_lower.starts_with("gcc ") || cmd_lower.starts_with("clang ") ||
       cmd_lower.starts_with("tar ") || cmd_lower.starts_with("unzip ") || cmd_lower.starts_with("zip ") ||
       cmd_lower == "pwd" || cmd_lower == "ls" || cmd_lower == "clear" || cmd_lower == "exit" ||
       cmd_lower == "history" || cmd_lower == "top" || cmd_lower == "htop" || cmd_lower == "whoami" ||
       cmd_lower.starts_with("/") || cmd_lower.starts_with("~") {
        return false;
    }
    
    // Highly specific natural language patterns that we want to catch
    let high_confidence_patterns = [
        "go home", "go to home", "go home directory", "go to home directory",
        "go to parent", "go to parent directory", "go up", "go back",
        "show files", "list files", "show me files", "display files",
        "what files", "what's here", "what is here",
        "where am i", "current directory", "present working directory",
        "create file", "make file", "new file", "add file",
        "create folder", "make folder", "make directory", "create directory",
        "git status", "check git", "git state", "repository status",
        "install package", "add package", "npm install",
        "run project", "start project", "build project"
    ];
    
    // Check for exact matches or substring matches of high confidence patterns
    for pattern in &high_confidence_patterns {
        if cmd_lower == *pattern || cmd_lower.contains(pattern) {
            return true;
        }
    }
    
    // Check for natural language sentence structure patterns
    let natural_patterns = [
        "go to", "navigate to", "change to", "move to", "switch to",
        "show me", "list", "display", "what", "where", "how",
        "create", "make", "build", "install", "run",
        "find", "search for", "look for", "locate",
        "home directory", "parent directory", "current directory",
        "files", "folder", "directory",
        "status", "help", "explain"
    ];
    
    let pattern_matches = natural_patterns.iter().filter(|&&pattern| cmd_lower.contains(pattern)).count();
    
    // If it contains natural language patterns
    if pattern_matches > 0 {
        return true;
    }
    
    // Check for sentence-like structure (contains common English words)
    let english_words = ["the", "a", "an", "to", "in", "on", "at", "for", "with", "by", "my", "me", "i"];
    let word_count = english_words.iter().filter(|&&word| cmd_lower.contains(word)).count();
    
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
            "ls", "cd", "pwd", "mkdir", "rmdir", "rm", "cp", "mv", "ln", "find", "grep", "cat", "less", "more",
            "head", "tail", "sort", "uniq", "wc", "chmod", "chown", "ps", "top", "kill", "jobs", "bg", "fg",
            "nohup", "ssh", "scp", "rsync", "tar", "gzip", "gunzip", "zip", "unzip", "curl", "wget", "ping",
            "traceroute", "netstat", "ifconfig", "iptables", "sudo", "su", "whoami", "id", "groups", "history",
            "alias", "which", "whereis", "locate", "man", "info", "help", "clear", "reset", "exit", "logout",
            // macOS specific commands
            "open", "say", "osascript", "pbcopy", "pbpaste", "sw_vers", "system_profiler", "diskutil", "hdiutil",
            "mdls", "mdfind", "spotlight", "launchctl", "scutil", "networksetup", "security", "keychain",
            // Development tools
            "git", "npm", "yarn", "pnpm", "cargo", "python", "python3", "node", "java", "javac", "rustc", 
            "gcc", "clang", "g++", "make", "cmake", "autoconf", "automake", "libtool", "pkg-config",
            // Package managers
            "brew", "pip", "pip3", "pipx", "conda", "apt", "yum", "dnf", "pacman", "snap", "flatpak",
            // Text editors
            "vim", "vi", "nvim", "nano", "emacs", "code", "subl", "atom", "pico",
            // System monitoring
            "htop", "iotop", "nettop", "activity", "fs_usage", "dtruss", "ktrace", "iostat", "vmstat",
            // Network tools
            "nc", "netcat", "telnet", "ftp", "sftp", "rsync", "scp", "dig", "nslookup", "host", "whois",
            // File operations
            "file", "stat", "du", "df", "lsof", "fuser", "basename", "dirname", "realpath", "readlink",
            // Process control
            "pgrep", "pkill", "killall", "nohup", "screen", "tmux", "at", "crontab", "watch",
            // Compression
            "compress", "uncompress", "bzip2", "bunzip2", "xz", "unxz", "7z", "rar", "unrar",
            // Database tools
            "sqlite3", "mysql", "psql", "mongo", "redis-cli",
            // Container tools
            "docker", "podman", "kubectl", "helm", "docker-compose",
            // Media tools
            "ffmpeg", "imagemagick", "convert", "identify", "exiftool",
            // Misc utilities
            "awk", "sed", "tr", "cut", "paste", "join", "tee", "xargs", "parallel", "jq", "yq",
            "base64", "uuencode", "uudecode", "hexdump", "od", "strings", "xxd"
        ];
        
        if !unix_commands.contains(&first_word) && words.len() >= 2 {
            return true;
        }
    }
    
    false
}

#[tauri::command]
pub async fn get_terminal_output(
    state: State<'_, AppState>,
    _session_id: String,
    limit: Option<usize>
) -> Result<Vec<CommandExecution>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    
    let history = terminal_manager.get_command_history(limit);
    Ok(history.into_iter().cloned().collect())
}

#[tauri::command]
pub async fn ai_suggest_command(
    state: State<'_, AppState>,
    context: String,
    intent: Option<String>
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    let prompt = match intent {
        Some(i) => format!("Suggest commands for: {}. Context: {}", i, context),
        None => format!("Suggest next commands based on context: {}", context),
    };
    
    Ok(model_manager.generate_response(&prompt, Some(&context)).await)
}

#[tauri::command]
pub async fn ai_explain_command(
    state: State<'_, AppState>,
    command: String
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    let prompt = format!("Explain this command: {}", command);
    
    Ok(model_manager.generate_response(&prompt, None).await)
}

#[tauri::command]
pub async fn ai_fix_error(
    state: State<'_, AppState>,
    error_output: String,
    command: String,
    context: Option<String>
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    let prompt = format!(
        "Fix this error - Command: '{}', Error: '{}', Context: '{}'",
        command, error_output, context.unwrap_or_default()
    );
    
    Ok(model_manager.generate_response(&prompt, Some(&error_output)).await)
}

#[tauri::command]
pub async fn ai_analyze_output(
    state: State<'_, AppState>,
    output: String,
    command: String
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    let prompt = format!(
        "Analyze this command output and provide insights: Command: '{}', Output: '{}'",
        command, output
    );
    
    Ok(model_manager.generate_response(&prompt, Some(&output)).await)
}

#[tauri::command]
pub async fn get_smart_completions(
    state: State<'_, AppState>,
    partial_command: String,
    session_id: String
) -> Result<Vec<String>, String> {
    let model_manager = state.inner().model_manager.lock().await;
    let terminal_manager = state.inner().terminal_manager.lock().await;
    
    let context = terminal_manager.get_smart_context(&session_id);
    
    // Get enhanced completions with session context
    let completions = model_manager.get_enhanced_completions(&partial_command, &context, &session_id).await;
    Ok(completions)
}

#[tauri::command]
pub async fn ai_translate_natural_language(
    state: State<'_, AppState>,
    natural_language: String,
    context: String,
) -> Result<AIResponse, String> {
    let model_manager = state.inner().model_manager.lock().await;
    
    // Use ML-powered command processing for better accuracy
    let ml_response = model_manager.process_command_with_ml(&natural_language, Some(&context)).await;
    
    // If ML processing has high confidence, use it directly
    if ml_response.confidence > 0.8 {
        return Ok(ml_response);
    }
    
    // Otherwise, try the enhanced approach as fallback
    let prompt = format!("Convert this natural language request to a terminal command: \"{}\"", natural_language);
    let response = model_manager.generate_response(&prompt, Some(&context)).await;
    
    // If the response looks like a comment, try a more specific approach
    if response.text.starts_with('#') || response.text.contains("need more") {
        let enhanced_prompt = format!("natural language: {}", natural_language);
        let enhanced_response = model_manager.generate_response(&enhanced_prompt, Some(&context)).await;
        Ok(enhanced_response)
    } else {
        Ok(response)
    }
}

/// Get user analytics from learning engine
#[tauri::command]
pub async fn get_user_analytics(
    state: State<'_, AppState>,
) -> Result<Option<ai::UserAnalytics>, String> {
    let model_manager = state.inner().model_manager.lock().await;
    Ok(model_manager.get_analytics().await)
}

/// Update feedback for learning
#[tauri::command]
pub async fn update_ai_feedback(
    state: State<'_, AppState>,
    command: String,
    feedback: f32,
) -> Result<(), String> {
    let model_manager = state.inner().model_manager.lock().await;
    model_manager.update_feedback(&command, feedback).await;
    Ok(())
}

/// Agent mode: Create autonomous task
#[tauri::command]
pub async fn create_agent_task(
    state: State<'_, AppState>,
    description: String,
) -> Result<String, String> {
    let model_manager = state.inner().model_manager.lock().await;
    model_manager.create_agent_task(&description).await
}

/// Get agent task status
#[tauri::command]
pub async fn get_agent_task_status(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Option<ai::TaskStatus>, String> {
    let model_manager = state.inner().model_manager.lock().await;
    Ok(model_manager.get_agent_task_status(&task_id).await)
}

/// Get all active agent tasks
#[tauri::command]
pub async fn get_active_agent_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let model_manager = state.inner().model_manager.lock().await;
    Ok(model_manager.get_active_agent_tasks().await)
}

/// Cancel agent task
#[tauri::command]
pub async fn cancel_agent_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    let model_manager = state.inner().model_manager.lock().await;
    model_manager.cancel_agent_task(&task_id).await
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
    Ok(terminal_manager.get_all_sessions().into_iter().cloned().collect())
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

/// Get command history for arrow key navigation
#[tauri::command]
pub async fn get_command_history_for_navigation(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<String>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager.get_command_history_for_navigation(&session_id))
}

/// Search command history
#[tauri::command]
pub async fn search_command_history(
    state: State<'_, AppState>,
    pattern: String,
) -> Result<Vec<String>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    Ok(terminal_manager.search_command_history(&pattern))
}

/// Store a command in history without executing it (for natural language commands)
#[tauri::command]
pub async fn store_command_in_history(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<(), String> {
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    terminal_manager.store_command_in_history(&session_id, &command)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_command() -> Result<String, String> {
    Ok("Test successful".to_string())
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
            PathBuf::from(&current_working_dir).join(&dir).to_string_lossy().to_string()
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
        dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        "/usr/local".to_string(),
        "/opt".to_string(),
        format!("{}/Desktop", dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()),
        format!("{}/Documents", dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()),
        format!("{}/Downloads", dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()),
    ];
    
    for location in search_locations {
        let potential_path = PathBuf::from(&location).join(&target_name);
        if potential_path.is_dir() {
            return Ok(Some(potential_path.to_string_lossy().to_string()));
        }
        
        // Also search one level deep in common directories
        if let Ok(entries) = std::fs::read_dir(&location) {
            for entry in entries.take(50) { // Limit search to prevent performance issues
                if let Ok(entry) = entry {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        let nested_path = entry.path().join(&target_name);
                        if nested_path.is_dir() {
                            return Ok(Some(nested_path.to_string_lossy().to_string()));
                        }
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
        PathBuf::from(&current_working_dir).join(&path).to_string_lossy().to_string()
    } else {
        path.clone()
    };
    
    // Check if the expanded path exists
    if PathBuf::from(&expanded_path).exists() {
        return Ok(Some(expanded_path));
    }
    
    // If not found, try to find it in frequent directories
    let path_buf = PathBuf::from(&path);
    let path_name = path_buf.file_name()
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

/// Get repository information for the current directory
#[tauri::command]
pub async fn get_repo_info(
    path: String,
) -> Result<RepoInfo, String> {
    let working_dir = path;

    let mut repo_info = RepoInfo {
        is_git_repo: false,
        current_branch: None,
        repo_name: None,
        remote_url: None,
        has_changes: false,
        ahead: 0,
        behind: 0,
    };

    // Check if we're in a git repository
    let git_dir = std::path::Path::new(&working_dir).join(".git");
    if git_dir.exists() || find_git_root(&working_dir).is_some() {
        repo_info.is_git_repo = true;

        // Get current branch
        if let Ok(output) = std::process::Command::new("git")
            .args(&["branch", "--show-current"])
            .current_dir(&working_dir)
            .output()
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() {
                    repo_info.current_branch = Some(branch.clone());
                }
            }
        }

        // Get repository name from remote URL
        if let Ok(output) = std::process::Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .current_dir(&working_dir)
            .output()
        {
            if output.status.success() {
                let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                repo_info.remote_url = Some(remote_url.clone());
                
                // Extract repo name from URL
                if let Some(repo_name) = extract_repo_name(&remote_url) {
                    repo_info.repo_name = Some(repo_name.clone());
                }
            }
        }

        // Check for uncommitted changes
        if let Ok(output) = std::process::Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(&working_dir)
            .output()
        {
            if output.status.success() {
                let status_output = String::from_utf8_lossy(&output.stdout);
                repo_info.has_changes = !status_output.trim().is_empty();
            }
        }

        // Get ahead/behind information
        if let Ok(output) = std::process::Command::new("git")
            .args(&["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .current_dir(&working_dir)
            .output()
        {
            if output.status.success() {
                let count_output = String::from_utf8_lossy(&output.stdout);
                let count_str = count_output.trim();
                if let Some((ahead, behind)) = parse_ahead_behind(count_str) {
                    repo_info.ahead = ahead;
                    repo_info.behind = behind;
                }
            }
        }
    }

    Ok(repo_info)
}

/// Get runtime/language version information
#[tauri::command]
pub async fn get_runtime_info(path: String) -> Result<RuntimeInfo, String> {
    let working_dir = path;
    
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
    runtime_info.project_type = detect_project_type(&working_dir);

    // Get Node.js version
    if let Ok(output) = std::process::Command::new("node").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            runtime_info.node_version = Some(version);
        }
    }

    // Get npm version
    if let Ok(output) = std::process::Command::new("npm").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            runtime_info.npm_version = Some(version);
        }
    }

    // Get Rust version
    if let Ok(output) = std::process::Command::new("rustc").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            runtime_info.rust_version = Some(version);
        }
    }

    // Get Python version
    if let Ok(output) = std::process::Command::new("python3").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            runtime_info.python_version = Some(version);
        }
    } else if let Ok(output) = std::process::Command::new("python").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            runtime_info.python_version = Some(version);
        }
    }

    // Get Go version
    if let Ok(output) = std::process::Command::new("go").args(&["version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Extract version number from "go version go1.21.0 darwin/amd64"
            if let Some(version_part) = version.split_whitespace().nth(2) {
                runtime_info.go_version = Some(version_part.to_string());
            }
        }
    }

    // Get Java version
    if let Ok(output) = std::process::Command::new("java").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Extract version from first line
            if let Some(line) = version.lines().next() {
                runtime_info.java_version = Some(line.to_string());
            }
        }
    } else if let Ok(output) = std::process::Command::new("java").args(&["-version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stderr).trim().to_string(); // Java outputs to stderr
            if let Some(line) = version.lines().next() {
                runtime_info.java_version = Some(line.to_string());
            }
        }
    }

    // Get Git version
    if let Ok(output) = std::process::Command::new("git").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            runtime_info.git_version = Some(version);
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
    
    if path.join("requirements.txt").exists() || 
       path.join("pyproject.toml").exists() || 
       path.join("setup.py").exists() ||
       path.join("Pipfile").exists() {
        return Some("python".to_string());
    }
    
    if path.join("pom.xml").exists() || 
       path.join("build.gradle").exists() || 
       path.join("build.gradle.kts").exists() {
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
pub async fn initialize_ml_system(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut model_manager = state.inner().model_manager.lock().await;
    
    match model_manager.load_model().await {
        Ok(_) => Ok("ML system initialized successfully".to_string()),
        Err(e) => Err(format!("Failed to initialize ML system: {}", e))
    }
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
            for entry in entries {
                if let Ok(entry) = entry {
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
        }
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    }
    
    // Sort with directories first, then files, both alphabetically
    children.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,    // Directories first
            (false, true) => std::cmp::Ordering::Greater, // Files second
            _ => a.name.cmp(&b.name),                      // Alphabetical within same type
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
    
    // Execute cd command in the terminal
    let command = format!("cd \"{}\"", new_path);
    match terminal_manager.execute_command(&session_id, &command).await {
        Ok(_) => Ok(new_path),
        Err(e) => Err(format!("Failed to change directory: {}", e)),
    }
}

/// Execute or open a file
#[tauri::command]
pub async fn execute_file(
    state: State<'_, AppState>,
    session_id: String,
    file_path: String,
) -> Result<String, String> {
    use std::path::Path;
    
    let path = Path::new(&file_path);
    let mut terminal_manager = state.inner().terminal_manager.lock().await;
    
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        
        let command = match ext.as_str() {
            // Executable scripts
            "sh" | "bash" => format!("bash \"{}\"", file_path),
            "py" => format!("python \"{}\"", file_path),
            "js" => format!("node \"{}\"", file_path),
            "ts" => format!("npx ts-node \"{}\"", file_path),
            "rs" => format!("cargo run --manifest-path \"{}\"", file_path),
            
            // Text files - open with default editor
            "txt" | "md" | "json" | "yaml" | "yml" | "toml" | "xml" | "html" | "css" | "scss" => {
                format!("open \"{}\"", file_path)
            },
            
            // Source code files - open with default editor
            "jsx" | "tsx" | "vue" | "svelte" | "php" | "rb" | "go" | "java" | "cpp" | "c" | "h" => {
                format!("open \"{}\"", file_path)
            },
            
            // Configuration files
            "env" | "gitignore" | "dockerfile" | "makefile" => {
                format!("open \"{}\"", file_path)
            },
            
            // Images and media - open with default application
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "pdf" | "mp4" | "mov" | "mp3" => {
                format!("open \"{}\"", file_path)
            },
            
            // Default: try to open with system default application
            _ => format!("open \"{}\"", file_path),
        };
        
        match terminal_manager.execute_command(&session_id, &command).await {
            Ok(_) => Ok(format!("Executed: {}", command)),
            Err(e) => Err(format!("Failed to execute file: {}", e)),
        }
    } else {
        // No extension - try to execute directly or open
        let command = if path.is_file() {
            // Check if file is executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&file_path) {
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o111 != 0 {
                        // File is executable
                        format!("\"{}\"", file_path)
                    } else {
                        format!("open \"{}\"", file_path)
                    }
                } else {
                    format!("open \"{}\"", file_path)
                }
            }
            #[cfg(not(unix))]
            {
                format!("\"{}\"", file_path)
            }
        } else {
            format!("open \"{}\"", file_path)
        };
        
        match terminal_manager.execute_command(&session_id, &command).await {
            Ok(_) => Ok(format!("Executed: {}", command)),
            Err(e) => Err(format!("Failed to execute file: {}", e)),
        }
    }
}

// Enhanced Context Commands for Intelligent Predictions

/// Get enhanced system context for intelligent predictions
#[tauri::command]
pub async fn get_enhanced_system_context(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<crate::ai::enhanced_context::SystemContext, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    let working_dir = terminal_manager.get_session(&session_id)
        .map(|session| session.working_directory.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());
    
    let mut context_provider = crate::ai::enhanced_context::EnhancedContextProvider::new();
    context_provider.get_system_context(&working_dir).await
}

/// Get learned workflow patterns
#[tauri::command]
pub async fn get_learned_workflow_patterns(
    _state: State<'_, AppState>,
) -> Result<Vec<crate::ai::enhanced_context::WorkflowPattern>, String> {
    // This would integrate with the learning engine to get patterns
    // For now, return empty vector as placeholder
    Ok(vec![])
}

/// Get recent command sequence for workflow detection
#[tauri::command]
pub async fn get_recent_command_sequence(
    state: State<'_, AppState>,
    _session_id: String,
    limit: usize,
) -> Result<Vec<String>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    let history = terminal_manager.get_command_history(Some(limit));
    Ok(history.into_iter().map(|cmd| cmd.command.clone()).collect())
}

/// Get proactive system suggestions
#[tauri::command]
pub async fn get_proactive_suggestions(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<crate::ai::enhanced_context::ProactiveSuggestion>, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    let working_dir = terminal_manager.get_session(&session_id)
        .map(|session| session.working_directory.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());
    
    let mut context_provider = crate::ai::enhanced_context::EnhancedContextProvider::new();
    let context = context_provider.get_system_context(&working_dir).await
        .map_err(|e| format!("Failed to get system context: {}", e))?;
    
    Ok(context_provider.get_proactive_suggestions(&context).await)
}

// Simple command execution for validation purposes
#[tauri::command]
pub async fn execute_simple_command(
    command: String,
    directory: Option<String>
) -> Result<String, String> {
    use std::process::Command;
    
    let working_dir = directory.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string())
    });
    
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", &command])
            .current_dir(&working_dir)
            .output()
    } else {
        Command::new("sh")
            .args(["-c", &command])
            .current_dir(&working_dir)
            .output()
    };
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if !stderr.is_empty() {
                Ok(format!("{}{}", stdout, stderr))
            } else {
                Ok(stdout.to_string())
            }
        }
        Err(e) => Err(format!("Failed to execute command: {}", e))
    }
}
