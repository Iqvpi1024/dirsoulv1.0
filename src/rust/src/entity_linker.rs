//! Entity Linker Module - Phase 4, Task 4.2
//!
//! Handles entity discovery and linking from events to entities.
//!
//! # Design Principles (from HEAD.md)
//! - 暂用 Postgres 数组+JSONB 模拟关系图谱
//! - 支持实体消歧和属性动态增长
//! - Context-based disambiguation for polysemous entities
//!
//! # Core Functionality
//! - `link_entity()`: Link mentions to existing or new entities
//! - Context disambiguation: "吃苹果" → fruit, "买苹果股票" → company
//! - Entity updates: occurrence_count, last_seen, attributes

use diesel::prelude::*;

use crate::error::Result;
use crate::models::{Entity, EntityType, NewEntity};

/// Entity linker for connecting mentions to entities
///
/// Handles entity discovery, linking, and disambiguation based on context.
pub struct EntityLinker {
    /// Similarity threshold for entity matching
    similarity_threshold: f64,
}

impl EntityLinker {
    /// Create a new entity linker with default configuration
    pub fn new() -> Self {
        Self {
            similarity_threshold: 0.75,
        }
    }

    /// Create a new entity linker with custom similarity threshold
    ///
    /// # Arguments
    /// * `similarity_threshold` - Threshold for fuzzy entity matching (0.0 to 1.0)
    pub fn with_threshold(similarity_threshold: f64) -> Self {
        Self {
            similarity_threshold: similarity_threshold.clamp(0.0, 1.0),
        }
    }

    /// Link a mention to an entity (existing or new)
    ///
    /// This is the main entry point for entity linking.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `user_id` - User who owns the entity
    /// * `mention` - The entity mention (e.g., "苹果", "Apple")
    /// * `context` - Context string for disambiguation (e.g., the full event text)
    ///
    /// # Returns
    /// The linked entity (either existing or newly created)
    ///
    /// # Example
    /// ```no_run
    /// # use dirsoul::entity_linker::EntityLinker;
    /// # use diesel::PgConnection;
    /// # fn main() -> dirsoul::Result<()> {
    /// # let mut conn: PgConnection = unimplemented!();
    /// let linker = EntityLinker::new();
    ///
    /// // Context: "吃苹果" → fruit entity
    /// let apple_fruit = linker.link_entity(
    ///     &mut conn,
    ///     "user123",
    ///     "苹果",
    ///     "我今天吃了一个苹果"
    /// )?;
    ///
    /// // Context: "买苹果股票" → company entity
    /// let apple_company = linker.link_entity(
    ///     &mut conn,
    ///     "user123",
    ///     "苹果",
    ///     "我买了一些苹果公司的股票"
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn link_entity(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        mention: &str,
        context: &str,
    ) -> Result<Entity> {
        // Normalize the mention
        let canonical_name = self.normalize_mention(mention);

        // Try to find exact match first
        if let Some(entity) = self.find_exact_match(conn, uid, &canonical_name)? {
            return self.update_entity(conn, entity);
        }

        // Try fuzzy match
        if let Some(entity) = self.find_fuzzy_match(conn, uid, &canonical_name, context)? {
            return self.update_entity(conn, entity);
        }

        // No match found - create new entity
        self.create_entity(conn, uid, &canonical_name, context)
    }

    /// Normalize entity mention to canonical form
    ///
    /// Handles:
    /// - Whitespace trimming
    /// - Case normalization for English
    /// - Common alias mapping (e.g., "苹果" → "Apple")
    fn normalize_mention(&self, mention: &str) -> String {
        let trimmed = mention.trim();

        // Common Chinese-English alias mapping
        // In production, this could be a more comprehensive dictionary
        match trimmed.to_lowercase().as_str() {
            "苹果" | "apple inc" | "apple computer" => "Apple".to_string(),
            "谷歌" | "google" => "Google".to_string(),
            "微软" | "microsoft" => "Microsoft".to_string(),
            "特斯拉" | "tesla" => "Tesla".to_string(),
            _ => {
                // Basic normalization: lowercase English, keep Chinese as-is
                if trimmed.chars().all(|c| c.is_ascii()) {
                    // All ASCII - lowercase and capitalize first letter
                    let lower = trimmed.to_lowercase();
                    if let Some(first_char) = lower.chars().next() {
                        let mut result = first_char.to_uppercase().to_string();
                        result.push_str(&lower[lower.len().min(1)..]);
                        result
                    } else {
                        lower
                    }
                } else {
                    // Contains non-ASCII (likely Chinese) - keep as-is
                    trimmed.to_string()
                }
            }
        }
    }

    /// Find exact match for canonical name
    fn find_exact_match(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        cname: &str,
    ) -> Result<Option<Entity>> {
        use crate::schema::entities::dsl::*;

        let results = entities
            .filter(user_id.eq(uid))
            .filter(canonical_name.eq(cname))
            .limit(1)
            .load::<Entity>(conn)?;

        Ok(results.into_iter().next())
    }

    /// Find fuzzy match based on string similarity and context
    ///
    /// Uses Jaro-Winkler similarity for fuzzy matching.
    /// Context-based disambiguation is performed when multiple candidates exist.
    fn find_fuzzy_match(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        cname: &str,
        context: &str,
    ) -> Result<Option<Entity>> {
        use crate::schema::entities::dsl::*;

        // Load all entities for the user
        // TODO: Add LIKE query filter for performance optimization
        let all_entities = entities
            .filter(user_id.eq(uid))
            .load::<Entity>(conn)?;

        // Calculate similarity scores
        let mut candidates: Vec<_> = all_entities
            .into_iter()
            .filter_map(|entity| {
                let similarity = self.jaro_winkler_similarity(&entity.canonical_name, cname);
                if similarity >= self.similarity_threshold {
                    Some((entity, similarity))
                } else {
                    None
                }
            })
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // Sort by similarity descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // If only one candidate, return it
        if candidates.len() == 1 {
            return Ok(Some(candidates.into_iter().next().unwrap().0));
        }

        // Multiple candidates - use context to disambiguate
        self.disambiguate_by_context(candidates, context)
    }

    /// Disambiguate between similar entities using context
    ///
    /// Examines entity attributes and context to determine the best match.
    ///
    /// # Example
    /// - "吃苹果" + context keywords like "吃", "水果" → fruit entity
    /// - "买苹果股票" + context keywords like "股票", "公司" → company entity
    fn disambiguate_by_context(
        &self,
        candidates: Vec<(Entity, f64)>,
        context: &str,
    ) -> Result<Option<Entity>> {
        // Extract keywords from context
        let context_keywords = self.extract_context_keywords(context);

        // Score each candidate based on context match
        let mut scored_candidates: Vec<_> = candidates
            .into_iter()
            .map(|(entity, similarity)| {
                let context_score = self.calculate_context_score(&entity, &context_keywords);
                let total_score = similarity * 0.6 + context_score * 0.4; // Weighted combination
                (entity, total_score)
            })
            .collect();

        // Sort by total score
        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Return the best match
        Ok(scored_candidates.into_iter().next().map(|(e, _)| e))
    }

    /// Extract keywords from context for disambiguation
    fn extract_context_keywords(&self, context: &str) -> Vec<String> {
        // Simple keyword extraction
        // In production, could use jieba or other NLP tokenization

        let mut keywords = Vec::new();

        // Domain-specific keyword mappings
        let keyword_patterns = [
            (vec!["吃", "喝", "水果", "食物"], "food"),
            (vec!["买", "股票", "公司", "投资"], "company"),
            (vec!["去", "到", "地方", "城市"], "place"),
            (vec!["人", "朋友", "同事"], "person"),
        ];

        for (patterns, domain) in keyword_patterns {
            if patterns.iter().any(|p| context.contains(p)) {
                keywords.push(domain.to_string());
            }
        }

        keywords
    }

    /// Calculate context match score for an entity
    fn calculate_context_score(&self, entity: &Entity, context_keywords: &[String]) -> f64 {
        let entity_type_str = entity.entity_type.to_lowercase();

        for keyword in context_keywords {
            match keyword.as_str() {
                "food" => {
                    if entity_type_str.contains("object") {
                        return 1.0;
                    }
                }
                "company" => {
                    if entity_type_str.contains("organization") {
                        return 1.0;
                    }
                }
                "place" => {
                    if entity_type_str.contains("place") {
                        return 1.0;
                    }
                }
                "person" => {
                    if entity_type_str.contains("person") {
                        return 1.0;
                    }
                }
                _ => {}
            }
        }

        0.0 // No context match
    }

    /// Calculate Jaro-Winkler similarity between two strings
    ///
    /// Returns a value between 0.0 (no match) and 1.0 (exact match).
    /// This is a common algorithm for fuzzy string matching.
    fn jaro_winkler_similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1 == s2 {
            return 1.0;
        }

        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        // Match distance
        let match_distance = (len1.max(len2) / 2).saturating_sub(1);

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        // Find matches
        let mut s1_matches = vec![false; len1];
        let mut s2_matches = vec![false; len2];
        let mut matches = 0;

        for i in 0..len1 {
            let start = i.saturating_sub(match_distance);
            let end = (i + match_distance + 1).min(len2);

            for j in start..end {
                if !s2_matches[j] && s1_chars[i] == s2_chars[j] {
                    s1_matches[i] = true;
                    s2_matches[j] = true;
                    matches += 1;
                    break;
                }
            }
        }

        if matches == 0 {
            return 0.0;
        }

        // Count transpositions
        let mut transpositions = 0;
        let mut k = 0;

        for i in 0..len1 {
            if s1_matches[i] {
                while !s2_matches[k] {
                    k += 1;
                }
                if s1_chars[i] != s2_chars[k] {
                    transpositions += 1;
                }
                k += 1;
            }
        }

        // Calculate Jaro similarity
        let jaro = (
            matches as f64 / len1 as f64
                + matches as f64 / len2 as f64
                + (matches - transpositions / 2) as f64 / matches as f64
        ) / 3.0;

        // Calculate Jaro-Winkler similarity (with prefix scaling)
        let prefix_length = s1_chars
            .iter()
            .zip(s2_chars.iter())
            .take_while(|(a, b)| a == b)
            .take(4)
            .count();

        let jaro_winkler = jaro + (prefix_length as f64 * 0.1 * (1.0 - jaro));

        jaro_winkler
    }

    /// Update existing entity (increment occurrence_count, update last_seen)
    fn update_entity(&self, conn: &mut PgConnection, mut entity: Entity) -> Result<Entity> {
        use crate::schema::entities::dsl::*;

        let now = chrono::Utc::now();

        // Update in memory
        entity.occurrence_count += 1;
        entity.last_seen = now;

        // Update in database
        diesel::update(entities.find(entity.entity_id))
            .set((
                occurrence_count.eq(entity.occurrence_count),
                last_seen.eq(entity.last_seen),
            ))
            .execute(conn)?;

        Ok(entity)
    }

    /// Create new entity based on mention and context
    fn create_entity(
        &self,
        conn: &mut PgConnection,
        uid: &str,
        cname: &str,
        context: &str,
    ) -> Result<Entity> {
        // Infer entity type from context
        let etype = self.infer_entity_type(context);

        // Create new entity
        let new_entity = NewEntity::new(
            uid.to_string(),
            cname.to_string(),
            etype,
        );

        // Insert into database and query it back
        diesel::insert_into(crate::schema::entities::table)
            .values(&new_entity)
            .execute(conn)?;

        // Query the inserted entity (ordered by last_seen DESC to get the most recent)
        use crate::schema::entities::dsl::*;
        let inserted_entity = entities
            .filter(user_id.eq(uid))
            .filter(canonical_name.eq(cname))
            .order(last_seen.desc())
            .first::<Entity>(conn)?;

        Ok(inserted_entity)
    }

    /// Infer entity type from context
    ///
    /// Uses simple keyword matching to determine if entity is:
    /// - Person, Place, Object, Concept, Organization, Event
    fn infer_entity_type(&self, context: &str) -> EntityType {
        let context_lower = context.to_lowercase();

        // Check for concept/idea indicators FIRST (to avoid "人工" triggering "人")
        if context_lower.contains("想法")
            || context_lower.contains("概念")
            || context_lower.contains("理论")
        {
            return EntityType::Concept;
        }

        // Check for person indicators (use more specific patterns)
        // Check before place indicators to avoid "我和朋友去" triggering Place
        if context_lower.contains("朋友")
            || context_lower.contains("同事")
            || context_lower.contains("先生")
            || context_lower.contains("女士")
            || context_lower.contains("医生")
            || context_lower.contains("老师")
        {
            return EntityType::Person;
        }

        // Check for organization/company indicators
        if context_lower.contains("公司")
            || context_lower.contains("股票")
            || context_lower.contains("企业")
            || context_lower.contains("机构")
        {
            return EntityType::Organization;
        }

        // Check for place indicators
        if context_lower.contains("去")
            || context_lower.contains("到")
            || context_lower.contains("地方")
            || context_lower.contains("城市")
            || context_lower.contains("国家")
        {
            return EntityType::Place;
        }

        // Default to Object
        EntityType::Object
    }
}

impl Default for EntityLinker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_mention_chinese() {
        let linker = EntityLinker::new();
        assert_eq!(linker.normalize_mention("  苹果  "), "Apple");
    }

    #[test]
    fn test_normalize_mention_english() {
        let linker = EntityLinker::new();
        assert_eq!(linker.normalize_mention("  apple  "), "Apple");
    }

    #[test]
    fn test_jaro_winkler_exact_match() {
        let linker = EntityLinker::new();
        assert!((linker.jaro_winkler_similarity("Apple", "Apple") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_winkler_no_match() {
        let linker = EntityLinker::new();
        assert!((linker.jaro_winkler_similarity("ABC", "XYZ") - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_winkler_partial_match() {
        let linker = EntityLinker::new();
        let sim = linker.jaro_winkler_similarity("Apple", "Apply");
        assert!(sim > 0.8); // Should be high similarity
        assert!(sim < 1.0); // But not exact
    }

    #[test]
    fn test_extract_context_keywords_food() {
        let linker = EntityLinker::new();
        let keywords = linker.extract_context_keywords("我今天吃了一个苹果");
        assert!(keywords.contains(&"food".to_string()));
    }

    #[test]
    fn test_extract_context_keywords_company() {
        let linker = EntityLinker::new();
        let keywords = linker.extract_context_keywords("我买了一些苹果公司的股票");
        assert!(keywords.contains(&"company".to_string()));
    }

    #[test]
    fn test_infer_entity_type_person() {
        let linker = EntityLinker::new();
        assert_eq!(
            linker.infer_entity_type("我和朋友张三去吃饭"),
            EntityType::Person
        );
    }

    #[test]
    fn test_infer_entity_type_place() {
        let linker = EntityLinker::new();
        assert_eq!(
            linker.infer_entity_type("我去了北京这个城市"),
            EntityType::Place
        );
    }

    #[test]
    fn test_infer_entity_type_organization() {
        let linker = EntityLinker::new();
        assert_eq!(
            linker.infer_entity_type("我买了一些苹果公司的股票"),
            EntityType::Organization
        );
    }

    #[test]
    fn test_infer_entity_type_object() {
        let linker = EntityLinker::new();
        assert_eq!(
            linker.infer_entity_type("我今天吃了一个苹果"),
            EntityType::Object
        );
    }

    #[test]
    fn test_infer_entity_type_concept() {
        let linker = EntityLinker::new();
        assert_eq!(
            linker.infer_entity_type("我有一个关于人工智能的想法"),
            EntityType::Concept
        );
    }
}
