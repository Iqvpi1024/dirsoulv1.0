//! Derived View Generator - Convert patterns to Cognitive Views
//!
//! This module connects the PatternDetector with the Cognitive Memory layer,
//! converting detected patterns into CognitiveView instances that can be stored
//! and potentially promoted to StableConcepts.
//!
//! # Design Principles (HEAD.md)
//! - **慢抽象原则**: Generate Derived Views first (discardable)
//! - **Promotion Gate 把关**: Views must pass validation before becoming concepts
//! - **避免 LLM 幻觉放大**: Isolate AI judgments from system structure

use crate::cognitive::{NewCognitiveView, ViewStatus};
use crate::error::Result;
use crate::pattern_detector::{DetectedPattern, PatternMetadata, PatternType};
use chrono::{Duration, Utc};
use uuid::Uuid;

/// Configuration for view generation
#[derive(Debug, Clone)]
pub struct ViewGeneratorConfig {
    /// Default expiration time in days (30 days per HEAD.md)
    pub default_expiration_days: i64,
    /// Base confidence multiplier for high-frequency patterns
    pub high_frequency_confidence_multiplier: f64,
    /// Base confidence multiplier for trend patterns
    pub trend_confidence_multiplier: f64,
    /// Base confidence multiplier for anomaly patterns
    pub anomaly_confidence_multiplier: f64,
    /// Base confidence multiplier for temporal patterns
    pub temporal_confidence_multiplier: f64,
    /// Minimum confidence threshold for view creation
    pub min_confidence_threshold: f64,
}

impl Default for ViewGeneratorConfig {
    fn default() -> Self {
        Self {
            default_expiration_days: 30,  // Per HEAD.md requirement
            high_frequency_confidence_multiplier: 1.0,
            trend_confidence_multiplier: 0.9,  // Trends may be less stable
            anomaly_confidence_multiplier: 0.8,  // Anomalies are less certain
            temporal_confidence_multiplier: 1.1,  // Temporal patterns are reliable
            min_confidence_threshold: 0.5,
        }
    }
}

/// Derived View Generator - Converts patterns to Cognitive Views
pub struct ViewGenerator {
    config: ViewGeneratorConfig,
}

impl ViewGenerator {
    /// Create a new view generator with default config
    pub fn new() -> Self {
        Self {
            config: ViewGeneratorConfig::default(),
        }
    }

    /// Create a new view generator with custom config
    pub fn with_config(config: ViewGeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate a CognitiveView from a DetectedPattern
    ///
    /// This is the main entry point for converting patterns into views.
    /// It calculates confidence, sets expiration time, and determines view type.
    ///
    /// # Arguments
    /// * `pattern` - The detected pattern from PatternDetector
    /// * `user_id` - The user who owns this view
    ///
    /// # Returns
    /// A NewCognitiveView ready to be inserted into the database
    pub fn generate_view(
        &self,
        pattern: &DetectedPattern,
        user_id: &str,
    ) -> Result<NewCognitiveView> {
        // Calculate confidence based on pattern type and metadata
        let confidence = self.calculate_confidence(pattern);

        // Apply minimum threshold
        if confidence < self.config.min_confidence_threshold {
            return Err(crate::error::DirSoulError::NotFound(
                format!("Pattern confidence {:.2} below threshold {:.2}",
                       confidence, self.config.min_confidence_threshold)
            ));
        }

        // Extract event IDs from pattern evidence
        let derived_from = self.extract_event_ids(pattern);

        // Calculate expiration time
        let expires_at = self.calculate_expiration(pattern);

        // Determine view type
        let view_type = self.determine_view_type(pattern);

        // Create the view
        let view = NewCognitiveView::new(
            user_id.to_string(),
            pattern.description.clone(),
            view_type,
            derived_from,
        )
        .with_confidence(confidence)
        .with_expiration(expires_at)
        .with_description(&pattern.description)
        .with_source("pattern_detector");

        Ok(view)
    }

    /// Generate multiple views from a pattern detection result
    ///
    /// # Arguments
    /// * `detection_result` - The complete pattern detection result
    /// * `user_id` - The user who owns these views
    ///
    /// # Returns
    /// A vector of NewCognitiveView instances (only those passing confidence threshold)
    pub fn generate_views_from_result(
        &self,
        detection_result: &crate::pattern_detector::PatternDetectionResult,
        user_id: &str,
    ) -> Result<Vec<NewCognitiveView>> {
        let mut views = Vec::new();

        for pattern in &detection_result.patterns {
            match self.generate_view(pattern, user_id) {
                Ok(view) => views.push(view),
                Err(_) => {
                    // Skip patterns that don't meet confidence threshold
                    continue;
                }
            }
        }

        Ok(views)
    }

    /// Calculate confidence based on pattern type and metadata
    fn calculate_confidence(&self, pattern: &DetectedPattern) -> f64 {
        let base_confidence = pattern.confidence;

        // Apply type-specific multiplier
        let multiplier = match pattern.pattern_type {
            PatternType::HighFrequency => self.config.high_frequency_confidence_multiplier,
            PatternType::Trend => self.config.trend_confidence_multiplier,
            PatternType::Anomaly => self.config.anomaly_confidence_multiplier,
            PatternType::Temporal => self.config.temporal_confidence_multiplier,
        };

        // Boost confidence based on evidence count
        let evidence_boost = self.calculate_evidence_boost(pattern.evidence_count);

        // Boost confidence based on time span (longer = more reliable)
        let time_span_boost = self.calculate_time_span_boost(pattern.time_span_days);

        // Combine all factors
        let combined = base_confidence * multiplier * evidence_boost * time_span_boost;

        // Clamp to [0, 1]
        combined.max(0.0).min(1.0)
    }

    /// Calculate evidence boost based on number of supporting events
    fn calculate_evidence_boost(&self, evidence_count: i32) -> f64 {
        // More evidence = higher confidence, but with diminishing returns
        // Use log-like scaling: 1 -> 1.0, 10 -> 1.3, 100 -> 1.5
        let count = evidence_count as f64;
        1.0 + (count.ln() / 20.0).min(0.5)
    }

    /// Calculate time span boost based on pattern duration
    fn calculate_time_span_boost(&self, time_span_days: i32) -> f64 {
        // Longer time spans indicate more stable patterns
        // 1 day -> 1.0, 30 days -> 1.2, 90 days -> 1.3
        let days = time_span_days as f64;
        let normalized = (days / 30.0).ln().max(0.0);  // Normalize to 30-day periods
        1.0 + (normalized / 10.0).min(0.3)
    }

    /// Extract event IDs from pattern metadata
    fn extract_event_ids(&self, pattern: &DetectedPattern) -> Vec<Uuid> {
        // For now, generate placeholder UUIDs based on evidence count
        // In production, this would extract actual event IDs from pattern evidence
        (0..pattern.evidence_count.min(100))
            .map(|_| Uuid::new_v4())
            .collect()
    }

    /// Calculate expiration time based on pattern characteristics
    fn calculate_expiration(&self, pattern: &DetectedPattern) -> chrono::DateTime<Utc> {
        let base_days = self.config.default_expiration_days;

        // Adjust expiration based on confidence
        // Higher confidence = longer expiration
        let confidence_multiplier = pattern.confidence;
        let adjusted_days = (base_days as f64 * confidence_multiplier) as i64;

        // Range: [15 days, 60 days]
        let clamped_days = adjusted_days.max(15).min(60);

        Utc::now() + Duration::days(clamped_days)
    }

    /// Determine view type string from pattern type
    fn determine_view_type(&self, pattern: &DetectedPattern) -> String {
        match pattern.pattern_type {
            PatternType::HighFrequency => "habit".to_string(),
            PatternType::Trend => "trend".to_string(),
            PatternType::Anomaly => "anomaly".to_string(),
            PatternType::Temporal => "routine".to_string(),
        }
    }

    /// Generate a view with custom expiration time
    pub fn generate_view_with_expiration(
        &self,
        pattern: &DetectedPattern,
        user_id: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<NewCognitiveView> {
        let confidence = self.calculate_confidence(pattern);

        if confidence < self.config.min_confidence_threshold {
            return Err(crate::error::DirSoulError::NotFound(
                format!("Pattern confidence {:.2} below threshold {:.2}",
                       confidence, self.config.min_confidence_threshold)
            ));
        }

        let derived_from = self.extract_event_ids(pattern);
        let view_type = self.determine_view_type(pattern);

        let view = NewCognitiveView::new(
            user_id.to_string(),
            pattern.description.clone(),
            view_type,
            derived_from,
        )
        .with_confidence(confidence)
        .with_expiration(expires_at)
        .with_description(&pattern.description)
        .with_source("pattern_detector");

        Ok(view)
    }

    /// Batch generate views with confidence filtering
    pub fn generate_views_filtered(
        &self,
        patterns: &[DetectedPattern],
        user_id: &str,
        min_confidence: f64,
    ) -> Result<Vec<NewCognitiveView>> {
        let mut views = Vec::new();

        for pattern in patterns {
            let confidence = self.calculate_confidence(pattern);
            if confidence >= min_confidence {
                match self.generate_view(pattern, user_id) {
                    Ok(view) => views.push(view),
                    Err(_) => continue,
                }
            }
        }

        Ok(views)
    }
}

impl Default for ViewGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for ViewGenerator
pub struct ViewGeneratorBuilder {
    config: ViewGeneratorConfig,
}

impl ViewGeneratorBuilder {
    pub fn new() -> Self {
        Self {
            config: ViewGeneratorConfig::default(),
        }
    }

    pub fn with_expiration_days(mut self, days: i64) -> Self {
        self.config.default_expiration_days = days;
        self
    }

    pub fn with_min_confidence(mut self, confidence: f64) -> Self {
        self.config.min_confidence_threshold = confidence;
        self
    }

    pub fn with_high_frequency_multiplier(mut self, mult: f64) -> Self {
        self.config.high_frequency_confidence_multiplier = mult;
        self
    }

    pub fn with_trend_multiplier(mut self, mult: f64) -> Self {
        self.config.trend_confidence_multiplier = mult;
        self
    }

    pub fn with_anomaly_multiplier(mut self, mult: f64) -> Self {
        self.config.anomaly_confidence_multiplier = mult;
        self
    }

    pub fn with_temporal_multiplier(mut self, mult: f64) -> Self {
        self.config.temporal_confidence_multiplier = mult;
        self
    }

    pub fn build(self) -> ViewGenerator {
        ViewGenerator::with_config(self.config)
    }
}

impl Default for ViewGeneratorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_detector::{PatternMetadata, PatternType, TrendDirection};
    use chrono::Utc;

    fn create_test_pattern(pattern_type: PatternType, confidence: f64) -> DetectedPattern {
        DetectedPattern {
            pattern_type,
            pattern_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            description: "Test pattern".to_string(),
            action: "eat".to_string(),
            target: "apple".to_string(),
            confidence,
            evidence_count: 10,
            time_span_days: 30,
            metadata: PatternMetadata::HighFrequency {
                average_frequency_per_day: 1.0,
                consistency_score: 0.8,
                typical_times: vec![],
            },
            detected_at: Utc::now(),
        }
    }

    #[test]
    fn test_view_generator_creation() {
        let generator = ViewGenerator::new();
        assert_eq!(generator.config.default_expiration_days, 30);
    }

    #[test]
    fn test_calculate_confidence_high_frequency() {
        let generator = ViewGenerator::new();
        let pattern = create_test_pattern(PatternType::HighFrequency, 0.7);

        let confidence = generator.calculate_confidence(&pattern);

        // Base 0.7 * multiplier 1.0 * evidence_boost * time_span_boost
        assert!(confidence > 0.7);
        assert!(confidence <= 1.0);
    }

    #[test]
    fn test_calculate_confidence_temporal() {
        let generator = ViewGenerator::new();
        let pattern = create_test_pattern(PatternType::Temporal, 0.6);

        let confidence = generator.calculate_confidence(&pattern);

        // Temporal patterns get 1.1x multiplier
        assert!(confidence > 0.6);
        assert!(confidence <= 1.0);
    }

    #[test]
    fn test_calculate_confidence_anomaly() {
        let generator = ViewGenerator::new();
        let pattern = create_test_pattern(PatternType::Anomaly, 0.8);

        let confidence = generator.calculate_confidence(&pattern);

        // Anomalies get 0.8x multiplier (lower confidence)
        assert!(confidence < 0.8);
    }

    #[test]
    fn test_evidence_boost() {
        let generator = ViewGenerator::new();

        let boost_10 = generator.calculate_evidence_boost(10);
        let boost_100 = generator.calculate_evidence_boost(100);

        // More evidence = higher boost
        assert!(boost_100 > boost_10);
        // But with diminishing returns
        assert!(boost_100 < 1.5);  // Should be < 1.5x
    }

    #[test]
    fn test_time_span_boost() {
        let generator = ViewGenerator::new();

        let boost_7 = generator.calculate_time_span_boost(7);
        let boost_90 = generator.calculate_time_span_boost(90);

        // Longer time span = higher boost
        assert!(boost_90 > boost_7);
    }

    #[test]
    fn test_determine_view_type() {
        let generator = ViewGenerator::new();

        let hf_pattern = create_test_pattern(PatternType::HighFrequency, 0.7);
        let trend_pattern = create_test_pattern(PatternType::Trend, 0.7);
        let anomaly_pattern = create_test_pattern(PatternType::Anomaly, 0.7);
        let temporal_pattern = create_test_pattern(PatternType::Temporal, 0.7);

        assert_eq!(generator.determine_view_type(&hf_pattern), "habit");
        assert_eq!(generator.determine_view_type(&trend_pattern), "trend");
        assert_eq!(generator.determine_view_type(&anomaly_pattern), "anomaly");
        assert_eq!(generator.determine_view_type(&temporal_pattern), "routine");
    }

    #[test]
    fn test_calculate_expiration() {
        let generator = ViewGenerator::new();
        let pattern = create_test_pattern(PatternType::HighFrequency, 0.7);

        let expires_at = generator.calculate_expiration(&pattern);
        let now = Utc::now();
        let days_until_expiration = (expires_at - now).num_days();

        // Should be around 21 days (30 * 0.7)
        assert!(days_until_expiration >= 15);
        assert!(days_until_expiration <= 60);
    }

    #[test]
    fn test_generate_view_success() {
        let generator = ViewGenerator::new();
        let pattern = create_test_pattern(PatternType::HighFrequency, 0.8);

        let result = generator.generate_view(&pattern, "test_user");

        assert!(result.is_ok());
        let view = result.unwrap();
        assert_eq!(view.user_id, "test_user");
        assert_eq!(view.view_type, "habit");
        assert!(view.confidence >= 0.8);
    }

    #[test]
    fn test_generate_view_low_confidence() {
        let generator = ViewGenerator::new();
        let pattern = create_test_pattern(PatternType::HighFrequency, 0.3);

        let result = generator.generate_view(&pattern, "test_user");

        assert!(result.is_err());
    }

    #[test]
    fn test_view_generator_builder() {
        let generator = ViewGeneratorBuilder::new()
            .with_expiration_days(60)
            .with_min_confidence(0.7)
            .with_high_frequency_multiplier(1.2)
            .build();

        assert_eq!(generator.config.default_expiration_days, 60);
        assert_eq!(generator.config.min_confidence_threshold, 0.7);
        assert_eq!(generator.config.high_frequency_confidence_multiplier, 1.2);
    }

    #[test]
    fn test_generate_views_filtered() {
        let generator = ViewGenerator::new();

        let pattern1 = create_test_pattern(PatternType::HighFrequency, 0.8);
        let pattern2 = create_test_pattern(PatternType::Anomaly, 0.4);
        let pattern3 = create_test_pattern(PatternType::Temporal, 0.9);

        let patterns = vec![pattern1, pattern2, pattern3];
        let result = generator.generate_views_filtered(&patterns, "test_user", 0.6);

        assert!(result.is_ok());
        let views = result.unwrap();
        // Should only include pattern1 and pattern3 (confidence >= 0.6)
        assert_eq!(views.len(), 2);
    }

    #[test]
    fn test_view_generator_config_default() {
        let config = ViewGeneratorConfig::default();
        assert_eq!(config.default_expiration_days, 30);
        assert_eq!(config.high_frequency_confidence_multiplier, 1.0);
        assert_eq!(config.trend_confidence_multiplier, 0.9);
        assert_eq!(config.anomaly_confidence_multiplier, 0.8);
        assert_eq!(config.temporal_confidence_multiplier, 1.1);
        assert_eq!(config.min_confidence_threshold, 0.5);
    }
}
