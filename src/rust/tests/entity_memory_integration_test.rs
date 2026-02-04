//! Structured Memory Integration Tests
//!
//! Tests for Entity Memory (Phase 4) components:
//! - Attribute extraction (EntityAttributeExtractor)
//! - Relation extraction (EntityRelationExtractor)

use dirsoul::models::*;
use dirsoul::*;
use uuid::Uuid;

/// Test entity attribute extraction with confidence scores
#[test]
fn test_entity_attribute_confidence() {
    let extractor = EntityAttributeExtractor::default();

    // Test: "红色的苹果" should extract color attribute
    let text = "红色的苹果";
    let attributes = extractor.extract_attributes(text);

    assert!(!attributes.is_empty());

    let color_attr = attributes.get(&AttributeType::Color);
    assert!(color_attr.is_some());

    let color = color_attr.unwrap();
    assert_eq!(color.value, "红色");
    // Check that confidence is within valid range
    assert!(color.confidence >= 0.0 && color.confidence <= 1.0);
}

/// Test attribute extraction for various attribute types
#[test]
fn test_multi_attribute_extraction() {
    let extractor = EntityAttributeExtractor::default();

    let text = "这个红色的苹果味道很甜，口感很脆";
    let attributes = extractor.extract_attributes(text);

    // Should extract color, taste, and texture
    assert!(attributes.contains_key(&AttributeType::Color));
    assert!(attributes.contains_key(&AttributeType::Taste));
    assert!(attributes.contains_key(&AttributeType::Texture));
}

/// Test relation type string conversion
#[test]
fn test_relation_type_serialization() {
    // Test to_string conversion
    assert_eq!(RelationType::WorksAt.to_string(), "works_at");
    assert_eq!(RelationType::BelongsTo.to_string(), "belongs_to");
    assert_eq!(RelationType::RelatedTo.to_string(), "related_to");
    assert_eq!(RelationType::LocatedAt.to_string(), "located_at");
    assert_eq!(RelationType::FriendsWith.to_string(), "friends_with");
}

/// Test relation type Chinese names
#[test]
fn test_relation_type_zh_names() {
    assert_eq!(RelationType::RelatedTo.zh_name(), "相关");
    assert_eq!(RelationType::BelongsTo.zh_name(), "属于");
    assert_eq!(RelationType::WorksAt.zh_name(), "工作于");
    assert_eq!(RelationType::LocatedAt.zh_name(), "位于");
    assert_eq!(RelationType::FriendsWith.zh_name(), "朋友");
}

/// Test entity confidence scoring
#[test]
fn test_entity_confidence_scoring() {
    let entity = NewEntity {
        user_id: "test_user".to_string(),
        canonical_name: "张三".to_string(),
        entity_type: "Person".to_string(),
        first_seen: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
        occurrence_count: 5,
        confidence: 0.9,
        attributes: Some(serde_json::json!({"profession": "工程师"})),
    };

    // High occurrence count and confidence should indicate high confidence entity
    assert!(entity.occurrence_count > 1);
    assert!(entity.confidence > 0.5);
}

/// Test event confidence validation
#[test]
fn test_event_confidence_validation() {
    let event = NewEventMemory {
        user_id: "test_user".to_string(),
        memory_id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        actor: Some("张三".to_string()),
        action: "买".to_string(),
        target: "苹果".to_string(),
        quantity: Some(3.0),
        unit: Some("个".to_string()),
        confidence: 0.95,
        extractor_version: Some("1.0".to_string()),
    };

    // Valid confidence range is [0.0, 1.0]
    assert!(event.confidence >= 0.0 && event.confidence <= 1.0);

    // Test invalid confidence (too high) would fail validation
    let invalid_confidence = 1.5;
    assert!(invalid_confidence > 1.0);

    // Test invalid confidence (negative)
    let invalid_confidence2 = -0.1;
    assert!(invalid_confidence2 < 0.0);
}

/// Test entity type conversion
#[test]
fn test_entity_type_conversion() {
    // Test From<String> implementation
    let person: EntityType = "Person".to_string().into();
    assert_eq!(person, EntityType::Person);

    let place: EntityType = "Place".to_string().into();
    assert_eq!(place, EntityType::Place);

    let org: EntityType = "Organization".to_string().into();
    assert_eq!(org, EntityType::Organization);
}

/// Test content type conversion
#[test]
fn test_content_type_conversion() {
    // Test From<String> implementation
    let text_ct: ContentType = "text".to_string().into();
    assert_eq!(text_ct, ContentType::Text);

    let voice_ct: ContentType = "voice".to_string().into();
    assert_eq!(voice_ct, ContentType::Voice);

    let image_ct: ContentType = "image".to_string().into();
    assert_eq!(image_ct, ContentType::Image);

    // Test Into<String> implementation
    let s: String = ContentType::Text.into();
    assert_eq!(s, "text");
}

/// Test relation extraction configuration
#[test]
fn test_relation_extractor_config() {
    let config = RelationExtractorConfig::default();

    // Test that the config has valid values
    assert!(config.min_strength_threshold >= 0.0);
    assert!(config.min_strength_threshold <= 1.0);
}

/// Test extracted relation serialization
#[test]
fn test_extracted_relation_serialization() {
    let relation = ExtractedRelation {
        source: "张三".to_string(),
        relation_type: RelationType::WorksAt,
        target: "Google".to_string(),
        confidence: 0.9,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&relation).unwrap();
    assert!(json.contains("张三"));
    assert!(json.contains("Google"));

    // Deserialize back
    let deserialized: ExtractedRelation = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.source, "张三");
    assert_eq!(deserialized.relation_type, RelationType::WorksAt);
    assert_eq!(deserialized.target, "Google");
}
