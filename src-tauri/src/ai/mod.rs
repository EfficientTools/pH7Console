// Advanced AI module with real machine learning capabilities
// Integrates learning engine and intelligent agent mode

pub mod learning_engine;
pub mod agent;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

use learning_engine::LearningEngine;
use agent::IntelligentAgent;
use crate::models::{LightweightLLM, LLMFactory, InferenceRequest, Capability};

// Re-export public types
pub use learning_engine::UserAnalytics;
pub use agent::TaskStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    pub text: String,
    pub confidence: f32,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub model_path: PathBuf,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_name: "microsoft/Phi-3-mini-4k-instruct".to_string(),
            model_path: PathBuf::from("models/phi3-mini"),
            max_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
        }
    }
}

pub struct ModelManager {
    learning_engine: Arc<Mutex<LearningEngine>>,
    agent: Arc<Mutex<IntelligentAgent>>,
    llm_engine: Arc<Mutex<Option<LightweightLLM>>>,
    config: ModelConfig,
    is_loaded: bool,
    data_directory: PathBuf,
}

impl ModelManager {
    pub fn new() -> Self {
        // Setup data directory for learning engine
        let data_directory = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("ai_data");
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&data_directory).ok();
        
        // Initialize learning engine
        let learning_engine = Arc::new(Mutex::new(LearningEngine::new(data_directory.clone())));
        
        // Initialize intelligent agent
        let agent = {
            let engine = learning_engine.clone();
            Arc::new(Mutex::new(IntelligentAgent::new(
                // We'll need to clone the learning engine data for the agent
                LearningEngine::new(data_directory.clone())
            )))
        };
        
        Self {
            learning_engine,
            agent,
            llm_engine: Arc::new(Mutex::new(None)),
            config: ModelConfig::default(),
            is_loaded: false,
            data_directory,
        }
    }

    pub async fn load_model(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_loaded {
            return Ok(());
        }

        println!("🔄 Loading AI learning system with lightweight LLM: {}", self.config.model_name);
        
        // Initialize the enhanced pattern-based LLM
        let mut llm = LLMFactory::create_for_capability(Capability::NaturalLanguageToCommand).await?;
        
        // Load the model
        llm.load_model().await?;
        
        // Store the LLM instance
        {
            let mut llm_engine = self.llm_engine.lock().await;
            *llm_engine = Some(llm);
        }

        // Initialize the learning system
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        self.is_loaded = true;
        println!("✅ AI learning system with lightweight LLM loaded successfully");
        println!("🧠 Ready to process natural language commands with ML accuracy");
        
        Ok(())
    }

    pub async fn generate_response(&self, prompt: &str, context: Option<&str>) -> AIResponse {
        if !self.is_loaded {
            return AIResponse {
                text: "AI learning system not loaded".to_string(),
                confidence: 0.0,
                reasoning: Some("AI learning system needs to be initialized".to_string()),
            };
        }

        // Use learning engine for intelligent responses
        let learning_engine = self.learning_engine.lock().await;
        
        // Check if this is a request for command suggestions
        if prompt.contains("suggest command") || prompt.contains("what command") {
            let context_str = context.unwrap_or("");
            let suggestions = learning_engine.suggest_commands(context_str, "", 3);
            
            if !suggestions.is_empty() {
                return AIResponse {
                    text: suggestions.join(", "),
                    confidence: 0.9,
                    reasoning: Some("Based on learned patterns and context".to_string()),
                };
            }
        }

        // Fall back to pattern-based responses with learning context
        self.generate_learned_response(prompt, context, &learning_engine).await
    }

    // Generate responses using learned patterns and enhanced heuristics
    async fn generate_learned_response(&self, prompt: &str, context: Option<&str>, learning_engine: &LearningEngine) -> AIResponse {
        let prompt_lower = prompt.to_lowercase();
        
        // Enhanced command suggestion logic
        let response = if prompt_lower.contains("suggest command") || prompt_lower.contains("recommend") {
            if let Some(ctx) = context {
                let suggestions = learning_engine.suggest_commands(ctx, "", 5);
                if !suggestions.is_empty() {
                    format!("Based on your usage patterns, I suggest: {}", suggestions.join(", "))
                } else {
                    self.fallback_command_suggestions(ctx)
                }
            } else {
                "I can suggest commands based on context. What are you trying to accomplish?".to_string()
            }
        } else if prompt_lower.contains("explain") {
            if let Some(ctx) = context {
                format!("This command appears to be related to {}. Based on similar commands you've used, it likely performs operations on files or data in your current context.", 
                    self.analyze_command_context(ctx))
            } else {
                "I can explain commands when you provide the specific command you'd like to understand.".to_string()
            }
        } else if prompt_lower.contains("error") || prompt_lower.contains("fix") {
            "Let me analyze the error. Common solutions include checking file permissions, ensuring required dependencies are installed, or verifying the command syntax. What specific error are you encountering?".to_string()
        } else if prompt_lower.contains("natural language") {
            // This will be handled differently - we'll need async processing here
            self.natural_language_to_command(prompt, context)
        } else {
            "I'm learning from your command patterns to provide better assistance. How can I help you with your terminal tasks?".to_string()
        };

        // Calculate confidence based on learning data
        let analytics = learning_engine.get_user_analytics();
        let confidence = if analytics.total_commands > 10 { 0.85 } else { 0.6 };

        AIResponse {
            text: response.to_string(),
            confidence,
            reasoning: Some(format!("Generated using {} learned patterns from {} commands", 
                analytics.patterns_learned, analytics.total_commands)),
        }
    }

    fn fallback_command_suggestions(&self, context: &str) -> String {
        if context.contains("git") {
            "git status, git add ., git commit -m \"message\", git push"
        } else if context.contains(".js") || context.contains(".ts") || context.contains("package.json") {
            "npm install, npm run dev, npm test, npm run build"
        } else if context.contains(".rs") || context.contains("Cargo.toml") {
            "cargo build, cargo test, cargo run, cargo check"
        } else if context.contains("docker") {
            "docker ps, docker build, docker run, docker-compose up"
        } else {
            "ls -la, pwd, cd .., find . -name, grep -r"
        }.to_string()
    }

    fn analyze_command_context(&self, context: &str) -> &str {
        if context.contains("git") { "version control" }
        else if context.contains("npm") || context.contains("node") { "Node.js development" }
        else if context.contains("cargo") || context.contains("rust") { "Rust development" }
        else if context.contains("docker") { "containerization" }
        else if context.contains("python") || context.contains(".py") { "Python development" }
        else { "file system operations" }
    }

    pub async fn natural_language_to_command_ml(&self, prompt: &str, context: Option<&str>) -> String {
        // Try ML-powered processing first
        if let Some(llm_result) = self.try_llm_processing(prompt, context).await {
            return llm_result;
        }
        
        // Fallback to pattern-based processing
        self.natural_language_to_command(prompt, context)
    }

    async fn try_llm_processing(&self, prompt: &str, context: Option<&str>) -> Option<String> {
        let llm_guard = self.llm_engine.lock().await;
        if let Some(ref llm) = *llm_guard {
            if llm.is_loaded() {
                let request = InferenceRequest {
                    prompt: prompt.to_string(),
                    max_tokens: Some(128),
                    temperature: Some(0.3), // Lower temperature for more deterministic command generation
                    capability: Capability::NaturalLanguageToCommand,
                    context: context.map(|s| s.to_string()),
                };

                if let Ok(response) = llm.generate(request).await {
                    // Only use LLM result if confidence is high enough
                    if response.confidence > 0.6 {
                        println!("🤖 LLM generated command with {:.1}% confidence: {}", 
                               response.confidence * 100.0, response.text);
                        return Some(response.text);
                    }
                }
            }
        }
        None
    }

    fn natural_language_to_command(&self, prompt: &str, context: Option<&str>) -> String {
        let prompt_lower = prompt.to_lowercase();
        
        // Enhanced natural language processing with more patterns
        
        // Navigation operations (prioritized first)
        if prompt_lower.contains("go to") || prompt_lower.contains("navigate to") || prompt_lower.contains("change to") {
            if prompt_lower.contains("home") {
                return "cd ~".to_string();
            } else if prompt_lower.contains("parent") || prompt_lower.contains("up") || prompt_lower.contains("..") {
                return "cd ..".to_string();
            } else if prompt_lower.contains("root") {
                return "cd /".to_string();
            } else if prompt_lower.contains("desktop") {
                return "cd ~/Desktop".to_string();
            } else if prompt_lower.contains("documents") {
                return "cd ~/Documents".to_string();
            } else if prompt_lower.contains("downloads") {
                return "cd ~/Downloads".to_string();
            } else if prompt_lower.contains("applications") {
                return "cd /Applications".to_string();
            } else if let Some(path) = self.extract_path_from_prompt(&prompt_lower) {
                return format!("cd {}", path);
            } else {
                return "cd directory_name".to_string();
            }
        } else if (prompt_lower.contains("cd") || prompt_lower.contains("change directory")) && !prompt_lower.contains("git") {
            if prompt_lower.contains("home") {
                return "cd ~".to_string();
            } else if prompt_lower.contains("back") || prompt_lower.contains("previous") {
                return "cd -".to_string();
            } else if prompt_lower.contains("parent") || prompt_lower.contains("up") {
                return "cd ..".to_string();
            } else if prompt_lower.contains("root") {
                return "cd /".to_string();
            } else {
                return "cd directory_name".to_string();
            }
        } else if prompt_lower.contains("where am i") || prompt_lower.contains("current directory") || prompt_lower.contains("pwd") {
            return "pwd".to_string();
        }
        
        // File operations
        else if prompt_lower.contains("show") || prompt_lower.contains("list") {
            if prompt_lower.contains("large files") || prompt_lower.contains("big files") {
                "find . -type f -size +100M -exec ls -lh {} \\; | sort -k5 -hr".to_string()
            } else if prompt_lower.contains("recent") || prompt_lower.contains("latest") {
                "ls -lt | head -10".to_string()
            } else if prompt_lower.contains("hidden") {
                "ls -la".to_string()
            } else if prompt_lower.contains("files") {
                "ls -la".to_string()
            } else if prompt_lower.contains("directories") || prompt_lower.contains("folders") {
                "ls -d */".to_string()
            } else {
                "ls -la".to_string()
            }
        } else if prompt_lower.contains("find") {
            if prompt_lower.contains("name") {
                if let Some(name) = self.extract_filename_from_prompt(&prompt_lower) {
                    format!("find . -name \"*{}*\" -type f", name)
                } else {
                    "find . -name \"*.txt\" -type f".to_string()
                }
            } else if prompt_lower.contains("empty") {
                if prompt_lower.contains("files") {
                    "find . -type f -empty".to_string()
                } else {
                    "find . -type d -empty".to_string()
                }
            } else if prompt_lower.contains("modified") {
                "find . -type f -mtime -1".to_string()
            } else {
                "find . -type f".to_string()
            }
        } else if prompt_lower.contains("delete") || prompt_lower.contains("remove") {
            if prompt_lower.contains("empty") {
                "find . -type f -empty -delete".to_string()
            } else {
                "rm -i filename".to_string()
            }
        } else if prompt_lower.contains("copy") {
            "cp source destination".to_string()
        } else if prompt_lower.contains("move") {
            "mv source destination".to_string()
        } else if prompt_lower.contains("create") {
            if prompt_lower.contains("directory") || prompt_lower.contains("folder") {
                "mkdir new_directory".to_string()
            } else {
                "touch new_file.txt".to_string()
            }
        }
        
        // System information
        else if prompt_lower.contains("disk space") || prompt_lower.contains("storage") {
            if prompt_lower.contains("usage") {
                "du -sh * | sort -hr".to_string()
            } else {
                "df -h".to_string()
            }
        } else if prompt_lower.contains("memory") || prompt_lower.contains("ram") {
            if prompt_lower.contains("usage") {
                "free -h && ps aux --sort=-%mem | head -10".to_string()
            } else {
                "free -h".to_string()
            }
        } else if prompt_lower.contains("cpu") || prompt_lower.contains("processor") {
            "top -bn1 | grep \"Cpu(s)\" && ps aux --sort=-%cpu | head -10".to_string()
        } else if prompt_lower.contains("processes") || prompt_lower.contains("running") {
            if prompt_lower.contains("port") {
                if let Some(port) = self.extract_port_from_prompt(&prompt_lower) {
                    format!("lsof -i :{}", port)
                } else {
                    "lsof -i".to_string()
                }
            } else {
                "ps aux | grep -v grep".to_string()
            }
        }
        
        // Git operations
        else if prompt_lower.contains("git") {
            if prompt_lower.contains("status") || prompt_lower.contains("changes") {
                "git status --porcelain -b".to_string()
            } else if prompt_lower.contains("commit") {
                if prompt_lower.contains("all") {
                    "git add . && git commit -m \"your message\"".to_string()
                } else {
                    "git commit -m \"your message\"".to_string()
                }
            } else if prompt_lower.contains("push") {
                "git push origin main".to_string()
            } else if prompt_lower.contains("pull") {
                "git pull origin main".to_string()
            } else if prompt_lower.contains("log") || prompt_lower.contains("history") {
                "git log --oneline -10".to_string()
            } else if prompt_lower.contains("branch") {
                "git branch -a".to_string()
            } else if prompt_lower.contains("diff") {
                "git diff".to_string()
            } else {
                "git status".to_string()
            }
        }
        
        // Network operations
        else if prompt_lower.contains("download") {
            if prompt_lower.contains("curl") {
                "curl -O URL".to_string()
            } else {
                "wget URL".to_string()
            }
        } else if prompt_lower.contains("ping") || prompt_lower.contains("connectivity") {
            if let Some(host) = self.extract_host_from_prompt(&prompt_lower) {
                format!("ping -c 4 {}", host)
            } else {
                "ping -c 4 google.com".to_string()
            }
        } else if prompt_lower.contains("port") && prompt_lower.contains("open") {
            "netstat -tuln | grep LISTEN".to_string()
        }
        
        // Text processing
        else if prompt_lower.contains("search") || prompt_lower.contains("grep") {
            if let Some(pattern) = self.extract_search_pattern(&prompt_lower) {
                format!("grep -r \"{}\" .", pattern)
            } else {
                "grep -r \"pattern\" .".to_string()
            }
        } else if prompt_lower.contains("count") {
            if prompt_lower.contains("lines") {
                "wc -l filename".to_string()
            } else if prompt_lower.contains("words") {
                "wc -w filename".to_string()
            } else if prompt_lower.contains("files") {
                "find . -type f | wc -l".to_string()
            } else {
                "wc filename".to_string()
            }
        } else if prompt_lower.contains("sort") {
            if prompt_lower.contains("size") {
                "ls -lS".to_string()
            } else if prompt_lower.contains("time") {
                "ls -lt".to_string()
            } else {
                "sort filename".to_string()
            }
        }
        
        // Development operations
        else if prompt_lower.contains("install") {
            if prompt_lower.contains("npm") || prompt_lower.contains("node") {
                "npm install".to_string()
            } else if prompt_lower.contains("python") || prompt_lower.contains("pip") {
                "pip install package_name".to_string()
            } else if prompt_lower.contains("rust") || prompt_lower.contains("cargo") {
                "cargo install package_name".to_string()
            } else {
                "sudo apt install package_name".to_string()
            }
        } else if prompt_lower.contains("build") {
            if prompt_lower.contains("npm") || prompt_lower.contains("node") {
                "npm run build".to_string()
            } else if prompt_lower.contains("rust") || prompt_lower.contains("cargo") {
                "cargo build".to_string()
            } else if prompt_lower.contains("make") {
                "make".to_string()
            } else {
                "npm run build".to_string()
            }
        } else if prompt_lower.contains("test") {
            if prompt_lower.contains("npm") || prompt_lower.contains("node") {
                "npm test".to_string()
            } else if prompt_lower.contains("rust") || prompt_lower.contains("cargo") {
                "cargo test".to_string()
            } else if prompt_lower.contains("python") {
                "python -m pytest".to_string()
            } else {
                "npm test".to_string()
            }
        } else if prompt_lower.contains("start") || prompt_lower.contains("run") {
            if prompt_lower.contains("dev") || prompt_lower.contains("development") {
                "npm run dev".to_string()
            } else if prompt_lower.contains("server") {
                "npm start".to_string()
            } else {
                "npm start".to_string()
            }
        }
        
        // Permission and ownership
        else if prompt_lower.contains("permission") || prompt_lower.contains("chmod") {
            if prompt_lower.contains("executable") {
                "chmod +x filename".to_string()
            } else if prompt_lower.contains("read") {
                "chmod 644 filename".to_string()
            } else {
                "chmod 755 filename".to_string()
            }
        } else if prompt_lower.contains("owner") || prompt_lower.contains("chown") {
            "sudo chown user:group filename".to_string()
        }
        
        // Archive operations
        else if prompt_lower.contains("compress") || prompt_lower.contains("zip") {
            if prompt_lower.contains("tar") {
                "tar -czf archive.tar.gz directory/".to_string()
            } else {
                "zip -r archive.zip directory/".to_string()
            }
        } else if prompt_lower.contains("extract") || prompt_lower.contains("unzip") {
            if prompt_lower.contains("tar") {
                "tar -xzf archive.tar.gz".to_string()
            } else {
                "unzip archive.zip".to_string()
            }
        }
        
        // Environment variables
        else if prompt_lower.contains("environment") || prompt_lower.contains("env") {
            if prompt_lower.contains("set") {
                "export VARIABLE=value".to_string()
            } else {
                "env | grep -i".to_string()
            }
        } else if prompt_lower.contains("path") && prompt_lower.contains("add") {
            "export PATH=$PATH:/new/path".to_string()
        }
        
        // Default fallback with context awareness
        else {
            // Try to infer from context
            if let Some(ctx) = context {
                if ctx.contains(".git") && prompt_lower.contains("status") {
                    "git status".to_string()
                } else if ctx.contains("package.json") && prompt_lower.contains("install") {
                    "npm install".to_string()
                } else if ctx.contains("Cargo.toml") && prompt_lower.contains("build") {
                    "cargo build".to_string()
                } else {
                    "# I need more specific details to convert that to a command. Try describing the exact task you want to accomplish.".to_string()
                }
            } else {
                "# I need more specific details to convert that to a command. Try describing the exact task you want to accomplish.".to_string()
            }
        }
    }

    // Helper methods for extracting information from natural language
    fn extract_filename_from_prompt(&self, prompt: &str) -> Option<String> {
        // Look for quoted strings or file extensions
        if let Some(start) = prompt.find('"') {
            if let Some(end) = prompt[start + 1..].find('"') {
                return Some(prompt[start + 1..start + 1 + end].to_string());
            }
        }
        
        // Look for common file extensions
        let extensions = ["txt", "log", "js", "ts", "rs", "py", "json", "xml", "html", "css"];
        for ext in extensions {
            if prompt.contains(&format!(".{}", ext)) {
                return Some(format!("*.{}", ext));
            }
        }
        
        None
    }

    fn extract_port_from_prompt(&self, prompt: &str) -> Option<String> {
        // Look for port numbers (typically 1-65535)
        let words: Vec<&str> = prompt.split_whitespace().collect();
        for word in words {
            if let Ok(port) = word.parse::<u16>() {
                if port > 0 && port <= 65535 {
                    return Some(port.to_string());
                }
            }
        }
        None
    }

    fn extract_host_from_prompt(&self, prompt: &str) -> Option<String> {
        // Look for domain names or IP addresses
        let words: Vec<&str> = prompt.split_whitespace().collect();
        for word in words {
            if word.contains('.') && !word.starts_with('.') {
                // Simple domain/IP detection
                if word.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
                    return Some(word.to_string());
                }
            }
        }
        None
    }

    fn extract_path_from_prompt(&self, prompt: &str) -> Option<String> {
        // Look for quoted paths
        if let Some(start) = prompt.find('"') {
            if let Some(end) = prompt[start + 1..].find('"') {
                let path = prompt[start + 1..start + 1 + end].to_string();
                if path.starts_with('/') || path.starts_with('~') || path.starts_with('.') {
                    return Some(path);
                }
            }
        }
        
        // Look for path-like strings (starts with /, ~, or .)
        let words: Vec<&str> = prompt.split_whitespace().collect();
        for word in words {
            if word.starts_with('/') || word.starts_with('~') || word.starts_with("./") || word.starts_with("../") {
                return Some(word.to_string());
            }
        }
        
        // Look for common directory names after "to"
        if let Some(to_index) = prompt.find(" to ") {
            let after_to = &prompt[to_index + 4..];
            let next_word = after_to.split_whitespace().next();
            if let Some(word) = next_word {
                // Don't include words that are likely not directories
                if !["the", "a", "an", "my", "this", "that"].contains(&word) {
                    return Some(word.to_string());
                }
            }
        }
        
        None
    }

    fn extract_search_pattern(&self, prompt: &str) -> Option<String> {
        // Look for quoted patterns
        if let Some(start) = prompt.find('"') {
            if let Some(end) = prompt[start + 1..].find('"') {
                return Some(prompt[start + 1..start + 1 + end].to_string());
            }
        }
        
        // Look for patterns after "for" or "search"
        let words: Vec<&str> = prompt.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if (*word == "for" || *word == "search") && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
        }
        
        None
    }

    /// Enhanced natural language processing using lightweight LLM
    pub async fn process_command_with_ml(&self, prompt: &str, context: Option<&str>) -> AIResponse {
        if !self.is_loaded {
            return AIResponse {
                text: "AI system not loaded. Please wait for initialization.".to_string(),
                confidence: 0.0,
                reasoning: Some("System not ready".to_string()),
            };
        }

        let start_time = std::time::Instant::now();
        
        // Try ML-powered processing first
        let command_result = self.natural_language_to_command_ml(prompt, context).await;
        
        let processing_time = start_time.elapsed().as_millis() as f32;
        let has_ml_marker = command_result.contains("🤖");
        
        AIResponse {
            text: command_result,
            confidence: if has_ml_marker { 0.9 } else { 0.7 },
            reasoning: Some(format!("Processed in {:.1}ms using {} approach", 
                          processing_time,
                          if has_ml_marker { "ML" } else { "pattern-based" })),
        }
    }

    pub fn is_model_loaded(&self) -> bool {
        self.is_loaded
    }

    pub async fn get_smart_completions(&self, partial_command: &str, context: &str) -> Vec<String> {
        if !self.is_loaded {
            return vec![];
        }

        let learning_engine = self.learning_engine.lock().await;
        learning_engine.get_smart_completions(partial_command, context)
    }

    /// Learn from user interactions
    pub async fn learn_from_command(
        &self,
        command: &str,
        output: &str,
        context: &str,
        success: bool,
        execution_time_ms: Option<u64>,
    ) {
        if self.is_loaded {
            let mut learning_engine = self.learning_engine.lock().await;
            learning_engine.learn_from_interaction(
                command.to_string(),
                output.to_string(),
                context.to_string(),
                success,
                execution_time_ms,
            );
        }
    }

    /// Update user feedback for learning
    pub async fn update_feedback(&self, command: &str, feedback: f32) {
        if self.is_loaded {
            let mut learning_engine = self.learning_engine.lock().await;
            learning_engine.update_feedback(command, feedback);
        }
    }

    /// Get user analytics
    pub async fn get_analytics(&self) -> Option<UserAnalytics> {
        if self.is_loaded {
            let learning_engine = self.learning_engine.lock().await;
            Some(learning_engine.get_user_analytics())
        } else {
            None
        }
    }

    /// Agent mode: Create autonomous task
    pub async fn create_agent_task(&self, description: &str) -> Result<String, String> {
        if !self.is_loaded {
            return Err("AI system not loaded".to_string());
        }

        let mut agent = self.agent.lock().await;
        agent.create_task_from_description(description).await
    }

    /// Get agent task status
    pub async fn get_agent_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let agent = self.agent.lock().await;
        agent.get_task_status(task_id)
    }

    /// Get all active agent tasks
    pub async fn get_active_agent_tasks(&self) -> Vec<String> {
        let agent = self.agent.lock().await;
        agent.get_active_tasks()
            .iter()
            .map(|task| format!("{}: {}", task.id, task.description))
            .collect()
    }

    /// Cancel agent task
    pub async fn cancel_agent_task(&self, task_id: &str) -> Result<(), String> {
        let mut agent = self.agent.lock().await;
        agent.cancel_task(task_id)
    }
}
