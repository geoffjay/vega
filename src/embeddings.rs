use anyhow::Result;
use tracing::{debug, warn};

/// Simple embedding service that generates embeddings for text
/// In a production system, you'd want to use a proper embedding model
/// like sentence-transformers, OpenAI embeddings, or local models
#[derive(Debug)]
pub struct EmbeddingService {
    dimension: usize,
}

impl EmbeddingService {
    /// Create a new embedding service with the specified dimension
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// Generate an embedding for the given text
    /// This is a simple hash-based embedding for demonstration
    /// In production, replace with actual embedding model
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if text.is_empty() {
            warn!("Attempting to embed empty text");
            return Ok(vec![0.0; self.dimension]);
        }

        // Simple deterministic embedding based on text characteristics
        // This is NOT a real embedding - replace with proper model
        let mut embedding = vec![0.0; self.dimension];

        // Use text characteristics to generate pseudo-embedding
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        let char_count = text.len() as f32;
        let word_count = words.len() as f32;

        // Fill embedding with deterministic values based on text
        for (i, value) in embedding.iter_mut().enumerate() {
            let base = (i as f32 + 1.0) / (self.dimension as f32);
            let text_hash = self.simple_hash(&text_lower) as f32;
            let word_influence = if i < words.len() {
                self.simple_hash(words[i]) as f32 / 1000.0
            } else {
                0.0
            };

            *value = (base
                + text_hash / 10000.0
                + word_influence
                + char_count / 1000.0
                + word_count / 100.0)
                % 1.0;

            // Normalize to [-1, 1] range
            *value = (*value - 0.5) * 2.0;
        }

        debug!("Generated embedding for text of length {}", text.len());
        Ok(embedding)
    }

    /// Generate embeddings for multiple texts
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            embeddings.push(self.embed(text).await?);
        }

        debug!("Generated {} embeddings", embeddings.len());
        Ok(embeddings)
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Simple hash function for deterministic pseudo-embeddings
    fn simple_hash(&self, text: &str) -> u32 {
        let mut hash = 5381u32;
        for byte in text.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
        }
        hash
    }
}

/// Configuration for different embedding providers
#[derive(Debug, Clone)]
pub enum EmbeddingProvider {
    /// Simple hash-based embeddings (for development/testing)
    Simple { dimension: usize },
    /// OpenAI embeddings (requires API key)
    OpenAI { model: String, api_key: String },
    /// Local model embeddings (requires model path)
    Local {
        model_path: String,
        dimension: usize,
    },
}

impl EmbeddingProvider {
    /// Create an embedding service from the provider configuration
    pub fn create_service(&self) -> Result<EmbeddingService> {
        match self {
            EmbeddingProvider::Simple { dimension } => Ok(EmbeddingService::new(*dimension)),
            EmbeddingProvider::OpenAI { .. } => {
                // TODO: Implement OpenAI embeddings
                Err(anyhow::anyhow!("OpenAI embeddings not yet implemented"))
            }
            EmbeddingProvider::Local { dimension, .. } => {
                // TODO: Implement local model embeddings
                warn!("Local embeddings not yet implemented, falling back to simple");
                Ok(EmbeddingService::new(*dimension))
            }
        }
    }
}

impl Default for EmbeddingProvider {
    fn default() -> Self {
        EmbeddingProvider::Simple { dimension: 384 }
    }
}

/// Utility functions for working with embeddings
pub mod utils {
    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Normalize an embedding vector to unit length
    pub fn normalize_embedding(embedding: &mut [f32]) {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in embedding.iter_mut() {
                *value /= norm;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_service_creation() {
        let service = EmbeddingService::new(384);
        assert_eq!(service.dimension(), 384);
    }

    #[tokio::test]
    async fn test_embed_text() {
        let service = EmbeddingService::new(10);
        let embedding = service.embed("Hello, world!").await.unwrap();

        assert_eq!(embedding.len(), 10);
        // Embeddings should be in [-1, 1] range
        for value in &embedding {
            assert!(*value >= -1.0 && *value <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_embed_empty_text() {
        let service = EmbeddingService::new(5);
        let embedding = service.embed("").await.unwrap();

        assert_eq!(embedding.len(), 5);
        assert_eq!(embedding, vec![0.0; 5]);
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let service = EmbeddingService::new(3);
        let texts = vec!["First text".to_string(), "Second text".to_string()];

        let embeddings = service.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 3);
        assert_eq!(embeddings[1].len(), 3);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = utils::cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 1e-6);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        let similarity = utils::cosine_similarity(&c, &d);
        assert!((similarity - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_embedding() {
        let mut embedding = vec![3.0, 4.0, 0.0];
        utils::normalize_embedding(&mut embedding);

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_provider_default() {
        let provider = EmbeddingProvider::default();
        match provider {
            EmbeddingProvider::Simple { dimension } => {
                assert_eq!(dimension, 384);
            }
            _ => panic!("Default should be Simple provider"),
        }
    }
}
