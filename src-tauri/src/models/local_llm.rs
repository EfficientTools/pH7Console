// Local LLM model definitions and configurations
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelInfo {
    pub name: String,
    pub size_mb: u64,
    pub model_type: ModelType,
    pub capabilities: Vec<Capability>,
    pub download_url: String,
    pub local_path: Option<String>,
    pub is_downloaded: bool,
    pub performance_tier: PerformanceTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    LocalPatternEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Capability {
    CodeGeneration,
    CommandSuggestion,
    ErrorAnalysis,
    NaturalLanguageToCommand,
    OutputAnalysis,
    SystemDiagnostics,
    FileSearch,
    LogAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTier {
    Ultra,    // < 2GB RAM, < 1B params
    Fast,     // 2-4GB RAM, 1-3B params
    Balanced, // 4-8GB RAM, 3-7B params
    Premium,  // 8GB+ RAM, 7B+ params
}
