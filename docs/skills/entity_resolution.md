# Skill: Entity Resolution

> **Purpose**: Guide entity discovery and disambiguation, ensuring structured memory accuracy by dynamically growing entities from events without introducing hallucinations.

---

## Entity Schema (Declarative)

```rust
/// Core entity structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Entity {
    pub entity_id: Uuid,
    pub user_id: String,

    /// Canonical name after disambiguation
    pub canonical_name: String,

    /// Entity type (food, person, location, etc.)
    pub entity_type: EntityType,

    /// Dynamic attributes (JSONB for flexibility)
    pub attributes: serde_json::Value,

    /// Confidence in entity typing
    pub type_confidence: f32,

    /// First and last mention timestamps
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,

    /// How many times mentioned
    pub mention_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EntityType {
    Food,
    Person,
    Location,
    Activity,
    Object,
    Concept,
    Organization,
    Unknown,
}
```

---

## Disambiguation Rules

### Context-Based Pattern Matching

```rust
/// Disambiguate entity mentions based on surrounding context
pub struct EntityDisambiguator {
    entity_store: EntityStore,
}

impl EntityDisambiguator {
    /// Resolve "苹果" → Apple (company) vs apple (fruit) based on context
    pub fn resolve_entity(
        &self,
        mention: &str,
        context: &str,
        timestamp: DateTime<Utc>
    ) -> Result<Entity> {
        // 1. Check for existing entities
        let candidates = self.entity_store.find_by_name(mention)?;

        if candidates.is_empty() {
            // No existing entity - create new one
            return self.create_new_entity(mention, context, timestamp);
        }

        // 2. Disambiguate if multiple candidates exist
        if candidates.len() == 1 {
            return Ok(candidates[0].clone());
        }

        // 3: Multiple matches - use context to disambiguate
        self.disambiguate_by_context(mention, context, &candidates, timestamp)
    }

    fn disambiguate_by_context(
        &self,
        mention: &str,
        context: &str,
        candidates: &[Entity],
        timestamp: DateTime<Utc>
    ) -> Result<Entity> {
        let mut scores: Vec<(f32, &Entity)> = candidates
            .iter()
            .map(|e| (self.context_similarity(context, e), e))
            .collect();

        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        // Return best match if score is above threshold
        if let Some((score, entity)) = scores.first() {
            if *score > 0.6 {
                return Ok((*entity).clone());
            }
        }

        // Create new entity if no good match
        self.create_new_entity(mention, context, timestamp)
    }

    /// Calculate similarity between context and entity's historical context
    fn context_similarity(&self, context: &str, entity: &Entity) -> f32 {
        let entity_context = self.entity_store.get_entity_context(&entity.entity_id);

        // Keyword overlap scoring
        let context_words: std::collections::HashSet<&str> =
            context.split_whitespace().collect();
        let entity_words: std::collections::HashSet<&str> =
            entity_context.split_whitespace().collect();

        let intersection = context_words.intersection(&entity_words).count();
        let union = context_words.union(&entity_words).count();

        if union == 0 { return 0.0; }

        intersection as f32 / union as f32
    }
}
```

### Context Indicators

```rust
/// Disambiguation hints from context keywords
pub struct ContextHints {
    food_keywords: &'static [&'static str],
    company_keywords: &'static [&'static str],
    activity_keywords: &'static [&'static str],
}

impl ContextHints {
    pub const fn new() -> Self {
        Self {
            food_keywords: &["吃", "喝", "煮", "炒", "烤", "味道", "口感", "甜", "咸"],
            company_keywords: &["股票", "股价", "上市", "公司", "投资", "财报", "市值"],
            activity_keywords: &["去", "在", "运动", "玩", "参加"],
        }
    }

    /// Detect entity type from context
    pub fn detect_type(&self, mention: &str, context: &str) -> EntityType {
        // Special cases
        if mention == "苹果" {
            if self.company_keywords.iter().any(|k| context.contains(k)) {
                return EntityType::Organization;
            }
            if self.food_keywords.iter().any(|k| context.contains(k)) {
                return EntityType::Food;
            }
        }

        // General context-based detection
        if self.food_keywords.iter().any(|k| context.contains(k)) {
            return EntityType::Food;
        }
        if self.activity_keywords.iter().any(|k| context.contains(k)) {
            return EntityType::Activity;
        }

        EntityType::Unknown
    }
}
```

---

## Attribute Growth Patterns

### JSONB Dynamic Updates

```sql
-- PostgreSQL schema for flexible entity attributes
CREATE TABLE entities (
    entity_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    canonical_name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    attributes JSONB DEFAULT '{}',
    type_confidence FLOAT NOT NULL DEFAULT 0.5,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    mention_count INTEGER NOT NULL DEFAULT 1,
    UNIQUE(user_id, canonical_name)
);

-- Example attributes structure:
-- Food: {"category": "水果", "color": "红色", "taste": "甜", "typical_quantity": 3}
-- Person: {"relationship": "friend", "contact": "wechat", "last_contact": "2026-01-15"}
-- Location: {"type": "restaurant", "area": "朝阳区", "visit_frequency": "weekly"}
```

### Attribute Extraction and Update

```rust
/// Extract and merge attributes from new mentions
pub fn update_attributes(
    entity: &Entity,
    new_context: &str
) -> serde_json::Value {
    let mut attributes = entity.attributes.clone();

    // Extract new attributes from context
    if let Some(new_attrs) = extract_attributes(new_context, &entity.entity_type) {
        // Merge with confidence weighting
        merge_with_confidence(&mut attributes, new_attrs);
    }

    attributes
}

fn extract_attributes(context: &str, entity_type: &EntityType) -> Option<serde_json::Value> {
    match entity_type {
        EntityType::Food => {
            let mut attrs = json!({});

            // Extract color
            if let Some(color) = extract_color(context) {
                attrs["color"] = json!(color);
            }

            // Extract taste
            if let Some(taste) = extract_taste(context) {
                attrs["taste"] = json!(taste);
            }

            // Extract category
            if let Some(category) = extract_category(context) {
                attrs["category"] = json!(category);
            }

            if attrs.as_object().map_or(false, |m| !m.is_empty()) {
                Some(attrs)
            } else {
                None
            }
        }
        _ => None,  // Implement for other types
    }
}

fn merge_with_confidence(
    existing: &mut serde_json::Value,
    new: serde_json::Value
) {
    if let (Some(existing_obj), Some(new_obj)) = (existing.as_object_mut(), new.as_object()) {
        for (key, new_value) in new_obj {
            if let Some(old_value) = existing_obj.get(&key) {
                // Both exist - use higher confidence or merge
                if is_more_confident(&new_value, old_value) {
                    existing_obj.insert(key, new_value);
                }
            } else {
                // New attribute - add it
                existing_obj.insert(key, new_value);
            }
        }
    }
}
```

### Confidence Calculation Formula

```rust
/// Calculate entity attribute confidence
pub fn calculate_attribute_confidence(
    mention_count: i32,
    consistency_score: f32,
    recency_factor: f32
) -> f32 {
    // Base confidence from mention frequency
    let frequency_confidence = (mention_count as f32 / 100.0).min(0.5);

    // Consistency across mentions
    let consistency_weight = 0.3;

    // Recency bonus (recent mentions are more relevant)
    let recency_weight = 0.2;

    frequency_confidence
        + (consistency_score * consistency_weight)
        + (recency_factor * recency_weight)
}

/// Recency factor decays with time
pub fn recency_factor(last_seen: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
    let days_ago = (now - last_seen).num_days() as f32;

    // Decay over 90 days
    (-days_ago / 90.0).exp().max(0.1)
}
```

---

## Relationship Building

### Co-occurrence Strength Calculation

```rust
/// Entity relationship based on co-occurrence in same event/context
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub relation_id: Uuid,
    pub source_entity_id: Uuid,
    pub target_entity_id: Uuid,
    pub relation_type: RelationType,
    pub strength: f32,
    pub first_co_occurrence: DateTime<Utc>,
    pub last_co_occurrence: DateTime<Utc>,
    pub co_occurrence_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RelationType {
    RelatedTo,
    PartOf,
    UsedFor,
    LocatedAt,
    SimilarTo,
    Custom(String),
}

impl EntityRelationship {
    /// Calculate relationship strength
    pub fn calculate_strength(
        co_occurrence_count: i32,
        time_span_days: i64,
        context_relevance: f32
    ) -> f32 {
        // Frequency component
        let frequency = (co_occurrence_count as f32).log10() / 10.0;

        // Time decay (recent co-occurrences are stronger)
        let time_decay = (-time_span_days as f32 / 365.0).exp();

        // Context relevance (from NLP similarity)
        let context_weight = 0.4;

        (frequency * 0.3 + time_decay * 0.3 + context_relevance * context_weight)
            .min(1.0)
            .max(0.0)
    }
}
```

### Relationship Discovery

```rust
/// Discover relationships from event co-occurrences
pub struct RelationshipBuilder {
    store: EntityStore,
}

impl RelationshipBuilder {
    /// Find entities mentioned in same time window
    pub async fn discover_co_occurrences(
        &self,
        entity_id: Uuid,
        time_window: Duration
    ) -> Result<Vec<EntityRelationship>> {
        let entity = self.store.get_entity(entity_id)?;

        // Find mentions of this entity
        let mentions = self.store.get_entity_mentions(
            entity_id,
            entity.first_seen..entity.last_seen
        )?;

        let mut co_occurrences: std::collections::Map<Uuid, i32> = std::collections::HashMap::new();

        // For each mention, find other entities mentioned nearby
        for mention in &mentions {
            let nearby_entities = self.store.find_entities_nearby(
                mention.timestamp,
                time_window
            )?;

            for other_id in nearby_entities {
                if other_id != entity_id {
                    *co_occurrences.entry(other_id).or_insert(0) += 1;
                }
            }
        }

        // Convert to relationships
        co_occurrences
            .into_iter()
            .map(|(target_id, count)| {
                Ok(EntityRelationship {
                    relation_id: Uuid::new_v4(),
                    source_entity_id: entity_id,
                    target_entity_id,
                    relation_type: RelationType::RelatedTo,
                    strength: Self::estimate_strength(count),
                    first_co_occurrence: entity.first_seen,
                    last_co_occurrence: entity.last_seen,
                    co_occurrence_count: count,
                })
            })
            .collect()
    }

    fn estimate_strength(count: i32) -> f32 {
        // Simple strength estimate from co-occurrence count
        match count {
            1..=3 => 0.2,
            4..=10 => 0.5,
            11..=30 => 0.7,
            _ => 0.9,
        }
    }
}
```

---

## Anti-Hallucination Measures

### Confidence Thresholds

```rust
/// Only create/update high-confidence entities
pub const ENTITY_CONFIDENCE_THRESHOLD: f32 = 0.5;
pub const ATTRIBUTE_CONFIDENCE_THRESHOLD: f32 = 0.6;

/// Verify entity extraction before committing
pub fn verify_entity_extraction(
    entity: &Entity,
    source_events: &[Event]
) -> bool {
    // Must have at least 2 mentions
    if entity.mention_count < 2 {
        return false;
    }

    // Type confidence must be above threshold
    if entity.type_confidence < ENTITY_CONFIDENCE_THRESHOLD {
        return false;
    }

    // Attributes must be consistent across mentions
    if !verify_attribute_consistency(entity, source_events) {
        return false;
    }

    true
}

fn verify_attribute_consistency(entity: &Entity, events: &[Event]) -> bool {
    // Check that key attributes don't contradict
    // Implementation depends on entity type
    true
}
```

---

## Recommended Combinations

Use this skill together with:
- **EventExtractionPatterns**: For initial entity mentions from events
- **CognitiveViewGeneration**: For building patterns from entity relationships
- **PostgresSchemaDesign**: For entities table schema with JSONB attributes
- **OllamaPromptEngineering**: For entity type classification prompts
