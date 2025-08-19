use crate::{AppState, ai};
use crate::ai::{AIResponse};
use crate::terminal::CommandExecution;
use tauri::State;

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
        
        model_manager.learn_from_command(
            &command, // Use original command for learning
            &execution.output,
            &context,
            success,
            Some(execution.duration_ms),
        ).await;
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
    
    Ok(model_manager.get_smart_completions(&partial_command, &context).await)
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

/// Repository information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoInfo {
    pub is_git_repo: bool,
    pub branch: Option<String>,
    pub repo_name: Option<String>,
    pub remote_url: Option<String>,
    pub has_changes: bool,
    pub ahead_behind: Option<(i32, i32)>, // (ahead, behind)
}

/// Language/runtime information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeInfo {
    pub node_version: Option<String>,
    pub npm_version: Option<String>,
    pub rust_version: Option<String>,
    pub python_version: Option<String>,
    pub git_version: Option<String>,
}

/// Get repository information for the current directory
#[tauri::command]
pub async fn get_repo_info(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<RepoInfo, String> {
    let terminal_manager = state.inner().terminal_manager.lock().await;
    
    // Get current working directory for the session
    let sessions = &terminal_manager.sessions;
    let working_dir = if let Some(session) = sessions.get(&session_id) {
        session.working_directory.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string()
    };

    let mut repo_info = RepoInfo {
        is_git_repo: false,
        branch: None,
        repo_name: None,
        remote_url: None,
        has_changes: false,
        ahead_behind: None,
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
                    repo_info.branch = Some(branch);
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
                    repo_info.repo_name = Some(repo_name);
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
                let count_output = String::from_utf8_lossy(&output.stdout).trim();
                if let Some((ahead, behind)) = parse_ahead_behind(count_output) {
                    repo_info.ahead_behind = Some((ahead, behind));
                }
            }
        }
    }

    Ok(repo_info)
}

/// Get runtime/language version information
#[tauri::command]
pub async fn get_runtime_info() -> Result<RuntimeInfo, String> {
    let mut runtime_info = RuntimeInfo {
        node_version: None,
        npm_version: None,
        rust_version: None,
        python_version: None,
        git_version: None,
    };

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

    // Get Git version
    if let Ok(output) = std::process::Command::new("git").args(&["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            runtime_info.git_version = Some(version);
        }
    }

    Ok(runtime_info)
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
