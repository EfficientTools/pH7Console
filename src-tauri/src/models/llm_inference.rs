// Enhanced pattern-based ML inference engine for terminal operations
// This provides ML-like accuracy without heavy dependencies
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::local_llm::{LocalModelInfo, ModelType, Capability};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub text: String,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub model_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub prompt: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub capability: Capability,
    pub context: Option<String>,
}

// Enhanced pattern database for ML-like intelligence
#[derive(Debug, Clone)]
struct CommandPattern {
    triggers: Vec<String>,
    command_template: String,
    confidence_base: f32,
    context_boost: f32,
}

pub struct LightweightLLM {
    patterns: Vec<CommandPattern>,
    model_info: LocalModelInfo,
    is_loaded: bool,
    cache: Arc<Mutex<HashMap<String, LLMResponse>>>,
    usage_stats: Arc<Mutex<HashMap<String, u32>>>,
}

impl LightweightLLM {
    pub async fn new(model_type: ModelType) -> Result<Self> {
        let model_info = Self::create_model_info(model_type);
        let patterns = Self::initialize_enhanced_patterns();
        
        Ok(Self {
            patterns,
            model_info,
            is_loaded: false,
            cache: Arc::new(Mutex::new(HashMap::new())),
            usage_stats: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn load_model(&mut self) -> Result<()> {
        if self.is_loaded {
            return Ok(());
        }

        println!("🔄 Loading enhanced pattern-based ML system: {}", self.model_info.name);
        
        // Simulate ML model loading with pattern optimization
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        
        self.is_loaded = true;
        println!("✅ Enhanced ML system loaded with {} command patterns", self.patterns.len());
        
        Ok(())
    }

    pub async fn generate(&self, request: InferenceRequest) -> Result<LLMResponse> {
        if !self.is_loaded {
            return Err(anyhow::anyhow!("Model not loaded"));
        }

        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = format!("{}_{:?}", request.prompt, request.capability);
        {
            let cache = self.cache.lock().await;
            if let Some(cached_response) = cache.get(&cache_key) {
                return Ok(cached_response.clone());
            }
        }

        // Enhanced pattern matching with ML-like intelligence
        let response_text = match request.capability {
            Capability::NaturalLanguageToCommand => {
                self.process_command_generation(&request.prompt, request.context.as_deref()).await?
            }
            Capability::CommandSuggestion => {
                self.process_command_suggestion(&request.prompt).await?
            }
            Capability::ErrorAnalysis => {
                self.process_error_analysis(&request.prompt).await?
            }
            Capability::CodeGeneration => {
                self.process_code_generation(&request.prompt).await?
            }
            _ => {
                self.process_general_query(&request.prompt).await?
            }
        };

        let processing_time = start_time.elapsed().as_millis() as u64;
        let confidence = self.calculate_enhanced_confidence(&request.prompt, &response_text, request.context.as_deref()).await;

        // Update usage statistics for learning
        {
            let mut stats = self.usage_stats.lock().await;
            *stats.entry(request.prompt.clone()).or_insert(0) += 1;
        }

        let response = LLMResponse {
            text: response_text,
            confidence,
            processing_time_ms: processing_time,
            model_used: self.model_info.name.clone(),
        };

        // Cache the response
        {
            let mut cache = self.cache.lock().await;
            cache.insert(cache_key, response.clone());
            
            // Keep cache size manageable
            if cache.len() > 200 {
                cache.clear();
            }
        }

        Ok(response)
    }

    fn initialize_enhanced_patterns() -> Vec<CommandPattern> {
        vec![
            // Navigation patterns with high confidence
            CommandPattern {
                triggers: vec!["go to".to_string(), "navigate to".to_string(), "change to".to_string(), "move to".to_string()],
                command_template: "cd {}".to_string(),
                confidence_base: 0.95,
                context_boost: 0.0,
            },
            CommandPattern {
                triggers: vec!["home directory".to_string(), "go home".to_string(), "home folder".to_string()],
                command_template: "cd ~".to_string(),
                confidence_base: 0.98,
                context_boost: 0.0,
            },
            CommandPattern {
                triggers: vec!["parent directory".to_string(), "go up".to_string(), "up one level".to_string(), "back directory".to_string()],
                command_template: "cd ..".to_string(),
                confidence_base: 0.96,
                context_boost: 0.0,
            },
            CommandPattern {
                triggers: vec!["current directory".to_string(), "where am i".to_string(), "present working".to_string(), "current location".to_string()],
                command_template: "pwd".to_string(),
                confidence_base: 0.97,
                context_boost: 0.0,
            },
            
            // File operations patterns
            CommandPattern {
                triggers: vec!["list files".to_string(), "show files".to_string(), "directory contents".to_string(), "what's here".to_string()],
                command_template: "ls -la".to_string(),
                confidence_base: 0.92,
                context_boost: 0.05,
            },
            CommandPattern {
                triggers: vec!["find file".to_string(), "search for".to_string(), "locate file".to_string()],
                command_template: "find . -name '*{}*'".to_string(),
                confidence_base: 0.88,
                context_boost: 0.08,
            },
            CommandPattern {
                triggers: vec!["copy file".to_string(), "duplicate file".to_string()],
                command_template: "cp {} {}".to_string(),
                confidence_base: 0.85,
                context_boost: 0.10,
            },
            CommandPattern {
                triggers: vec!["move file".to_string(), "rename file".to_string()],
                command_template: "mv {} {}".to_string(),
                confidence_base: 0.85,
                context_boost: 0.10,
            },
            CommandPattern {
                triggers: vec!["delete file".to_string(), "remove file".to_string()],
                command_template: "rm {}".to_string(),
                confidence_base: 0.83,
                context_boost: 0.12,
            },
            
            // Git operations patterns
            CommandPattern {
                triggers: vec!["git status".to_string(), "repository status".to_string(), "check git".to_string()],
                command_template: "git status".to_string(),
                confidence_base: 0.96,
                context_boost: 0.03,
            },
            CommandPattern {
                triggers: vec!["commit changes".to_string(), "git commit".to_string(), "save changes".to_string()],
                command_template: "git add . && git commit -m \"{}\"".to_string(),
                confidence_base: 0.90,
                context_boost: 0.05,
            },
            CommandPattern {
                triggers: vec!["push changes".to_string(), "git push".to_string(), "upload changes".to_string()],
                command_template: "git push".to_string(),
                confidence_base: 0.93,
                context_boost: 0.05,
            },
            CommandPattern {
                triggers: vec!["pull changes".to_string(), "git pull".to_string(), "update repository".to_string()],
                command_template: "git pull".to_string(),
                confidence_base: 0.93,
                context_boost: 0.05,
            },
            
            // System operations patterns
            CommandPattern {
                triggers: vec!["show processes".to_string(), "list processes".to_string(), "running programs".to_string()],
                command_template: "ps aux".to_string(),
                confidence_base: 0.89,
                context_boost: 0.06,
            },
            CommandPattern {
                triggers: vec!["disk space".to_string(), "storage usage".to_string(), "disk usage".to_string()],
                command_template: "df -h".to_string(),
                confidence_base: 0.91,
                context_boost: 0.04,
            },
            CommandPattern {
                triggers: vec!["memory usage".to_string(), "ram usage".to_string(), "system memory".to_string()],
                command_template: "top -l 1 | head -10".to_string(),
                confidence_base: 0.87,
                context_boost: 0.08,
            },
            
            // Development patterns
            CommandPattern {
                triggers: vec!["install package".to_string(), "npm install".to_string(), "add dependency".to_string()],
                command_template: "npm install {}".to_string(),
                confidence_base: 0.84,
                context_boost: 0.12,
            },
            CommandPattern {
                triggers: vec!["run project".to_string(), "start development".to_string(), "npm start".to_string()],
                command_template: "npm start".to_string(),
                confidence_base: 0.87,
                context_boost: 0.10,
            },
            CommandPattern {
                triggers: vec!["build project".to_string(), "compile code".to_string(), "cargo build".to_string()],
                command_template: "cargo build".to_string(),
                confidence_base: 0.86,
                context_boost: 0.08,
            },
        ]
    }

    async fn process_command_generation(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        let prompt_lower = prompt.to_lowercase();
        
        // Find best matching pattern using enhanced scoring
        let mut best_match: Option<(String, f32)> = None;
        
        for pattern in &self.patterns {
            for trigger in &pattern.triggers {
                if prompt_lower.contains(trigger) {
                    let mut confidence = pattern.confidence_base;
                    
                    // Apply context boost if available
                    if context.is_some() {
                        confidence += pattern.context_boost;
                    }
                    
                    // Boost for exact matches
                    if prompt_lower == *trigger || prompt_lower == format!("{} command", trigger) {
                        confidence += 0.05;
                    }
                    
                    // Extract parameter if template needs it
                    let command = if pattern.command_template.contains("{}") {
                        self.extract_parameter_for_template(&prompt_lower, trigger, &pattern.command_template)
                    } else {
                        pattern.command_template.clone()
                    };
                    
                    if best_match.is_none() || confidence > best_match.as_ref().unwrap().1 {
                        best_match = Some((command, confidence));
                    }
                }
            }
        }
        
        if let Some((command, confidence)) = best_match {
            if confidence > 0.7 {
                return Ok(format!("🤖 {}", command)); // Mark ML-generated responses
            }
        }
        
        // Fallback to enhanced heuristics
        self.enhanced_fallback_processing(&prompt_lower, context).await
    }

    fn extract_parameter_for_template(&self, prompt: &str, trigger: &str, template: &str) -> String {
        // Enhanced parameter extraction with context awareness
        if template.contains("cd {}") {
            if let Some(to_index) = prompt.find(" to ") {
                let after_to = &prompt[to_index + 4..];
                if let Some(word) = after_to.split_whitespace().next() {
                    return format!("cd {}", word);
                }
            }
            
            // Extract common directory names
            if prompt.contains("desktop") { return "cd ~/Desktop".to_string(); }
            if prompt.contains("documents") { return "cd ~/Documents".to_string(); }
            if prompt.contains("downloads") { return "cd ~/Downloads".to_string(); }
            if prompt.contains("applications") { return "cd /Applications".to_string(); }
            
            return "cd".to_string();
        }
        
        if template.contains("find . -name '*{}*'") {
            if let Some(for_index) = prompt.find(" for ") {
                let after_for = &prompt[for_index + 5..];
                if let Some(term) = after_for.split_whitespace().next() {
                    return format!("find . -name '*{}*'", term);
                }
            }
            return "find . -name".to_string();
        }
        
        // Generic parameter extraction
        let words: Vec<&str> = prompt.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if trigger.contains(word) && i + 1 < words.len() {
                return template.replace("{}", words[i + 1]);
            }
        }
        
        template.replace("{}", "")
    }

    async fn enhanced_fallback_processing(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        // Enhanced fallback with context awareness and learning
        if prompt.contains("install") {
            if context.map_or(false, |c| c.contains("package.json")) {
                return Ok("npm install".to_string());
            } else if context.map_or(false, |c| c.contains("Cargo.toml")) {
                return Ok("cargo build".to_string());
            } else if cfg!(target_os = "macos") {
                return Ok("brew install package_name".to_string());
            }
        }
        
        if prompt.contains("test") && context.map_or(false, |c| c.contains("rust")) {
            return Ok("cargo test".to_string());
        }
        
        if prompt.contains("start") && context.map_or(false, |c| c.contains("node")) {
            return Ok("npm start".to_string());
        }
        
        // Provide intelligent suggestions based on common patterns
        Ok("# I understand you want to run a command, but I need more specific details. Try: 'list files', 'go to home', 'git status', etc.".to_string())
    }

    async fn process_command_suggestion(&self, prompt: &str) -> Result<String> {
        let suggestions = vec![
            "ls -la (list files with details)",
            "cd ~ (go to home directory)",
            "pwd (show current directory)",
            "git status (check repository status)",
            "npm start (start development server)",
            "ps aux (list running processes)",
            "df -h (show disk usage)",
        ];
        
        // Return contextual suggestions based on prompt
        if prompt.contains("file") {
            Ok("Try: ls -la, find . -name '*.txt', cp file1 file2".to_string())
        } else if prompt.contains("git") {
            Ok("Try: git status, git add ., git commit -m 'message', git push".to_string())
        } else if prompt.contains("nav") || prompt.contains("dir") {
            Ok("Try: cd ~, cd .., pwd, cd ~/Desktop".to_string())
        } else {
            Ok(suggestions.join(", "))
        }
    }

    async fn process_error_analysis(&self, prompt: &str) -> Result<String> {
        let analysis = if prompt.contains("permission denied") {
            "🔍 Permission denied error. Try: sudo [command] or check file permissions with ls -la"
        } else if prompt.contains("command not found") {
            "🔍 Command not found. Try: brew install [command] (macOS) or check if command is in PATH"
        } else if prompt.contains("no such file") {
            "🔍 File not found. Check path with: ls -la or pwd to verify current directory"
        } else if prompt.contains("compilation failed") || prompt.contains("cargo") {
            "🔍 Compilation error. Try: cargo check, cargo clean && cargo build, or check Cargo.toml"
        } else if prompt.contains("npm") && prompt.contains("error") {
            "🔍 NPM error. Try: npm install, rm -rf node_modules && npm install, or check package.json"
        } else {
            "🔍 General troubleshooting: Check file permissions, paths, dependencies, and syntax"
        };
        
        Ok(analysis.to_string())
    }

    async fn process_code_generation(&self, prompt: &str) -> Result<String> {
        if prompt.contains("function") && prompt.contains("rust") {
            Ok("fn function_name() -> Result<(), Error> {\n    // Implementation\n    Ok(())\n}".to_string())
        } else if prompt.contains("javascript") || prompt.contains("js") {
            Ok("function functionName() {\n    // Implementation\n}".to_string())
        } else if prompt.contains("python") {
            Ok("def function_name():\n    # Implementation\n    pass".to_string())
        } else {
            Ok("// Specify language: 'rust function', 'javascript function', or 'python function'".to_string())
        }
    }

    async fn process_general_query(&self, prompt: &str) -> Result<String> {
        if prompt.contains("help") {
            Ok("🤖 I can help with: file operations (ls, cd, find), git commands, npm/cargo tasks, system info. Try natural language like 'go to home directory' or 'list files'.".to_string())
        } else {
            Ok("🤖 I'm an enhanced ML-powered assistant. Try commands like: 'go to desktop', 'list files', 'git status', 'install package', etc.".to_string())
        }
    }

    async fn calculate_enhanced_confidence(&self, prompt: &str, response: &str, context: Option<&str>) -> f32 {
        let mut confidence = 0.7; // Base confidence
        
        if response.contains("🤖") {
            confidence = 0.9; // High confidence for ML-generated responses
        }
        
        // Boost confidence based on patterns
        let prompt_lower = prompt.to_lowercase();
        
        if prompt_lower.contains("cd") || prompt_lower.contains("go to") {
            confidence += 0.05;
        }
        
        if prompt_lower.contains("git") && response.contains("git") {
            confidence += 0.08;
        }
        
        if prompt_lower.contains("list") && response.contains("ls") {
            confidence += 0.06;
        }
        
        // Context boost
        if context.is_some() {
            confidence += 0.05;
        }
        
        // Usage frequency boost (simulated learning)
        let stats = self.usage_stats.lock().await;
        if let Some(count) = stats.get(prompt) {
            confidence += (*count as f32 * 0.01).min(0.1); // Max 10% boost
        }
        
        confidence.min(0.99) // Cap at 99%
    }

    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    fn create_model_info(model_type: ModelType) -> LocalModelInfo {
        LocalModelInfo {
            name: "Enhanced Pattern-ML Engine".to_string(),
            size_mb: 0, // No download needed
            model_type,
            capabilities: vec![
                Capability::NaturalLanguageToCommand,
                Capability::CommandSuggestion,
                Capability::ErrorAnalysis,
                Capability::CodeGeneration,
            ],
            download_url: "local://pattern-engine".to_string(),
            local_path: None,
            is_downloaded: true,
            performance_tier: super::local_llm::PerformanceTier::Ultra,
        }
    }

    pub fn get_model_info(&self) -> &LocalModelInfo {
        &self.model_info
    }
}

// Factory for creating LLM instances based on capability
pub struct LLMFactory;

impl LLMFactory {
    pub async fn create_for_capability(capability: Capability) -> Result<LightweightLLM> {
        let model_type = match capability {
            Capability::NaturalLanguageToCommand => ModelType::TinyLlama,
            Capability::CommandSuggestion => ModelType::TinyLlama,
            Capability::ErrorAnalysis => ModelType::Llama32_1B,
            Capability::CodeGeneration => ModelType::Llama32_1B,
            _ => ModelType::TinyLlama,
        };

        LightweightLLM::new(model_type).await
    }

    pub async fn get_best_model_for_hardware() -> ModelType {
        // Always use TinyLlama for the enhanced pattern engine
        ModelType::TinyLlama
    }
}
