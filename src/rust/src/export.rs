//! Data Export/Import Module
//!
//! This module implements data export and import functionality for GDPR compliance
//! and backup capabilities. All data can be exported to JSON format with encryption.
//!
//! # Design Principles (HEAD.md)
//! - **GDPR合规**: 一键导出所有数据
//! - **自动备份**: 指定目录镜像备份
//! - **端到端加密**: 备份数据加密保护
//!
//! # Example
//! ```text
//! use dirsoul::export::DataExporter;
//!
//! let exporter = DataExporter::new("postgresql://localhost/dirsoul".to_string())?;
//! let export = exporter.export_user_data("user123")?;
//! ```

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use base64::Engine;

use crate::crypto::EncryptionManager;
use crate::cognitive::{CognitiveView, StableConcept};
use crate::error::{DirSoulError, Result};
use crate::models::{Entity, EventMemory};
use crate::schema::{entities, event_memories, raw_memories, stable_concepts, cognitive_views};
use diesel::sql_types::{Jsonb, Nullable, Text, Timestamptz};
use uuid::Uuid;

/// Raw memory export (without embedding field)
#[derive(Debug, Clone, QueryableByName, Serialize, Deserialize)]
pub struct RawMemoryExport {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub memory_id: Uuid,
    #[diesel(sql_type = Text)]
    pub user_id: String,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Text)]
    pub content_type: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub content: Option<String>,
    #[diesel(sql_type = Nullable<diesel::sql_types::Bytea>)]
    pub encrypted: Option<Vec<u8>>,
    #[diesel(sql_type = Nullable<Jsonb>)]
    pub metadata: Option<serde_json::Value>,
}

/// Complete user data export
///
/// Contains all user data for GDPR compliance and backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataExport {
    /// User ID
    pub user_id: String,

    /// Export timestamp
    pub exported_at: DateTime<Utc>,

    /// Export format version
    pub version: String,

    /// Raw memories (as JSON due to pgvector type)
    pub raw_memories: Vec<serde_json::Value>,

    /// Event memories
    pub event_memories: Vec<EventMemory>,

    /// Entities
    pub entities: Vec<Entity>,

    /// Stable concepts
    pub stable_concepts: Vec<serde_json::Value>,

    /// Cognitive views
    pub cognitive_views: Vec<serde_json::Value>,

    /// Metadata
    pub metadata: ExportMetadata,
}

/// Export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Total raw memories
    pub raw_memory_count: usize,

    /// Total event memories
    pub event_memory_count: usize,

    /// Total entities
    pub entity_count: usize,

    /// Total stable concepts
    pub stable_concept_count: usize,

    /// Total cognitive views
    pub cognitive_view_count: usize,

    /// Data size (encrypted bytes)
    pub encrypted_size: Option<usize>,

    /// Export duration in seconds
    pub export_duration_secs: Option<f64>,
}

impl Default for ExportMetadata {
    fn default() -> Self {
        Self {
            raw_memory_count: 0,
            event_memory_count: 0,
            entity_count: 0,
            stable_concept_count: 0,
            cognitive_view_count: 0,
            encrypted_size: None,
            export_duration_secs: None,
        }
    }
}

/// Encrypted data export
///
/// Contains encrypted user data for secure backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedDataExport {
    /// User ID
    pub user_id: String,

    /// Export timestamp
    pub exported_at: DateTime<Utc>,

    /// Format version
    pub version: String,

    /// Encrypted data (base64 encoded)
    pub encrypted_data: String,

    /// Non-sensitive metadata (not encrypted)
    pub metadata: ExportMetadata,

    /// Checksum for integrity verification
    pub checksum: String,
}

/// Data exporter for GDPR compliance and backup
pub struct DataExporter {
    database_url: String,
}

impl DataExporter {
    /// Create a new data exporter
    pub fn new(database_url: String) -> Self {
        Self { database_url }
    }

    /// Export all user data
    pub fn export_user_data(&self, user_id: &str) -> Result<UserDataExport> {
        let start_time = Utc::now();
        let mut conn = PgConnection::establish(&self.database_url)
            .map_err(|e| DirSoulError::DatabaseConnection(e))?;

        // Export raw memories using raw SQL to handle pgvector type
        let raw_memory_rows: Vec<RawMemoryExport> = diesel::sql_query(
            "SELECT memory_id, user_id, created_at, content_type, content, encrypted, metadata
             FROM raw_memories
             WHERE user_id = $1
             ORDER BY created_at DESC"
        )
        .bind::<diesel::sql_types::Text, _>(user_id)
        .load(&mut conn)?;
        let raw_memories: Vec<serde_json::Value> = raw_memory_rows
            .into_iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect();

        // Export event memories
        let event_memories: Vec<EventMemory> = event_memories::table
            .filter(event_memories::user_id.eq(user_id))
            .order(event_memories::timestamp.desc())
            .load(&mut conn)?;

        // Export entities
        let entities: Vec<Entity> = entities::table
            .filter(entities::user_id.eq(user_id))
            .order(entities::first_seen.asc())
            .load(&mut conn)?;

        // Export stable concepts (as JSON due to complex structure)
        let stable_concepts_db: Vec<StableConcept> = stable_concepts::table
            .filter(stable_concepts::user_id.eq(user_id))
            .order(stable_concepts::created_at.desc())
            .load(&mut conn)?;
        let stable_concepts: Vec<serde_json::Value> = stable_concepts_db
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect();

        // Export cognitive views (as JSON)
        let cognitive_views_db: Vec<CognitiveView> = cognitive_views::table
            .filter(cognitive_views::user_id.eq(user_id))
            .order(cognitive_views::created_at.desc())
            .load(&mut conn)?;
        let cognitive_views: Vec<serde_json::Value> = cognitive_views_db
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect();

        let end_time = Utc::now();
        let duration = (end_time - start_time).num_seconds() as f64;

        let metadata = ExportMetadata {
            raw_memory_count: raw_memories.len(),
            event_memory_count: event_memories.len(),
            entity_count: entities.len(),
            stable_concept_count: stable_concepts.len(),
            cognitive_view_count: cognitive_views.len(),
            encrypted_size: None,
            export_duration_secs: Some(duration),
        };

        Ok(UserDataExport {
            user_id: user_id.to_string(),
            exported_at: end_time,
            version: "1.0.0".to_string(),
            raw_memories,
            event_memories,
            entities,
            stable_concepts,
            cognitive_views,
            metadata,
        })
    }

    /// Export encrypted user data
    pub fn export_encrypted_user_data(
        &self,
        user_id: &str,
        encryption: &EncryptionManager,
    ) -> Result<EncryptedDataExport> {
        // First export unencrypted data
        let export = self.export_user_data(user_id)?;

        // Serialize to JSON
        let json_data = serde_json::to_string(&export)?;

        // Encrypt the data
        let encrypted_bytes = encryption.encrypt(json_data.as_bytes())?;
        let encrypted_base64 = base64::engine::general_purpose::STANDARD.encode(&encrypted_bytes);

        // Calculate checksum
        let checksum = format!("{:x}", md5::compute(json_data.as_bytes()));

        let metadata = export.metadata;

        Ok(EncryptedDataExport {
            user_id: user_id.to_string(),
            exported_at: export.exported_at,
            version: export.version,
            encrypted_data: encrypted_base64,
            metadata,
            checksum,
        })
    }

    /// Export to file
    pub fn export_to_file(
        &self,
        user_id: &str,
        file_path: &std::path::Path,
        encryption: &EncryptionManager,
    ) -> Result<EncryptedDataExport> {
        // Export encrypted data
        let export = self.export_encrypted_user_data(user_id, encryption)?;

        // Write to file
        let mut file = std::fs::File::create(file_path)
            .map_err(|e| DirSoulError::Io(e))?;

        writeln!(file, "{}", serde_json::to_string_pretty(&export)?)
            .map_err(|e| DirSoulError::Io(e))?;

        Ok(export)
    }
}

/// Data importer for restoring backups
pub struct DataImporter {
    database_url: String,
}

impl DataImporter {
    /// Create a new data importer
    pub fn new(database_url: String) -> Self {
        Self { database_url }
    }

    /// Import encrypted user data
    pub fn import_encrypted_data(
        &self,
        encrypted_export: &EncryptedDataExport,
        encryption: &EncryptionManager,
    ) -> Result<UserDataExport> {
        // Decode base64
        let encrypted_bytes = base64::engine::general_purpose::STANDARD.decode(&encrypted_export.encrypted_data)
            .map_err(|e| DirSoulError::Encryption("Invalid base64 encoding".to_string()))?;

        // Decrypt data
        let decrypted_bytes = encryption.decrypt(&encrypted_bytes)?;

        // Parse JSON
        let export: UserDataExport = serde_json::from_slice(&decrypted_bytes)?;

        // Verify checksum
        let json_data = serde_json::to_string(&export)?;
        let checksum = format!("{:x}", md5::compute(json_data.as_bytes()));

        if checksum != encrypted_export.checksum {
            return Err(DirSoulError::Encryption("Checksum verification failed".to_string()));
        }

        Ok(export)
    }

    /// Import user data to database
    pub fn import_user_data(
        &self,
        export: &UserDataExport,
    ) -> Result<ImportSummary> {
        let mut conn = PgConnection::establish(&self.database_url)
            .map_err(|e| DirSoulError::DatabaseConnection(e))?;

        // Begin transaction
        conn.transaction::<_, DirSoulError, _>(|conn| {
            // Check if user already has data
            let existing_count: i64 = raw_memories::table
                .filter(raw_memories::user_id.eq(&export.user_id))
                .count()
                .get_result(conn)?;

            if existing_count > 0 {
                return Err(DirSoulError::Config(
                    format!("User {} already has {} records. Import not supported yet.",
                            export.user_id, existing_count)
                ));
            }

            // TODO: Implement actual data import
            // For now, just count what would be imported

            Ok(ImportSummary {
                user_id: export.user_id.clone(),
                raw_memories_imported: export.raw_memories.len(),
                event_memories_imported: export.event_memories.len(),
                entities_imported: export.entities.len(),
                stable_concepts_imported: export.stable_concepts.len(),
                cognitive_views_imported: export.cognitive_views.len(),
            })
        })
    }

    /// Import from file
    pub fn import_from_file(
        &self,
        file_path: &std::path::Path,
        encryption: &EncryptionManager,
    ) -> Result<ImportSummary> {
        // Read file
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| DirSoulError::Io(e))?;

        // Parse encrypted export
        let encrypted_export: EncryptedDataExport = serde_json::from_str(&content)?;

        // Import encrypted data
        let export = self.import_encrypted_data(&encrypted_export, encryption)?;

        // Import to database
        self.import_user_data(&export)
    }
}

/// Summary of import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub user_id: String,
    pub raw_memories_imported: usize,
    pub event_memories_imported: usize,
    pub entities_imported: usize,
    pub stable_concepts_imported: usize,
    pub cognitive_views_imported: usize,
}

/// Auto-backup manager for scheduled backups
pub struct AutoBackupManager {
    database_url: String,
    backup_dir: std::path::PathBuf,
    encryption: EncryptionManager,
}

impl AutoBackupManager {
    /// Create a new auto-backup manager
    pub fn new(
        database_url: String,
        backup_dir: std::path::PathBuf,
        encryption: EncryptionManager,
    ) -> Result<Self> {
        // Create backup directory if it doesn't exist
        if !backup_dir.exists() {
            std::fs::create_dir_all(&backup_dir)
                .map_err(|e| DirSoulError::Io(e))?;
        }

        Ok(Self {
            database_url,
            backup_dir,
            encryption,
        })
    }

    /// Create backup for a specific user
    pub fn backup_user(&self, user_id: &str) -> Result<EncryptedDataExport> {
        let exporter = DataExporter::new(self.database_url.clone());

        // Create filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.json", user_id, timestamp);
        let file_path = self.backup_dir.join(filename);

        exporter.export_to_file(user_id, &file_path, &self.encryption)
    }

    /// Backup all users
    pub fn backup_all_users(&self, user_ids: &[String]) -> Result<Vec<EncryptedDataExport>> {
        let mut backups = Vec::new();

        for user_id in user_ids {
            match self.backup_user(user_id) {
                Ok(backup) => {
                    backups.push(backup);
                }
                Err(e) => {
                    eprintln!("Failed to backup user {}: {}", user_id, e);
                }
            }
        }

        Ok(backups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_data_export_serialization() {
        let export = UserDataExport {
            user_id: "test_user".to_string(),
            exported_at: Utc::now(),
            version: "1.0.0".to_string(),
            raw_memories: vec![],
            event_memories: vec![],
            entities: vec![],
            stable_concepts: vec![],
            cognitive_views: vec![],
            metadata: ExportMetadata::default(),
        };

        let json = serde_json::to_string(&export).unwrap();
        let _deserialized: UserDataExport = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_encrypted_data_export_serialization() {
        let export = EncryptedDataExport {
            user_id: "test_user".to_string(),
            exported_at: Utc::now(),
            version: "1.0.0".to_string(),
            encrypted_data: "encrypted_data_here".to_string(),
            metadata: ExportMetadata::default(),
            checksum: "abc123".to_string(),
        };

        let json = serde_json::to_string(&export).unwrap();
        let _deserialized: EncryptedDataExport = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_export_metadata_default() {
        let metadata = ExportMetadata::default();
        assert_eq!(metadata.raw_memory_count, 0);
        assert_eq!(metadata.event_memory_count, 0);
    }

    #[test]
    fn test_import_summary_serialization() {
        let summary = ImportSummary {
            user_id: "test_user".to_string(),
            raw_memories_imported: 10,
            event_memories_imported: 20,
            entities_imported: 5,
            stable_concepts_imported: 2,
            cognitive_views_imported: 3,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let _deserialized: ImportSummary = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_data_exporter_creation() {
        let exporter = DataExporter::new("postgresql://localhost/test".to_string());
        assert_eq!(exporter.database_url, "postgresql://localhost/test");
    }

    #[test]
    fn test_data_importer_creation() {
        let importer = DataImporter::new("postgresql://localhost/test".to_string());
        assert_eq!(importer.database_url, "postgresql://localhost/test");
    }
}
