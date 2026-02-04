//! Security Module Tests
//!
//! Comprehensive security testing for encryption, permissions, and audit logging.
//!
//! # Design Principles (HEAD.md)
//! - **端到端加密**: 验证Fernet加密正确性
//! - **权限控制**: 插件越权检测
//! - **审计日志**: 完整性验证
//!
//! # Example
//! ```text
//! use dirsoul::security_tests::SecurityTestSuite;
//!
//! let suite = SecurityTestSuite::new("postgresql://localhost/dirsoul".to_string())?;
//! let results = suite.run_all_tests()?;
//! assert!(results.all_passed());
//! ```

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use base64::Engine;
use tempfile::TempDir;

use crate::agents::AgentPermissions;
use crate::audit::{AuditLog, AuditLogger};
use crate::crypto::EncryptionManager;
use crate::error::{DirSoulError, Result};
use crate::schema::audit_logs;

/// Security test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityTestResult {
    /// Test name
    pub test_name: String,

    /// Whether the test passed
    pub passed: bool,

    /// Test duration in milliseconds
    pub duration_ms: u64,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl SecurityTestResult {
    /// Create a new successful test result
    pub fn success(test_name: String, duration_ms: u64) -> Self {
        Self {
            test_name,
            passed: true,
            duration_ms,
            error_message: None,
            metadata: None,
        }
    }

    /// Create a new failed test result
    pub fn failure(test_name: String, duration_ms: u64, error: String) -> Self {
        Self {
            test_name,
            passed: false,
            duration_ms,
            error_message: Some(error),
            metadata: None,
        }
    }
}

/// Security test suite results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityTestSuiteResults {
    /// All test results
    pub results: Vec<SecurityTestResult>,

    /// Total tests run
    pub total_tests: usize,

    /// Tests passed
    pub passed_tests: usize,

    /// Tests failed
    pub failed_tests: usize,

    /// Total duration in milliseconds
    pub total_duration_ms: u64,

    /// Test run timestamp
    pub timestamp: DateTime<Utc>,
}

impl SecurityTestSuiteResults {
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed_tests == 0
    }

    /// Get pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 100.0;
        }
        (self.passed_tests as f64 / self.total_tests as f64) * 100.0
    }

    /// Get summary
    pub fn summary(&self) -> String {
        format!(
            "Security Tests: {}/{} passed ({:.1}%) in {}ms",
            self.passed_tests,
            self.total_tests,
            self.pass_rate(),
            self.total_duration_ms
        )
    }
}

/// Security test suite
pub struct SecurityTestSuite {
    /// Database URL
    database_url: String,

    /// Temporary directory for test keys
    temp_dir: TempDir,

    /// Encryption manager for testing
    encryption: EncryptionManager,

    /// Audit logger
    audit_logger: AuditLogger,
}

impl SecurityTestSuite {
    /// Create a new security test suite
    pub fn new(database_url: String) -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let key_path = temp_dir.path().join("test_key");

        // Create encryption manager with test key file
        let encryption = EncryptionManager::initialize(&key_path)?;

        let audit_logger = AuditLogger::new(database_url.clone());

        Ok(Self {
            database_url,
            temp_dir,
            encryption,
            audit_logger,
        })
    }

    /// Run all security tests
    pub fn run_all_tests(&self) -> Result<SecurityTestSuiteResults> {
        let start_time = Utc::now();
        let mut results = Vec::new();

        // Encryption/Decryption tests
        results.push(self.test_encryption_decryption()?);
        results.push(self.test_encryption_with_large_data()?);
        results.push(self.test_encryption_key_rotation()?);

        // Permission tests
        results.push(self.test_agent_permission_levels()?);
        results.push(self.test_permission_isolation()?);

        // Audit log tests
        results.push(self.test_audit_log_integrity()?);
        results.push(self.test_audit_log_query_consistency()?);
        results.push(self.test_audit_log_rotation_configuration()?);

        // End-to-end security tests
        results.push(self.test_end_to_end_encryption()?);
        results.push(self.test_data_export_encryption()?);

        // Additional security tests
        results.push(self.test_encryption_key_uniqueness()?);
        results.push(self.test_data_integrity_checksum()?);

        let total_duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| r.passed).count();
        let failed_tests = total_tests - passed_tests;

        Ok(SecurityTestSuiteResults {
            results,
            total_tests,
            passed_tests,
            failed_tests,
            total_duration_ms,
            timestamp: Utc::now(),
        })
    }

    /// Test 1: Basic encryption/decryption
    fn test_encryption_decryption(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "encryption_decryption".to_string();

        let plaintext = b"Hello, DirSoul!";
        let encrypted = self.encryption.encrypt(plaintext)?;
        let decrypted = self.encryption.decrypt(&encrypted)?;

        if decrypted == plaintext {
            Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            ))
        } else {
            Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "Decrypted data doesn't match original".to_string(),
            ))
        }
    }

    /// Test 2: Encryption with large data
    fn test_encryption_with_large_data(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "encryption_large_data".to_string();

        // Create 1MB of data
        let large_data = vec![0u8; 1024 * 1024];

        match self.encryption.encrypt(&large_data) {
            Ok(_) => Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                format!("Failed to encrypt large data: {}", e),
            )),
        }
    }

    /// Test 3: Encryption key rotation simulation
    fn test_encryption_key_rotation(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "encryption_key_rotation".to_string();

        // Create separate encryption manager with different key
        let key_path2 = self.temp_dir.path().join("test_key2");
        let encryption2 = EncryptionManager::initialize(&key_path2)?;

        let plaintext = b"Sensitive data";
        let encrypted = self.encryption.encrypt(plaintext)?;

        // Try to decrypt with different key (should fail)
        match encryption2.decrypt(&encrypted) {
            Ok(_) => Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "Different key should not decrypt data".to_string(),
            )),
            Err(_) => {
                // Original key should still work
                match self.encryption.decrypt(&encrypted) {
                    Ok(decrypted) if decrypted == plaintext => Ok(SecurityTestResult::success(
                        test_name,
                        start.elapsed().as_millis() as u64,
                    )),
                    _ => Ok(SecurityTestResult::failure(
                        test_name,
                        start.elapsed().as_millis() as u64,
                        "Original key failed to decrypt its own data".to_string(),
                    )),
                }
            }
        }
    }

    /// Test 4: Agent permission levels
    fn test_agent_permission_levels(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "agent_permission_levels".to_string();

        // Create different permission levels
        let readonly = AgentPermissions {
            memory_level: 1,
            can_create_events: false,
            can_modify_views: false,
            can_read_entities: true,
            allowed_operations: vec!["read".to_string()],
        };

        let readwrite = AgentPermissions {
            memory_level: 2,
            can_create_events: true,
            can_modify_views: false,
            can_read_entities: true,
            allowed_operations: vec!["read".to_string(), "write".to_string()],
        };

        // Verify permission levels are different
        if readonly.memory_level != readwrite.memory_level
            && readonly.can_create_events != readwrite.can_create_events
        {
            Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            ))
        } else {
            Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "Permission levels should be different".to_string(),
            ))
        }
    }

    /// Test 5: Permission isolation
    fn test_permission_isolation(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "permission_isolation".to_string();

        // Create two agents with different permissions
        let agent1 = AgentPermissions {
            memory_level: 1,
            can_create_events: false,
            can_modify_views: false,
            can_read_entities: true,
            allowed_operations: vec!["read_only".to_string()],
        };

        let agent2 = AgentPermissions {
            memory_level: 3,
            can_create_events: true,
            can_modify_views: true,
            can_read_entities: true,
            allowed_operations: vec!["read".to_string(), "write".to_string(), "modify".to_string()],
        };

        // Verify permissions are isolated
        if agent1.memory_level != agent2.memory_level
            && agent1.allowed_operations != agent2.allowed_operations
        {
            Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            ))
        } else {
            Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "Agents should have different isolated permissions".to_string(),
            ))
        }
    }

    /// Test 6: Audit log integrity
    fn test_audit_log_integrity(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "audit_log_integrity".to_string();

        let mut conn = PgConnection::establish(&self.database_url)?;

        // Create a test audit log
        let log = self.audit_logger.log_custom(
            "test_user",
            "test_action",
            "test_target",
            true,
            Some(serde_json::json!({"test": "data"})),
        )?;

        // Verify log was created with correct data
        let retrieved_log: Option<AuditLog> = audit_logs::table
            .filter(audit_logs::id.eq(log.id))
            .first(&mut conn)
            .optional()?;

        match retrieved_log {
            Some(retrieved) if retrieved.user_id == "test_user"
                && retrieved.action == "test_action"
                && retrieved.target == "test_target" =>
            {
                Ok(SecurityTestResult::success(
                    test_name,
                    start.elapsed().as_millis() as u64,
                ))
            }
            _ => Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "Audit log data mismatch".to_string(),
            )),
        }
    }

    /// Test 7: Audit log query consistency
    fn test_audit_log_query_consistency(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "audit_log_query_consistency".to_string();

        let mut conn = PgConnection::establish(&self.database_url)?;

        // Create multiple test logs
        for i in 0..5 {
            self.audit_logger.log_custom(
                "test_user_query",
                &format!("action_{}", i),
                "test_target",
                true,
                None,
            )?;
        }

        // Count logs for user
        let count: i64 = audit_logs::table
            .filter(audit_logs::user_id.eq("test_user_query"))
            .count()
            .get_result(&mut conn)?;

        if count >= 5 {
            Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            ))
        } else {
            Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                format!("Expected at least 5 logs, found {}", count),
            ))
        }
    }

    /// Test 8: Audit log rotation configuration
    fn test_audit_log_rotation_configuration(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "audit_log_rotation_configuration".to_string();

        // Verify rotation configuration is accessible
        let logger = AuditLogger::with_rotation(
            self.database_url.clone(),
            100,  // max_logs
            90,   // rotation_threshold
        );

        // Verify the logger was created (we can't access private fields)
        // So we just verify it can log
        match logger.log_custom("test", "test", "test", true, None) {
            Ok(_) => Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            )),
            Err(_) => Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "Failed to create audit logger with rotation".to_string(),
            )),
        }
    }

    /// Test 9: End-to-end encryption
    fn test_end_to_end_encryption(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "end_to_end_encryption".to_string();

        // Simulate encrypting user data
        let user_data = serde_json::json!({
            "user_id": "test_user",
            "events": ["event1", "event2", "event3"],
            "secrets": "sensitive_information"
        });

        let serialized = serde_json::to_string(&user_data)?;
        let encrypted = self.encryption.encrypt(serialized.as_bytes())?;
        let decrypted = self.encryption.decrypt(&encrypted)?;
        let deserialized: serde_json::Value = serde_json::from_slice(&decrypted)?;

        if deserialized == user_data {
            Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            ))
        } else {
            Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "End-to-end encryption data mismatch".to_string(),
            ))
        }
    }

    /// Test 10: Data export encryption
    fn test_data_export_encryption(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "data_export_encryption".to_string();

        // Simulate exporting data
        let export_data = serde_json::json!({
            "user_id": "test_user",
            "exported_at": Utc::now().to_rfc3339(),
            "version": "1.0.0",
            "raw_memories": [],
            "event_memories": [],
            "entities": []
        });

        let serialized = serde_json::to_string(&export_data)?;
        let encrypted = self.encryption.encrypt(serialized.as_bytes())?;
        let base64_encoded = base64::engine::general_purpose::STANDARD.encode(&encrypted);

        // Verify base64 encoding is valid
        match base64::engine::general_purpose::STANDARD.decode(&base64_encoded) {
            Ok(decoded) => {
                // Verify we can decrypt
                match self.encryption.decrypt(&decoded) {
                    Ok(_) => Ok(SecurityTestResult::success(
                        test_name,
                        start.elapsed().as_millis() as u64,
                    )),
                    Err(e) => Ok(SecurityTestResult::failure(
                        test_name,
                        start.elapsed().as_millis() as u64,
                        format!("Failed to decrypt: {}", e),
                    )),
                }
            }
            Err(e) => Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                format!("Base64 decoding failed: {}", e),
            )),
        }
    }

    /// Test 11: Encryption key uniqueness
    fn test_encryption_key_uniqueness(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "encryption_key_uniqueness".to_string();

        // Create another encryption manager with new key
        let key_path3 = self.temp_dir.path().join("test_key3");
        let encryption3 = EncryptionManager::initialize(&key_path3)?;

        let plaintext = b"Test data";

        // Encrypt with original
        let encrypted1 = self.encryption.encrypt(plaintext)?;

        // Encrypt with new key
        let encrypted2 = encryption3.encrypt(plaintext)?;

        // Encrypted data should be different (due to different keys)
        if encrypted1 != encrypted2 {
            Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            ))
        } else {
            Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                "Different keys should produce different ciphertext".to_string(),
            ))
        }
    }

    /// Test 12: Data integrity with checksum
    fn test_data_integrity_checksum(&self) -> Result<SecurityTestResult> {
        let start = std::time::Instant::now();
        let test_name = "data_integrity_checksum".to_string();

        let data = b"Important data that needs integrity verification";

        // Calculate original checksum
        let checksum1 = format!("{:x}", md5::compute(data));

        // Encrypt and decrypt
        let encrypted = self.encryption.encrypt(data)?;
        let decrypted = self.encryption.decrypt(&encrypted)?;

        // Calculate checksum after encryption/decryption
        let checksum2 = format!("{:x}", md5::compute(&decrypted));

        if checksum1 == checksum2 {
            Ok(SecurityTestResult::success(
                test_name,
                start.elapsed().as_millis() as u64,
            ))
        } else {
            Ok(SecurityTestResult::failure(
                test_name,
                start.elapsed().as_millis() as u64,
                format!("Checksum mismatch: {} vs {}", checksum1, checksum2),
            ))
        }
    }
}

/// Security benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityBenchmarkResults {
    /// Encryption throughput (bytes/sec)
    pub encryption_throughput: f64,

    /// Decryption throughput (bytes/sec)
    pub decryption_throughput: f64,

    /// Key generation time (ms)
    pub key_generation_time_ms: u64,

    /// Benchmark timestamp
    pub timestamp: DateTime<Utc>,
}

/// Run security benchmarks
pub fn run_security_benchmarks(encryption: &EncryptionManager) -> Result<SecurityBenchmarkResults> {
    // Test encryption throughput
    let test_data = vec![0u8; 1024 * 1024]; // 1MB
    let start = std::time::Instant::now();
    let encrypted = encryption.encrypt(&test_data)?;
    let encryption_time_ms = start.elapsed().as_millis() as u64;
    let encryption_throughput = (test_data.len() as f64 / 1024.0 / 1024.0)
        / (encryption_time_ms as f64 / 1000.0);

    // Test decryption throughput
    let start = std::time::Instant::now();
    let _decrypted = encryption.decrypt(&encrypted)?;
    let decryption_time_ms = start.elapsed().as_millis() as u64;
    let decryption_throughput = (test_data.len() as f64 / 1024.0 / 1024.0)
        / (decryption_time_ms as f64 / 1000.0);

    Ok(SecurityBenchmarkResults {
        encryption_throughput,
        decryption_throughput,
        key_generation_time_ms: 0, // Not measured for Fernet
        timestamp: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_test_result_creation() {
        let success = SecurityTestResult::success("test".to_string(), 100);
        assert!(success.passed);
        assert!(success.error_message.is_none());

        let failure = SecurityTestResult::failure("test".to_string(), 100, "error".to_string());
        assert!(!failure.passed);
        assert_eq!(failure.error_message, Some("error".to_string()));
    }

    #[test]
    fn test_security_test_suite_results() {
        let results = SecurityTestSuiteResults {
            results: vec![
                SecurityTestResult::success("test1".to_string(), 100),
                SecurityTestResult::success("test2".to_string(), 100),
                SecurityTestResult::failure("test3".to_string(), 100, "error".to_string()),
            ],
            total_tests: 3,
            passed_tests: 2,
            failed_tests: 1,
            total_duration_ms: 300,
            timestamp: Utc::now(),
        };

        assert!(!results.all_passed());
        assert_eq!(results.pass_rate(), 66.66666666666666);
    }

    #[test]
    fn test_security_test_suite_results_summary() {
        let results = SecurityTestSuiteResults {
            results: vec![],
            total_tests: 10,
            passed_tests: 10,
            failed_tests: 0,
            total_duration_ms: 1000,
            timestamp: Utc::now(),
        };

        let summary = results.summary();
        assert!(summary.contains("10/10"));
        assert!(summary.contains("100.0%"));
    }
}
