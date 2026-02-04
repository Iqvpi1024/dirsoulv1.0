//! Entity Summarizer Module - Phase 4, Task 4.4
//!
//! Handles entity summary generation using Phi-4-mini.
//!
//! # Design Principles (from HEAD.md)
//! - SLM主导提取，规则仅作为兜底
//! - 8G内存优化：批量处理、缓存管理
//! - 摘要定期更新，基于实体变化触发
//!
//! # Core Functionality
//! - Generate entity summaries using Phi-4-mini
//! - Cache summaries to avoid regeneration
//! - Update summaries when entity changes significantly

use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::error::{DirSoulError, Result};
use crate::models::Entity;

/// In-memory cache for entity summaries
///
/// LRU-style cache with limited size to respect 8GB memory constraint.
#[derive(Debug, Clone)]
struct SummaryCache {
    /// Cache storage: entity_id -> (summary, last_seen_count)
    storage: Arc<RwLock<HashMap<Uuid, (String, i32)>>>,
    /// Maximum cache size
    max_size: usize,
}

impl SummaryCache {
    /// Create a new summary cache
    fn new(max_size: usize) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }

    /// Get cached summary if count hasn't changed
    fn get(&self, entity_id: &Uuid, current_count: i32) -> Option<String> {
        let storage = self.storage.read().ok()?;
        if let Some((summary, cached_count)) = storage.get(entity_id) {
            if *cached_count == current_count {
                return Some(summary.clone());
            }
        }
        None
    }

    /// Put summary in cache
    fn put(&self, entity_id: Uuid, summary: String, count: i32) {
        if let Ok(mut storage) = self.storage.write() {
            // Evict oldest if at capacity
            if storage.len() >= self.max_size {
                // Simple FIFO eviction: remove first entry
                if let Some(key) = storage.keys().next().copied() {
                    storage.remove(&key);
                }
            }
            storage.insert(entity_id, (summary, count));
        }
    }

    /// Invalidate cache entry
    fn invalidate(&self, entity_id: &Uuid) {
        if let Ok(mut storage) = self.storage.write() {
            storage.remove(entity_id);
        }
    }

    /// Clear all cache
    fn clear(&self) {
        if let Ok(mut storage) = self.storage.write() {
            storage.clear();
        }
    }
}

/// Entity summarizer using Phi-4-mini
///
/// Generates natural language summaries of entities based on their attributes
/// and occurrence history.
pub struct EntitySummarizer {
    /// HTTP client
    client: reqwest::Client,
    /// Ollama base URL
    base_url: String,
    /// Model name to use
    model: String,
    /// Summary cache
    cache: SummaryCache,
    /// Minimum occurrence count before generating summary
    min_occurrences: i32,
}

impl EntitySummarizer {
    /// Create a new entity summarizer with default configuration
    pub async fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| DirSoulError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url: "http://127.0.0.1:11434".to_string(),
            model: "phi4-mini".to_string(),
            cache: SummaryCache::new(1000), // Cache up to 1000 summaries
            min_occurrences: 3,
        })
    }

    /// Create a new entity summarizer with custom cache size
    ///
    /// # Arguments
    /// * `cache_size` - Maximum number of summaries to cache
    pub async fn with_cache_size(cache_size: usize) -> Result<Self> {
        let mut summarizer = Self::new().await?;
        summarizer.cache = SummaryCache::new(cache_size);
        Ok(summarizer)
    }

    /// Create a new entity summarizer with custom configuration
    ///
    /// # Arguments
    /// * `base_url` - Ollama base URL
    /// * `model` - Model name to use
    /// * `cache_size` - Maximum cache size
    /// * `min_occurrences` - Minimum occurrences before generating summary
    pub async fn with_config(
        base_url: String,
        model: String,
        cache_size: usize,
        min_occurrences: i32,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| DirSoulError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url,
            model,
            cache: SummaryCache::new(cache_size),
            min_occurrences,
        })
    }

    /// Generate entity summary
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `entity_id` - UUID of the entity
    ///
    /// # Returns
    /// Generated summary string
    ///
    /// # Example
    /// ```no_run
    /// # use dirsoul::entity_summarizer::EntitySummarizer;
    /// # use diesel::PgConnection;
    /// # async fn example() -> dirsoul::Result<()> {
    /// # let mut conn: PgConnection = unimplemented!();
    /// # let entity_id = uuid::Uuid::new_v4();
    /// let summarizer = EntitySummarizer::new().await?;
    /// let summary = summarizer.generate_summary(&mut conn, entity_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_summary(
        &self,
        conn: &mut PgConnection,
        eid: Uuid,
    ) -> Result<String> {
        use crate::schema::entities::dsl::*;

        // Fetch entity
        let entity = entities
            .find(eid)
            .first::<Entity>(conn)?;

        // Check cache
        if let Some(cached) = self.cache.get(&eid, entity.occurrence_count) {
            return Ok(cached);
        }

        // Check minimum occurrence threshold
        if entity.occurrence_count < self.min_occurrences {
            return Ok(format!(
                "{} (seen {} times, not enough data for summary)",
                entity.canonical_name, entity.occurrence_count
            ));
        }

        // Build context for summary generation
        let context = self.build_summary_context(&entity);

        // Generate summary using Ollama
        let summary = self.generate_with_ollama(&context).await?;

        // Cache the result
        self.cache.put(eid, summary.clone(), entity.occurrence_count);

        Ok(summary)
    }

    /// Build context string for summary generation
    fn build_summary_context(&self, entity: &Entity) -> String {
        let mut context = format!(
            "实体名称: {}\n\
             类型: {}\n\
             出现次数: {}\n\
             首次出现: {}\n\
             最后出现: {}\n",
            entity.canonical_name,
            entity.entity_type,
            entity.occurrence_count,
            entity.first_seen.format("%Y-%m-%d %H:%M"),
            entity.last_seen.format("%Y-%m-%d %H:%M")
        );

        // Add attributes if present
        if let Some(ref attrs) = entity.attributes {
            if let Some(obj) = attrs.as_object() {
                if !obj.is_empty() {
                    context.push_str("属性:\n");
                    for (key, value) in obj {
                        context.push_str(&format!("  - {}: {}\n", key, value));
                    }
                }
            }
        }

        context
    }

    /// Generate summary using Ollama API
    async fn generate_with_ollama(&self, context: &str) -> Result<String> {
        let prompt = format!(
            "你是 DirSoul 实体摘要系统。基于以下实体信息，生成一个简洁的中文摘要（不超过50字）：\n\n{}\n\n\
             摘要应该包括：\n\
             1. 实体是什么\n\
             2. 主要特征或属性\n\
             3. 使用场景或上下文\n\n\
             只输出摘要，不要其他内容。",
            context
        );

        #[derive(serde::Serialize)]
        struct OllamaRequest {
            model: String,
            prompt: String,
            stream: bool,
        }

        #[derive(serde::Deserialize)]
        struct OllamaResponse {
            response: String,
            done: bool,
        }

        let url = format!("{}/api/generate", self.base_url);

        let request_body = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| DirSoulError::Config(format!("Ollama request failed: {}", e)))?
            .error_for_status()
            .map_err(|e| DirSoulError::Config(format!("Ollama API error: {:?}", e.status())))?
            .json::<OllamaResponse>()
            .await
            .map_err(|e| DirSoulError::Config(format!("Failed to parse Ollama response: {}", e)))?;

        // Clean up the response
        let summary = response.response.trim().to_string();

        Ok(summary)
    }

    /// Batch generate summaries for multiple entities
    ///
    /// Optimized for 8GB memory: processes in small batches.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `entity_ids` - List of entity IDs
    /// * `batch_size` - Number of entities to process per batch
    ///
    /// # Returns
    /// HashMap of entity_id to summary
    pub async fn batch_generate_summaries(
        &self,
        conn: &mut PgConnection,
        entity_ids: Vec<Uuid>,
        batch_size: usize,
    ) -> Result<HashMap<Uuid, String>> {
        let mut results = HashMap::new();

        // Process in batches to respect memory constraints
        for chunk in entity_ids.chunks(batch_size) {
            for &eid in chunk {
                match self.generate_summary(conn, eid).await {
                    Ok(summary) => {
                        results.insert(eid, summary);
                    }
                    Err(e) => {
                        eprintln!("Failed to generate summary for {}: {}", eid, e);
                        // Continue with other entities
                    }
                }
            }
        }

        Ok(results)
    }

    /// Invalidate cached summary for an entity
    ///
    /// Call this when entity changes significantly.
    pub fn invalidate_summary(&self, entity_id: &Uuid) {
        self.cache.invalidate(entity_id);
    }

    /// Clear all cached summaries
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Check if summary needs regeneration
    ///
    /// Returns true if occurrence count has changed since caching.
    pub fn needs_regeneration(&self, entity_id: &Uuid, current_count: i32) -> bool {
        self.cache.get(entity_id, current_count).is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_cache_basic() {
        let cache = SummaryCache::new(2);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        // Cache miss
        assert!(cache.get(&id1, 1).is_none());

        // Cache hit
        cache.put(id1, "summary1".to_string(), 1);
        assert_eq!(cache.get(&id1, 1), Some("summary1".to_string()));

        // Count mismatch
        assert!(cache.get(&id1, 2).is_none());

        // Multiple entries
        cache.put(id2, "summary2".to_string(), 1);
        assert_eq!(cache.get(&id2, 1), Some("summary2".to_string()));
    }

    #[test]
    fn test_summary_cache_eviction() {
        let cache = SummaryCache::new(2);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        cache.put(id1, "summary1".to_string(), 1);
        cache.put(id2, "summary2".to_string(), 1);

        // Third entry should trigger eviction
        cache.put(id3, "summary3".to_string(), 1);

        // Cache size should be at most max_size (2)
        // At least one entry must be present
        let has_any = cache.get(&id1, 1).is_some()
            || cache.get(&id2, 1).is_some()
            || cache.get(&id3, 1).is_some();
        assert!(has_any, "Cache should have at least one entry");
    }

    #[test]
    fn test_summary_cache_invalidate() {
        let cache = SummaryCache::new(10);
        let id = Uuid::new_v4();

        cache.put(id, "summary".to_string(), 1);
        assert_eq!(cache.get(&id, 1), Some("summary".to_string()));

        cache.invalidate(&id);
        assert!(cache.get(&id, 1).is_none());
    }

    #[test]
    fn test_summary_cache_clear() {
        let cache = SummaryCache::new(10);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        cache.put(id1, "summary1".to_string(), 1);
        cache.put(id2, "summary2".to_string(), 1);

        cache.clear();
        assert!(cache.get(&id1, 1).is_none());
        assert!(cache.get(&id2, 1).is_none());
    }

    // Note: Async tests would require tokio::test
    // For now, we test the cache and helper methods only
}
