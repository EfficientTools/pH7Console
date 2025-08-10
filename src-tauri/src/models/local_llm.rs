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
    Phi3Mini,      // 3.8B parameters - Best balance for MacBook Air
    Llama32_1B,    // 1B parameters - Ultra lightweight
    Llama32_3B,    // 3B parameters - Good performance
    CodeQwen,      // Code-specific model
    TinyLlama,     // 1.1B parameters - Fastest
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

impl LocalModelInfo {
    pub fn get_recommended_models() -> Vec<LocalModelInfo> {
        vec![
            LocalModelInfo {
                name: "Phi-3 Mini (Recommended)".to_string(),
                size_mb: 3800,
                model_type: ModelType::Phi3Mini,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::CommandSuggestion,
                    Capability::ErrorAnalysis,
                    Capability::NaturalLanguageToCommand,
                    Capability::OutputAnalysis,
                ],
                download_url: "microsoft/Phi-3-mini-4k-instruct".to_string(),
                local_path: None,
                is_downloaded: false,
                performance_tier: PerformanceTier::Balanced,
            },
            LocalModelInfo {
                name: "Llama 3.2 1B (Ultra Light)".to_string(),
                size_mb: 1200,
                model_type: ModelType::Llama32_1B,
                capabilities: vec![
                    Capability::CommandSuggestion,
                    Capability::NaturalLanguageToCommand,
                    Capability::ErrorAnalysis,
                ],
                download_url: "meta-llama/Llama-3.2-1B-Instruct".to_string(),
                local_path: None,
                is_downloaded: false,
                performance_tier: PerformanceTier::Fast,
            },
            LocalModelInfo {
                name: "TinyLlama (Fastest)".to_string(),
                size_mb: 1100,
                model_type: ModelType::TinyLlama,
                capabilities: vec![
                    Capability::CommandSuggestion,
                    Capability::NaturalLanguageToCommand,
                ],
                download_url: "TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string(),
                local_path: None,
                is_downloaded: false,
                performance_tier: PerformanceTier::Ultra,
            },
            LocalModelInfo {
                name: "CodeQwen 1.5B (Code Expert)".to_string(),
                size_mb: 1500,
                model_type: ModelType::CodeQwen,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::CommandSuggestion,
                    Capability::ErrorAnalysis,
                    Capability::FileSearch,
                ],
                download_url: "Qwen/CodeQwen1.5-7B-Chat".to_string(),
                local_path: None,
                is_downloaded: false,
                performance_tier: PerformanceTier::Fast,
            },
        ]
    }
    
    pub fn get_model_for_capability(capability: &Capability) -> Option<ModelType> {
        match capability {
            Capability::CodeGeneration => Some(ModelType::Phi3Mini),
            Capability::CommandSuggestion => Some(ModelType::Llama32_1B),
            Capability::ErrorAnalysis => Some(ModelType::Phi3Mini),
            Capability::NaturalLanguageToCommand => Some(ModelType::TinyLlama),
            Capability::OutputAnalysis => Some(ModelType::Phi3Mini),
            Capability::SystemDiagnostics => Some(ModelType::Llama32_3B),
            Capability::FileSearch => Some(ModelType::CodeQwen),
            Capability::LogAnalysis => Some(ModelType::Phi3Mini),
        }
    }
}
