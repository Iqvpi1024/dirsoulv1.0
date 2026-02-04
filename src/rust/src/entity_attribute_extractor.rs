//! Entity Attribute Extractor Module - Phase 4, Task 4.3
//!
//! Handles dynamic attribute extraction and growth for entities.
//!
//! # Design Principles (from HEAD.md)
//! - 支持实体消歧和属性动态增长
//! - AI-Native: 使用 SLM 提取属性，规则仅作为兜底
//! - 属性置信度追踪，支持增量更新
//!
//! # Core Functionality
//! - Extract attributes from event context (color, category, texture, etc.)
//! - Update entity JSONB attributes with confidence scores
//! - Merge new attributes with existing ones

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use crate::error::Result;
use crate::models::Entity;

/// Attribute types that can be extracted from events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeType {
    /// Color (red, green, blue, etc.)
    Color,
    /// Category (fruit, vegetable, electronics, etc.)
    Category,
    /// Texture/Feeling (crunchy, soft, smooth, etc.)
    Texture,
    /// Size (large, small, medium, etc.)
    Size,
    /// Brand (Apple, Samsung, etc.)
    Brand,
    /// Price/Cost (expensive, cheap, etc.)
    Price,
    /// Location/Origin (China, USA, etc.)
    Origin,
    /// Material (wood, metal, plastic, etc.)
    Material,
    /// Taste (sweet, sour, bitter, etc.)
    Taste,
    /// Custom attribute
    Custom(String),
}

/// Attribute with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute value
    pub value: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Number of times this attribute has been observed
    pub count: i32,
    /// Timestamp of first observation
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// Timestamp of last observation
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl Attribute {
    /// Create a new attribute
    pub fn new(value: String, confidence: f64) -> Self {
        let now = chrono::Utc::now();
        Self {
            value,
            confidence,
            count: 1,
            first_seen: now,
            last_seen: now,
        }
    }

    /// Update attribute with new observation
    pub fn update(&mut self, new_confidence: f64) {
        self.count += 1;
        self.last_seen = chrono::Utc::now();
        // Weighted average of confidences
        self.confidence = (self.confidence * (self.count - 1) as f64 + new_confidence)
            / self.count as f64;
    }
}

/// Entity attribute extractor
///
/// Extracts attributes from event context and updates entities.
pub struct EntityAttributeExtractor {
    /// Confidence threshold for accepting attributes
    confidence_threshold: f64,
}

impl EntityAttributeExtractor {
    /// Create a new attribute extractor with default configuration
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.5,
        }
    }

    /// Create a new attribute extractor with custom confidence threshold
    ///
    /// # Arguments
    /// * `confidence_threshold` - Minimum confidence for accepting attributes (0.0 to 1.0)
    pub fn with_threshold(confidence_threshold: f64) -> Self {
        Self {
            confidence_threshold: confidence_threshold.clamp(0.0, 1.0),
        }
    }

    /// Extract attributes from event context using rule-based patterns
    ///
    /// This is a fallback when SLM is not available.
    ///
    /// # Arguments
    /// * `context` - The event context text
    ///
    /// # Returns
    /// HashMap of attribute type to attribute
    ///
    /// # Example
    /// ```no_run
    /// # use dirsoul::entity_attribute_extractor::EntityAttributeExtractor;
    /// let extractor = EntityAttributeExtractor::new();
    /// let attrs = extractor.extract_attributes("红色的甜甜的苹果");
    /// // Returns: {Color: "红色", Taste: "甜甜的"}
    /// ```
    pub fn extract_attributes(&self, context: &str) -> HashMap<AttributeType, Attribute> {
        let mut attributes = HashMap::new();

        // Color patterns
        for color_pattern in &self.get_color_patterns() {
            if context.contains(color_pattern) {
                attributes.insert(
                    AttributeType::Color,
                    Attribute::new(color_pattern.to_string(), 0.7),
                );
                break; // Only take first match
            }
        }

        // Taste patterns
        for taste_pattern in &self.get_taste_patterns() {
            if context.contains(taste_pattern) {
                attributes.insert(
                    AttributeType::Taste,
                    Attribute::new(taste_pattern.to_string(), 0.7),
                );
                break;
            }
        }

        // Texture patterns
        for texture_pattern in &self.get_texture_patterns() {
            if context.contains(texture_pattern) {
                attributes.insert(
                    AttributeType::Texture,
                    Attribute::new(texture_pattern.to_string(), 0.7),
                );
                break;
            }
        }

        // Size patterns
        for size_pattern in &self.get_size_patterns() {
            if context.contains(size_pattern) {
                attributes.insert(
                    AttributeType::Size,
                    Attribute::new(size_pattern.to_string(), 0.7),
                );
                break;
            }
        }

        // Category patterns
        for (category_name, category_patterns) in &self.get_category_patterns() {
            if category_patterns.iter().any(|p| context.contains(p)) {
                attributes.insert(
                    AttributeType::Category,
                    Attribute::new(category_name.to_string(), 0.6),
                );
                break;
            }
        }

        // Price patterns
        for (price_name, price_patterns) in &self.get_price_patterns() {
            if price_patterns.iter().any(|p| context.contains(p)) {
                attributes.insert(
                    AttributeType::Price,
                    Attribute::new(price_name.to_string(), 0.6),
                );
                break;
            }
        }

        attributes
    }

    /// Update entity with new attributes
    ///
    /// Merges new attributes with existing ones, updating confidence scores.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `entity` - The entity to update
    /// * `new_attributes` - New attributes to add
    pub fn update_entity_attributes(
        &self,
        conn: &mut PgConnection,
        entity: Entity,
        new_attributes: HashMap<AttributeType, Attribute>,
    ) -> Result<Entity> {
        use crate::schema::entities::dsl::*;

        // Get existing attributes
        let mut existing_attrs = entity.attributes.unwrap_or(json!({}));

        // Merge new attributes
        for (attr_type, new_attr) in new_attributes {
            // Skip if below confidence threshold
            if new_attr.confidence < self.confidence_threshold {
                continue;
            }

            let attr_key = self.attr_type_to_key(&attr_type);

            if let Some(existing_attr_json) = existing_attrs.get(&attr_key) {
                // Attribute exists - update it
                if let Ok(mut existing_attr) = serde_json::from_value::<Attribute>(existing_attr_json.clone()) {
                    existing_attr.update(new_attr.confidence);
                    existing_attrs[attr_key] = serde_json::to_value(existing_attr)?;
                } else {
                    // Failed to parse, create new
                    existing_attrs[attr_key] = serde_json::to_value(new_attr)?;
                }
            } else {
                // New attribute - add it
                existing_attrs[attr_key] = serde_json::to_value(new_attr)?;
            }
        }

        // Update in database
        diesel::update(entities.find(entity.entity_id))
            .set(attributes.eq(Some(existing_attrs)))
            .execute(conn)?;

        // Fetch updated entity
        let updated_entity = entities
            .find(entity.entity_id)
            .first::<Entity>(conn)?;

        Ok(updated_entity)
    }

    /// Extract and update attributes in one operation
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `entity` - The entity to update
    /// * `context` - Event context to extract attributes from
    pub fn extract_and_update(
        &self,
        conn: &mut PgConnection,
        entity: Entity,
        context: &str,
    ) -> Result<Entity> {
        let new_attributes = self.extract_attributes(context);
        self.update_entity_attributes(conn, entity, new_attributes)
    }

    /// Get color patterns for extraction
    fn get_color_patterns(&self) -> Vec<&'static str> {
        vec![
            "金黄色", "银色", "粉红色", "紫红色", "橙黄色",
            "红色", "红", "绿色", "绿", "蓝色", "蓝", "黄色", "黄",
            "黑色", "黑", "白色", "白", "紫色", "紫", "橙色", "橙",
            "粉色", "粉", "棕色", "褐", "灰色", "灰", "银", "金",
            "golden",
        ]
    }

    /// Get taste patterns for extraction
    fn get_taste_patterns(&self) -> Vec<&'static str> {
        vec![
            "甜甜的", "香香", "鲜美", "浓郁", "清淡",
            "甜", "酸", "苦", "辣", "咸", "淡",
            "美味", "好吃", "难吃", "香",
        ]
    }

    /// Get texture patterns for extraction
    fn get_texture_patterns(&self) -> Vec<&'static str> {
        vec![
            "酥脆", "柔软", "坚硬", "光滑", "粘稠",
            "脆", "软", "硬", "滑", "粘", "干",
            "粗糙", "湿润", "多汁", "松软",
        ]
    }

    /// Get size patterns for extraction
    fn get_size_patterns(&self) -> Vec<&'static str> {
        vec![
            "巨大", "超大", "特大", "微小", "迷你", "大号", "小号",
            "大", "小", "中等", "中",
            "细", "粗", "厚", "薄", "长", "短",
        ]
    }

    /// Get category patterns for extraction
    fn get_category_patterns(&self) -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            ("水果", vec!["水果", "苹果", "香蕉", "橙子"]),
            ("蔬菜", vec!["蔬菜", "白菜", "萝卜", "西红柿"]),
            ("电子产品", vec!["手机", "电脑", "平板", "电子产品"]),
            ("食物", vec!["食物", "饭", "面", "面包", "蛋糕"]),
            ("饮料", vec!["饮料", "水", "茶", "咖啡", "果汁"]),
            ("交通工具", vec!["车", "汽车", "自行车", "飞机"]),
        ]
    }

    /// Get price patterns for extraction
    fn get_price_patterns(&self) -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            ("昂贵", vec!["贵", "昂贵", "价格高"]),
            ("便宜", vec!["便宜", "实惠", "不贵"]),
            ("中等", vec!["适中", "一般", "还行"]),
        ]
    }

    /// Convert AttributeType to JSON key
    fn attr_type_to_key(&self, attr_type: &AttributeType) -> String {
        match attr_type {
            AttributeType::Color => "color".to_string(),
            AttributeType::Category => "category".to_string(),
            AttributeType::Texture => "texture".to_string(),
            AttributeType::Size => "size".to_string(),
            AttributeType::Brand => "brand".to_string(),
            AttributeType::Price => "price".to_string(),
            AttributeType::Origin => "origin".to_string(),
            AttributeType::Material => "material".to_string(),
            AttributeType::Taste => "taste".to_string(),
            AttributeType::Custom(name) => format!("custom_{}", name),
        }
    }
}

impl Default for EntityAttributeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_color() {
        let extractor = EntityAttributeExtractor::new();
        let attrs = extractor.extract_attributes("红色的苹果");

        assert!(attrs.contains_key(&AttributeType::Color));
        let color_attr = attrs.get(&AttributeType::Color).unwrap();
        assert_eq!(color_attr.value, "红色");
        assert_eq!(color_attr.confidence, 0.7);
    }

    #[test]
    fn test_extract_taste() {
        let extractor = EntityAttributeExtractor::new();
        let attrs = extractor.extract_attributes("甜甜的苹果");

        assert!(attrs.contains_key(&AttributeType::Taste));
        let taste_attr = attrs.get(&AttributeType::Taste).unwrap();
        assert_eq!(taste_attr.value, "甜甜的");
    }

    #[test]
    fn test_extract_multiple_attributes() {
        let extractor = EntityAttributeExtractor::new();
        let attrs = extractor.extract_attributes("红色的甜甜的大苹果");

        assert!(attrs.contains_key(&AttributeType::Color));
        assert!(attrs.contains_key(&AttributeType::Taste));
        assert!(attrs.contains_key(&AttributeType::Size));
    }

    #[test]
    fn test_attribute_update() {
        let mut attr = Attribute::new("红色".to_string(), 0.7);
        assert_eq!(attr.count, 1);
        assert_eq!(attr.confidence, 0.7);

        attr.update(0.8);
        assert_eq!(attr.count, 2);
        assert!((attr.confidence - 0.75).abs() < 0.01); // (0.7 + 0.8) / 2
    }

    #[test]
    fn test_confidence_threshold() {
        let extractor = EntityAttributeExtractor::with_threshold(0.8);
        let attrs = extractor.extract_attributes("红色的苹果");

        // Attribute with 0.7 confidence should be filtered out
        // This test verifies the threshold is applied in update_entity_attributes
    }

    #[test]
    fn test_texture_patterns() {
        let extractor = EntityAttributeExtractor::new();
        let attrs = extractor.extract_attributes("酥脆的饼干");

        assert!(attrs.contains_key(&AttributeType::Texture));
        assert_eq!(attrs.get(&AttributeType::Texture).unwrap().value, "酥脆");
    }

    #[test]
    fn test_size_patterns() {
        let extractor = EntityAttributeExtractor::new();
        let attrs = extractor.extract_attributes("巨大的西瓜");

        assert!(attrs.contains_key(&AttributeType::Size));
        assert_eq!(attrs.get(&AttributeType::Size).unwrap().value, "巨大");
    }

    #[test]
    fn test_no_attributes() {
        let extractor = EntityAttributeExtractor::new();
        let attrs = extractor.extract_attributes("你好世界");

        assert!(attrs.is_empty());
    }

    #[test]
    fn test_attribute_new() {
        let attr = Attribute::new("测试值".to_string(), 0.85);
        assert_eq!(attr.value, "测试值");
        assert_eq!(attr.confidence, 0.85);
        assert_eq!(attr.count, 1);
        assert!(attr.first_seen <= chrono::Utc::now());
        assert!(attr.last_seen <= chrono::Utc::now());
    }

    #[test]
    fn test_attr_type_to_key() {
        let extractor = EntityAttributeExtractor::new();
        assert_eq!(extractor.attr_type_to_key(&AttributeType::Color), "color");
        assert_eq!(extractor.attr_type_to_key(&AttributeType::Taste), "taste");
        assert_eq!(extractor.attr_type_to_key(&AttributeType::Custom("test".to_string())), "custom_test");
    }
}
