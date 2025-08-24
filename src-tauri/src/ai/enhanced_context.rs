// Enhanced System Context Provider for Intelligent Predictions
// src-tauri/src/ai/enhanced_context.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub working_directory: String,
    pub project_type: Option<String>,
    pub running_processes: Vec<String>,
    pub system_resources: SystemResources,
    pub recent_files: Vec<String>,
    pub git_status: Option<GitStatus>,
    pub environment_variables: HashMap<String, String>,
    pub network_interfaces: Vec<NetworkInterface>,
    pub installed_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub cpu: f32,
    pub memory: f32,
    pub disk: f32,
    pub load_average: Vec<f32>,
    pub process_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub has_changes: bool,
    pub ahead: i32,
    pub behind: i32,
    pub last_commit: String,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPattern {
    pub pattern: Vec<String>,
    pub frequency: u32,
    pub context: String,
    pub success_rate: f32,
    pub next_likely_commands: Vec<String>,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveSuggestion {
    pub suggestion_type: String,
    pub priority: f32,
    pub description: String,
    pub commands: Vec<String>,
    pub trigger_condition: String,
}

pub struct EnhancedContextProvider {
    cache_ttl: u64,
    last_update: u64,
    cached_context: Option<SystemContext>,
}

impl EnhancedContextProvider {
    pub fn new() -> Self {
        Self {
            cache_ttl: 5000, // 5 seconds
            last_update: 0,
            cached_context: None,
        }
    }

    pub async fn get_system_context(&mut self, working_dir: &str) -> Result<SystemContext, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Return cached context if still valid
        if let Some(ref context) = self.cached_context {
            if now - self.last_update < self.cache_ttl {
                return Ok(context.clone());
            }
        }

        // Gather fresh context
        let context = SystemContext {
            working_directory: working_dir.to_string(),
            project_type: self.detect_project_type(working_dir),
            running_processes: self.get_running_processes().await,
            system_resources: self.get_system_resources().await,
            recent_files: self.get_recent_files(working_dir).await,
            git_status: self.get_git_status(working_dir).await,
            environment_variables: self.get_relevant_env_vars(),
            network_interfaces: self.get_network_interfaces().await,
            installed_tools: self.get_installed_tools().await,
        };

        self.cached_context = Some(context.clone());
        self.last_update = now;

        Ok(context)
    }

    fn detect_project_type(&self, working_dir: &str) -> Option<String> {
        let path = PathBuf::from(working_dir);

        // Node.js project
        if path.join("package.json").exists() {
            return Some("node".to_string());
        }

        // Rust project
        if path.join("Cargo.toml").exists() {
            return Some("rust".to_string());
        }

        // Python project
        if path.join("requirements.txt").exists() || 
           path.join("setup.py").exists() || 
           path.join("pyproject.toml").exists() {
            return Some("python".to_string());
        }

        // Go project
        if path.join("go.mod").exists() {
            return Some("go".to_string());
        }

        // Docker project
        if path.join("Dockerfile").exists() || path.join("docker-compose.yml").exists() {
            return Some("docker".to_string());
        }

        // Java/Maven project
        if path.join("pom.xml").exists() {
            return Some("maven".to_string());
        }

        // Java/Gradle project
        if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
            return Some("gradle".to_string());
        }

        None
    }

    async fn get_running_processes(&self) -> Vec<String> {
        let output = Command::new("ps")
            .args(&["aux", "--sort=-%cpu"])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout
                    .lines()
                    .skip(1) // Skip header
                    .take(10) // Top 10 processes
                    .map(|line| {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 10 {
                            format!("{} ({}%)", parts[10], parts[2])
                        } else {
                            line.to_string()
                        }
                    })
                    .collect()
            }
            Err(_) => vec![]
        }
    }

    async fn get_system_resources(&self) -> SystemResources {
        let cpu = self.get_cpu_usage().await;
        let memory = self.get_memory_usage().await;
        let disk = self.get_disk_usage().await;
        let load_average = self.get_load_average().await;
        let process_count = self.get_process_count().await;

        SystemResources {
            cpu,
            memory,
            disk,
            load_average,
            process_count,
        }
    }

    async fn get_cpu_usage(&self) -> f32 {
        // Get CPU usage via top command
        let output = Command::new("top")
            .args(&["-l", "1", "-n", "0"])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("CPU usage:") {
                        // Parse macOS top output: "CPU usage: 5.0% user, 2.5% sys, 92.5% idle"
                        if let Some(start) = line.find("CPU usage: ") {
                            let rest = &line[start + 11..];
                            if let Some(end) = rest.find('%') {
                                if let Ok(cpu) = rest[..end].parse::<f32>() {
                                    return cpu;
                                }
                            }
                        }
                    }
                }
                0.0
            }
            Err(_) => 0.0
        }
    }

    async fn get_memory_usage(&self) -> f32 {
        let output = Command::new("vm_stat")
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut pages_free = 0u64;
                let mut pages_active = 0u64;
                let mut pages_inactive = 0u64;
                let mut pages_wired = 0u64;

                for line in stdout.lines() {
                    if line.starts_with("Pages free:") {
                        pages_free = line.split_whitespace().nth(2)
                            .and_then(|s| s.trim_end_matches('.').parse().ok())
                            .unwrap_or(0);
                    } else if line.starts_with("Pages active:") {
                        pages_active = line.split_whitespace().nth(2)
                            .and_then(|s| s.trim_end_matches('.').parse().ok())
                            .unwrap_or(0);
                    } else if line.starts_with("Pages inactive:") {
                        pages_inactive = line.split_whitespace().nth(2)
                            .and_then(|s| s.trim_end_matches('.').parse().ok())
                            .unwrap_or(0);
                    } else if line.starts_with("Pages wired down:") {
                        pages_wired = line.split_whitespace().nth(3)
                            .and_then(|s| s.trim_end_matches('.').parse().ok())
                            .unwrap_or(0);
                    }
                }

                let total_used = pages_active + pages_inactive + pages_wired;
                let total_pages = total_used + pages_free;
                
                if total_pages > 0 {
                    (total_used as f32 / total_pages as f32) * 100.0
                } else {
                    0.0
                }
            }
            Err(_) => 0.0
        }
    }

    async fn get_disk_usage(&self) -> f32 {
        let output = Command::new("df")
            .args(&["-h", "/"])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines().skip(1) { // Skip header
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        if let Ok(usage) = parts[4].trim_end_matches('%').parse::<f32>() {
                            return usage;
                        }
                    }
                }
                0.0
            }
            Err(_) => 0.0
        }
    }

    async fn get_load_average(&self) -> Vec<f32> {
        let output = Command::new("uptime")
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(load_start) = stdout.find("load averages: ") {
                    let load_str = &stdout[load_start + 15..];
                    load_str.split_whitespace()
                        .take(3)
                        .map(|s| s.parse::<f32>().unwrap_or(0.0))
                        .collect()
                } else {
                    vec![0.0, 0.0, 0.0]
                }
            }
            Err(_) => vec![0.0, 0.0, 0.0]
        }
    }

    async fn get_process_count(&self) -> u32 {
        let output = Command::new("ps")
            .args(&["aux"])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().count().saturating_sub(1) as u32 // Subtract header
            }
            Err(_) => 0
        }
    }

    async fn get_recent_files(&self, working_dir: &str) -> Vec<String> {
        let output = Command::new("find")
            .args(&[working_dir, "-type", "f", "-mtime", "-1", "-not", "-path", "*/.*"])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout
                    .lines()
                    .take(20) // Limit to 20 recent files
                    .map(|s| s.to_string())
                    .collect()
            }
            Err(_) => vec![]
        }
    }

    async fn get_git_status(&self, working_dir: &str) -> Option<GitStatus> {
        let git_dir = PathBuf::from(working_dir).join(".git");
        if !git_dir.exists() {
            return None;
        }

        let mut git_status = GitStatus {
            branch: String::new(),
            has_changes: false,
            ahead: 0,
            behind: 0,
            last_commit: String::new(),
            remote_url: None,
        };

        // Get current branch
        if let Ok(output) = Command::new("git")
            .args(&["branch", "--show-current"])
            .current_dir(working_dir)
            .output() {
            git_status.branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        // Check for changes
        if let Ok(output) = Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(working_dir)
            .output() {
            git_status.has_changes = !output.stdout.is_empty();
        }

        // Get ahead/behind status
        if let Ok(output) = Command::new("git")
            .args(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .current_dir(working_dir)
            .output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = stdout.trim().split('\t').collect();
            if parts.len() >= 2 {
                git_status.ahead = parts[0].parse().unwrap_or(0);
                git_status.behind = parts[1].parse().unwrap_or(0);
            }
        }

        // Get last commit
        if let Ok(output) = Command::new("git")
            .args(&["log", "-1", "--pretty=format:%h %s"])
            .current_dir(working_dir)
            .output() {
            git_status.last_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        // Get remote URL
        if let Ok(output) = Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .current_dir(working_dir)
            .output() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !url.is_empty() {
                git_status.remote_url = Some(url);
            }
        }

        Some(git_status)
    }

    fn get_relevant_env_vars(&self) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();
        
        // Important environment variables for development
        let important_vars = [
            "PATH", "HOME", "USER", "SHELL", "TERM", "PWD",
            "LANG", "LC_ALL", "EDITOR", "VISUAL",
            "NODE_ENV", "PYTHON_PATH", "GOPATH", "CARGO_HOME",
            "DOCKER_HOST", "KUBECONFIG"
        ];

        for var in &important_vars {
            if let Ok(value) = std::env::var(var) {
                env_vars.insert(var.to_string(), value);
            }
        }

        env_vars
    }

    async fn get_network_interfaces(&self) -> Vec<NetworkInterface> {
        let output = Command::new("ifconfig")
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut interfaces = Vec::new();
                let mut current_interface = String::new();
                let mut current_ip = String::new();

                for line in stdout.lines() {
                    if !line.starts_with('\t') && !line.starts_with(' ') {
                        // New interface
                        if !current_interface.is_empty() {
                            interfaces.push(NetworkInterface {
                                name: current_interface.clone(),
                                ip: current_ip.clone(),
                                status: "active".to_string(),
                            });
                        }
                        current_interface = line.split(':').next().unwrap_or("").to_string();
                        current_ip = String::new();
                    } else if line.contains("inet ") {
                        // Extract IP address
                        if let Some(start) = line.find("inet ") {
                            let rest = &line[start + 5..];
                            if let Some(end) = rest.find(' ') {
                                current_ip = rest[..end].to_string();
                            } else {
                                current_ip = rest.to_string();
                            }
                        }
                    }
                }

                // Add the last interface
                if !current_interface.is_empty() {
                    interfaces.push(NetworkInterface {
                        name: current_interface,
                        ip: current_ip,
                        status: "active".to_string(),
                    });
                }

                interfaces
            }
            Err(_) => vec![]
        }
    }

    async fn get_installed_tools(&self) -> Vec<String> {
        let mut tools = Vec::new();
        
        let common_tools = [
            "git", "node", "npm", "yarn", "python", "python3", "pip", "pip3",
            "cargo", "rustc", "go", "java", "javac", "docker", "docker-compose",
            "kubectl", "helm", "terraform", "ansible", "vim", "nvim", "emacs",
            "code", "curl", "wget", "jq", "yq", "sed", "awk", "grep", "find",
            "rsync", "ssh", "scp", "htop", "tree", "fd", "ripgrep", "bat"
        ];

        for tool in &common_tools {
            if Command::new("which").arg(tool).output().is_ok() {
                tools.push(tool.to_string());
            }
        }

        tools
    }

    pub async fn get_proactive_suggestions(&self, context: &SystemContext) -> Vec<ProactiveSuggestion> {
        let mut suggestions = Vec::new();

        // Disk space warning
        if context.system_resources.disk > 90.0 {
            suggestions.push(ProactiveSuggestion {
                suggestion_type: "maintenance".to_string(),
                priority: 0.9,
                description: "Disk space is running low".to_string(),
                commands: vec![
                    "du -sh * | sort -hr | head -10".to_string(),
                    "find . -name '*.log' -size +10M".to_string(),
                    "docker system prune".to_string(),
                ],
                trigger_condition: "disk_usage > 90%".to_string(),
            });
        }

        // High CPU usage
        if context.system_resources.cpu > 85.0 {
            suggestions.push(ProactiveSuggestion {
                suggestion_type: "performance".to_string(),
                priority: 0.8,
                description: "High CPU usage detected".to_string(),
                commands: vec![
                    "ps aux --sort=-%cpu | head -10".to_string(),
                    "top -o cpu".to_string(),
                ],
                trigger_condition: "cpu_usage > 85%".to_string(),
            });
        }

        // Git repository with uncommitted changes
        if let Some(ref git_status) = context.git_status {
            if git_status.has_changes {
                suggestions.push(ProactiveSuggestion {
                    suggestion_type: "git_workflow".to_string(),
                    priority: 0.7,
                    description: "You have uncommitted changes".to_string(),
                    commands: vec![
                        "git status".to_string(),
                        "git add . && git commit -m 'WIP: '".to_string(),
                        "git stash".to_string(),
                    ],
                    trigger_condition: "git_has_changes".to_string(),
                });
            }
        }

        suggestions
    }
}
