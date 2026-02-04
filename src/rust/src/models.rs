//! DirSoul Data Models
//!
//! Core data structures for the memory system, following Rust memory safety
//! principles and Diesel ORM patterns.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{entities, entity_relations, event_memories, raw_memories};

/// Content type enumeration for raw memories
///
/// Defines the type of input being stored in the raw memory layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    /// Plain text input
    Text,
    /// Voice/audio input
    Voice,
    /// Image input
    Image,
    /// Document input (PDF, Word, etc.)
    Document,
    /// Action/event input
    Action,
    /// External data import
    External,
}

impl From<String> for ContentType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text" => ContentType::Text,
            "voice" => ContentType::Voice,
            "image" => ContentType::Image,
            "document" => ContentType::Document,
            "action" => ContentType::Action,
            "external" => ContentType::External,
            _ => ContentType::Text, // Default fallback
        }
    }
}

impl From<ContentType> for String {
    fn from(ct: ContentType) -> Self {
        match ct {
            ContentType::Text => "text".to_string(),
            ContentType::Voice => "voice".to_string(),
            ContentType::Image => "image".to_string(),
            ContentType::Document => "document".to_string(),
            ContentType::Action => "action".to_string(),
            ContentType::External => "external".to_string(),
        }
    }
}

impl From<ContentType> for &'static str {
    fn from(ct: ContentType) -> Self {
        match ct {
            ContentType::Text => "text",
            ContentType::Voice => "voice",
            ContentType::Image => "image",
            ContentType::Document => "document",
            ContentType::Action => "action",
            ContentType::External => "external",
        }
    }
}

/// Raw memory representation - Layer 1 of the memory hierarchy
///
/// This is the append-only storage layer for all user inputs.
/// Each memory contains either plaintext content OR encrypted content, never both.
///
/// # Memory Safety Notes
/// - `encrypted` field uses `Vec<u8>` for BYTEA from PostgreSQL
/// - No circular references - uses plain ownership
/// - Optimized for 8GB memory environment
#[derive(Debug, Clone, Queryable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = raw_memories)]
#[diesel(primary_key(memory_id))]
pub struct RawMemory {
    /// Unique identifier for this memory
    pub memory_id: Uuid,
    /// User who owns this memory
    pub user_id: String,
    /// Precise timestamp when memory was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Type of content
    pub content_type: String,
    /// Plaintext content (NULL if encrypted)
    pub content: Option<String>,
    /// Encrypted content (BYTEA, NULL if plaintext)
    pub encrypted: Option<Vec<u8>>,
    /// Flexible metadata stored as JSONB
    pub metadata: Option<serde_json::Value>,
    /// Vector embedding for semantic search (768 dimensions)
    /// Stored as Vec<f32> representing the vector
    pub embedding: Option<Vec<f32>>,
}

impl RawMemory {
    /// Check if this memory is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encrypted.is_some()
    }

    /// Get the effective size in bytes
    ///
    /// Useful for memory tracking in 8GB environment
    pub fn size_bytes(&self) -> usize {
        self.content.as_ref().map(|s| s.len()).unwrap_or(0)
            + self.encrypted.as_ref().map(|v| v.len()).unwrap_or(0)
            + self.user_id.len()
            + self.content_type.len()
    }
}

/// New raw memory for insertion
///
/// Used when creating new memories. This struct doesn't have auto-generated
/// fields like `memory_id` and `created_at`.
///
/// Note: embedding field must be set separately via raw SQL due to
/// pgvector type requirements.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = raw_memories)]
pub struct NewRawMemory {
    pub user_id: String,
    pub content_type: String,
    pub content: Option<String>,
    pub encrypted: Option<Vec<u8>>,
    pub metadata: Option<serde_json::Value>,
}

impl NewRawMemory {
    /// Create a new plaintext memory
    ///
    /// # Arguments
    /// * `user_id` - Owner of the memory
    /// * `content_type` - Type of content
    /// * `content` - Plaintext content
    pub fn new_plaintext(
        user_id: String,
        content_type: ContentType,
        content: String,
    ) -> Self {
        Self {
            user_id,
            content_type: String::from(content_type),
            content: Some(content),
            encrypted: None,
            metadata: Some(serde_json::json!({})),
        }
    }

    /// Create a new encrypted memory
    ///
    /// # Arguments
    /// * `user_id` - Owner of the memory
    /// * `content_type` - Type of content
    /// * `encrypted` - Encrypted content bytes
    pub fn new_encrypted(
        user_id: String,
        content_type: ContentType,
        encrypted: Vec<u8>,
    ) -> Self {
        Self {
            user_id,
            content_type: String::from(content_type),
            content: None,
            encrypted: Some(encrypted),
            metadata: Some(serde_json::json!({})),
        }
    }

    /// Add metadata to the memory
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Memory update structure
///
/// For updating existing memories (metadata, etc.)
///
/// Note: embedding must be updated via raw SQL due to pgvector type requirements.
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = raw_memories)]
pub struct UpdateRawMemory {
    pub metadata: Option<Option<serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_conversion() {
        let ct = ContentType::Text;
        let s: String = ct.into();
        assert_eq!(s, "text");

        let ct2: ContentType = s.into();
        assert_eq!(ct2, ContentType::Text);
    }

    #[test]
    fn test_new_plaintext_memory() {
        let memory = NewRawMemory::new_plaintext(
            "user123".to_string(),
            ContentType::Text,
            "test content".to_string(),
        );

        assert_eq!(memory.user_id, "user123");
        assert_eq!(memory.content_type, "text");
        assert_eq!(memory.content, Some("test content".to_string()));
        assert!(memory.encrypted.is_none());
    }

    #[test]
    fn test_new_encrypted_memory() {
        let encrypted = vec![1u8, 2, 3, 4];
        let memory = NewRawMemory::new_encrypted(
            "user123".to_string(),
            ContentType::Voice,
            encrypted.clone(),
        );

        assert_eq!(memory.user_id, "user123");
        assert_eq!(memory.content_type, "voice");
        assert!(memory.content.is_none());
        assert_eq!(memory.encrypted, Some(encrypted));
    }

    #[test]
    fn test_with_metadata() {
        let memory = NewRawMemory::new_plaintext(
            "user123".to_string(),
            ContentType::Text,
            "test".to_string(),
        )
        .with_metadata(serde_json::json!({"source": "api"}));

        assert_eq!(
            memory.metadata,
            Some(serde_json::json!({"source": "api"}))
        );
    }

    #[test]
    fn test_raw_memory_size_bytes() {
        let memory = RawMemory {
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            created_at: chrono::Utc::now(),
            content_type: "text".to_string(),
            content: Some("hello world".to_string()),
            encrypted: None,
            metadata: Some(serde_json::json!({})),
            embedding: None,
        };

        let size = memory.size_bytes();
        assert!(size > 0);
        assert!(size < 1000); // Should be small for this simple case
    }

    #[test]
    fn test_raw_memory_is_encrypted() {
        let plaintext = RawMemory {
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            created_at: chrono::Utc::now(),
            content_type: "text".to_string(),
            content: Some("hello".to_string()),
            encrypted: None,
            metadata: None,
            embedding: None,
        };

        assert!(!plaintext.is_encrypted());

        let encrypted = RawMemory {
            encrypted: Some(vec![1, 2, 3]),
            content: None,
            ..plaintext
        };

        assert!(encrypted.is_encrypted());
    }
}

/// Event memory representation - Layer 2 of the memory hierarchy
///
/// Structured events extracted from raw memories using AI or rule-based extraction.
/// Each event represents a structured understanding of what happened.
///
/// # Design Principles (from HEAD.md)
/// - 每个事件必须有精确时间戳
/// - 数量必须结构化存储
/// - 行为必须类型化 (action field)
/// - 支持时间范围查询 (indexed by user_id, timestamp)
///
/// # Memory Safety Notes
/// - Uses `Option<f64>` for quantity since not all events have quantities
/// - Actor is optional since many events don't specify who performed the action
/// - Confidence is required (0.0 to 1.0) for promotion gate decisions
#[derive(Debug, Clone, Queryable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = event_memories)]
#[diesel(primary_key(event_id))]
pub struct EventMemory {
    /// Unique identifier for this event
    pub event_id: Uuid,
    /// Reference to source raw memory (CASCADE DELETE)
    pub memory_id: Uuid,
    /// User who owns this event (denormalized for query performance)
    pub user_id: String,
    /// Precise timestamp when event occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Entity that performed the action (optional)
    pub actor: Option<String>,
    /// Action performed (e.g., "eat", "buy", "travel")
    pub action: String,
    /// Object/entity affected by the action
    pub target: String,
    /// Structured quantity (HEAD.md: 数量必须结构化存储)
    pub quantity: Option<f64>,
    /// Unit of measurement (e.g., "个", "kg", "次")
    pub unit: Option<String>,
    /// Extraction confidence (0-1), used in promotion gate
    pub confidence: f64,
    /// Version of extractor that created this event
    pub extractor_version: Option<String>,
}

impl EventMemory {
    /// Check if this event has a quantity
    pub fn has_quantity(&self) -> bool {
        self.quantity.is_some()
    }

    /// Check if this event is high-confidence
    ///
    /// Used in promotion gate to determine if view should be promoted.
    pub fn is_high_confidence(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }

    /// Get a human-readable description of the event
    pub fn description(&self) -> String {
        let mut desc = String::new();

        if let Some(ref actor) = self.actor {
            desc.push_str(actor);
            desc.push_str(" ");
        }

        desc.push_str(&self.action);
        desc.push_str(" ");

        if let Some(qty) = self.quantity {
            desc.push_str(&qty.to_string());
            desc.push_str(" ");

            if let Some(ref unit) = self.unit {
                desc.push_str(unit);
                desc.push_str(" ");
            }
        }

        desc.push_str(&self.target);

        desc
    }

    /// Validate event constraints
    ///
    /// Ensures confidence is in [0, 1] range and quantity/unit consistency.
    pub fn validate(&self) -> Result<(), String> {
        if self.confidence < 0.0 || self.confidence > 1.0 {
            return Err(format!(
                "Confidence must be between 0 and 1, got {}",
                self.confidence
            ));
        }

        // Check quantity/unit consistency
        match (&self.quantity, &self.unit) {
            (Some(_), None) | (None, Some(_)) => {
                return Err(
                    "Quantity and unit must both be present or both be None".to_string()
                );
            }
            _ => {}
        }

        Ok(())
    }
}

/// New event memory for insertion
///
/// Used when creating new events from extracted information.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = event_memories)]
pub struct NewEventMemory {
    pub memory_id: Uuid,
    pub user_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub actor: Option<String>,
    pub action: String,
    pub target: String,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub confidence: f64,
    pub extractor_version: Option<String>,
}

impl NewEventMemory {
    /// Create a new event with default confidence
    ///
    /// # Arguments
    /// * `memory_id` - Source raw memory ID
    /// * `user_id` - Owner of the event
    /// * `timestamp` - When the event occurred
    /// * `action` - Action performed
    /// * `target` - Object/entity affected
    pub fn new(
        memory_id: Uuid,
        user_id: String,
        timestamp: chrono::DateTime<chrono::Utc>,
        action: String,
        target: String,
    ) -> Self {
        Self {
            memory_id,
            user_id,
            timestamp,
            actor: None,
            action,
            target,
            quantity: None,
            unit: None,
            confidence: 0.5, // Default confidence
            extractor_version: Some("0.1.0".to_string()),
        }
    }

    /// Set the actor for the event
    pub fn with_actor(mut self, actor: String) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Set the quantity and unit for the event
    ///
    /// Both must be set together due to database constraint.
    pub fn with_quantity(mut self, quantity: f64, unit: String) -> Self {
        self.quantity = Some(quantity);
        self.unit = Some(unit);
        self
    }

    /// Set the confidence level for the event
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set the extractor version
    pub fn with_extractor_version(mut self, version: String) -> Self {
        self.extractor_version = Some(version);
        self
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;

    #[test]
    fn test_new_event_memory() {
        let memory_id = Uuid::new_v4();
        let event = NewEventMemory::new(
            memory_id,
            "user123".to_string(),
            chrono::Utc::now(),
            "eat".to_string(),
            "apple".to_string(),
        );

        assert_eq!(event.memory_id, memory_id);
        assert_eq!(event.user_id, "user123");
        assert_eq!(event.action, "eat");
        assert_eq!(event.target, "apple");
        assert_eq!(event.confidence, 0.5);
        assert!(event.actor.is_none());
        assert!(event.quantity.is_none());
    }

    #[test]
    fn test_event_with_actor() {
        let memory_id = Uuid::new_v4();
        let event = NewEventMemory::new(
            memory_id,
            "user123".to_string(),
            chrono::Utc::now(),
            "eat".to_string(),
            "apple".to_string(),
        )
        .with_actor("John".to_string());

        assert_eq!(event.actor, Some("John".to_string()));
    }

    #[test]
    fn test_event_with_quantity() {
        let memory_id = Uuid::new_v4();
        let event = NewEventMemory::new(
            memory_id,
            "user123".to_string(),
            chrono::Utc::now(),
            "eat".to_string(),
            "apple".to_string(),
        )
        .with_quantity(3.0_f64, "个".to_string());

        assert_eq!(event.quantity, Some(3.0));
        assert_eq!(event.unit, Some("个".to_string()));
    }

    #[test]
    fn test_event_with_confidence() {
        let memory_id = Uuid::new_v4();
        let event = NewEventMemory::new(
            memory_id,
            "user123".to_string(),
            chrono::Utc::now(),
            "eat".to_string(),
            "apple".to_string(),
        )
        .with_confidence(0.85);

        assert_eq!(event.confidence, 0.85);
    }

    #[test]
    fn test_event_description() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: Some("John".to_string()),
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: Some(3.0),
            unit: Some("个".to_string()),
            confidence: 0.9,
            extractor_version: Some("0.1.0".to_string()),
        };

        let desc = event.description();
        assert_eq!(desc, "John eat 3 个 apple");
    }

    #[test]
    fn test_event_description_no_actor() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: Some(3.0),
            unit: Some("个".to_string()),
            confidence: 0.9,
            extractor_version: Some("0.1.0".to_string()),
        };

        let desc = event.description();
        assert_eq!(desc, "eat 3 个 apple");
    }

    #[test]
    fn test_event_description_no_quantity() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "sleep".to_string(),
            target: "bed".to_string(),
            quantity: None,
            unit: None,
            confidence: 0.7,
            extractor_version: Some("0.1.0".to_string()),
        };

        let desc = event.description();
        assert_eq!(desc, "sleep bed");
    }

    #[test]
    fn test_validate_good_confidence() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 0.8,
            extractor_version: Some("0.1.0".to_string()),
        };

        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_validate_bad_confidence_too_high() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 1.5,
            extractor_version: Some("0.1.0".to_string()),
        };

        assert!(event.validate().is_err());
    }

    #[test]
    fn test_validate_bad_confidence_negative() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: -0.1,
            extractor_version: Some("0.1.0".to_string()),
        };

        assert!(event.validate().is_err());
    }

    #[test]
    fn test_validate_quantity_without_unit() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: Some(3.0),
            unit: None,
            confidence: 0.8,
            extractor_version: Some("0.1.0".to_string()),
        };

        assert!(event.validate().is_err());
    }

    #[test]
    fn test_is_high_confidence() {
        let event = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 0.85,
            extractor_version: Some("0.1.0".to_string()),
        };

        assert!(event.is_high_confidence(0.8));
        assert!(!event.is_high_confidence(0.9));
    }

    #[test]
    fn test_has_quantity() {
        let event_with_qty = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            timestamp: chrono::Utc::now(),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: Some(3.0),
            unit: Some("个".to_string()),
            confidence: 0.9,
            extractor_version: Some("0.1.0".to_string()),
        };

        let event_no_qty = EventMemory {
            quantity: None,
            unit: None,
            ..event_with_qty.clone()
        };

        assert!(event_with_qty.has_quantity());
        assert!(!event_no_qty.has_quantity());
    }
}

/// Entity type enumeration
///
/// Defines the type of entity extracted from events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    /// Person (人名、角色)
    Person,
    /// Place (地点、位置)
    Place,
    /// Object (物体、产品)
    Object,
    /// Concept (概念、想法)
    Concept,
    /// Organization (公司、机构)
    Organization,
    /// Event (事件名称)
    Event,
}

impl From<String> for EntityType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "person" => EntityType::Person,
            "place" => EntityType::Place,
            "object" => EntityType::Object,
            "concept" => EntityType::Concept,
            "organization" => EntityType::Organization,
            "event" => EntityType::Event,
            _ => EntityType::Object, // Default fallback
        }
    }
}

impl From<EntityType> for String {
    fn from(et: EntityType) -> Self {
        match et {
            EntityType::Person => "person".to_string(),
            EntityType::Place => "place".to_string(),
            EntityType::Object => "object".to_string(),
            EntityType::Concept => "concept".to_string(),
            EntityType::Organization => "organization".to_string(),
            EntityType::Event => "event".to_string(),
        }
    }
}

impl From<EntityType> for &'static str {
    fn from(et: EntityType) -> Self {
        match et {
            EntityType::Person => "person",
            EntityType::Place => "place",
            EntityType::Object => "object",
            EntityType::Concept => "concept",
            EntityType::Organization => "organization",
            EntityType::Event => "event",
        }
    }
}

/// Entity representation - Layer 2 of Structured Memory
///
/// Represents an entity (person, place, object, concept) extracted from events.
/// Each entity has a canonical name, type, and dynamic attributes.
///
/// # Memory Safety Notes
/// - Uses JSONB for attributes (flexible schema)
/// - Occurrence count tracks entity importance
#[derive(Debug, Clone, Queryable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = entities)]
#[diesel(primary_key(entity_id))]
pub struct Entity {
    /// Unique identifier for this entity
    pub entity_id: Uuid,
    /// User who owns this entity
    pub user_id: String,
    /// Standard/canonical name (e.g., "Apple" for "苹果")
    pub canonical_name: String,
    /// Entity type (person/place/object/concept/organization)
    pub entity_type: String,
    /// Dynamic attributes stored as JSONB
    pub attributes: Option<serde_json::Value>,
    /// First time this entity appeared
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// Most recent time this entity appeared
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Number of times entity has been seen
    pub occurrence_count: i32,
    /// Confidence in entity classification (0-1)
    pub confidence: f64,
}

impl Entity {
    /// Check if this is a high-confidence entity
    pub fn is_high_confidence(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }

    /// Get the size of this entity in bytes (approximate)
    pub fn size_bytes(&self) -> usize {
        self.canonical_name.len()
            + self.entity_type.len()
            + self.user_id.len()
            + self.attributes.as_ref().map(|a| a.to_string().len()).unwrap_or(0)
    }
}

/// New entity for insertion
///
/// Used when creating new entities from event extraction.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = entities)]
pub struct NewEntity {
    pub user_id: String,
    pub canonical_name: String,
    pub entity_type: String,
    pub attributes: Option<serde_json::Value>,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub occurrence_count: i32,
    pub confidence: f64,
}

impl NewEntity {
    /// Create a new entity
    ///
    /// # Arguments
    /// * `user_id` - Owner of the entity
    /// * `canonical_name` - Standard name
    /// * `entity_type` - Type of entity
    pub fn new(
        user_id: String,
        canonical_name: String,
        entity_type: EntityType,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id,
            canonical_name,
            entity_type: String::from(entity_type),
            attributes: Some(serde_json::json!({})),
            first_seen: now,
            last_seen: now,
            occurrence_count: 1,
            confidence: 0.5,
        }
    }

    /// Set attributes for the entity
    pub fn with_attributes(mut self, attributes: serde_json::Value) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Set confidence level
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }
}

/// Entity relation representation
///
/// Represents a relationship between two entities.
/// Simulates graph structure using PostgreSQL foreign keys.
#[derive(Debug, Clone, Queryable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = entity_relations)]
#[diesel(primary_key(relation_id))]
pub struct EntityRelation {
    /// Unique identifier for this relation
    pub relation_id: Uuid,
    /// User who owns this relation
    pub user_id: String,
    /// Source entity (subject of the relation)
    pub source_entity_id: Uuid,
    /// Target entity (object of the relation)
    pub target_entity_id: Uuid,
    /// Type of relationship (belongs_to, related_to, etc.)
    pub relation_type: String,
    /// Confidence in this relationship (0-1)
    pub confidence: f64,
    /// First time this relationship was observed
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// Most recent time this relationship was observed
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Strength of relationship (based on co-occurrence frequency)
    pub strength: f64,
}

/// New entity relation for insertion
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = entity_relations)]
pub struct NewEntityRelation {
    pub user_id: String,
    pub source_entity_id: Uuid,
    pub target_entity_id: Uuid,
    pub relation_type: String,
    pub confidence: f64,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub strength: f64,
}

impl NewEntityRelation {
    /// Create a new entity relation
    ///
    /// # Arguments
    /// * `user_id` - Owner of the relation
    /// * `source_entity_id` - Source entity ID
    /// * `target_entity_id` - Target entity ID
    /// * `relation_type` - Type of relationship
    pub fn new(
        user_id: String,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id,
            source_entity_id,
            target_entity_id,
            relation_type,
            confidence: 0.5,
            first_seen: now,
            last_seen: now,
            strength: 1.0,
        }
    }

    /// Set confidence level
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set relationship strength
    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }
}

#[cfg(test)]
mod entity_tests {
    use super::*;

    #[test]
    fn test_entity_type_conversion() {
        let et = EntityType::Object;
        let s: String = et.into();
        assert_eq!(s, "object");

        let et2: EntityType = s.into();
        assert_eq!(et2, EntityType::Object);
    }

    #[test]
    fn test_new_entity() {
        let entity = NewEntity::new(
            "user123".to_string(),
            "Apple".to_string(),
            EntityType::Object,
        );

        assert_eq!(entity.user_id, "user123");
        assert_eq!(entity.canonical_name, "Apple");
        assert_eq!(entity.entity_type, "object");
        assert_eq!(entity.occurrence_count, 1);
    }

    #[test]
    fn test_new_entity_with_attributes() {
        let attrs = serde_json::json!({"color": "red", "category": "fruit"});
        let entity = NewEntity::new(
            "user123".to_string(),
            "Apple".to_string(),
            EntityType::Object,
        )
        .with_attributes(attrs)
        .with_confidence(0.8);

        assert_eq!(entity.confidence, 0.8);
        assert_eq!(entity.attributes, Some(serde_json::json!({"color": "red", "category": "fruit"})));
    }

    #[test]
    fn test_entity_is_high_confidence() {
        let entity = Entity {
            entity_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            canonical_name: "Apple".to_string(),
            entity_type: "object".to_string(),
            attributes: None,
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            occurrence_count: 5,
            confidence: 0.8,
        };

        assert!(entity.is_high_confidence(0.7));
        assert!(!entity.is_high_confidence(0.9));
    }

    #[test]
    fn test_new_entity_relation() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        let relation = NewEntityRelation::new(
            "user123".to_string(),
            source_id,
            target_id,
            "belongs_to".to_string(),
        );

        assert_eq!(relation.user_id, "user123");
        assert_eq!(relation.relation_type, "belongs_to");
        assert_eq!(relation.strength, 1.0);
    }

    #[test]
    fn test_new_entity_relation_with_strength() {
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        let relation = NewEntityRelation::new(
            "user123".to_string(),
            source_id,
            target_id,
            "belongs_to".to_string(),
        )
        .with_confidence(0.9)
        .with_strength(0.8);

        assert_eq!(relation.confidence, 0.9);
        assert_eq!(relation.strength, 0.8);
    }
}
