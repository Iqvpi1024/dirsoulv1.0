//! Cognitive Memory Layer - Derived Views + Promotion Gate
//!
//! This module implements the core innovation from HEAD.md:
//! - **Derived Cognitive Views**: Temporary hypotheses about user patterns
//! - **Promotion Gate**: Programmatic validation before becoming stable concepts
//!
//! # Design Principles (HEAD.md)
//! - **慢抽象原则**: Derived Views 先行，可丢弃
//! - **Promotion Gate 把关**: 程序判定是否晋升为稳定概念
//! - **避免 LLM 幻觉放大**: 隔离 AI 判断与系统结构

use crate::schema::{cognitive_views, stable_concepts};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// View status enum - represents the lifecycle of a cognitive view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewStatus {
    /// Active - being tested
    Active,
    /// Expired - discarded after validation period
    Expired,
    /// Promoted - graduated to stable concept
    Promoted,
    /// Rejected - deemed invalid
    Rejected,
}

impl ViewStatus {
    /// Check if the view is still active
    pub fn is_active(&self) -> bool {
        matches!(self, ViewStatus::Active)
    }

    /// Check if the view can be promoted
    pub fn can_be_promoted(&self) -> bool {
        matches!(self, ViewStatus::Active)
    }
}

impl From<String> for ViewStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "active" => ViewStatus::Active,
            "expired" => ViewStatus::Expired,
            "promoted" => ViewStatus::Promoted,
            "rejected" => ViewStatus::Rejected,
            _ => ViewStatus::Active, // Default fallback
        }
    }
}

impl From<ViewStatus> for String {
    fn from(status: ViewStatus) -> Self {
        match status {
            ViewStatus::Active => "active".to_string(),
            ViewStatus::Expired => "expired".to_string(),
            ViewStatus::Promoted => "promoted".to_string(),
            ViewStatus::Rejected => "rejected".to_string(),
        }
    }
}

impl From<&str> for ViewStatus {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

/// Cognitive View - a temporary hypothesis about user behavior
///
/// # Example (HEAD.md)
/// ```text
/// struct DerivedView {
///     hypothesis: String,        // "用户喜欢吃水果"
///     derived_from: Vec<UUID>,   // 基于的事件ID
///     confidence: f32,           // 0.73
///     expires_at: DateTime,      // 30天后过期
///     status: ViewStatus,        // active | expired | promoted
/// }
/// ```
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
pub struct CognitiveView {
    pub view_id: Uuid,
    pub user_id: String,
    pub hypothesis: String,
    pub view_type: String,
    pub description: Option<String>,
    pub derived_from: serde_json::Value,
    pub evidence_count: i32,
    pub confidence: f64,
    pub validation_count: i32,
    pub last_validated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub promoted_to: Option<Uuid>,
    pub source: String,
    pub tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    /// Events that contradict this view hypothesis (for Promotion Gate)
    pub counter_evidence: serde_json::Value,
    /// Cached count of counter-evidence events
    pub counter_evidence_count: i32,
}

impl CognitiveView {
    /// Check if the view has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < chrono::Utc::now()
    }

    /// Get the actual ViewStatus from the string
    pub fn get_status(&self) -> ViewStatus {
        self.status.as_str().into()
    }

    /// Calculate default expiration time (30 days from now)
    pub fn default_expiration() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() + chrono::Duration::days(30)
    }

    /// Check if this view is ready for promotion
    ///
    /// # Promotion Gate (HEAD.md + skill)
    /// ```text
    /// fn should_promote(view: &DerivedView) -> bool {
    ///     view.confidence > 0.85
    ///         && view.time_span > 30.days()
    ///         && view.validated_count >= 3
    ///         && counter_evidence_ratio < 0.15
    ///         && !has_conflicting_views()
    /// }
    /// ```
    pub fn is_ready_for_promotion(&self) -> bool {
        // Basic criteria
        if self.confidence <= 0.85 {
            return false;
        }
        if self.validation_count < 3 {
            return false;
        }
        if (self.expires_at - self.created_at).num_days() < 30 {
            return false;
        }
        if !self.get_status().can_be_promoted() {
            return false;
        }

        // Check counter-evidence ratio (< 15% per skill)
        let counter_ratio = self.counter_evidence_ratio();
        if counter_ratio >= 0.15 {
            return false;
        }

        // All checks passed
        true
    }

    /// Calculate counter-evidence ratio
    ///
    /// Returns the ratio of counter-evidence to supporting evidence.
    /// Higher ratio = more contradictions.
    pub fn counter_evidence_ratio(&self) -> f64 {
        if self.evidence_count == 0 {
            return 0.0;
        }
        self.counter_evidence_count as f64 / self.evidence_count as f64
    }

    /// Check if this view should be rejected due to high counter-evidence
    ///
    /// Per skill: if counter_ratio > 0.3, automatically reject
    pub fn should_be_rejected(&self) -> bool {
        self.counter_evidence_ratio() > 0.3
    }

    /// Check for contradictions with another view (programmatic keyword matching)
    ///
    /// This is a simplified conflict detection based on keyword pairs.
    /// Per skill, this checks for contradictory hypotheses like:
    /// - "喜欢" vs "讨厌"
    /// - "经常" vs "很少"
    /// - "总是" vs "从不"
    pub fn has_conflict_with(&self, other: &CognitiveView) -> bool {
        // Skip if same view or same user
        if self.view_id == other.view_id || self.user_id != other.user_id {
            return false;
        }

        // Get hypothesis texts
        let self_hypothesis = &self.hypothesis;
        let other_hypothesis = &other.hypothesis;

        // Define contradiction pairs
        let contradiction_pairs = vec![
            ("喜欢", "讨厌"),
            ("喜欢", "不喜欢"),
            ("爱", "恨"),
            ("经常", "很少"),
            ("总是", "从不"),
            ("每天", "从不"),
            ("习惯", "讨厌"),
        ];

        // Check if any contradiction pair exists in the two hypotheses
        for (positive, negative) in contradiction_pairs {
            let self_has_positive = self_hypothesis.contains(positive);
            let self_has_negative = self_hypothesis.contains(negative);
            let other_has_positive = other_hypothesis.contains(positive);
            let other_has_negative = other_hypothesis.contains(negative);

            // Contradiction: one has positive, other has negative for same concept
            if (self_has_positive && other_has_negative) || (self_has_negative && other_has_positive) {
                // Additional check: must be about same target/action
                if self.hypothesis_matches_target(other_hypothesis) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if two hypotheses refer to the same target/action
    ///
    /// This is a simplified check for whether contradictions are relevant.
    /// For example, "喜欢吃水果" and "讨厌吃水果" should conflict,
    /// but "喜欢吃水果" and "讨厌吃蔬菜" should not.
    fn hypothesis_matches_target(&self, other: &str) -> bool {
        // Extract action/target from hypothesis (simplified)
        // This is a basic implementation - can be enhanced with NLP

        // Split hypothesis by common delimiters
        let self_parts: Vec<&str> = self.hypothesis.split(|c| c == '的' || c == '是' || c == '吃').collect();
        let other_parts: Vec<&str> = other.split(|c| c == '的' || c == '是' || c == '吃').collect();

        // Check if any part matches (excluding the sentiment words)
        for self_part in &self_parts {
            for other_part in &other_parts {
                let self_clean = self_part.trim();
                let other_clean = other_part.trim();

                // Skip empty strings and sentiment words
                if self_clean.is_empty() || other_clean.is_empty() {
                    continue;
                }

                // Skip common sentiment words
                let sentiment_words = ["喜欢", "讨厌", "爱", "恨", "经常", "很少", "总是", "从不"];
                if sentiment_words.contains(&self_clean) || sentiment_words.contains(&other_clean) {
                    continue;
                }

                // If any significant word matches, consider it a conflict
                if self_clean == other_clean || self_clean.len() > 1 && other_clean.contains(self_clean) {
                    return true;
                }
            }
        }

        false
    }

    /// Add counter-evidence to this view
    ///
    /// Returns updated counter_evidence_count
    pub fn add_counter_evidence(&mut self, event_id: Uuid) -> i32 {
        // Add to counter_evidence array
        if let Ok(mut arr) = serde_json::from_value::<Vec<Uuid>>(self.counter_evidence.clone()) {
            arr.push(event_id);
            self.counter_evidence = serde_json::to_value(arr).unwrap_or_default();
            self.counter_evidence_count += 1;
        } else {
            // If parsing failed, create new array
            self.counter_evidence = serde_json::json!([event_id]);
            self.counter_evidence_count = 1;
        }

        self.counter_evidence_count
    }
}

/// New Cognitive View for insertion
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = cognitive_views)]
pub struct NewCognitiveView {
    pub user_id: String,
    pub hypothesis: String,
    pub view_type: String,
    pub description: Option<String>,
    pub derived_from: serde_json::Value,
    pub evidence_count: i32,
    pub confidence: f64,
    pub validation_count: i32,
    pub last_validated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub promoted_to: Option<Uuid>,
    pub source: String,
    pub tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    /// Events that contradict this view hypothesis
    pub counter_evidence: serde_json::Value,
    /// Cached count of counter-evidence events
    pub counter_evidence_count: i32,
}

impl NewCognitiveView {
    /// Create a new cognitive view
    ///
    /// # Arguments
    /// * `user_id` - Owner of the view
    /// * `hypothesis` - The hypothesis/pattern
    /// * `view_type` - Type of view (pattern, preference, habit, trend)
    /// * `derived_from` - Event IDs that support this hypothesis
    pub fn new(
        user_id: String,
        hypothesis: String,
        view_type: String,
        derived_from: Vec<Uuid>,
    ) -> Self {
        let now = chrono::Utc::now();
        let evidence_count = derived_from.len() as i32;
        Self {
            user_id,
            hypothesis,
            view_type,
            description: None,
            derived_from: serde_json::to_value(&derived_from).unwrap_or_default(),
            evidence_count,
            confidence: 0.5,
            validation_count: 0,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: now,
            updated_at: now,
            expires_at: now + chrono::Duration::days(30),
            promoted_to: None,
            source: "pattern_detector".to_string(),
            tags: Some(serde_json::json!({})),
            metadata: Some(serde_json::json!({})),
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        }
    }

    /// Set confidence level
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set view type
    pub fn with_view_type(mut self, view_type: &str) -> Self {
        self.view_type = view_type.to_string();
        self
    }

    /// Set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set custom expiration time
    pub fn with_expiration(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = expires_at;
        self
    }

    /// Set source
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }
}

/// Stable Concept - a promoted view that has passed the promotion gate
///
/// This represents stable, validated knowledge about the user.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
pub struct StableConcept {
    pub concept_id: Uuid,
    pub user_id: String,
    pub canonical_name: String,
    pub display_name: String,
    pub concept_type: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub version: i32,
    pub parent_concept_id: Option<Uuid>,
    pub is_deprecated: bool,
    pub promoted_from: Option<Uuid>,
    pub promoted_at: chrono::DateTime<chrono::Utc>,
    pub promotion_confidence: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deprecated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub access_count: i32,
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source: String,
    pub tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

impl StableConcept {
    /// Check if this concept is active (not deprecated)
    pub fn is_active(&self) -> bool {
        !self.is_deprecated
    }

    /// Increment access count
    pub fn increment_access(&self) -> Self {
        Self {
            access_count: self.access_count + 1,
            last_accessed_at: Some(chrono::Utc::now()),
            ..self.clone()
        }
    }

    /// Create a new version of this concept
    ///
    /// # Arguments
    /// * `new_display_name` - Updated display name (if changed)
    /// * `new_description` - Updated description (if changed)
    /// * `new_definition` - Updated definition (if changed)
    ///
    /// # Returns
    /// A NewStableConcept with incremented version and parent link to this concept
    pub fn create_new_version(
        &self,
        new_display_name: Option<String>,
        new_description: Option<String>,
        new_definition: Option<serde_json::Value>,
    ) -> NewStableConcept {
        let now = chrono::Utc::now();

        NewStableConcept {
            user_id: self.user_id.clone(),
            canonical_name: self.canonical_name.clone(),
            display_name: new_display_name.unwrap_or_else(|| self.display_name.clone()),
            concept_type: self.concept_type.clone(),
            description: new_description.or(self.description.clone()),
            definition: new_definition.unwrap_or_else(|| self.definition.clone()),
            version: self.version + 1,  // Increment version
            parent_concept_id: Some(self.concept_id),  // Link to parent
            is_deprecated: false,  // New version is active
            promoted_from: self.promoted_from,
            promoted_at: self.promoted_at,
            promotion_confidence: self.promotion_confidence,
            created_at: now,
            updated_at: now,
            deprecated_at: None,
            access_count: 0,  // Reset access count for new version
            last_accessed_at: Some(now),
            source: format!("{}_v{}", self.source, self.version + 1),
            tags: self.tags.clone(),
            metadata: self.metadata.clone(),
        }
    }

    /// Create a deprecated version of this concept (for rollback)
    ///
    /// This marks the current concept as deprecated and returns a NewStableConcept
    /// that represents the deprecation event.
    pub fn deprecate(&self, reason: Option<String>) -> NewStableConcept {
        let now = chrono::Utc::now();

        // Update metadata with deprecation reason
        let mut metadata = self.metadata.clone().unwrap_or_else(|| serde_json::json!({}));
        if let Some(meta_obj) = metadata.as_object_mut() {
            if let Some(reason_str) = reason {
                meta_obj.insert("deprecation_reason".to_string(), serde_json::json!(reason_str));
            }
            meta_obj.insert("deprecated_at".to_string(), serde_json::json!(now.to_rfc3339()));
            meta_obj.insert("deprecated_by".to_string(), serde_json::json!("version_update"));
        }

        NewStableConcept {
            user_id: self.user_id.clone(),
            canonical_name: self.canonical_name.clone(),
            display_name: self.display_name.clone(),
            concept_type: self.concept_type.clone(),
            description: self.description.clone(),
            definition: self.definition.clone(),
            version: self.version,  // Keep same version
            parent_concept_id: self.parent_concept_id,
            is_deprecated: true,  // Mark as deprecated
            promoted_from: self.promoted_from,
            promoted_at: self.promoted_at,
            promotion_confidence: self.promotion_confidence,
            created_at: self.created_at,
            updated_at: now,  // Update timestamp
            deprecated_at: Some(now),
            access_count: self.access_count,
            last_accessed_at: self.last_accessed_at,
            source: self.source.clone(),
            tags: self.tags.clone(),
            metadata: Some(metadata),
        }
    }

    /// Check if this is the latest version in the version chain
    ///
    /// This is a simplified check - in production, you'd query the database
    /// to see if there are any newer versions with this as parent_concept_id.
    pub fn is_latest_version(&self) -> bool {
        !self.is_deprecated
    }

    /// Get version number as string
    pub fn version_string(&self) -> String {
        format!("v{}", self.version)
    }

    /// Check if this concept can be rolled back
    ///
    /// A concept can be rolled back if it has a parent (previous version)
    pub fn can_rollback(&self) -> bool {
        self.parent_concept_id.is_some()
    }

    /// Get a summary of this concept
    pub fn summary(&self) -> String {
        format!(
            "{} ({}): {} - {}",
            self.canonical_name,
            self.version_string(),
            self.display_name,
            if self.is_active() { "Active" } else { "Deprecated" }
        )
    }

    /// Create a rollback concept (new version based on parent)
    ///
    /// # Arguments
    /// * `parent_concept` - The parent concept to rollback to
    pub fn create_rollback_version(&self, parent_concept: &StableConcept) -> NewStableConcept {
        let now = chrono::Utc::now();

        // Create metadata documenting the rollback
        let metadata = serde_json::json!({
            "rollback_from_version": self.version,
            "rollback_to_version": parent_concept.version,
            "rollback_at": now.to_rfc3339(),
            "original_created_at": self.created_at.to_rfc3339(),
        });

        NewStableConcept {
            user_id: self.user_id.clone(),
            canonical_name: self.canonical_name.clone(),
            display_name: parent_concept.display_name.clone(),
            concept_type: self.concept_type.clone(),
            description: parent_concept.description.clone(),
            definition: parent_concept.definition.clone(),
            version: parent_concept.version + 1,  // New version number
            parent_concept_id: Some(parent_concept.concept_id),
            is_deprecated: false,
            promoted_from: parent_concept.promoted_from,
            promoted_at: parent_concept.promoted_at,
            promotion_confidence: parent_concept.promotion_confidence,
            created_at: now,
            updated_at: now,
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: Some(now),
            source: format!("rollback_from_v{}", self.version),
            tags: parent_concept.tags.clone(),
            metadata: Some(metadata),
        }
    }
}

/// New Stable Concept for insertion
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = stable_concepts)]
pub struct NewStableConcept {
    pub user_id: String,
    pub canonical_name: String,
    pub display_name: String,
    pub concept_type: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub version: i32,
    pub parent_concept_id: Option<Uuid>,
    pub is_deprecated: bool,
    pub promoted_from: Option<Uuid>,
    pub promoted_at: chrono::DateTime<chrono::Utc>,
    pub promotion_confidence: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deprecated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub access_count: i32,
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source: String,
    pub tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

impl NewStableConcept {
    /// Create a new stable concept from a promoted view
    ///
    /// # Arguments
    /// * `user_id` - Owner of the concept
    /// * `canonical_name` - Machine-readable name (e.g., "likes_fruit")
    /// * `display_name` - Human-readable name (e.g., "喜欢吃水果")
    /// * `concept_type` - Type of concept
    /// * `promoted_from` - Source view ID
    /// * `promotion_confidence` - Confidence at promotion time
    pub fn from_view(
        user_id: String,
        canonical_name: String,
        display_name: String,
        concept_type: String,
        promoted_from: Uuid,
        promotion_confidence: f64,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id,
            canonical_name,
            display_name,
            concept_type,
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: Some(promoted_from),
            promoted_at: now,
            promotion_confidence,
            created_at: now,
            updated_at: now,
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: Some(now),
            source: "promotion_gate".to_string(),
            tags: Some(serde_json::json!([])),
            metadata: Some(serde_json::json!({})),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_status_conversion() {
        let status = ViewStatus::Active;
        let s: String = status.into();
        assert_eq!(s, "active");

        let status2: ViewStatus = s.into();
        assert_eq!(status2, ViewStatus::Active);
    }

    #[test]
    fn test_view_status_active() {
        assert!(ViewStatus::Active.is_active());
        assert!(ViewStatus::Active.can_be_promoted());
        assert!(!ViewStatus::Expired.is_active());
        assert!(!ViewStatus::Promoted.can_be_promoted());
    }

    #[test]
    fn test_new_cognitive_view() {
        let view = NewCognitiveView::new(
            "test_user".to_string(),
            "用户喜欢吃水果".to_string(),
            "preference".to_string(),
            vec![],
        );

        assert_eq!(view.confidence, 0.5);
        assert_eq!(view.validation_count, 0);
        assert_eq!(view.status, "active");
        assert!(view.expires_at > chrono::Utc::now());
    }

    #[test]
    fn test_cognitive_view_ready_for_promotion() {
        let mut view = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "用户喜欢吃水果".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 5,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        // Should be ready for promotion
        assert!(view.is_ready_for_promotion());

        // Not enough confidence
        view.confidence = 0.8;
        assert!(!view.is_ready_for_promotion());

        // Restore confidence
        view.confidence = 0.9;

        // Not enough validation
        view.validation_count = 2;
        assert!(!view.is_ready_for_promotion());
    }

    #[test]
    fn test_stable_concept_active() {
        let concept = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        assert!(concept.is_active());

        let incremented = concept.increment_access();
        assert_eq!(incremented.access_count, 1);
    }

    #[test]
    fn test_counter_evidence_ratio() {
        let view = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "用户喜欢吃水果".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 10,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        // No counter-evidence
        assert_eq!(view.counter_evidence_ratio(), 0.0);

        // Some counter-evidence but still acceptable
        let mut view_with_ce = view.clone();
        view_with_ce.counter_evidence_count = 1;
        assert_eq!(view_with_ce.counter_evidence_ratio(), 0.1);
    }

    #[test]
    fn test_should_be_rejected() {
        let mut view = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "用户喜欢吃水果".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 10,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        // Low counter-evidence - should not be rejected
        assert!(!view.should_be_rejected());

        // High counter-evidence (> 30%) - should be rejected
        view.counter_evidence_count = 4;
        assert!(view.should_be_rejected());
    }

    #[test]
    fn test_promotion_gate_with_counter_evidence() {
        let mut view = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "用户喜欢吃水果".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 20,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        // Should be ready without counter-evidence
        assert!(view.is_ready_for_promotion());

        // Add counter-evidence at threshold (15% of 20 = 3)
        view.counter_evidence_count = 3;
        assert!(!view.is_ready_for_promotion()); // Should fail at >= 15%
    }

    #[test]
    fn test_has_conflict_with() {
        let view_like = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "喜欢吃水果".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 10,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        let view_hate = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "讨厌吃水果".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 10,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        let view_like_veggie = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "喜欢吃蔬菜".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 10,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        // Should detect conflict between "喜欢" and "讨厌" for same target
        assert!(view_like.has_conflict_with(&view_hate));

        // Should NOT detect conflict for different targets
        assert!(!view_like.has_conflict_with(&view_like_veggie));
    }

    #[test]
    fn test_add_counter_evidence() {
        let mut view = CognitiveView {
            view_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            hypothesis: "用户喜欢吃水果".to_string(),
            view_type: "preference".to_string(),
            description: None,
            derived_from: serde_json::json!([]),
            evidence_count: 10,
            confidence: 0.9,
            validation_count: 5,
            last_validated_at: None,
            status: ViewStatus::Active.into(),
            created_at: chrono::Utc::now() - chrono::Duration::days(35),
            updated_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(5),
            promoted_to: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
            counter_evidence: serde_json::json!([]),
            counter_evidence_count: 0,
        };

        assert_eq!(view.counter_evidence_count, 0);

        let event_id = Uuid::new_v4();
        let new_count = view.add_counter_evidence(event_id);

        assert_eq!(new_count, 1);
        assert_eq!(view.counter_evidence_count, 1);
    }

    // Stable Concept Versioning Tests

    #[test]
    fn test_stable_concept_is_active() {
        let concept = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        assert!(concept.is_active());
    }

    #[test]
    fn test_stable_concept_increment_access() {
        let concept = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 5,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        assert_eq!(concept.access_count, 5);

        let incremented = concept.increment_access();
        assert_eq!(incremented.access_count, 6);
        assert!(incremented.last_accessed_at.is_some());
    }

    #[test]
    fn test_create_new_version() {
        let concept_v1 = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: Some("原描述".to_string()),
            definition: serde_json::json!({"key": "value"}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 10,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        // Create v2 with updated description
        let v2 = concept_v1.create_new_version(
            Some("爱吃水果".to_string()),  // Updated display name
            Some("更新后的描述".to_string()),
            Some(serde_json::json!({"key": "new_value"})),
        );

        assert_eq!(v2.version, 2);  // Version incremented
        assert_eq!(v2.parent_concept_id, Some(concept_v1.concept_id));  // Parent link
        assert_eq!(v2.display_name, "爱吃水果");
        assert_eq!(v2.description, Some("更新后的描述".to_string()));
        assert!(!v2.is_deprecated);  // New version is active
        assert_eq!(v2.access_count, 0);  // Reset access count
    }

    #[test]
    fn test_deprecate_concept() {
        let concept = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 5,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        let deprecated = concept.deprecate(Some("版本过时".to_string()));

        assert!(deprecated.is_deprecated);
        assert!(deprecated.deprecated_at.is_some());
        assert_eq!(deprecated.version, 1);  // Version unchanged
    }

    #[test]
    fn test_version_string() {
        let concept_v1 = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        assert_eq!(concept_v1.version_string(), "v1");

        let concept_v5 = concept_v1.clone();
        let mut v5 = concept_v5;
        v5.version = 5;
        assert_eq!(v5.version_string(), "v5");
    }

    #[test]
    fn test_can_rollback() {
        let concept_v1 = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        assert!(!concept_v1.can_rollback());  // No parent, cannot rollback

        // Create v2
        let v2 = concept_v1.create_new_version(None, None, None);
        assert!(v2.parent_concept_id.is_some());
    }

    #[test]
    fn test_concept_summary() {
        let concept = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 2,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        let summary = concept.summary();
        assert!(summary.contains("likes_fruit"));
        assert!(summary.contains("v2"));
        assert!(summary.contains("Active"));
    }

    #[test]
    fn test_create_rollback_version() {
        let concept_v1 = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: Some("原版本".to_string()),
            definition: serde_json::json!({"v": 1}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        // Create full StableConcept v2
        let concept_v2 = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: concept_v1.user_id.clone(),
            canonical_name: concept_v1.canonical_name.clone(),
            display_name: "爱吃水果".to_string(),
            concept_type: concept_v1.concept_type.clone(),
            description: Some("新版本".to_string()),
            definition: serde_json::json!({"v": 2}),
            version: 2,
            parent_concept_id: Some(concept_v1.concept_id),
            is_deprecated: false,
            promoted_from: concept_v1.promoted_from,
            promoted_at: concept_v1.promoted_at,
            promotion_confidence: concept_v1.promotion_confidence,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: Some(chrono::Utc::now()),
            source: "test_v2".to_string(),
            tags: concept_v1.tags.clone(),
            metadata: concept_v1.metadata.clone(),
        };

        // Rollback from v2 to v1
        let rollback = concept_v2.create_rollback_version(&concept_v1);
        assert_eq!(rollback.version, 2);  // New version number (v1 + 1)
        assert_eq!(rollback.parent_concept_id, Some(concept_v1.concept_id));
        assert_eq!(rollback.display_name, "喜欢吃水果");  // Restored v1 display name
        assert_eq!(rollback.description, Some("原版本".to_string()));
        assert!(rollback.source.contains("rollback_from_v2"));
    }

    #[test]
    fn test_is_latest_version() {
        let mut concept = StableConcept {
            concept_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            canonical_name: "likes_fruit".to_string(),
            display_name: "喜欢吃水果".to_string(),
            concept_type: "preference".to_string(),
            description: None,
            definition: serde_json::json!({}),
            version: 1,
            parent_concept_id: None,
            is_deprecated: false,
            promoted_from: None,
            promoted_at: chrono::Utc::now(),
            promotion_confidence: 0.9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deprecated_at: None,
            access_count: 0,
            last_accessed_at: None,
            source: "test".to_string(),
            tags: None,
            metadata: None,
        };

        assert!(concept.is_latest_version());

        // When deprecated, not latest
        concept.is_deprecated = true;
        assert!(!concept.is_latest_version());
    }
}
