use anyhow::Result;
use rig::client::EmbeddingsClient;
use rig::embeddings::EmbeddingsBuilder;
use rig::providers;
use tracing::{debug, warn};

/// Embedding service that generates embeddings for text using real models
#[derive(Debug)]
pub struct EmbeddingService {
    provider: EmbeddingProvider,
}

impl EmbeddingService {
    /// Create a new embedding service from a provider
    pub fn new(provider: EmbeddingProvider) -> Self {
        Self { provider }
    }

    /// Generate an embedding for the given text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if text.is_empty() {
            warn!("Attempting to embed empty text");
            return Ok(vec![0.0; self.dimension()]);
        }

        match &self.provider {
            EmbeddingProvider::Simple { dimension } => self.embed_simple(text, *dimension).await,
            EmbeddingProvider::OpenAI { client, model } => {
                let embedding_model = client.embedding_model(model);
                let embeddings = EmbeddingsBuilder::new(embedding_model)
                    .document(text)?
                    .build()
                    .await?;

                if let Some((_, embedding)) = embeddings.into_iter().next() {
                    if let Some(emb) = embedding.into_iter().next() {
                        Ok(emb.vec.into_iter().map(|x| x as f32).collect())
                    } else {
                        Ok(vec![0.0; self.dimension()])
                    }
                } else {
                    Ok(vec![0.0; self.dimension()])
                }
            }
            EmbeddingProvider::Ollama { client, model } => {
                let embedding_model = client.embedding_model(model);
                let embeddings = EmbeddingsBuilder::new(embedding_model)
                    .document(text)?
                    .build()
                    .await?;

                if let Some((_, embedding)) = embeddings.into_iter().next() {
                    if let Some(emb) = embedding.into_iter().next() {
                        Ok(emb.vec.into_iter().map(|x| x as f32).collect())
                    } else {
                        Ok(vec![0.0; self.dimension()])
                    }
                } else {
                    Ok(vec![0.0; self.dimension()])
                }
            }
        }
    }

    /// Generate embeddings for multiple texts
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        match &self.provider {
            EmbeddingProvider::Simple { dimension } => {
                let mut embeddings = Vec::with_capacity(texts.len());
                for text in texts {
                    embeddings.push(self.embed_simple(text, *dimension).await?);
                }
                Ok(embeddings)
            }
            EmbeddingProvider::OpenAI { client, model } => {
                let embedding_model = client.embedding_model(model);
                let mut builder = EmbeddingsBuilder::new(embedding_model);
                for text in texts {
                    builder = builder.document(text)?;
                }
                let embeddings = builder.build().await?;
                Ok(embeddings
                    .into_iter()
                    .map(|(_, embedding)| {
                        embedding
                            .into_iter()
                            .next()
                            .map(|emb| emb.vec.into_iter().map(|x| x as f32).collect())
                            .unwrap_or_default()
                    })
                    .collect())
            }
            EmbeddingProvider::Ollama { client, model } => {
                let embedding_model = client.embedding_model(model);
                let mut builder = EmbeddingsBuilder::new(embedding_model);
                for text in texts {
                    builder = builder.document(text)?;
                }
                let embeddings = builder.build().await?;
                Ok(embeddings
                    .into_iter()
                    .map(|(_, embedding)| {
                        embedding
                            .into_iter()
                            .next()
                            .map(|emb| emb.vec.into_iter().map(|x| x as f32).collect())
                            .unwrap_or_default()
                    })
                    .collect())
            }
        }
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        match &self.provider {
            EmbeddingProvider::Simple { dimension } => *dimension,
            EmbeddingProvider::OpenAI { client: _, model } => {
                // Common OpenAI embedding dimensions
                match model.as_str() {
                    "text-embedding-3-large" => 3072,
                    "text-embedding-3-small" => 1536,
                    "text-embedding-ada-002" => 1536,
                    _ => 1536, // Default fallback
                }
            }
            EmbeddingProvider::Ollama { client: _, model } => {
                // Return dimensions based on the specific Ollama model
                // Common Ollama embedding models and their dimensions:
                match model.as_str() {
                    "nomic-embed-text" => 768,
                    "all-minilm" => 384,
                    "mxbai-embed-large" => 1024,
                    _ => {
                        // For unknown models, default to 768 as it's more common for newer models
                        // Users can extend this match statement for other models
                        // Note: If you change dimensions, you may need to delete existing context databases
                        768
                    }
                }
            }
        }
    }

    /// Simple hash-based embedding for development/testing
    async fn embed_simple(&self, text: &str, dimension: usize) -> Result<Vec<f32>> {
        // Simple deterministic embedding based on text characteristics
        // This is NOT a real embedding - only for fallback/testing
        let mut embedding = vec![0.0; dimension];

        // Use text characteristics to generate pseudo-embedding
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        let char_count = text.len() as f32;
        let word_count = words.len() as f32;

        // Fill embedding with deterministic values based on text
        for (i, value) in embedding.iter_mut().enumerate() {
            let base = (i as f32 + 1.0) / (dimension as f32);
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

        debug!(
            "Generated simple embedding for text of length {}",
            text.len()
        );
        Ok(embedding)
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
#[derive(Clone, Debug)]
pub enum EmbeddingProvider {
    /// Simple hash-based embeddings (for development/testing)
    Simple { dimension: usize },
    /// OpenAI embeddings (requires API key)
    OpenAI {
        client: providers::openai::Client,
        model: String,
    },
    /// Ollama embeddings (local models)
    Ollama {
        client: providers::ollama::Client,
        model: String,
    },
}

impl EmbeddingProvider {
    /// Create a new embedding provider from configuration
    pub fn new(
        provider_name: &str,
        model: Option<&str>,
        openai_api_key: Option<&str>,
    ) -> Result<Self> {
        match provider_name {
            "simple" => Ok(EmbeddingProvider::Simple { dimension: 384 }),
            "openai" => {
                let api_key = openai_api_key.ok_or_else(|| {
                    anyhow::anyhow!("OpenAI API key is required for OpenAI embedding provider. Set --openai-api-key or OPENAI_API_KEY environment variable.")
                })?;

                let client = providers::openai::Client::new(api_key);
                let model = model.unwrap_or("text-embedding-3-small").to_string();

                Ok(EmbeddingProvider::OpenAI { client, model })
            }
            "ollama" => {
                let client = providers::ollama::Client::new();
                let model = model.unwrap_or("nomic-embed-text").to_string();

                Ok(EmbeddingProvider::Ollama { client, model })
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported embedding provider: {}. Supported providers: simple, openai, ollama",
                provider_name
            )),
        }
    }

    /// Create an embedding service from the provider configuration
    pub fn create_service(&self) -> EmbeddingService {
        EmbeddingService::new(self.clone())
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
        let provider = EmbeddingProvider::Simple { dimension: 384 };
        let service = EmbeddingService::new(provider);
        assert_eq!(service.dimension(), 384);
    }

    #[tokio::test]
    async fn test_embed_text() {
        let provider = EmbeddingProvider::Simple { dimension: 10 };
        let service = EmbeddingService::new(provider);
        let embedding = service.embed("Hello, world!").await.unwrap();

        assert_eq!(embedding.len(), 10);
        // Embeddings should be in [-1, 1] range
        for value in &embedding {
            assert!(*value >= -1.0 && *value <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_embed_empty_text() {
        let provider = EmbeddingProvider::Simple { dimension: 5 };
        let service = EmbeddingService::new(provider);
        let embedding = service.embed("").await.unwrap();

        assert_eq!(embedding.len(), 5);
        assert_eq!(embedding, vec![0.0; 5]);
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let provider = EmbeddingProvider::Simple { dimension: 3 };
        let service = EmbeddingService::new(provider);
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

    #[test]
    fn test_ollama_embedding_dimensions() {
        // Test nomic-embed-text model returns 768 dimensions
        let provider = EmbeddingProvider::Ollama {
            client: rig::providers::ollama::Client::new(),
            model: "nomic-embed-text".to_string(),
        };
        let service = EmbeddingService::new(provider);
        assert_eq!(service.dimension(), 768);

        // Test all-minilm model returns 384 dimensions
        let provider = EmbeddingProvider::Ollama {
            client: rig::providers::ollama::Client::new(),
            model: "all-minilm".to_string(),
        };
        let service = EmbeddingService::new(provider);
        assert_eq!(service.dimension(), 384);

        // Test mxbai-embed-large model returns 1024 dimensions
        let provider = EmbeddingProvider::Ollama {
            client: rig::providers::ollama::Client::new(),
            model: "mxbai-embed-large".to_string(),
        };
        let service = EmbeddingService::new(provider);
        assert_eq!(service.dimension(), 1024);

        // Test unknown model defaults to 768 dimensions
        let provider = EmbeddingProvider::Ollama {
            client: rig::providers::ollama::Client::new(),
            model: "unknown-model".to_string(),
        };
        let service = EmbeddingService::new(provider);
        assert_eq!(service.dimension(), 768);
    }
}
