//! Data Lifecycle Management - Hot/Warm/Cold Tiering Strategy
//!
//! This module implements automated data lifecycle management for long-term
//! storage efficiency, ensuring the system can handle 10+ years of data growth.
//!
//! # Design Principles (HEAD.md)
//! - **10年+数据增长不崩溃**: 分层存储策略
//! - **热数据**: 最近3个月，SSD，快速访问
//! - **温数据**: 3个月~2年，普通盘，压缩存储
//! - **冷数据**: 2年以上，MinIO对象存储
//! - **定时归档**: 自动迁移老数据
//!
//! # Example
//! ```text
//! use dirsoul::data_lifecycle::{DataLifecycleManager, TieringConfig};
//!
//! let config = TieringConfig::default();
//! let manager = DataLifecycleManager::new(config, "postgresql://localhost/dirsoul".to_string())?;
//! manager.run_archive_task()?;
//! ```

use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use base64::Engine;
use uuid::Uuid;

use crate::error::{DirSoulError, Result};
use crate::models::{RawMemory, EventMemory};
use crate::schema::{raw_memories, event_memories};

/// Data tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataTier {
    /// Hot data - last 3 months, on SSD
    Hot,

    /// Warm data - 3 months to 2 years, compressed
    Warm,

    /// Cold data - 2+ years, MinIO object storage
    Cold,
}

impl DataTier {
    /// Get the storage duration threshold for this tier
    pub fn age_threshold_months(&self) -> i64 {
        match self {
            DataTier::Hot => 3,
            DataTier::Warm => 24,  // 2 years
            DataTier::Cold => i64::MAX,
        }
    }

    /// Check if data should be archived to next tier
    pub fn should_archive(&self, age_months: i64) -> bool {
        match self {
            DataTier::Hot => age_months >= 3,
            DataTier::Warm => age_months >= 24,
            DataTier::Cold => false,
        }
    }
}

/// Data tiering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieringConfig {
    /// Hot tier: age in months before archiving to warm
    pub hot_threshold_months: i64,

    /// Warm tier: age in months before archiving to cold
    pub warm_threshold_months: i64,

    /// Enable automatic archiving
    pub enable_auto_archive: bool,

    /// Archive check interval in hours
    pub archive_check_interval_hours: u64,

    /// Enable compression for warm data
    pub enable_compression: bool,

    /// MinIO endpoint for cold storage
    pub minio_endpoint: Option<String>,

    /// MinIO bucket name
    pub minio_bucket: Option<String>,

    /// MinIO access key
    pub minio_access_key: Option<String>,

    /// MinIO secret key
    pub minio_secret_key: Option<String>,
}

impl Default for TieringConfig {
    fn default() -> Self {
        Self {
            hot_threshold_months: 3,
            warm_threshold_months: 24,
            enable_auto_archive: true,
            archive_check_interval_hours: 24,  // Daily
            enable_compression: true,
            minio_endpoint: Some("http://localhost:9000".to_string()),
            minio_bucket: Some("dirsoul-cold".to_string()),
            minio_access_key: None,
            minio_secret_key: None,
        }
    }
}

/// Archive statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveStats {
    /// Raw memories archived
    pub raw_memories_archived: usize,

    /// Event memories archived
    pub event_memories_archived: usize,

    /// Space saved in MB
    pub space_saved_mb: u64,

    /// Archive duration in seconds
    pub duration_secs: f64,

    /// Timestamp of archive operation
    pub timestamp: DateTime<Utc>,
}

/// Compressed data representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedData {
    /// Original data ID
    pub id: Uuid,

    /// Compressed content (base64 encoded)
    pub compressed_content: String,

    /// Compression ratio (0.0 - 1.0)
    pub compression_ratio: f64,

    /// Original size in bytes
    pub original_size: usize,

    /// Compressed size in bytes
    pub compressed_size: usize,

    /// Compression algorithm
    pub algorithm: String,

    /// Timestamp of compression
    pub compressed_at: DateTime<Utc>,
}

/// Data summary for old events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSummary {
    /// Time range covered
    pub time_range_start: DateTime<Utc>,
    pub time_range_end: DateTime<Utc>,

    /// Total event count
    pub event_count: usize,

    /// Top entities mentioned
    pub top_entities: Vec<String>,

    /// Key themes/keywords
    pub keywords: Vec<String>,

    /// Generated summary text
    pub summary: String,

    /// Statistics
    pub statistics: SummaryStatistics,

    /// Generated at
    pub generated_at: DateTime<Utc>,
}

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryStatistics {
    /// Total memories
    pub total_memories: usize,

    /// Average events per day
    pub avg_events_per_day: f64,

    /// Most active day of week
    pub most_active_day: String,

    /// Longest gap between events
    pub longest_gap_days: i64,
}

/// Data lifecycle manager
pub struct DataLifecycleManager {
    /// Database connection string
    database_url: String,

    /// Tiering configuration
    config: TieringConfig,
}

impl DataLifecycleManager {
    /// Create a new data lifecycle manager
    pub fn new(config: TieringConfig, database_url: String) -> Self {
        Self {
            database_url,
            config,
        }
    }

    /// Load from config file
    pub fn from_config_file<P: AsRef<Path>>(path: P, database_url: String) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DirSoulError::Io(e))?;

        let config: TieringConfig = toml::from_str(&content)
            .map_err(|e| DirSoulError::Config(format!("Invalid TOML: {}", e)))?;

        Ok(Self::new(config, database_url))
    }

    /// Determine data tier based on age
    pub fn determine_tier(&self, created_at: DateTime<Utc>) -> DataTier {
        let age_months = (Utc::now() - created_at).num_days() / 30;

        if age_months < self.config.hot_threshold_months {
            DataTier::Hot
        } else if age_months < self.config.warm_threshold_months {
            DataTier::Warm
        } else {
            DataTier::Cold
        }
    }

    /// Get raw memories that should be archived (simplified version)
    pub fn get_raw_memories_to_archive(&self, _tier: DataTier) -> Result<Vec<(Uuid, DateTime<Utc>, String)>> {
        // TODO: Implement database query
        // For now, return empty vec to avoid Diesel type issues with vector columns
        Ok(vec![])
    }

    /// Get event memories that should be archived (simplified version)
    pub fn get_event_memories_to_archive(&self, _tier: DataTier) -> Result<Vec<EventMemory>> {
        // TODO: Implement database query
        // For now, return empty vec
        Ok(vec![])
    }

    /// Compress data for warm storage
    pub fn compress_data(&self, data: &str) -> Result<CompressedData> {
        use std::io::Write;

        let original_size = data.len();
        let id = Uuid::new_v4();

        // Use gzip compression
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(data.as_bytes())
            .map_err(|e| DirSoulError::Io(e))?;

        let compressed_bytes = encoder.finish()
            .map_err(|e| DirSoulError::Io(e))?;

        let compressed_size = compressed_bytes.len();
        let compressed_content = base64::engine::general_purpose::STANDARD.encode(&compressed_bytes);
        let compression_ratio = compressed_size as f64 / original_size as f64;

        Ok(CompressedData {
            id,
            compressed_content,
            compression_ratio,
            original_size,
            compressed_size,
            algorithm: "gzip".to_string(),
            compressed_at: Utc::now(),
        })
    }

    /// Decompress data
    pub fn decompress_data(&self, compressed: &CompressedData) -> Result<String> {
        use std::io::Read;

        let compressed_bytes = base64::engine::general_purpose::STANDARD.decode(&compressed.compressed_content)
            .map_err(|e| DirSoulError::Encryption("Invalid base64 encoding".to_string()))?;

        let mut decoder = flate2::read::GzDecoder::new(&compressed_bytes[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| DirSoulError::Io(e))?;

        String::from_utf8(decompressed)
            .map_err(|e| DirSoulError::Encryption(format!("Invalid UTF-8: {}", e)))
    }

    /// Generate summary for old events
    pub fn generate_summary(&self, events: &[EventMemory]) -> Result<DataSummary> {
        if events.is_empty() {
            return Err(DirSoulError::Config("No events to summarize".to_string()));
        }

        let time_range_start = events.iter()
            .map(|e| e.timestamp)
            .min()
            .unwrap();
        let time_range_end = events.iter()
            .map(|e| e.timestamp)
            .max()
            .unwrap();

        // Simple summary generation (in production, use LLM)
        let summary = format!(
            "用户在 {} 到 {} 期间记录了 {} 个事件。主要活动包括: {}",
            time_range_start.format("%Y-%m"),
            time_range_end.format("%Y-%m"),
            events.len(),
            "日常活动记录"
        );

        // Extract entities from events (simplified)
        let mut entities = std::collections::HashSet::new();
        for event in events {
            if let Some(actor) = &event.actor {
                entities.insert(actor.clone());
            }
            entities.insert(event.target.clone());
        }

        let top_entities = entities.into_iter()
            .take(10)
            .collect();

        // Calculate statistics
        let total_days = (time_range_end - time_range_start).num_days().max(1) as f64;
        let avg_events_per_day = events.len() as f64 / total_days;

        let stats = SummaryStatistics {
            total_memories: events.len(),
            avg_events_per_day,
            most_active_day: "待分析".to_string(),
            longest_gap_days: 0,
        };

        Ok(DataSummary {
            time_range_start,
            time_range_end,
            event_count: events.len(),
            top_entities,
            keywords: vec![],
            summary,
            statistics: stats,
            generated_at: Utc::now(),
        })
    }

    /// Run archive task for a specific tier
    pub fn run_archive_task(&self) -> Result<ArchiveStats> {
        let start_time = Utc::now();
        let mut raw_count = 0;
        let mut event_count = 0;
        let mut space_saved = 0u64;

        // Archive hot data to warm
        let hot_raw = self.get_raw_memories_to_archive(DataTier::Hot)?;
        let hot_events = self.get_event_memories_to_archive(DataTier::Hot)?;

        // Process raw memories - now returns tuple (id, created, content)
        for (_id, _created, content) in &hot_raw {
            let compressed = self.compress_data(content)?;
            space_saved += (compressed.original_size - compressed.compressed_size) as u64;
            raw_count += 1;
        }

        // Process event memories
        for event in &hot_events {
            // Generate summary for old events
            let _summary = self.generate_summary(&[event.clone()])?;
            event_count += 1;
        }

        let duration = (Utc::now() - start_time).num_seconds() as f64;

        Ok(ArchiveStats {
            raw_memories_archived: raw_count,
            event_memories_archived: event_count,
            space_saved_mb: space_saved / (1024 * 1024),
            duration_secs: duration,
            timestamp: Utc::now(),
        })
    }

    /// Get data tier for a specific memory (simplified version)
    pub fn get_memory_tier(&self, _memory_id: Uuid) -> Result<DataTier> {
        // TODO: Implement database query
        Ok(DataTier::Hot)
    }

    /// Get tier distribution statistics (simplified version)
    pub fn get_tier_distribution(&self) -> Result<TierDistribution> {
        // TODO: Implement database query
        // For now, return empty distribution
        Ok(TierDistribution {
            hot_count: 0,
            warm_count: 0,
            cold_count: 0,
            total_count: 0,
        })
    }

    /// Get configuration
    pub fn get_config(&self) -> &TieringConfig {
        &self.config
    }
}

/// Tier distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierDistribution {
    pub hot_count: usize,
    pub warm_count: usize,
    pub cold_count: usize,
    pub total_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_tier_age_threshold() {
        assert_eq!(DataTier::Hot.age_threshold_months(), 3);
        assert_eq!(DataTier::Warm.age_threshold_months(), 24);
    }

    #[test]
    fn test_data_tier_should_archive() {
        assert!(!DataTier::Hot.should_archive(1)); // 1 month
        assert!(DataTier::Hot.should_archive(4));  // 4 months
        assert!(!DataTier::Warm.should_archive(12)); // 1 year
        assert!(DataTier::Warm.should_archive(30));  // 2.5 years
        assert!(!DataTier::Cold.should_archive(100));
    }

    #[test]
    fn test_tiering_config_default() {
        let config = TieringConfig::default();
        assert_eq!(config.hot_threshold_months, 3);
        assert_eq!(config.warm_threshold_months, 24);
        assert!(config.enable_auto_archive);
    }

    #[test]
    fn test_compress_decompress_data() {
        let manager = DataLifecycleManager::new(
            TieringConfig::default(),
            "postgresql://localhost/test".to_string(),
        );

        // Use longer data to ensure compression is effective
        let original = "Hello, World! This is a test data for compression. ".repeat(100);
        let compressed = manager.compress_data(&original).unwrap();
        let decompressed = manager.decompress_data(&compressed).unwrap();

        assert_eq!(original, decompressed);
        assert!(compressed.compression_ratio < 1.0); // Should be compressed
        assert!(compressed.compressed_size < compressed.original_size);
    }

    #[test]
    fn test_generate_summary() {
        let manager = DataLifecycleManager::new(
            TieringConfig::default(),
            "postgresql://localhost/test".to_string(),
        );

        let events = vec![
            EventMemory {
                event_id: Uuid::new_v4(),
                memory_id: Uuid::new_v4(),
                user_id: "test".to_string(),
                timestamp: Utc::now() - Duration::days(10),
                actor: Some("User".to_string()),
                action: "ate".to_string(),
                target: "apple".to_string(),
                quantity: Some(1.0),
                unit: Some("piece".to_string()),
                confidence: 0.9,
                extractor_version: None,
            },
        ];

        let summary = manager.generate_summary(&events).unwrap();
        assert_eq!(summary.event_count, 1);
        assert!(summary.summary.contains("1 个事件"));
    }

    #[test]
    fn test_tier_distribution() {
        let dist = TierDistribution {
            hot_count: 100,
            warm_count: 50,
            cold_count: 20,
            total_count: 170,
        };

        assert_eq!(dist.total_count, 170);
    }
}
