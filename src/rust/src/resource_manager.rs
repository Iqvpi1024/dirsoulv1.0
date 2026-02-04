//! Resource Manager for 8GB Memory Environment
//!
//! This module implements dynamic resource management to prevent the system
//! from running out of memory in an 8GB environment where Ollama + PostgreSQL
//! compete for resources.
//!
//! # Design Principles (HEAD.md)
//! - **8G内存优化**: Ollama (4-5GB) + PostgreSQL (1-2GB) + System (1GB) + Cache (~500MB)
//! - **Model Offloading**: 空闲时卸载模型，使用时加载
//! - **资源熔断**: 高负载时暂停非关键任务
//! - **动态调度**: 实时监控内存使用
//!
//! # Example
//! ```text
//! use dirsoul::resource_manager::{ResourceManager, ResourceManagerConfig};
//!
//! let config = ResourceManagerConfig::default();
//! let manager = ResourceManager::new(config)?;
//! manager.monitor_memory()?;
//! if manager.should_offload_model()? {
//!     manager.offload_model()?;
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

use crate::error::{DirSoulError, Result};

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Total memory in MB
    pub total_mb: u64,

    /// Used memory in MB
    pub used_mb: u64,

    /// Available memory in MB
    pub available_mb: u64,

    /// Percentage of memory used (0-100)
    pub used_percent: f64,

    /// Timestamp of measurement
    pub timestamp: DateTime<Utc>,
}

impl MemoryUsage {
    /// Check if memory is under pressure (>85% used)
    pub fn is_under_pressure(&self) -> bool {
        self.used_percent > 85.0
    }

    /// Check if memory is critical (>95% used)
    pub fn is_critical(&self) -> bool {
        self.used_percent > 95.0
    }

    /// Get remaining memory in MB
    pub fn remaining_mb(&self) -> u64 {
        self.total_mb - self.used_mb
    }
}

/// Resource manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManagerConfig {
    /// Maximum memory threshold in MB (trigger cleanup when exceeded)
    pub max_memory_mb: u64,

    /// Model offload timeout in seconds (unload model after idle period)
    pub offload_timeout_sec: u64,

    /// Memory check interval in seconds
    pub check_interval_sec: u64,

    /// Enable model offloading
    pub enable_model_offloading: bool,

    /// Enable automatic cleanup
    pub enable_auto_cleanup: bool,

    /// Critical memory threshold (trigger circuit breaker)
    pub critical_memory_threshold: f64,
}

impl Default for ResourceManagerConfig {
    fn default() -> Self {
        Self {
            // 8GB system, leave ~1.5GB for OS + other processes
            max_memory_mb: 6500,
            // Offload model after 10 minutes of inactivity
            offload_timeout_sec: 600,
            // Check memory every 30 seconds
            check_interval_sec: 30,
            enable_model_offloading: true,
            enable_auto_cleanup: true,
            critical_memory_threshold: 90.0, // 90%
        }
    }
}

/// Resource manager for dynamic memory management
pub struct ResourceManager {
    /// Configuration
    config: ResourceManagerConfig,

    /// Last activity timestamp
    last_activity: SystemTime,

    /// Model loaded status
    model_loaded: bool,

    /// Memory usage history
    memory_history: Vec<MemoryUsage>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new(config: ResourceManagerConfig) -> Self {
        Self {
            config,
            last_activity: SystemTime::now(),
            model_loaded: true, // Assume model is loaded initially
            memory_history: Vec::with_capacity(100), // Keep last 100 measurements
        }
    }

    /// Create from config file
    pub fn from_config_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| DirSoulError::Io(e))?;

        let config: ResourceManagerConfig = toml::from_str(&content)
            .map_err(|e| DirSoulError::Config(format!("Invalid TOML: {}", e)))?;

        Ok(Self::new(config))
    }

    /// Load config from default location
    pub fn load_or_default() -> Result<Self> {
        let config_path = PathBuf::from("config/resources.toml");

        if config_path.exists() {
            Self::from_config_file(&config_path)
        } else {
            // Create default config
            let config = ResourceManagerConfig::default();
            fs::create_dir_all(config_path.parent().unwrap())?;

            let toml_str = toml::to_string_pretty(&config)
                .map_err(|e| DirSoulError::Config(format!("Failed to serialize TOML: {}", e)))?;

            fs::write(&config_path, toml_str)
                .map_err(|e| DirSoulError::Io(e))?;

            Ok(Self::new(config))
        }
    }

    /// Get current memory usage
    pub fn get_memory_usage(&self) -> Result<MemoryUsage> {
        // Read /proc/meminfo on Linux
        let meminfo = fs::read_to_string("/proc/meminfo")
            .map_err(|e| DirSoulError::Io(e))?;

        let mut total_kb: u64 = 0;
        let mut available_kb: u64 = 0;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("MemAvailable:") {
                available_kb = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }

        if total_kb == 0 {
            return Err(DirSoulError::Config("Failed to read memory info".to_string()));
        }

        let total_mb = total_kb / 1024;
        let available_mb = available_kb / 1024;
        let used_mb = total_mb - available_mb;
        let used_percent = (used_mb as f64 / total_mb as f64) * 100.0;

        Ok(MemoryUsage {
            total_mb,
            used_mb,
            available_mb,
            used_percent,
            timestamp: Utc::now(),
        })
    }

    /// Check if model should be offloaded
    pub fn should_offload_model(&self) -> Result<bool> {
        if !self.config.enable_model_offloading {
            return Ok(false);
        }

        // Check if model has been idle for timeout period
        match self.last_activity.elapsed() {
            Ok(elapsed) => {
                let idle_sec = elapsed.as_secs();
                if idle_sec < self.config.offload_timeout_sec {
                    return Ok(false);
                }
            }
            Err(_) => return Ok(false),
        }

        // Check memory pressure
        let usage = self.get_memory_usage()?;
        Ok(usage.used_percent > 80.0) // Only offload if memory is tight
    }

    /// Offload Ollama model
    pub fn offload_model(&mut self) -> Result<()> {
        if !self.config.enable_model_offloading {
            return Ok(());
        }

        // Unload model by stopping Ollama service
        let status = Command::new("systemctl")
            .args(["stop", "ollama"])
            .status();

        match status {
            Ok(_) => {
                self.model_loaded = false;
                Ok(())
            }
            Err(e) => {
                // Try alternative: kill ollama process
                let kill_result = Command::new("pkill")
                    .arg("ollama")
                    .status();

                match kill_result {
                    Ok(_) => {
                        self.model_loaded = false;
                        Ok(())
                    }
                    Err(_) => Err(DirSoulError::Io(e))
                }
            }
        }
    }

    /// Load Ollama model
    pub fn load_model(&mut self) -> Result<()> {
        if self.model_loaded {
            return Ok(());
        }

        // Start Ollama service
        let status = Command::new("systemctl")
            .args(["start", "ollama"])
            .status()
            .map_err(|e| DirSoulError::Io(e))?;

        if status.success() {
            self.model_loaded = true;
            self.last_activity = SystemTime::now();
            Ok(())
        } else {
            Err(DirSoulError::Config("Failed to start Ollama".to_string()))
        }
    }

    /// Check if model is loaded
    pub fn is_model_loaded(&self) -> bool {
        self.model_loaded
    }

    /// Record activity (resets idle timer)
    pub fn record_activity(&mut self) {
        self.last_activity = SystemTime::now();
    }

    /// Perform automatic cleanup
    pub fn perform_cleanup(&self) -> Result<()> {
        if !self.config.enable_auto_cleanup {
            return Ok(());
        }

        let usage = self.get_memory_usage()?;

        if usage.is_under_pressure() {
            // Clean up various caches

            // 1. Try to clean PostgreSQL cache
            let _ = Command::new("sync")
                .arg("-e")
                .arg("3")
                .status();

            // 2. On Linux, suggest to drop caches
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("echo 3 > /proc/sys/vm/drop_caches")
                    .status();
            }
        }

        Ok(())
    }

    /// Check if circuit breaker should trip (pause non-critical tasks)
    pub fn should_trip_circuit_breaker(&self) -> Result<bool> {
        let usage = self.get_memory_usage()?;
        Ok(usage.used_percent > self.config.critical_memory_threshold)
    }

    /// Monitor memory and take action
    pub fn monitor_memory(&mut self) -> Result<MemoryUsage> {
        let usage = self.get_memory_usage()?;

        // Add to history
        self.memory_history.push(usage.clone());
        if self.memory_history.len() > 100 {
            self.memory_history.remove(0);
        }

        // Check if we need to offload model
        if self.should_offload_model()? {
            self.offload_model()?;
        }

        // Perform cleanup if needed
        if usage.is_under_pressure() {
            self.perform_cleanup()?;
        }

        Ok(usage)
    }

    /// Get memory history
    pub fn get_memory_history(&self) -> &[MemoryUsage] {
        &self.memory_history
    }

    /// Get average memory usage over history
    pub fn get_average_memory_usage(&self) -> Option<f64> {
        if self.memory_history.is_empty() {
            return None;
        }

        let sum: f64 = self.memory_history.iter()
            .map(|m| m.used_percent)
            .sum();

        Some(sum / self.memory_history.len() as f64)
    }

    /// Get configuration
    pub fn get_config(&self) -> &ResourceManagerConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ResourceManagerConfig) {
        self.config = config;
    }
}

/// Circuit breaker for non-critical tasks
pub struct CircuitBreaker {
    /// Is circuit open (tasks blocked)
    is_open: bool,

    /// Last check timestamp
    last_check: SystemTime,

    /// Cooldown period in seconds
    cooldown_sec: u64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(cooldown_sec: u64) -> Self {
        Self {
            is_open: false,
            last_check: SystemTime::now(),
            cooldown_sec,
        }
    }

    /// Check if task should be allowed
    pub fn allow_task(&mut self) -> bool {
        // Check if cooldown has passed
        if let Ok(elapsed) = self.last_check.elapsed() {
            if elapsed.as_secs() > self.cooldown_sec {
                self.is_open = false;
            }
        }

        !self.is_open
    }

    /// Trip the circuit breaker (block tasks)
    pub fn trip(&mut self) {
        self.is_open = true;
        self.last_check = SystemTime::now();
    }

    /// Reset the circuit breaker
    pub fn reset(&mut self) {
        self.is_open = false;
        self.last_check = SystemTime::now();
    }

    /// Check if circuit is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }
}

/// Task priority for resource management
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// Critical - always run (memory monitoring, core operations)
    Critical = 0,

    /// High - user-facing operations
    High = 1,

    /// Medium - background processing
    Medium = 2,

    /// Low - deferrable tasks
    Low = 3,
}

/// Scheduled task with priority
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Task identifier
    pub id: String,

    /// Task priority
    pub priority: TaskPriority,

    /// Estimated memory usage in MB
    pub estimated_memory_mb: u64,

    /// Task description
    pub description: String,
}

impl ScheduledTask {
    /// Create a new scheduled task
    pub fn new(id: String, priority: TaskPriority, estimated_memory_mb: u64, description: String) -> Self {
        Self {
            id,
            priority,
            estimated_memory_mb,
            description,
        }
    }

    /// Check if task can run given current memory
    pub fn can_run(&self, available_memory_mb: u64) -> bool {
        self.estimated_memory_mb <= available_memory_mb
    }
}

/// Task scheduler with resource awareness
pub struct ResourceAwareScheduler {
    /// Resource manager
    resource_manager: ResourceManager,

    /// Circuit breaker
    circuit_breaker: CircuitBreaker,
}

impl ResourceAwareScheduler {
    /// Create a new resource-aware scheduler
    pub fn new(resource_manager: ResourceManager) -> Self {
        Self {
            resource_manager,
            circuit_breaker: CircuitBreaker::new(60), // 1 minute cooldown
        }
    }

    /// Check if task should be scheduled
    pub fn should_schedule(&mut self, task: &ScheduledTask) -> Result<bool> {
        // Critical tasks always run
        if task.priority == TaskPriority::Critical {
            return Ok(true);
        }

        // Check circuit breaker
        if self.circuit_breaker.is_open() {
            if !self.circuit_breaker.allow_task() {
                return Ok(false);
            }
        }

        // Check memory availability
        let usage = self.resource_manager.get_memory_usage()?;

        // Circuit breaker logic: trip if memory is critical
        if usage.is_critical() {
            self.circuit_breaker.trip();
            return Ok(false);
        }

        // Check if task can run with available memory
        Ok(task.can_run(usage.available_mb))
    }

    /// Get resource manager
    pub fn get_resource_manager(&self) -> &ResourceManager {
        &self.resource_manager
    }

    /// Get mutable resource manager
    pub fn get_resource_manager_mut(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }
}

/// Background memory monitor task
pub async fn background_memory_monitor(
    mut resource_manager: ResourceManager,
    interval_secs: u64,
) -> Result<()> {
    loop {
        sleep(Duration::from_secs(interval_secs)).await;

        match resource_manager.monitor_memory() {
            Ok(usage) => {
                if usage.is_critical() {
                    eprintln!("CRITICAL: Memory usage at {:.1}%", usage.used_percent);
                }
            }
            Err(e) => {
                eprintln!("Memory monitor error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_usage_under_pressure() {
        let usage = MemoryUsage {
            total_mb: 8000,
            used_mb: 7000,
            available_mb: 1000,
            used_percent: 87.5,
            timestamp: Utc::now(),
        };

        assert!(usage.is_under_pressure());
        assert!(!usage.is_critical());
        assert_eq!(usage.remaining_mb(), 1000);
    }

    #[test]
    fn test_memory_usage_critical() {
        let usage = MemoryUsage {
            total_mb: 8000,
            used_mb: 7700,
            available_mb: 300,
            used_percent: 96.25,
            timestamp: Utc::now(),
        };

        assert!(usage.is_critical());
    }

    #[test]
    fn test_resource_manager_config_default() {
        let config = ResourceManagerConfig::default();
        assert_eq!(config.max_memory_mb, 6500);
        assert_eq!(config.offload_timeout_sec, 600);
        assert!(config.enable_model_offloading);
    }

    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(10);
        assert!(!cb.is_open());
        assert!(cb.allow_task());

        cb.trip();
        assert!(cb.is_open());
        assert!(!cb.allow_task());
    }

    #[test]
    fn test_scheduled_task() {
        let task = ScheduledTask::new(
            "test_task".to_string(),
            TaskPriority::High,
            500,
            "Test task".to_string(),
        );

        assert!(task.can_run(1000));
        assert!(!task.can_run(100));
    }

    #[test]
    fn test_task_priority_ord() {
        assert!(TaskPriority::Critical < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Medium);
        assert!(TaskPriority::Medium < TaskPriority::Low);
    }
}
