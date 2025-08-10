// Local embeddings for semantic search and context understanding
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    pub id: String,
    pub text: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub text: String,
    pub similarity: f32,
    pub context_type: ContextType,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    Command,
    ErrorMessage,
    FileContent,
    LogEntry,
    Documentation,
    SystemInfo,
}

pub struct LocalEmbeddingStore {
    embeddings: Vec<EmbeddingVector>,
    dimension: usize,
}

impl LocalEmbeddingStore {
    pub fn new() -> Self {
        Self {
            embeddings: Vec::new(),
            dimension: 384, // Using smaller embeddings for efficiency
        }
    }

    pub fn add_embedding(&mut self, embedding: EmbeddingVector) {
        self.embeddings.push(embedding);
    }

    pub fn semantic_search(&self, query_vector: &[f32], top_k: usize) -> Vec<SemanticSearchResult> {
        let mut results: Vec<(f32, &EmbeddingVector)> = self.embeddings
            .iter()
            .map(|emb| (cosine_similarity(query_vector, &emb.vector), emb))
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        
        results
            .into_iter()
            .take(top_k)
            .map(|(similarity, emb)| SemanticSearchResult {
                text: emb.text.clone(),
                similarity,
                context_type: self.infer_context_type(&emb.text),
                metadata: emb.metadata.clone(),
            })
            .collect()
    }

    fn infer_context_type(&self, text: &str) -> ContextType {
        if text.starts_with("Error:") || text.contains("error") {
            ContextType::ErrorMessage
        } else if text.ends_with(".log") || text.contains("log:") {
            ContextType::LogEntry
        } else if text.starts_with("$") || text.starts_with("sudo") {
            ContextType::Command
        } else if text.contains("CPU") || text.contains("Memory") {
            ContextType::SystemInfo
        } else {
            ContextType::Documentation
        }
    }

    // Simple text-to-embedding conversion (would be replaced with actual model)
    pub fn text_to_embedding(&self, text: &str) -> Vec<f32> {
        // Simplified hash-based embedding for demo
        // In production, use a proper embedding model like sentence-transformers
        let mut embedding = vec![0.0; self.dimension];
        
        for (i, byte) in text.bytes().enumerate() {
            if i >= self.dimension { break; }
            embedding[i] = (byte as f32) / 255.0;
        }
        
        // Normalize the vector
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }
        
        embedding
    }

    pub fn index_command_history(&mut self, commands: &[String]) {
        for (i, command) in commands.iter().enumerate() {
            let embedding_vector = self.text_to_embedding(command);
            let mut metadata = HashMap::new();
            metadata.insert("type".to_string(), "command".to_string());
            metadata.insert("index".to_string(), i.to_string());
            
            let embedding = EmbeddingVector {
                id: format!("cmd_{}", i),
                text: command.clone(),
                vector: embedding_vector,
                metadata,
                timestamp: chrono::Utc::now(),
            };
            
            self.add_embedding(embedding);
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        0.0
    } else {
        dot_product / (magnitude_a * magnitude_b)
    }
}
