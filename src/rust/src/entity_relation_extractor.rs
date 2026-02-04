//! Entity Relation Extractor Module - Phase 4, Task 4.5
//!
//! Handles entity relationship extraction, strength calculation, and graph queries.
//!
//! # Design Principles (from HEAD.md)
//! - 暂用 Postgres 数组+JSONB 模拟关系图谱
//! - 支持实体消歧和属性动态增长
//! - 规则引擎作为兜底，优先使用 SLM
//!
//! # Core Functionality
//! - `extract_relations()`: Extract relations from events
//! - `update_relation_strength()`: Calculate strength based on co-occurrence
//! - `find_related_entities()`: Graph query for finding connected entities

use diesel::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{DirSoulError, Result};
use crate::models::{Entity, EntityRelation, NewEntityRelation};

/// Relation type enumeration
///
/// Defines common relationship types between entities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// Belongs to (e.g., Apple -> Fruit)
    BelongsTo,
    /// Related to (generic association)
    RelatedTo,
    /// Located at (e.g., Person -> Place)
    LocatedAt,
    /// Works at (e.g., Person -> Organization)
    WorksAt,
    /// Friends with (e.g., Person -> Person)
    FriendsWith,
    /// Family of (e.g., Person -> Person)
    FamilyOf,
    /// Owns (e.g., Person -> Object)
    Owns,
    /// Created by (e.g., Object -> Person)
    CreatedBy,
    /// Part of (e.g., Component -> Product)
    PartOf,
    /// Custom relation type
    Custom(String),
}

impl RelationType {
    /// Get relation type from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "belongs_to" | "属于" => RelationType::BelongsTo,
            "related_to" | "相关" => RelationType::RelatedTo,
            "located_at" | "位于" => RelationType::LocatedAt,
            "works_at" | "工作于" => RelationType::WorksAt,
            "friends_with" | "朋友" => RelationType::FriendsWith,
            "family_of" | "家人" => RelationType::FamilyOf,
            "owns" | "拥有" => RelationType::Owns,
            "created_by" | "创建于" => RelationType::CreatedBy,
            "part_of" | "部分" => RelationType::PartOf,
            other => RelationType::Custom(other.to_string()),
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        match self {
            RelationType::BelongsTo => "belongs_to".to_string(),
            RelationType::RelatedTo => "related_to".to_string(),
            RelationType::LocatedAt => "located_at".to_string(),
            RelationType::WorksAt => "works_at".to_string(),
            RelationType::FriendsWith => "friends_with".to_string(),
            RelationType::FamilyOf => "family_of".to_string(),
            RelationType::Owns => "owns".to_string(),
            RelationType::CreatedBy => "created_by".to_string(),
            RelationType::PartOf => "part_of".to_string(),
            RelationType::Custom(s) => s.clone(),
        }
    }

    /// Get Chinese display name
    pub fn zh_name(&self) -> String {
        match self {
            RelationType::BelongsTo => "属于".to_string(),
            RelationType::RelatedTo => "相关".to_string(),
            RelationType::LocatedAt => "位于".to_string(),
            RelationType::WorksAt => "工作于".to_string(),
            RelationType::FriendsWith => "朋友".to_string(),
            RelationType::FamilyOf => "家人".to_string(),
            RelationType::Owns => "拥有".to_string(),
            RelationType::CreatedBy => "创建于".to_string(),
            RelationType::PartOf => "部分".to_string(),
            RelationType::Custom(s) => s.clone(),
        }
    }
}

/// Extracted relation from context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    /// Source entity mention
    pub source: String,
    /// Target entity mention
    pub target: String,
    /// Type of relationship
    pub relation_type: RelationType,
    /// Confidence in this extraction (0-1)
    pub confidence: f64,
}

/// Entity relation extractor configuration
#[derive(Debug, Clone)]
pub struct RelationExtractorConfig {
    /// Ollama API URL
    pub ollama_url: String,
    /// Model to use for extraction
    pub model: String,
    /// Timeout for API requests (seconds)
    pub timeout_secs: u64,
    /// Co-occurrence window for strength calculation (hours)
    pub co_occurrence_window_hours: i64,
    /// Minimum strength threshold for keeping relations
    pub min_strength_threshold: f64,
}

impl Default for RelationExtractorConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://127.0.0.1:11434".to_string(),
            model: "phi4-mini".to_string(),
            timeout_secs: 30,
            co_occurrence_window_hours: 24, // 24 hour window
            min_strength_threshold: 0.1,
        }
    }
}

/// Entity relation extractor
///
/// Handles extraction of relationships between entities from events.
pub struct EntityRelationExtractor {
    config: RelationExtractorConfig,
    http_client: Client,
}

impl EntityRelationExtractor {
    /// Create a new relation extractor with default config
    pub fn new() -> Self {
        Self::with_config(RelationExtractorConfig::default())
    }

    /// Create a new relation extractor with custom config
    pub fn with_config(config: RelationExtractorConfig) -> Self {
        let timeout = std::time::Duration::from_secs(config.timeout_secs);
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { config, http_client }
    }

    /// Extract relations from event text using rule-based approach
    ///
    /// This is the fallback method when SLM is unavailable.
    /// Uses patterns like "X 是 Y" (X is Y), "X 属于 Y" (X belongs to Y).
    ///
    /// # Arguments
    /// * `text` - The event text to analyze
    /// * `entities` - Entities mentioned in the text
    ///
    /// # Returns
    /// List of extracted relations
    pub fn extract_relations_rule_based(
        &self,
        text: &str,
        entities: &[Entity],
    ) -> Vec<ExtractedRelation> {
        let mut relations = Vec::new();

        let entity_names: Vec<&str> = entities.iter().map(|e| e.canonical_name.as_str()).collect();

        // Pattern 1: "X 是 Y" or "X 是 ... Y" - often indicates classification
        // This handles "苹果是一种水果" where there are words between "是" and the target
        for (i, source) in entity_names.iter().enumerate() {
            for target in entity_names.iter().skip(i + 1) {
                // Check for "source ... 是 ... target" pattern (source before 是, target after 是)
                if let Some(is_pos) = text.find("是") {
                    let before_is = &text[..is_pos.min(text.len())];
                    let after_is = &text[is_pos + 3..text.len().min(is_pos + 50)]; // +3 for "是" utf8

                    // Check if source appears before "是" and target appears after "是"
                    if before_is.contains(source) && after_is.contains(target) {
                        relations.push(ExtractedRelation {
                            source: source.to_string(),
                            target: target.to_string(),
                            relation_type: RelationType::BelongsTo,
                            confidence: 0.7,
                        });
                    }
                }

                // Check for exact patterns (more specific, higher confidence)
                let patterns = [
                    (format!("{}属于{}", source, target), RelationType::BelongsTo, 0.9),
                    (format!("{} 位于 {}", source, target), RelationType::LocatedAt, 0.9),
                    (format!("{}位于{}", source, target), RelationType::LocatedAt, 0.85),
                ];

                for (pattern, rel_type, conf) in patterns {
                    if text.contains(&pattern) {
                        relations.push(ExtractedRelation {
                            source: source.to_string(),
                            target: target.to_string(),
                            relation_type: rel_type,
                            confidence: conf,
                        });
                    }
                }
            }
        }

        // Pattern 2: Context-based inference
        // If entities appear together in an action context, they're related
        if entity_names.len() >= 2 {
            if text.contains("买") || text.contains("卖") {
                // Commercial activity -> ownership relation
                for (i, source) in entity_names.iter().enumerate() {
                    for target in entity_names.iter().skip(i + 1) {
                        relations.push(ExtractedRelation {
                            source: source.to_string(),
                            target: target.to_string(),
                            relation_type: RelationType::RelatedTo,
                            confidence: 0.5,
                        });
                    }
                }
            }
        }

        relations
    }

    /// Extract relations using SLM (Phi-4-mini via Ollama)
    ///
    /// # Arguments
    /// * `text` - The event text to analyze
    /// * `entities` - Entities mentioned in the text
    ///
    /// # Returns
    /// List of extracted relations
    pub async fn extract_relations_slm(
        &self,
        text: &str,
        entities: &[Entity],
    ) -> Result<Vec<ExtractedRelation>> {
        if entities.is_empty() || entities.len() < 2 {
            return Ok(Vec::new());
        }

        let entity_list: String = entities
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{}. {}", i + 1, e.canonical_name))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"你是 DirSoul 实体关系抽取系统。从文本中提取实体之间的关系。

文本：{}

实体列表：
{}

请分析这些实体之间的关系，输出 JSON 数组格式。每个关系包含：
- source: 源实体名称（必须从上面列表中选择）
- target: 目标实体名称（必须从上面列表中选择）
- relation_type: 关系类型（belongs_to/related_to/located_at/works_at/friends_with/family_of/owns/created_by/part_of）
- confidence: 置信度（0-1之间的浮点数）

只返回最确定的关系，不要过度推断。如果没有明确关系，返回空数组。

输出格式示例：
[
  {{"source": "苹果", "target": "水果", "relation_type": "belongs_to", "confidence": 0.9}},
  {{"source": "张三", "target": "北京", "relation_type": "located_at", "confidence": 0.8}}
]

请只输出 JSON 数组，不要其他内容："#,
            text, entity_list
        );

        let response = self
            .http_client
            .post(format!("{}/api/generate", self.config.ollama_url))
            .json(&serde_json::json!({
                "model": self.config.model,
                "prompt": prompt,
                "stream": false,
                "options": {
                    "temperature": 0.3,
                    "num_predict": 500
                }
            }))
            .send()
            .await
            .map_err(|e| DirSoulError::ExternalError(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DirSoulError::ExternalError(format!(
                "Ollama returned status: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| DirSoulError::ExternalError(format!("Failed to parse Ollama response: {}", e)))?;

        let response_text = json["response"]
            .as_str()
            .ok_or_else(|| DirSoulError::ExternalError("No response text".to_string()))?;

        // Parse JSON array from response
        let parsed_relations: Vec<serde_json::Value> = serde_json::from_str(response_text)
            .unwrap_or_else(|_| serde_json::from_str::<Vec<serde_json::Value>>("[]").unwrap());

        let mut relations = Vec::new();
        for rel in parsed_relations {
            if let (Some(source), Some(target), Some(rel_type), Some(confidence)) = (
                rel.get("source").and_then(|v| v.as_str()),
                rel.get("target").and_then(|v| v.as_str()),
                rel.get("relation_type").and_then(|v| v.as_str()),
                rel.get("confidence").and_then(|v| v.as_f64()),
            ) {
                relations.push(ExtractedRelation {
                    source: source.to_string(),
                    target: target.to_string(),
                    relation_type: RelationType::from_str(rel_type),
                    confidence: confidence.clamp(0.0, 1.0),
                });
            }
        }

        Ok(relations)
    }

    /// Save relations to database
    ///
    /// Creates or updates relation records based on extracted relations.
    pub fn save_relations(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        source_id: Uuid,
        target_id: Uuid,
        rel_type: RelationType,
        conf_value: f64,
    ) -> Result<EntityRelation> {
        use crate::schema::entity_relations::dsl::*;

        let relation_type_str = rel_type.to_string();

        // Check if relation already exists
        let existing = entity_relations
            .filter(user_id.eq(uid))
            .filter(source_entity_id.eq(source_id))
            .filter(target_entity_id.eq(target_id))
            .filter(relation_type.eq(&relation_type_str))
            .first::<EntityRelation>(conn);

        match existing {
            Ok(mut rel) => {
                // Update existing relation
                let now = chrono::Utc::now();

                // Update confidence using weighted average
                let new_confidence = (rel.confidence * rel.strength + conf_value) / (rel.strength + 1.0);
                rel.strength += 1.0;
                rel.confidence = new_confidence;
                rel.last_seen = now;

                diesel::update(entity_relations.find(rel.relation_id))
                    .set((
                        strength.eq(rel.strength),
                        confidence.eq(rel.confidence),
                        last_seen.eq(rel.last_seen),
                    ))
                    .execute(conn)?;

                Ok(rel)
            }
            Err(_) => {
                // Create new relation
                let new_relation = NewEntityRelation::new(
                    uid.to_string(),
                    source_id,
                    target_id,
                    relation_type_str.clone(),
                )
                .with_confidence(conf_value)
                .with_strength(1.0);

                diesel::insert_into(entity_relations)
                    .values(&new_relation)
                    .execute(conn)?;

                // Query the inserted relation
                let inserted = entity_relations
                    .filter(user_id.eq(uid))
                    .filter(source_entity_id.eq(source_id))
                    .filter(target_entity_id.eq(target_id))
                    .filter(relation_type.eq(&relation_type_str))
                    .order(first_seen.desc())
                    .first::<EntityRelation>(conn)?;

                Ok(inserted)
            }
        }
    }

    /// Calculate relation strength based on co-occurrence
    ///
    /// Analyzes events to find how often entities appear together within a time window.
    pub fn calculate_co_occurrence_strength(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        entity_id_1: Uuid,
        entity_id_2: Uuid,
    ) -> Result<f64> {
        use crate::schema::event_memories::dsl::*;
        use crate::schema::entities::dsl as entities_dsl;

        let window_start = chrono::Utc::now() - chrono::Duration::hours(self.config.co_occurrence_window_hours);

        // Find events where both entities appear in the target field
        // This is a simplified approach - in production, we'd need full entity linking
        let events = event_memories
            .filter(user_id.eq(uid))
            .filter(timestamp.ge(window_start))
            .load::<crate::models::EventMemory>(conn)?;

        let mut co_occurrence_count = 0;
        let mut entity1_count = 0;
        let mut entity2_count = 0;

        // Get entity names
        let entity1_name = entities_dsl::entities
            .find(entity_id_1)
            .first::<Entity>(conn);
        let entity2_name = entities_dsl::entities
            .find(entity_id_2)
            .first::<Entity>(conn);

        let e1_name = entity1_name.map(|e| e.canonical_name.to_lowercase()).ok();
        let e2_name = entity2_name.map(|e| e.canonical_name.to_lowercase()).ok();

        for event in &events {
            // Check if entity names appear in event target
            // In production, we'd have proper entity_id references in events
            let target_lower = event.target.to_lowercase();

            let entity1_present = e1_name.as_ref().map_or(false, |n| target_lower.contains(n));
            let entity2_present = e2_name.as_ref().map_or(false, |n| target_lower.contains(n));

            if entity1_present {
                entity1_count += 1;
            }
            if entity2_present {
                entity2_count += 1;
            }
            if entity1_present && entity2_present {
                co_occurrence_count += 1;
            }
        }

        // Calculate strength using Jaccard-like coefficient
        let strength = if entity1_count == 0 || entity2_count == 0 {
            0.0
        } else {
            let union = entity1_count + entity2_count - co_occurrence_count;
            if union == 0 {
                0.0
            } else {
                co_occurrence_count as f64 / union as f64
            }
        };

        Ok(strength)
    }

    /// Find entities related to a given entity
    ///
    /// Graph query: find all entities that have relations with the given entity.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `uid` - User ID
    /// * `entity_id` - Entity to find relations for
    /// * `min_strength` - Minimum relation strength threshold
    ///
    /// # Returns
    /// List of tuples: (related_entity, relation, reverse_relation)
    pub fn find_related_entities(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        entity_id: Uuid,
        min_strength: Option<f64>,
    ) -> Result<Vec<(Entity, EntityRelation, Option<EntityRelation>)>> {
        use crate::schema::entity_relations::dsl::*;
        use crate::schema::entities::dsl as entities_dsl;

        let strength_threshold = min_strength.unwrap_or(self.config.min_strength_threshold);

        // Find outgoing relations (source = entity_id)
        let outgoing = entity_relations
            .filter(user_id.eq(uid))
            .filter(source_entity_id.eq(entity_id))
            .filter(strength.ge(strength_threshold))
            .load::<EntityRelation>(conn)?;

        // Find incoming relations (target = entity_id)
        let incoming = entity_relations
            .filter(user_id.eq(uid))
            .filter(target_entity_id.eq(entity_id))
            .filter(strength.ge(strength_threshold))
            .load::<EntityRelation>(conn)?;

        let mut results = Vec::new();

        // Process outgoing relations
        for rel in outgoing {
            if let Ok(target_entity) = entities_dsl::entities.find(rel.target_entity_id).first::<Entity>(conn) {
                results.push((target_entity, rel.clone(), None));
            }
        }

        // Process incoming relations
        for rel in incoming {
            if let Ok(source_entity) = entities_dsl::entities.find(rel.source_entity_id).first::<Entity>(conn) {
                results.push((source_entity, rel.clone(), Some(rel)));
            }
        }

        Ok(results)
    }

    /// Find shortest path between two entities (BFS)
    ///
    /// Graph traversal to find how two entities are connected.
    ///
    /// # Returns
    /// List of entity IDs forming the path, or empty if no path exists
    pub fn find_path(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        start_id: Uuid,
        end_id: Uuid,
        max_depth: usize,
    ) -> Result<Vec<Uuid>> {
        use crate::schema::entity_relations::dsl::*;
        use std::collections::{HashMap, VecDeque};

        if start_id == end_id {
            return Ok(vec![start_id]);
        }

        // BFS
        let mut queue = VecDeque::new();
        let mut visited: HashMap<Uuid, Option<Uuid>> = HashMap::new();

        queue.push_back((start_id, 0));
        visited.insert(start_id, None);

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // Find all neighbors
            let outgoing = entity_relations
                .filter(user_id.eq(uid))
                .filter(source_entity_id.eq(current))
                .load::<EntityRelation>(conn)?;

            for rel in outgoing {
                if !visited.contains_key(&rel.target_entity_id) {
                    visited.insert(rel.target_entity_id, Some(current));
                    queue.push_back((rel.target_entity_id, depth + 1));

                    if rel.target_entity_id == end_id {
                        // Reconstruct path
                        let mut path = vec![end_id];
                        let mut current_id = Some(end_id);

                        while let Some(id) = current_id {
                            if let Some(&parent) = visited.get(&id) {
                                if let Some(p) = parent {
                                    path.push(p);
                                    current_id = Some(p);
                                } else {
                                    current_id = None;
                                }
                            } else {
                                current_id = None;
                            }
                        }

                        path.reverse();
                        return Ok(path);
                    }
                }
            }

            // Also check incoming relations
            let incoming = entity_relations
                .filter(user_id.eq(uid))
                .filter(target_entity_id.eq(current))
                .load::<EntityRelation>(conn)?;

            for rel in incoming {
                if !visited.contains_key(&rel.source_entity_id) {
                    visited.insert(rel.source_entity_id, Some(current));
                    queue.push_back((rel.source_entity_id, depth + 1));

                    if rel.source_entity_id == end_id {
                        // Same path reconstruction
                        let mut path = vec![end_id];
                        let mut current_id = Some(end_id);

                        while let Some(id) = current_id {
                            if let Some(&parent) = visited.get(&id) {
                                if let Some(p) = parent {
                                    path.push(p);
                                    current_id = Some(p);
                                } else {
                                    current_id = None;
                                }
                            } else {
                                current_id = None;
                            }
                        }

                        path.reverse();
                        return Ok(path);
                    }
                }
            }
        }

        // No path found
        Ok(Vec::new())
    }

    /// Get relation statistics for an entity
    ///
    /// Returns counts of different relation types for the entity.
    pub fn get_relation_stats(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        entity_id: Uuid,
    ) -> Result<HashMap<String, i64>> {
        use crate::schema::entity_relations::dsl::*;

        let relations = entity_relations
            .filter(user_id.eq(uid))
            .filter(source_entity_id.eq(entity_id))
            .or_filter(target_entity_id.eq(entity_id))
            .load::<EntityRelation>(conn)?;

        let mut stats = HashMap::new();

        for rel in relations {
            *stats.entry(rel.relation_type.clone()).or_insert(0) += 1;
        }

        Ok(stats)
    }
}

impl Default for EntityRelationExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_type_from_str() {
        assert_eq!(RelationType::from_str("belongs_to"), RelationType::BelongsTo);
        assert_eq!(RelationType::from_str("属于"), RelationType::BelongsTo);
        assert_eq!(RelationType::from_str("related_to"), RelationType::RelatedTo);
        assert_eq!(RelationType::from_str("located_at"), RelationType::LocatedAt);
    }

    #[test]
    fn test_relation_type_to_string() {
        assert_eq!(RelationType::BelongsTo.to_string(), "belongs_to");
        assert_eq!(RelationType::RelatedTo.to_string(), "related_to");
        assert_eq!(RelationType::LocatedAt.to_string(), "located_at");
    }

    #[test]
    fn test_relation_type_zh_name() {
        assert_eq!(RelationType::BelongsTo.zh_name(), "属于");
        assert_eq!(RelationType::RelatedTo.zh_name(), "相关");
        assert_eq!(RelationType::LocatedAt.zh_name(), "位于");
    }

    #[test]
    fn test_extract_relations_rule_based_simple() {
        let extractor = EntityRelationExtractor::new();

        let entities = vec![
            Entity {
                entity_id: Uuid::new_v4(),
                user_id: "test".to_string(),
                canonical_name: "苹果".to_string(),
                entity_type: "object".to_string(),
                attributes: None,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrence_count: 1,
                confidence: 0.8,
            },
            Entity {
                entity_id: Uuid::new_v4(),
                user_id: "test".to_string(),
                canonical_name: "水果".to_string(),
                entity_type: "concept".to_string(),
                attributes: None,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrence_count: 1,
                confidence: 0.8,
            },
        ];

        let relations = extractor.extract_relations_rule_based(
            "苹果是一种水果",
            &entities,
        );

        assert!(!relations.is_empty());
        assert_eq!(relations[0].source, "苹果");
        assert_eq!(relations[0].target, "水果");
        assert_eq!(relations[0].relation_type, RelationType::BelongsTo);
    }

    #[test]
    fn test_extract_relations_rule_based_location() {
        let extractor = EntityRelationExtractor::new();

        let entities = vec![
            Entity {
                entity_id: Uuid::new_v4(),
                user_id: "test".to_string(),
                canonical_name: "张三".to_string(),
                entity_type: "person".to_string(),
                attributes: None,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrence_count: 1,
                confidence: 0.8,
            },
            Entity {
                entity_id: Uuid::new_v4(),
                user_id: "test".to_string(),
                canonical_name: "北京".to_string(),
                entity_type: "place".to_string(),
                attributes: None,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrence_count: 1,
                confidence: 0.8,
            },
        ];

        let relations = extractor.extract_relations_rule_based(
            "张三位于北京",
            &entities,
        );

        assert!(!relations.is_empty());
        assert_eq!(relations[0].relation_type, RelationType::LocatedAt);
    }

    #[test]
    fn test_extracted_relation_serialization() {
        let rel = ExtractedRelation {
            source: "苹果".to_string(),
            target: "水果".to_string(),
            relation_type: RelationType::BelongsTo,
            confidence: 0.9,
        };

        let json = serde_json::to_string(&rel).unwrap();
        assert!(json.contains("苹果"));
        assert!(json.contains("水果"));
        assert!(json.contains("belongs_to"));
    }

    #[test]
    fn test_config_default() {
        let config = RelationExtractorConfig::default();
        assert_eq!(config.model, "phi4-mini");
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.co_occurrence_window_hours, 24);
    }
}
