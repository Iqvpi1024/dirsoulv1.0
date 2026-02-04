//! DirSoul Embedding Module
//!
//! Generates text embeddings using nomic-embed-text via Ollama.
//! Supports semantic search and similarity matching for memory retrieval.
//!
//! # Design Principles
//! - Memory efficient: Batch processing to minimize inference calls
//! - Async first: Uses tokio for non-blocking operations
//! - Error resilient: Graceful degradation on Ollama failures
//!
//! # Model
//! - Default: nomic-embed-text:v1.5 (512 dimensions)
//! - Fixed embedding model (not user-configurable) to avoid re-indexing
//! - Inference model is user-configurable (phi4-mini, deepseek-r1, etc.)
//!
//! # Example
//! ```no_run
//! use dirsoul::embedding::{EmbeddingGenerator, EmbeddingConfig};
//!
//! #[tokio::main]
//! async fn main() -> dirsoul::Result<()> {
//!     let config = EmbeddingConfig::default();
//!     let generator = EmbeddingGenerator::new(config).await?;
//!     let embedding = generator.generate("Hello, world!").await?;
//!     Ok(())
//! }
//! ```

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::Result;

/// Default embedding dimension (for nomic-embed-text:v1.5)
pub const EMBEDDING_DIM: usize = 512;

/// Default Ollama host
const DEFAULT_OLLAMA_HOST: &str = "http://127.0.0.1:11434";

/// Configuration for embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Ollama host URL
    pub host: String,
    /// Model name for embeddings
    pub model: String,
    /// Batch size for embedding multiple texts
    pub batch_size: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_OLLAMA_HOST.to_string(),
            model: "nomic-embed-text:v1.5".to_string(),
            batch_size: 8,
            timeout_secs: 120,
        }
    }
}

/// Ollama embedding response
#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

/// Ollama model list response
#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

/// Embedding cache to avoid regenerating embeddings
///
/// Uses LRU strategy to limit memory usage in 8GB environment.
struct EmbeddingCache {
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    max_size: usize,
}

impl EmbeddingCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }

    async fn get(&self, key: &str) -> Option<Vec<f32>> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    async fn set(&self, key: String, value: Vec<f32>) {
        let mut cache = self.cache.write().await;

        // Simple LRU: remove oldest if at capacity
        if cache.len() >= self.max_size {
            // Remove first entry (approximate LRU)
            if let Some(first_key) = cache.keys().next().cloned() {
                cache.remove(&first_key);
            }
        }

        cache.insert(key, value);
    }

    async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    async fn size(&self) -> usize {
        self.cache.read().await.len()
    }
}

/// Embedding generator for Phi-4-mini
///
/// Handles communication with Ollama to generate text embeddings.
pub struct EmbeddingGenerator {
    client: Client,
    config: EmbeddingConfig,
    cache: EmbeddingCache,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator
    ///
    /// # Arguments
    /// * `config` - Configuration for embedding generation
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        info!(
            "Initializing embedding generator: host={}, model={}",
            config.host, config.model
        );

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| {
                crate::DirSoulError::Encryption(format!("Failed to create HTTP client: {}", e))
            })?;

        // Verify model is available
        Self::verify_model(&client, &config.host, &config.model).await?;

        Ok(Self {
            client,
            config,
            cache: EmbeddingCache::new(1000), // Cache up to 1000 embeddings
        })
    }

    /// Create with default configuration
    pub async fn default_config() -> Result<Self> {
        Self::new(EmbeddingConfig::default()).await
    }

    /// Generate embedding for a single text
    ///
    /// # Arguments
    /// * `text` - Text to generate embedding for
    ///
    /// # Returns
    /// Embedding vector (512 dimensions for nomic-embed-text:v1.5)
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        if let Some(cached) = self.cache.get(text).await {
            debug!("Using cached embedding for text: {} chars", text.len());
            return Ok(cached);
        }

        debug!("Generating embedding for text: {} chars", text.len());

        let url = format!("{}/api/embeddings", self.config.host);
        let body = serde_json::json!({
            "model": self.config.model,
            "input": text
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::DirSoulError::Encryption(format!("HTTP request failed: {}", e))
            })?
            .error_for_status()
            .map_err(|e| {
                crate::DirSoulError::Encryption(format!(
                    "Ollama API error: {:?}",
                    e.status()
                ))
            })?
            .json::<OllamaEmbeddingResponse>()
            .await
            .map_err(|e| {
                crate::DirSoulError::Encryption(format!("Failed to parse response: {}", e))
            })?;

        let embedding = Self::normalize_embedding(response.embedding);

        // Cache the result
        self.cache.set(text.to_string(), embedding.clone()).await;

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts (batch processing)
    ///
    /// This is more efficient than generating embeddings one by one.
    ///
    /// # Arguments
    /// * `texts` - Texts to generate embeddings for
    ///
    /// # Returns
    /// Vector of embeddings (one per input text)
    pub async fn generate_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        if texts.len() == 1 {
            let emb = self.generate(&texts[0]).await?;
            return Ok(vec![emb]);
        }

        info!("Generating embeddings for {} texts", texts.len());

        let mut results = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(self.config.batch_size) {
            debug!("Processing batch of {} texts", chunk.len());

            // Check cache for each text
            let mut uncached = Vec::new();
            let mut cached_indices = Vec::new();

            for (i, text) in chunk.iter().enumerate() {
                if let Some(cached) = self.cache.get(text).await {
                    cached_indices.push((i, cached));
                } else {
                    uncached.push((i, text.clone()));
                }
            }

            // Generate embeddings for uncached texts
            for (idx, text) in uncached {
                let emb = self.generate(&text).await?;

                // Cache the result
                self.cache.set(text.clone(), emb.clone()).await;
                cached_indices.push((idx, emb));
            }

            // Sort by original index and collect
            cached_indices.sort_by_key(|(i, _)| *i);
            results.extend(cached_indices.into_iter().map(|(_, emb)| emb));
        }

        Ok(results)
    }

    /// Calculate cosine similarity between two embeddings
    ///
    /// Returns a value between 0.0 (orthogonal) and 1.0 (identical)
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            warn!(
                "Embedding dimension mismatch: {} vs {}",
                a.len(),
                b.len()
            );
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

    /// Normalize embedding vector to unit length
    ///
    /// This allows using cosine similarity via dot product.
    fn normalize_embedding(mut embedding: Vec<f32>) -> Vec<f32> {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }

        embedding
    }

    /// Verify that the model is available in Ollama
    async fn verify_model(client: &Client, host: &str, model: &str) -> Result<()> {
        let url = format!("{}/api/tags", host);

        match client.get(&url).send().await {
            Ok(response) => {
                if let Ok(models) = response.json::<OllamaModelsResponse>().await {
                    let model_names: Vec<_> = models.models.iter().map(|m| &m.name).collect();
                    debug!("Available models: {:?}", model_names);

                    if !model_names.iter().any(|m| *m == model) {
                        warn!("Model {} not found in available models", model);
                        warn!("Please run: ollama pull {}", model);
                    }
                }
                Ok(())
            }
            Err(e) => {
                warn!("Failed to list models: {:?}. Assuming model is available.", e);
                Ok(())
            }
        }
    }

    /// Get cache statistics
    pub async fn cache_size(&self) -> usize {
        self.cache.size().await
    }

    /// Clear the embedding cache
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
        info!("Embedding cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        let sim = EmbeddingGenerator::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let sim = EmbeddingGenerator::cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_partial() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 4.0, 3.0];

        let sim = EmbeddingGenerator::cosine_similarity(&a, &b);
        assert!(sim > 0.8 && sim < 1.0);
    }

    #[test]
    fn test_cosine_similarity_different_dims() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0]; // Different dimensions

        let sim = EmbeddingGenerator::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_normalize_embedding() {
        let embedding = vec![3.0, 4.0]; // Norm = 5
        let normalized = EmbeddingGenerator::normalize_embedding(embedding.clone());

        // Check unit length
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);

        // Check direction preserved
        let sim = EmbeddingGenerator::cosine_similarity(&embedding, &normalized);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let embedding = vec![0.0, 0.0, 0.0];
        let normalized = EmbeddingGenerator::normalize_embedding(embedding);

        assert_eq!(normalized, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.host, DEFAULT_OLLAMA_HOST);
        assert_eq!(config.model, "nomic-embed-text:v1.5");
        assert_eq!(config.batch_size, 8);
        assert_eq!(config.timeout_secs, 120);
    }
}
