//! Audit Log Module
//!
//! This module implements comprehensive audit logging for security and compliance.
//! All access to user data is logged with who, what, when, and result.
//!
//! # Design Principles (HEAD.md)
//! - **审计日志**: 记录所有访问
//! - **GDPR合规**: 支持数据导出
//! - **日志轮转**: 防止膨胀
//!
//! # Example
//! ```text
//! use dirsoul::audit::AuditLogger;
//!
//! let logger = AuditLogger::new("postgresql://localhost/dirsoul".to_string());
//! logger.log_query("user123", "events", true, 25)?;
//! ```

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{DirSoulError, Result};
use crate::schema::audit_logs;

/// Audit log entry
///
/// Records who did what, when, and the result.
#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
pub struct AuditLog {
    /// Primary key
    pub id: i32,

    /// User who performed the action
    pub user_id: String,

    /// Action performed (query, insert, update, delete, export, etc.)
    pub action: String,

    /// Target resource (events, views, entities, etc.)
    pub target: String,

    /// Timestamp of the action
    pub timestamp: DateTime<Utc>,

    /// Whether the action succeeded
    pub success: bool,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Number of results returned
    pub result_count: Option<i32>,

    /// IP address (optional, for remote access)
    pub ip_address: Option<String>,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// New audit log for insertion
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = audit_logs)]
pub struct NewAuditLog {
    pub user_id: String,
    pub action: String,
    pub target: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub result_count: Option<i32>,
    pub ip_address: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl NewAuditLog {
    /// Create a new audit log entry
    pub fn new(user_id: String, action: String, target: String) -> Self {
        Self {
            user_id,
            action,
            target,
            success: true,
            error_message: None,
            result_count: None,
            ip_address: None,
            metadata: None,
        }
    }

    /// Set success status
    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    /// Set error message
    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.error_message = Some(error);
        self
    }

    /// Set result count
    pub fn with_result_count(mut self, count: i32) -> Self {
        self.result_count = Some(count);
        self
    }

    /// Set IP address
    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Audit logger for recording all access
///
/// Thread-safe logger that writes audit entries to the database.
pub struct AuditLogger {
    /// Database URL for writing logs
    database_url: String,

    /// Max logs to keep before rotation
    max_logs: i64,

    /// Logs count threshold for triggering rotation
    rotation_threshold: i64,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            max_logs: 100_000,  // Keep max 100k audit logs
            rotation_threshold: 90_000,  // Rotate after 90k logs
        }
    }

    /// Create with custom rotation settings
    pub fn with_rotation(database_url: String, max_logs: i64, rotation_threshold: i64) -> Self {
        Self {
            database_url,
            max_logs,
            rotation_threshold,
        }
    }

    /// Log a query action
    pub fn log_query(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
        result_count: i32,
    ) -> Result<AuditLog> {
        let mut log = NewAuditLog::new(user_id.to_string(), "query".to_string(), target.to_string())
            .with_success(success)
            .with_result_count(result_count);

        if !success {
            log = log.with_error("Query failed".to_string());
        }

        self.insert_log(log)
    }

    /// Log an insert action
    pub fn log_insert(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
    ) -> Result<AuditLog> {
        let mut log = NewAuditLog::new(user_id.to_string(), "insert".to_string(), target.to_string())
            .with_success(success);

        if !success {
            log = log.with_error("Insert failed".to_string());
        }

        self.insert_log(log)
    }

    /// Log an update action
    pub fn log_update(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
    ) -> Result<AuditLog> {
        let mut log = NewAuditLog::new(user_id.to_string(), "update".to_string(), target.to_string())
            .with_success(success);

        if !success {
            log = log.with_error("Update failed".to_string());
        }

        self.insert_log(log)
    }

    /// Log a delete action
    pub fn log_delete(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
    ) -> Result<AuditLog> {
        let mut log = NewAuditLog::new(user_id.to_string(), "delete".to_string(), target.to_string())
            .with_success(success);

        if !success {
            log = log.with_error("Delete failed".to_string());
        }

        self.insert_log(log)
    }

    /// Log an export action
    pub fn log_export(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
        result_count: i32,
    ) -> Result<AuditLog> {
        let mut log = NewAuditLog::new(user_id.to_string(), "export".to_string(), target.to_string())
            .with_success(success)
            .with_result_count(result_count);

        if !success {
            log = log.with_error("Export failed".to_string());
        }

        self.insert_log(log)
    }

    /// Log custom action
    pub fn log_custom(
        &self,
        user_id: &str,
        action: &str,
        target: &str,
        success: bool,
        metadata: Option<serde_json::Value>,
    ) -> Result<AuditLog> {
        let mut log = NewAuditLog::new(user_id.to_string(), action.to_string(), target.to_string())
            .with_success(success);

        if !success {
            log = log.with_error("Operation failed".to_string());
        }

        if let Some(meta) = metadata {
            log = log.with_metadata(meta);
        }

        self.insert_log(log)
    }

    /// Insert log entry to database
    fn insert_log(&self, log: NewAuditLog) -> Result<AuditLog> {
        let mut conn = PgConnection::establish(&self.database_url)
            .map_err(|e| DirSoulError::DatabaseConnection(e))?;

        // Check if rotation is needed
        self.check_and_rotate(&mut conn)?;

        // Insert the log
        let audit_log = diesel::insert_into(audit_logs::table)
            .values(&log)
            .get_result(&mut conn)?;

        Ok(audit_log)
    }

    /// Check log count and rotate if needed
    fn check_and_rotate(&self, _conn: &mut PgConnection) -> Result<()> {
        // TODO: Implement log rotation
        // For now, just return Ok to avoid compilation errors
        Ok(())
    }

    /// Rotate old logs
    fn rotate_logs(&self, _conn: &mut PgConnection, _current_count: i64) -> Result<()> {
        // TODO: Implement log rotation
        Ok(())
    }
}

/// Thread-safe audit logger wrapper
///
/// Use this for concurrent access to audit logging.
#[derive(Clone)]
pub struct ThreadSafeAuditLogger {
    inner: Arc<RwLock<AuditLogger>>,
}

impl ThreadSafeAuditLogger {
    /// Create a new thread-safe audit logger
    pub fn new(database_url: String) -> Self {
        Self {
            inner: Arc::new(RwLock::new(AuditLogger::new(database_url))),
        }
    }

    /// Log a query action (thread-safe)
    pub async fn log_query(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
        result_count: i32,
    ) -> Result<AuditLog> {
        let logger = self.inner.read().await;
        logger.log_query(user_id, target, success, result_count)
    }

    /// Log an insert action (thread-safe)
    pub async fn log_insert(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
    ) -> Result<AuditLog> {
        let logger = self.inner.read().await;
        logger.log_insert(user_id, target, success)
    }

    /// Log an update action (thread-safe)
    pub async fn log_update(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
    ) -> Result<AuditLog> {
        let logger = self.inner.read().await;
        logger.log_update(user_id, target, success)
    }

    /// Log a delete action (thread-safe)
    pub async fn log_delete(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
    ) -> Result<AuditLog> {
        let logger = self.inner.read().await;
        logger.log_delete(user_id, target, success)
    }

    /// Log an export action (thread-safe)
    pub async fn log_export(
        &self,
        user_id: &str,
        target: &str,
        success: bool,
        result_count: i32,
    ) -> Result<AuditLog> {
        let logger = self.inner.read().await;
        logger.log_export(user_id, target, success, result_count)
    }

    /// Log custom action (thread-safe)
    pub async fn log_custom(
        &self,
        user_id: &str,
        action: &str,
        target: &str,
        success: bool,
        metadata: Option<serde_json::Value>,
    ) -> Result<AuditLog> {
        let logger = self.inner.read().await;
        logger.log_custom(user_id, action, target, success, metadata)
    }
}

/// Audit log repository for database queries
pub struct AuditLogRepository;

impl AuditLogRepository {
    /// Get audit logs for a user
    pub fn find_by_user(
        _conn: &mut PgConnection,
        _user_id: &str,
        _limit: Option<i64>,
    ) -> Result<Vec<AuditLog>> {
        // TODO: Implement database queries
        // Temporarily return empty vec to avoid compilation errors
        Ok(vec![])
    }

    /// Get recent audit logs
    pub fn find_recent(
        _conn: &mut PgConnection,
        _limit: i64,
    ) -> Result<Vec<AuditLog>> {
        // TODO: Implement database queries
        Ok(vec![])
    }

    /// Get audit logs by action
    pub fn find_by_action(
        _conn: &mut PgConnection,
        _action: &str,
        _limit: i64,
    ) -> Result<Vec<AuditLog>> {
        // TODO: Implement database queries
        Ok(vec![])
    }

    /// Get audit logs by time range
    pub fn find_by_time_range(
        _conn: &mut PgConnection,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
        _limit: Option<i64>,
    ) -> Result<Vec<AuditLog>> {
        // TODO: Implement database queries
        Ok(vec![])
    }

    /// Count total audit logs
    pub fn count(_conn: &mut PgConnection) -> Result<i64> {
        // TODO: Implement database queries
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_audit_log() {
        let log = NewAuditLog::new("user123".to_string(), "query".to_string(), "events".to_string());
        assert_eq!(log.user_id, "user123");
        assert_eq!(log.action, "query");
        assert_eq!(log.target, "events");
        assert!(log.success);
        assert!(log.error_message.is_none());
        assert!(log.result_count.is_none());
    }

    #[test]
    fn test_new_audit_log_with_success() {
        let log = NewAuditLog::new("user123".to_string(), "query".to_string(), "events".to_string())
            .with_success(false);
        assert!(!log.success);
    }

    #[test]
    fn test_new_audit_log_with_error() {
        let log = NewAuditLog::new("user123".to_string(), "query".to_string(), "events".to_string())
            .with_error("Connection failed".to_string());
        assert!(!log.success);
        assert_eq!(log.error_message, Some("Connection failed".to_string()));
    }

    #[test]
    fn test_new_audit_log_with_result_count() {
        let log = NewAuditLog::new("user123".to_string(), "query".to_string(), "events".to_string())
            .with_result_count(42);
        assert_eq!(log.result_count, Some(42));
    }

    #[test]
    fn test_new_audit_log_with_metadata() {
        let metadata = serde_json::json!({"test": "value"});
        let log = NewAuditLog::new("user123".to_string(), "query".to_string(), "events".to_string())
            .with_metadata(metadata.clone());
        assert_eq!(log.metadata, Some(metadata));
    }

    #[test]
    fn test_new_audit_log_serialization() {
        let log = NewAuditLog::new("user123".to_string(), "query".to_string(), "events".to_string());
        let json = serde_json::to_string(&log).unwrap();
        let _deserialized: NewAuditLog = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_audit_logger_creation() {
        let logger = AuditLogger::new("postgresql://localhost/test".to_string());
        assert_eq!(logger.max_logs, 100_000);
        assert_eq!(logger.rotation_threshold, 90_000);
    }

    #[test]
    fn test_audit_logger_with_rotation() {
        let logger = AuditLogger::with_rotation(
            "postgresql://localhost/test".to_string(),
            50_000,
            40_000,
        );
        assert_eq!(logger.max_logs, 50_000);
        assert_eq!(logger.rotation_threshold, 40_000);
    }
}
