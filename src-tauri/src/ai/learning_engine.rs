use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Timelike};

/// Learning data structure for AI training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningExample {
    pub input: String,
    pub output: String,
    pub context: String,
    pub user_feedback: Option<f32>, // 0.0 to 1.0 rating
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub command_type: CommandType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    FileManagement,
    GitOperation,
    SystemQuery,
    Development,
    NetworkOperation,
    TextProcessing,
    SystemAdministration,
    Other,
}

/// Neural network-like structure for pattern learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralPattern {
    pub input_features: Vec<f32>,
    pub output_weights: Vec<f32>,
    pub bias: f32,
    pub confidence: f32,
    pub usage_count: u32,
    pub success_rate: f32,
}

/// Command frequency and success tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStats {
    pub command: String,
    pub frequency: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub success_rate: f32,
    pub avg_execution_time: f32,
    pub contexts: Vec<String>,
    pub last_used: DateTime<Utc>,
}

/// Learning engine that adapts to user behavior
pub struct LearningEngine {
    learning_data: Vec<LearningExample>,
    patterns: HashMap<String, NeuralPattern>,
    command_stats: HashMap<String, CommandStats>,
    user_preferences: UserPreferences,
    data_file: PathBuf,
    learning_rate: f32,
    // Enhanced context tracking
    session_workflows: HashMap<String, Vec<String>>, // Track command sequences per session
    temporal_patterns: HashMap<String, Vec<DateTime<Utc>>>, // Track usage times
    context_memory: HashMap<String, f32>, // Remember successful contexts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub preferred_commands: HashMap<String, f32>, // command -> preference score
    pub command_aliases: HashMap<String, String>,
    pub context_weights: HashMap<String, f32>,
    pub learning_aggressiveness: f32, // 0.0 to 1.0
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            preferred_commands: HashMap::new(),
            command_aliases: HashMap::new(),
            context_weights: HashMap::new(),
            learning_aggressiveness: 0.7,
        }
    }
}

impl LearningEngine {
    pub fn new(data_dir: PathBuf) -> Self {
        let data_file = data_dir.join("learning_data.json");
        
        let (learning_data, patterns, command_stats, user_preferences) = 
            Self::load_or_create_data(&data_file);

        Self {
            learning_data,
            patterns,
            command_stats,
            user_preferences,
            data_file,
            learning_rate: 0.1,
            // Initialize enhanced context tracking
            session_workflows: HashMap::new(),
            temporal_patterns: HashMap::new(),
            context_memory: HashMap::new(),
        }
    }

    fn load_or_create_data(data_file: &PathBuf) -> (
        Vec<LearningExample>,
        HashMap<String, NeuralPattern>,
        HashMap<String, CommandStats>,
        UserPreferences
    ) {
        if let Ok(data) = fs::read_to_string(data_file) {
            if let Ok(saved_data) = serde_json::from_str::<SavedLearningData>(&data) {
                return (
                    saved_data.learning_data,
                    saved_data.patterns,
                    saved_data.command_stats,
                    saved_data.user_preferences,
                );
            }
        }

        // Initialize with empty data
        (
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
            UserPreferences::default(),
        )
    }

    /// Add a learning example and update patterns
    pub fn learn_from_interaction(
        &mut self,
        input: String,
        output: String,
        context: String,
        success: bool,
        execution_time_ms: Option<u64>,
    ) {
        // Create learning example
        let example = LearningExample {
            input: input.clone(),
            output: output.clone(),
            context: context.clone(),
            user_feedback: None,
            timestamp: Utc::now(),
            success,
            command_type: self.classify_command(&input),
        };

        // Update command statistics
        self.update_command_stats(&input, success, execution_time_ms);

        // Extract features and update neural patterns
        self.update_patterns(&example);

        // Enhanced context learning
        self.learn_context_association(&context, success);
        
        // Track temporal patterns
        self.update_temporal_patterns(&input);

        // Store the example
        self.learning_data.push(example);

        // Limit data size to prevent excessive memory usage
        if self.learning_data.len() > 10000 {
            self.learning_data.remove(0);
        }

        // Save data periodically
        if self.learning_data.len() % 10 == 0 {
            self.save_data();
        }
    }

    /// Update user feedback for a previous interaction
    pub fn update_feedback(&mut self, input: &str, feedback: f32) {
        if let Some(example) = self.learning_data.iter_mut()
            .rev()
            .find(|ex| ex.input == input) {
            example.user_feedback = Some(feedback);
            
            // Update preferences based on feedback
            let current_score = self.user_preferences.preferred_commands
                .entry(input.to_string())
                .or_insert(0.5);
            
            *current_score = (*current_score + feedback) / 2.0;
        }
    }

    /// Suggest commands based on learned patterns
    pub fn suggest_commands(&self, context: &str, input_prefix: &str, limit: usize) -> Vec<String> {
        let mut suggestions = Vec::new();
        let context_features = self.extract_context_features(context);
        
        // Get suggestions from patterns
        for (pattern_key, pattern) in &self.patterns {
            let similarity = self.calculate_similarity(&context_features, &pattern.input_features);
            if similarity > 0.3 {
                suggestions.push((pattern_key.clone(), similarity * pattern.confidence));
            }
        }

        // Sort by relevance and filter by prefix
        suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        suggestions.into_iter()
            .map(|(cmd, _)| cmd)
            .filter(|cmd| cmd.starts_with(input_prefix))
            .take(limit)
            .collect()
    }

    /// Get intelligent completions based on learning
    pub fn get_smart_completions(&self, partial_command: &str, context: &str) -> Vec<String> {
        let mut completions = Vec::new();
        
        // Find similar commands from history
        for stats in self.command_stats.values() {
            if stats.command.starts_with(partial_command) && stats.success_count > 0 {
                completions.push((
                    stats.command.clone(),
                    stats.success_rate * (stats.frequency as f32).log2(),
                ));
            }
        }

        // Add context-aware suggestions
        let context_suggestions = self.suggest_commands(context, partial_command, 5);
        for suggestion in context_suggestions {
            if !completions.iter().any(|(cmd, _)| cmd == &suggestion) {
                completions.push((suggestion, 0.8));
            }
        }

        // Sort by relevance
        completions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        completions.into_iter()
            .map(|(cmd, _)| cmd)
            .take(8)
            .collect()
    }

    /// Classify command type for better learning
    fn classify_command(&self, command: &str) -> CommandType {
        let cmd_lower = command.to_lowercase();
        
        if cmd_lower.starts_with("git") {
            CommandType::GitOperation
        } else if cmd_lower.starts_with("ls") || cmd_lower.starts_with("find") || 
                 cmd_lower.starts_with("cp") || cmd_lower.starts_with("mv") ||
                 cmd_lower.starts_with("rm") || cmd_lower.starts_with("mkdir") {
            CommandType::FileManagement
        } else if cmd_lower.starts_with("ps") || cmd_lower.starts_with("top") ||
                 cmd_lower.starts_with("htop") || cmd_lower.starts_with("df") {
            CommandType::SystemQuery
        } else if cmd_lower.starts_with("npm") || cmd_lower.starts_with("cargo") ||
                 cmd_lower.starts_with("node") || cmd_lower.starts_with("python") {
            CommandType::Development
        } else if cmd_lower.starts_with("ping") || cmd_lower.starts_with("curl") ||
                 cmd_lower.starts_with("wget") || cmd_lower.starts_with("ssh") {
            CommandType::NetworkOperation
        } else if cmd_lower.starts_with("grep") || cmd_lower.starts_with("sed") ||
                 cmd_lower.starts_with("awk") || cmd_lower.starts_with("sort") {
            CommandType::TextProcessing
        } else if cmd_lower.starts_with("sudo") || cmd_lower.starts_with("systemctl") ||
                 cmd_lower.starts_with("service") {
            CommandType::SystemAdministration
        } else {
            CommandType::Other
        }
    }

    /// Update command statistics
    fn update_command_stats(&mut self, command: &str, success: bool, execution_time_ms: Option<u64>) {
        let stats = self.command_stats.entry(command.to_string())
            .or_insert_with(|| CommandStats {
                command: command.to_string(),
                frequency: 0,
                success_count: 0,
                failure_count: 0,
                success_rate: 0.0,
                avg_execution_time: 0.0,
                contexts: Vec::new(),
                last_used: Utc::now(),
            });

        stats.frequency += 1;
        if success {
            stats.success_count += 1;
        } else {
            stats.failure_count += 1;
        }

        // Update success rate
        stats.success_rate = stats.success_count as f32 / stats.frequency as f32;

        // Update average execution time
        if let Some(exec_time) = execution_time_ms {
            stats.avg_execution_time = (stats.avg_execution_time + exec_time as f32) / 2.0;
        }

        stats.last_used = Utc::now();
    }

    /// Update neural patterns based on new example
    fn update_patterns(&mut self, example: &LearningExample) {
        let input_features = self.extract_input_features(&example.input, &example.context);
        let pattern_key = self.generate_pattern_key(&example.input);

        let pattern = self.patterns.entry(pattern_key)
            .or_insert_with(|| NeuralPattern {
                input_features: input_features.clone(),
                output_weights: vec![0.5; input_features.len()],
                bias: 0.0,
                confidence: 0.5,
                usage_count: 0,
                success_rate: 0.0,
            });

        // Update pattern using gradient descent-like approach
        pattern.usage_count += 1;
        let success_weight = if example.success { 1.0 } else { -0.5 };
        
        for (i, feature) in input_features.iter().enumerate() {
            if i < pattern.output_weights.len() {
                pattern.output_weights[i] += self.learning_rate * success_weight * feature;
                pattern.output_weights[i] = pattern.output_weights[i].clamp(-1.0, 1.0);
            }
        }

        // Update confidence based on success rate
        let success_rate = pattern.usage_count as f32 / 
            (pattern.usage_count + if example.success { 0 } else { 1 }) as f32;
        pattern.confidence = (pattern.confidence + success_rate) / 2.0;
        pattern.success_rate = success_rate;
    }

    /// Extract features from input and context
    fn extract_input_features(&self, input: &str, context: &str) -> Vec<f32> {
        let mut features = Vec::new();
        
        // Basic command features
        features.push(input.len() as f32 / 100.0); // Normalized length
        features.push(input.split_whitespace().count() as f32 / 10.0); // Word count
        
        // Command type features
        let cmd_type = self.classify_command(input);
        features.extend(self.command_type_to_features(&cmd_type));
        
        // Context features
        features.extend(self.extract_context_features(context));
        
        features
    }

    /// Extract features from context
    fn extract_context_features(&self, context: &str) -> Vec<f32> {
        let mut features = vec![0.0; 10]; // Fixed size feature vector
        
        // Working directory indicators
        if context.contains("/home") || context.contains("/Users") {
            features[0] = 1.0;
        }
        if context.contains("/tmp") {
            features[1] = 1.0;
        }
        if context.contains(".git") {
            features[2] = 1.0;
        }
        
        // Recent command patterns
        if context.contains("git") {
            features[3] = 1.0;
        }
        if context.contains("npm") || context.contains("node") {
            features[4] = 1.0;
        }
        if context.contains("error") || context.contains("failed") {
            features[5] = 1.0;
        }
        
        // File type indicators
        if context.contains(".js") || context.contains(".ts") {
            features[6] = 1.0;
        }
        if context.contains(".py") {
            features[7] = 1.0;
        }
        if context.contains(".rs") {
            features[8] = 1.0;
        }
        
        // Time-based features (simplified)
        let hour = Utc::now().hour();
        features[9] = (hour as f32) / 24.0; // Normalized hour
        
        features
    }

    /// Convert command type to feature vector
    fn command_type_to_features(&self, cmd_type: &CommandType) -> Vec<f32> {
        let mut features = vec![0.0; 8];
        
        match cmd_type {
            CommandType::FileManagement => features[0] = 1.0,
            CommandType::GitOperation => features[1] = 1.0,
            CommandType::SystemQuery => features[2] = 1.0,
            CommandType::Development => features[3] = 1.0,
            CommandType::NetworkOperation => features[4] = 1.0,
            CommandType::TextProcessing => features[5] = 1.0,
            CommandType::SystemAdministration => features[6] = 1.0,
            CommandType::Other => features[7] = 1.0,
        }
        
        features
    }

    /// Generate a pattern key for grouping similar commands
    fn generate_pattern_key(&self, input: &str) -> String {
        let words: Vec<&str> = input.split_whitespace().collect();
        if words.is_empty() {
            return "empty".to_string();
        }
        
        // Group by first word and command structure
        let base_cmd = words[0];
        let arg_count = words.len() - 1;
        
        format!("{}_{}", base_cmd, arg_count)
    }

    /// Calculate similarity between feature vectors
    fn calculate_similarity(&self, features1: &[f32], features2: &[f32]) -> f32 {
        if features1.len() != features2.len() {
            return 0.0;
        }
        
        let dot_product: f32 = features1.iter()
            .zip(features2.iter())
            .map(|(a, b)| a * b)
            .sum();
        
        let norm1: f32 = features1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = features2.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm1 * norm2)
    }

    /// Get analytics about user behavior
    pub fn get_user_analytics(&self) -> UserAnalytics {
        let total_commands = self.command_stats.values()
            .map(|stats| stats.frequency)
            .sum::<u32>();
        
        let success_rate = if total_commands > 0 {
            let total_successes: u32 = self.command_stats.values()
                .map(|stats| stats.success_count)
                .sum();
            total_successes as f32 / total_commands as f32
        } else {
            0.0
        };

        let mut most_used_commands: Vec<_> = self.command_stats.values()
            .collect();
        most_used_commands.sort_by(|a, b| b.frequency.cmp(&a.frequency));

        UserAnalytics {
            total_commands,
            success_rate,
            most_used_commands: most_used_commands.into_iter()
                .take(10)
                .map(|stats| (stats.command.clone(), stats.frequency))
                .collect(),
            learning_examples: self.learning_data.len(),
            patterns_learned: self.patterns.len(),
        }
    }

    /// Save learning data to disk
    pub fn save_data(&self) {
        let saved_data = SavedLearningData {
            learning_data: self.learning_data.clone(),
            patterns: self.patterns.clone(),
            command_stats: self.command_stats.clone(),
            user_preferences: self.user_preferences.clone(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&saved_data) {
            let _ = fs::write(&self.data_file, json);
        }
    }

    /// Enhanced learning: Track session workflows for pattern recognition
    pub fn track_session_workflow(&mut self, session_id: &str, command: &str) {
        let workflow = self.session_workflows.entry(session_id.to_string()).or_insert_with(Vec::new);
        workflow.push(command.to_string());
        
        // Keep only last 50 commands per session to prevent memory bloat
        if workflow.len() > 50 {
            workflow.remove(0);
        }
        
        // If we have enough commands, analyze workflow patterns
        if workflow.len() >= 3 {
            // Clone the workflow to avoid borrow checker issues
            let workflow_clone = workflow.clone();
            self.analyze_workflow_patterns(&workflow_clone);
        }
    }

    /// Learn context associations based on success/failure
    fn learn_context_association(&mut self, context: &str, success: bool) {
        let context_key = self.extract_context_signature(context);
        let weight_change = if success { 0.1 } else { -0.05 };
        
        let current_weight = self.context_memory.get(&context_key).unwrap_or(&0.5);
        let new_weight = (current_weight + weight_change).clamp(0.0, 1.0);
        
        self.context_memory.insert(context_key, new_weight);
    }

    /// Track when commands are used for temporal pattern recognition
    fn update_temporal_patterns(&mut self, command: &str) {
        let pattern_key = self.generate_pattern_key(command);
        let timestamps = self.temporal_patterns.entry(pattern_key).or_insert_with(Vec::new);
        timestamps.push(Utc::now());
        
        // Keep only last 100 timestamps per command
        if timestamps.len() > 100 {
            timestamps.remove(0);
        }
    }

    /// Analyze workflow patterns from session commands
    fn analyze_workflow_patterns(&mut self, workflow: &[String]) {
        if workflow.len() < 3 { return; }
        
        // Look for 3-command sequences
        for window in workflow.windows(3) {
            let pattern_key = format!("workflow:{}->{}->{}",
                self.generate_pattern_key(&window[0]),
                self.generate_pattern_key(&window[1]),
                self.generate_pattern_key(&window[2])
            );
            
            // Create or update workflow pattern
            let workflow_pattern = self.patterns.entry(pattern_key).or_insert_with(|| {
                NeuralPattern {
                    input_features: vec![1.0, 1.0, 1.0], // Simple workflow indicator
                    output_weights: vec![0.8, 0.8, 0.8], // High initial confidence for workflows
                    bias: 0.1,
                    confidence: 0.7,
                    usage_count: 0,
                    success_rate: 0.8, // Assume workflows are generally successful
                }
            });
            
            workflow_pattern.usage_count += 1;
            workflow_pattern.confidence = (workflow_pattern.confidence + 0.1).min(1.0);
        }
    }

    /// Extract a signature from context for memory association
    fn extract_context_signature(&self, context: &str) -> String {
        let mut signature_parts = Vec::new();
        
        // Extract key context elements
        if context.contains("git") { signature_parts.push("git"); }
        if context.contains("npm") || context.contains("node") { signature_parts.push("node"); }
        if context.contains("python") || context.contains(".py") { signature_parts.push("python"); }
        if context.contains("rust") || context.contains(".rs") { signature_parts.push("rust"); }
        if context.contains("error") || context.contains("failed") { signature_parts.push("error"); }
        if context.contains("/home") || context.contains("/Users") { signature_parts.push("home"); }
        
        signature_parts.join("_")
    }

    /// Get enhanced suggestions considering session context and temporal patterns
    pub fn get_enhanced_suggestions(&self, context: &str, session_id: &str, limit: usize) -> Vec<String> {
        let mut suggestions = Vec::new();
        let context_features = self.extract_context_features(context);
        let context_signature = self.extract_context_signature(context);
        
        // Boost suggestions based on context memory
        let context_boost = self.context_memory.get(&context_signature).unwrap_or(&0.5);
        
        // Check for workflow patterns from current session
        if let Some(session_workflow) = self.session_workflows.get(session_id) {
            if session_workflow.len() >= 2 {
                let recent_commands = &session_workflow[session_workflow.len().saturating_sub(2)..];
                let workflow_suggestions = self.get_workflow_suggestions(recent_commands);
                suggestions.extend(workflow_suggestions);
            }
        }
        
        // Get regular pattern-based suggestions with context boost
        for (pattern_key, pattern) in &self.patterns {
            let similarity = self.calculate_similarity(&context_features, &pattern.input_features);
            let boosted_confidence = pattern.confidence * (1.0 + context_boost);
            
            if similarity > 0.3 {
                suggestions.push((pattern_key.clone(), similarity * boosted_confidence));
            }
        }

        // Sort by relevance and return top suggestions
        suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        suggestions.into_iter()
            .map(|(cmd, _)| cmd)
            .take(limit)
            .collect()
    }

    /// Get workflow-based suggestions
    fn get_workflow_suggestions(&self, recent_commands: &[String]) -> Vec<(String, f32)> {
        let mut suggestions = Vec::new();
        
        for (pattern_key, pattern) in &self.patterns {
            if pattern_key.starts_with("workflow:") && pattern.usage_count > 2 {
                // Simple workflow matching - could be more sophisticated
                let workflow_confidence = pattern.confidence * 1.2; // Boost workflow suggestions
                suggestions.push((pattern_key.clone(), workflow_confidence));
            }
        }
        
        suggestions
    }
}

/// Data structure for saving/loading
#[derive(Serialize, Deserialize)]
struct SavedLearningData {
    learning_data: Vec<LearningExample>,
    patterns: HashMap<String, NeuralPattern>,
    command_stats: HashMap<String, CommandStats>,
    user_preferences: UserPreferences,
}

/// User analytics for insights
#[derive(Debug, Serialize, Deserialize)]
pub struct UserAnalytics {
    pub total_commands: u32,
    pub success_rate: f32,
    pub most_used_commands: Vec<(String, u32)>,
    pub learning_examples: usize,
    pub patterns_learned: usize,
}

impl Drop for LearningEngine {
    fn drop(&mut self) {
        self.save_data();
    }
}
