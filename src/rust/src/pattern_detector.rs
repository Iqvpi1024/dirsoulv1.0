//! Pattern Detection Engine - Discover patterns from event memories
//!
//! This module implements the V1 pattern detection system using SQL statistics.
//! V2 will extend with NetworkX plugin for complex graph-based patterns.
//!
//! # Design Principles (HEAD.md)
//! - **V1 Positioning**: Simple statistics (SQL) + vector similarity
//! - **V2 Extension**: NetworkX plugin for complex patterns
//! - **Scheduled Tasks**: Daily runs to detect emerging patterns
//!
//! # Pattern Types
//! - **High-frequency behavior**: Repeated actions (e.g., daily coffee)
//! - **Trend analysis**: Changes over time (e.g., increased exercise)
//! - **Anomaly detection**: Deviations from baseline (e.g., skipping breakfast)

use crate::error::Result;
use chrono::{Datelike, Duration, Utc};
use crate::models::EventMemory;
use crate::schema::event_memories;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Pattern type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    /// High-frequency repeated behavior
    HighFrequency,
    /// Increasing or decreasing trend
    Trend,
    /// Unexpected deviation from baseline
    Anomaly,
    /// Time-based pattern (e.g., weekly routine)
    Temporal,
}

impl From<String> for PatternType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "high_frequency" => PatternType::HighFrequency,
            "trend" => PatternType::Trend,
            "anomaly" => PatternType::Anomaly,
            "temporal" => PatternType::Temporal,
            _ => PatternType::HighFrequency,
        }
    }
}

impl From<PatternType> for String {
    fn from(pt: PatternType) -> Self {
        match pt {
            PatternType::HighFrequency => "high_frequency".to_string(),
            PatternType::Trend => "trend".to_string(),
            PatternType::Anomaly => "anomaly".to_string(),
            PatternType::Temporal => "temporal".to_string(),
        }
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Detected pattern with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub pattern_id: Uuid,
    pub user_id: String,
    pub description: String,
    pub action: String,
    pub target: String,
    pub confidence: f64,
    pub evidence_count: i32,
    pub time_span_days: i32,
    pub metadata: PatternMetadata,
    pub detected_at: chrono::DateTime<Utc>,
}

/// Pattern-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternMetadata {
    HighFrequency {
        average_frequency_per_day: f64,
        consistency_score: f64,
        typical_times: Vec<String>,
    },
    Trend {
        direction: TrendDirection,
        change_percentage: f64,
        start_value: f64,
        end_value: f64,
    },
    Anomaly {
        expected_value: f64,
        actual_value: f64,
        deviation_percentage: f64,
        baseline_window_days: i32,
    },
    Temporal {
        period: String, // "daily", "weekly", "monthly"
        occurrences_at_period: i32,
        total_periods_observed: i32,
    },
}

/// Pattern detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDetectionResult {
    pub patterns: Vec<DetectedPattern>,
    pub events_analyzed: i32,
    pub time_range_start: chrono::DateTime<Utc>,
    pub time_range_end: chrono::DateTime<Utc>,
    pub detection_timestamp: chrono::DateTime<Utc>,
}

/// Configuration for pattern detection
#[derive(Debug, Clone)]
pub struct PatternDetectorConfig {
    /// Minimum frequency threshold (occurrences per day)
    pub min_frequency_threshold: f64,
    /// Minimum confidence for pattern detection
    pub min_confidence: f64,
    /// Minimum time span (days) for trend analysis
    pub min_trend_days: i32,
    /// Minimum deviation percentage for anomaly detection
    pub min_anomaly_deviation: f64,
    /// Baseline window for anomaly detection (days)
    pub anomaly_baseline_days: i32,
}

impl Default for PatternDetectorConfig {
    fn default() -> Self {
        Self {
            min_frequency_threshold: 0.5,  // At least once every 2 days
            min_confidence: 0.6,
            min_trend_days: 7,             // 1 week minimum
            min_anomaly_deviation: 0.5,     // 50% deviation
            anomaly_baseline_days: 30,      // 30-day baseline
        }
    }
}

/// Pattern Detector - Detects patterns from event memories
pub struct PatternDetector {
    config: PatternDetectorConfig,
}

impl PatternDetector {
    /// Create a new pattern detector with default config
    pub fn new() -> Self {
        Self {
            config: PatternDetectorConfig::default(),
        }
    }

    /// Create a new pattern detector with custom config
    pub fn with_config(config: PatternDetectorConfig) -> Self {
        Self { config }
    }

    /// Detect all patterns for a user within a time range
    pub fn detect_patterns(
        &self,
        conn: &mut PgConnection,
        user_id: &str,
        time_range: DetectionTimeRange,
    ) -> Result<PatternDetectionResult> {
        let events = self.fetch_events(conn, user_id, &time_range)?;
        let events_analyzed = events.len() as i32;

        let mut patterns = Vec::new();

        // Detect high-frequency patterns
        patterns.extend(self.detect_high_frequency_patterns(
            conn,
            user_id,
            &events,
            &time_range,
        )?);

        // Detect trends
        patterns.extend(self.detect_trends(
            conn,
            user_id,
            &events,
            &time_range,
        )?);

        // Detect anomalies
        patterns.extend(self.detect_anomalies(
            conn,
            user_id,
            &events,
            &time_range,
        )?);

        // Detect temporal patterns
        patterns.extend(self.detect_temporal_patterns(
            conn,
            user_id,
            &events,
            &time_range,
        )?);

        Ok(PatternDetectionResult {
            patterns,
            events_analyzed,
            time_range_start: time_range.start,
            time_range_end: time_range.end,
            detection_timestamp: Utc::now(),
        })
    }

    /// Fetch events within time range
    fn fetch_events(
        &self,
        conn: &mut PgConnection,
        user_id: &str,
        time_range: &DetectionTimeRange,
    ) -> Result<Vec<EventMemory>> {
        let events = event_memories::table
            .filter(event_memories::user_id.eq(user_id))
            .filter(event_memories::timestamp.ge(time_range.start))
            .filter(event_memories::timestamp.le(time_range.end))
            .order(event_memories::timestamp.asc())
            .load::<EventMemory>(conn)?;

        Ok(events)
    }

    /// Detect high-frequency patterns
    fn detect_high_frequency_patterns(
        &self,
        conn: &mut PgConnection,
        user_id: &str,
        events: &[EventMemory],
        time_range: &DetectionTimeRange,
    ) -> Result<Vec<DetectedPattern>> {
        let mut patterns = Vec::new();

        // Group events by action + target
        let mut action_counts: HashMap<(String, String), Vec<&EventMemory>> = HashMap::new();

        for event in events {
            let key = (event.action.clone(), event.target.clone());
            action_counts.entry(key).or_default().push(event);
        }

        // Calculate time span in days
        let time_span_days = (time_range.end - time_range.start).num_days() as f64;
        let min_occurrences = (time_span_days * self.config.min_frequency_threshold).ceil() as i32;

        // Check each action-target pair for high frequency
        for ((action, target), event_list) in action_counts {
            if event_list.len() as i32 >= min_occurrences {
                let frequency_per_day = event_list.len() as f64 / time_span_days;

                // Calculate consistency score (how regular is the pattern)
                let consistency_score = self.calculate_consistency(&event_list, time_span_days);

                if frequency_per_day >= self.config.min_frequency_threshold
                    && consistency_score >= self.config.min_confidence
                {
                    let pattern = DetectedPattern {
                        pattern_type: PatternType::HighFrequency,
                        pattern_id: Uuid::new_v4(),
                        user_id: user_id.to_string(),
                        description: format!("Frequently {} {} ({} times/day)",
                                          action, target,
                                          format!("{:.2}", frequency_per_day)),
                        action,
                        target,
                        confidence: consistency_score,
                        evidence_count: event_list.len() as i32,
                        time_span_days: time_span_days as i32,
                        metadata: PatternMetadata::HighFrequency {
                            average_frequency_per_day: frequency_per_day,
                            consistency_score,
                            typical_times: vec![], // TODO: Extract typical times
                        },
                        detected_at: Utc::now(),
                    };
                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }

    /// Calculate consistency score based on regularity
    fn calculate_consistency(&self, events: &[&EventMemory], time_span_days: f64) -> f64 {
        if events.len() < 2 {
            return 0.0;
        }

        // Calculate gaps between consecutive events
        let mut gaps = Vec::new();
        for window in events.windows(2) {
            let gap = window[1].timestamp.signed_duration_since(window[0].timestamp);
            gaps.push(gap.num_seconds() as f64);
        }

        if gaps.is_empty() {
            return 0.0;
        }

        // Calculate coefficient of variation (lower = more consistent)
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        let variance = gaps.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / gaps.len() as f64;
        let std_dev = variance.sqrt();
        let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };

        // Convert to consistency score (0-1, lower cv = higher consistency)
        let consistency = (1.0 - cv.min(1.0)).max(0.0);
        consistency
    }

    /// Detect trends (increasing/decreasing patterns)
    fn detect_trends(
        &self,
        conn: &mut PgConnection,
        user_id: &str,
        events: &[EventMemory],
        time_range: &DetectionTimeRange,
    ) -> Result<Vec<DetectedPattern>> {
        let mut patterns = Vec::new();

        // Only analyze if we have enough data
        let time_span_days = (time_range.end - time_range.start).num_days();
        if time_span_days < self.config.min_trend_days as i64 {
            return Ok(patterns);
        }

        // Group by action + target
        let mut action_groups: HashMap<(String, String), Vec<&EventMemory>> = HashMap::new();
        for event in events {
            let key = (event.action.clone(), event.target.clone());
            action_groups.entry(key).or_default().push(event);
        }

        // Analyze each group for trends
        for ((action, target), event_list) in action_groups {
            if event_list.len() < 3 {
                continue; // Need at least 3 data points
            }

            // Split into two halves and compare
            let mid = event_list.len() / 2;
            let first_half = &event_list[..mid];
            let second_half = &event_list[mid..];

            // Calculate frequencies
            let first_duration = self.calculate_duration(first_half);
            let second_duration = self.calculate_duration(second_half);

            let first_freq = first_half.len() as f64 / first_duration.max(1.0);
            let second_freq = second_half.len() as f64 / second_duration.max(1.0);

            // Calculate change
            let change_pct = if first_freq > 0.0 {
                (second_freq - first_freq) / first_freq
            } else {
                0.0
            };

            // Determine if this is a significant trend
            let (direction, is_significant) = if change_pct > 0.3 {
                (TrendDirection::Increasing, true)
            } else if change_pct < -0.3 {
                (TrendDirection::Decreasing, true)
            } else {
                (TrendDirection::Stable, false)
            };

            if is_significant {
                let pattern = DetectedPattern {
                    pattern_type: PatternType::Trend,
                    pattern_id: Uuid::new_v4(),
                    user_id: user_id.to_string(),
                    description: format!("{} {} is {:?} ({:.0}% change)",
                                      action, target, direction,
                                      change_pct.abs() * 100.0),
                    action: action.clone(),
                    target,
                    confidence: change_pct.abs().min(1.0),
                    evidence_count: event_list.len() as i32,
                    time_span_days: time_span_days as i32,
                    metadata: PatternMetadata::Trend {
                        direction,
                        change_percentage: change_pct,
                        start_value: first_freq,
                        end_value: second_freq,
                    },
                    detected_at: Utc::now(),
                };
                patterns.push(pattern);
            }
        }

        Ok(patterns)
    }

    /// Calculate duration span of events in days
    fn calculate_duration(&self, events: &[&EventMemory]) -> f64 {
        if events.len() < 2 {
            return 1.0;
        }
        let first = events.first().unwrap().timestamp;
        let last = events.last().unwrap().timestamp;
        (last - first).num_days().abs().max(1) as f64
    }

    /// Detect anomalies (deviations from baseline)
    fn detect_anomalies(
        &self,
        conn: &mut PgConnection,
        user_id: &str,
        events: &[EventMemory],
        time_range: &DetectionTimeRange,
    ) -> Result<Vec<DetectedPattern>> {
        let mut patterns = Vec::new();

        // Need baseline period
        let baseline_start = time_range.start - Duration::days(self.config.anomaly_baseline_days as i64);
        let baseline_end = time_range.start;

        // Fetch baseline events
        let baseline_events: Vec<EventMemory> = event_memories::table
            .filter(event_memories::user_id.eq(user_id))
            .filter(event_memories::timestamp.ge(baseline_start))
            .filter(event_memories::timestamp.lt(baseline_end))
            .load(conn)?;

        // Calculate baseline frequencies
        let mut baseline_freqs: HashMap<(String, String), f64> = HashMap::new();
        let baseline_duration = (baseline_end - baseline_start).num_days().max(1) as f64;

        for event in &baseline_events {
            let key = (event.action.clone(), event.target.clone());
            *baseline_freqs.entry(key).or_insert(0.0) += 1.0;
        }

        // Normalize by duration
        for freq in baseline_freqs.values_mut() {
            *freq /= baseline_duration;
        }

        // Calculate current frequencies
        let mut current_freqs: HashMap<(String, String), f64> = HashMap::new();
        let current_duration = (time_range.end - time_range.start).num_days().max(1) as f64;

        for event in events {
            let key = (event.action.clone(), event.target.clone());
            *current_freqs.entry(key).or_insert(0.0) += 1.0;
        }

        // Normalize by duration
        for freq in current_freqs.values_mut() {
            *freq /= current_duration;
        }

        // Detect anomalies (significant deviation from baseline)
        for ((action, target), &current_freq) in &current_freqs {
            let expected_freq = baseline_freqs.get(&(action.clone(), target.clone()))
                .unwrap_or(&0.0);

            // Skip if baseline is too low
            if *expected_freq < 0.1 {
                continue;
            }

            let deviation = if *expected_freq > 0.0 {
                (current_freq - expected_freq) / expected_freq
            } else {
                0.0
            };

            if deviation.abs() >= self.config.min_anomaly_deviation {
                let pattern = DetectedPattern {
                    pattern_type: PatternType::Anomaly,
                    pattern_id: Uuid::new_v4(),
                    user_id: user_id.to_string(),
                    description: format!("Anomaly: {} {} is {:.0}% {} expected",
                                      action, target,
                                      deviation.abs() * 100.0,
                                      if deviation > 0.0 { "higher than" } else { "lower than" }),
                    action: action.clone(),
                    target: target.clone(),
                    confidence: deviation.abs().min(1.0),
                    evidence_count: events.iter()
                        .filter(|e| &e.action == action && &e.target == target)
                        .count() as i32,
                    time_span_days: current_duration as i32,
                    metadata: PatternMetadata::Anomaly {
                        expected_value: *expected_freq,
                        actual_value: current_freq,
                        deviation_percentage: deviation,
                        baseline_window_days: self.config.anomaly_baseline_days,
                    },
                    detected_at: Utc::now(),
                };
                patterns.push(pattern);
            }
        }

        // Check for missing patterns (things that stopped happening)
        for ((action, target), &expected_freq) in &baseline_freqs {
            if expected_freq >= self.config.min_frequency_threshold {
                let current_freq = current_freqs.get(&(action.clone(), target.clone())).unwrap_or(&0.0);

                if *current_freq < expected_freq * (1.0 - self.config.min_anomaly_deviation) {
                    let deviation = (current_freq - expected_freq) / expected_freq;

                    let pattern = DetectedPattern {
                        pattern_type: PatternType::Anomaly,
                        pattern_id: Uuid::new_v4(),
                        user_id: user_id.to_string(),
                        description: format!("Anomaly: {} {} stopped (was {:.2}/day, now {:.2}/day)",
                                          action, target, expected_freq, current_freq),
                        action: action.clone(),
                        target: target.clone(),
                        confidence: deviation.abs().min(1.0),
                        evidence_count: 0, // No occurrences in current period
                        time_span_days: current_duration as i32,
                        metadata: PatternMetadata::Anomaly {
                            expected_value: expected_freq,
                            actual_value: *current_freq,
                            deviation_percentage: deviation,
                            baseline_window_days: self.config.anomaly_baseline_days,
                        },
                        detected_at: Utc::now(),
                    };
                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }

    /// Detect temporal patterns (daily, weekly, monthly)
    fn detect_temporal_patterns(
        &self,
        conn: &mut PgConnection,
        user_id: &str,
        events: &[EventMemory],
        time_range: &DetectionTimeRange,
    ) -> Result<Vec<DetectedPattern>> {
        let mut patterns = Vec::new();

        // Group by action + target
        let mut action_groups: HashMap<(String, String), Vec<&EventMemory>> = HashMap::new();
        for event in events {
            let key = (event.action.clone(), event.target.clone());
            action_groups.entry(key).or_default().push(event);
        }

        // Check for weekly patterns (same day of week)
        for ((action, target), event_list) in action_groups {
            if event_list.len() < 4 {
                continue; // Need at least 4 occurrences
            }

            // Group by day of week
            let mut dow_counts: HashMap<u32, Vec<usize>> = HashMap::new();
            for (idx, event) in event_list.iter().enumerate() {
                let dow = event.timestamp.weekday().num_days_from_monday();
                dow_counts.entry(dow).or_default().push(idx);
            }

            // Check if any day of week has consistent pattern
            let time_span_weeks = (time_range.end - time_range.start).num_weeks() as f64;

            for (dow, occurrence_indices) in dow_counts {
                let frequency = occurrence_indices.len() as f64 / time_span_weeks;

                // If happens on this day more than 60% of weeks
                if frequency >= 0.6 {
                    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                    let pattern = DetectedPattern {
                        pattern_type: PatternType::Temporal,
                        pattern_id: Uuid::new_v4(),
                        user_id: user_id.to_string(),
                        description: format!("Weekly pattern: {} {} on {}s ({:.0}% of weeks)",
                                          action, target, day_names[dow as usize],
                                          frequency * 100.0),
                        action: action.clone(),
                        target: target.clone(),
                        confidence: frequency,
                        evidence_count: occurrence_indices.len() as i32,
                        time_span_days: (time_range.end - time_range.start).num_days() as i32,
                        metadata: PatternMetadata::Temporal {
                            period: format!("weekly_{}", day_names[dow as usize]),
                            occurrences_at_period: occurrence_indices.len() as i32,
                            total_periods_observed: time_span_weeks.ceil() as i32,
                        },
                        detected_at: Utc::now(),
                    };
                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }
}

/// Time range for pattern detection
#[derive(Debug, Clone)]
pub struct DetectionTimeRange {
    pub start: chrono::DateTime<Utc>,
    pub end: chrono::DateTime<Utc>,
}

impl DetectionTimeRange {
    /// Create a new time range
    pub fn new(start: chrono::DateTime<Utc>, end: chrono::DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Last N days from now
    pub fn last_n_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::days(days);
        Self { start, end }
    }

    /// Last N weeks from now
    pub fn last_n_weeks(weeks: i64) -> Self {
        Self::last_n_days(weeks * 7)
    }
}

/// Scheduled task runner for daily pattern detection
pub struct PatternDetectionScheduler {
    detector: PatternDetector,
}

impl PatternDetectionScheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            detector: PatternDetector::new(),
        }
    }

    /// Run pattern detection for all users
    pub fn run_daily_detection(
        &self,
        conn: &mut PgConnection,
        user_ids: &[String],
    ) -> Result<HashMap<String, PatternDetectionResult>> {
        let mut results = HashMap::new();

        // Analyze last 30 days for patterns
        let time_range = DetectionTimeRange::last_n_days(30);

        for user_id in user_ids {
            match self.detector.detect_patterns(conn, user_id, time_range.clone()) {
                Ok(result) => {
                    results.insert(user_id.clone(), result);
                }
                Err(e) => {
                    eprintln!("Failed to detect patterns for user {}: {}", user_id, e);
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_type_conversion() {
        let pt = PatternType::HighFrequency;
        let s: String = pt.into();
        assert_eq!(s, "high_frequency");

        let pt2: PatternType = s.into();
        assert_eq!(pt2, PatternType::HighFrequency);
    }

    #[test]
    fn test_detection_time_range_creation() {
        let range = DetectionTimeRange::last_n_days(7);
        let duration = (range.end - range.start).num_days();
        assert_eq!(duration, 7);
    }

    #[test]
    fn test_consistency_calculation() {
        let detector = PatternDetector::new();

        // Create mock events (timestamps)
        let now = Utc::now();
        let event1 = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "test".to_string(),
            timestamp: now,
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            extractor_version: None,
        };

        let event2 = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "test".to_string(),
            timestamp: now + Duration::days(1),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            extractor_version: None,
        };

        let event3 = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "test".to_string(),
            timestamp: now + Duration::days(2),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            extractor_version: None,
        };

        let events = vec![&event1, &event2, &event3];
        let consistency = detector.calculate_consistency(&events, 3.0);

        // Daily pattern should have high consistency
        assert!(consistency > 0.5);
    }

    #[test]
    fn test_pattern_detector_config_default() {
        let config = PatternDetectorConfig::default();
        assert_eq!(config.min_frequency_threshold, 0.5);
        assert_eq!(config.min_confidence, 0.6);
        assert_eq!(config.min_trend_days, 7);
        assert_eq!(config.min_anomaly_deviation, 0.5);
        assert_eq!(config.anomaly_baseline_days, 30);
    }

    #[test]
    fn test_duration_calculation() {
        let detector = PatternDetector::new();

        let now = Utc::now();
        let event1 = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "test".to_string(),
            timestamp: now,
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            extractor_version: None,
        };

        let event2 = EventMemory {
            event_id: Uuid::new_v4(),
            memory_id: Uuid::new_v4(),
            user_id: "test".to_string(),
            timestamp: now + Duration::days(5),
            actor: None,
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            extractor_version: None,
        };

        let events = vec![&event1, &event2];
        let duration = detector.calculate_duration(&events);

        assert_eq!(duration, 5.0);
    }

    #[test]
    fn test_scheduler_creation() {
        let scheduler = PatternDetectionScheduler::new();
        // Should not panic
        assert_eq!(scheduler.detector.config.min_frequency_threshold, 0.5);
    }
}
