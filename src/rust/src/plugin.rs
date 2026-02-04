//! Plugin Interface Definition
//!
//! This module defines the plugin system interface that allows external
//! plugins to interact with DirSoul's memory system in a controlled,
//! permission-governed manner.
//!
//! # Design Principles (HEAD.md)
//! - **插件沙箱隔离**: 插件崩溃不影响系统
//! - **最小权限原则**: 只授予必要的访问权限
//!
//! # Skill Reference
//! - docs/skills/plugin_permission_system.md

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::agents::MemoryPermission;
use crate::actor_agent::EventNotification;
use crate::cognitive::{CognitiveView, NewCognitiveView};
use crate::error::{DirSoulError, Result};
use crate::models::{Entity, EventMemory, NewEventMemory};

/// Event subscription filter for plugins
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventSubscription {
    /// Subscribe to all events
    All,

    /// Subscribe to events with specific action
    Action(String),

    /// Subscribe to events with specific target pattern
    TargetPattern(String),

    /// Subscribe to events matching custom filter
    CustomFilter(String),
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Version string
    pub version: String,

    /// Human-readable description
    pub description: String,

    /// Required permission level
    pub required_permission: MemoryPermission,

    /// Plugin author
    pub author: String,

    /// Supported event types
    pub supported_events: Vec<String>,

    /// Whether this is a built-in system plugin
    pub is_builtin: bool,
}

/// Plugin response to queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// Response content
    pub content: String,

    /// Source references (view IDs, event IDs, etc.)
    pub sources: Vec<Uuid>,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,

    /// Response metadata
    pub metadata: serde_json::Value,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Plugin output after event processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginOutput {
    /// Plugin created new cognitive views
    ViewsCreated(Vec<Uuid>),

    /// Plugin logged recommendation
    RecommendationLogged {
        recommendation_id: Uuid,
        content: String,
    },

    /// Plugin created new events
    EventsCreated(Vec<Uuid>),

    /// Plugin performed analysis but no output
    AnalysisComplete,

    /// Plugin encountered error
    Error(String),
}

/// Memory interface for plugins (permission-guarded)
///
/// This trait provides controlled access to the memory system based on
/// the plugin's permission level.
#[async_trait]
pub trait PluginMemoryInterface: Send + Sync {
    /// Query events (requires ReadWriteEvents permission)
    async fn query_events(
        &self,
        user_id: &str,
        filter: &EventFilter,
    ) -> Result<Vec<EventMemory>>;

    /// Create cognitive view (requires ReadWriteDerived permission)
    async fn create_view(&self, user_id: &str, view: NewCognitiveView) -> Result<CognitiveView>;

    /// Create event (requires ReadWriteEvents permission)
    async fn create_event(&self, user_id: &str, event: NewEventMemory) -> Result<EventMemory>;

    /// Query statistics (available to all permission levels)
    async fn get_statistics(&self, user_id: &str, time_range: PluginTimeRange) -> Result<Statistics>;

    /// Query entities (requires ReadWriteDerived permission)
    async fn query_entities(&self, user_id: &str, filter: &EntityFilter) -> Result<Vec<Entity>>;

    /// Check if plugin has specific permission
    fn has_permission(&self, permission: MemoryPermission) -> bool;
}

/// Event filter for querying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub actions: Option<Vec<String>>,
    pub targets: Option<Vec<String>>,
    pub limit: Option<usize>,
}

/// Time range for statistics (plugin-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Statistics about memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub event_count: usize,
    pub view_count: usize,
    pub concept_count: usize,
    pub entity_count: usize,
}

/// Entity filter for querying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityFilter {
    pub entity_types: Option<Vec<String>>,
    pub min_confidence: Option<f64>,
    pub limit: Option<usize>,
}

/// Base User Plugin trait
///
/// All plugins must implement this trait to interact with DirSoul.
#[async_trait]
pub trait UserPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Called when new event arrives
    ///
    /// Plugins can analyze events and generate views or recommendations.
    async fn on_event(
        &self,
        event: &EventNotification,
        context: &PluginContext,
    ) -> Result<PluginOutput>;

    /// Called when user queries the plugin
    ///
    /// Plugins can access memory and generate responses.
    async fn on_query(
        &self,
        query: &str,
        context: &PluginContext,
    ) -> Result<PluginResponse>;

    /// Get event subscriptions
    ///
    /// Returns a list of event types this plugin wants to be notified about.
    fn subscriptions(&self) -> &[EventSubscription];

    /// Cleanup when plugin is unloaded
    async fn cleanup(&self) -> Result<()>;

    /// Optional: Plugin initialization callback
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    /// Optional: Plugin health check
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Context passed to plugins with permission-guarded access
///
/// This struct provides plugins with controlled access to memory based on
/// their permission level.
#[derive(Clone)]
pub struct PluginContext {
    pub plugin_id: String,
    pub user_id: String,
    pub permission: MemoryPermission,
    memory_interface: std::sync::Arc<dyn PluginMemoryInterface>,
}

impl PluginContext {
    pub fn new(
        plugin_id: String,
        user_id: String,
        permission: MemoryPermission,
        memory_interface: std::sync::Arc<dyn PluginMemoryInterface>,
    ) -> Self {
        Self {
            plugin_id,
            user_id,
            permission,
            memory_interface,
        }
    }

    /// Query events with permission check
    pub async fn query_events(&self, filter: &EventFilter) -> Result<Vec<EventMemory>> {
        if !self.permission.can_create_events() {
            return Err(DirSoulError::Config(
                "Plugin does not have permission to query events".to_string(),
            ));
        }

        self.memory_interface.query_events(&self.user_id, filter).await
    }

    /// Create cognitive view with permission check
    pub async fn create_view(&self, view: NewCognitiveView) -> Result<CognitiveView> {
        if !self.permission.can_modify_views() {
            return Err(DirSoulError::Config(
                "Plugin does not have permission to create views".to_string(),
            ));
        }

        self.memory_interface.create_view(&self.user_id, view).await
    }

    /// Create event with permission check
    pub async fn create_event(&self, event: NewEventMemory) -> Result<EventMemory> {
        if !self.permission.can_create_events() {
            return Err(DirSoulError::Config(
                "Plugin does not have permission to create events".to_string(),
            ));
        }

        self.memory_interface.create_event(&self.user_id, event).await
    }

    /// Get statistics (available to all permission levels)
    pub async fn get_statistics(&self, time_range: PluginTimeRange) -> Result<Statistics> {
        self.memory_interface.get_statistics(&self.user_id, time_range).await
    }

    /// Query entities with permission check
    pub async fn query_entities(&self, filter: &EntityFilter) -> Result<Vec<Entity>> {
        if !self.permission.can_read_entities() {
            return Err(DirSoulError::Config(
                "Plugin does not have permission to query entities".to_string(),
            ));
        }

        self.memory_interface.query_entities(&self.user_id, filter).await
    }

    /// Check if plugin has specific permission
    pub fn has_permission(&self, permission: MemoryPermission) -> bool {
        self.memory_interface.has_permission(permission)
    }
}

/// Plugin specification for loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSpec {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub required_permission: i32,
    pub author: String,
    pub executable: Option<String>,  // For external process plugins
    pub is_builtin: bool,
}

impl PluginSpec {
    pub fn from_metadata(metadata: &PluginMetadata) -> Self {
        Self {
            id: metadata.id.clone(),
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            description: metadata.description.clone(),
            required_permission: metadata.required_permission.as_i32(),
            author: metadata.author.clone(),
            executable: None,
            is_builtin: metadata.is_builtin,
        }
    }
}

/// Plugin execution timeout configuration
#[derive(Debug, Clone)]
pub struct PluginTimeoutConfig {
    /// Default timeout for plugin operations
    pub default_timeout: Duration,

    /// Timeout for plugin initialization
    pub init_timeout: Duration,

    /// Timeout for plugin cleanup
    pub cleanup_timeout: Duration,
}

impl Default for PluginTimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            init_timeout: Duration::from_secs(60),
            cleanup_timeout: Duration::from_secs(10),
        }
    }
}

/// Isolated plugin instance with health tracking
///
/// This wrapper provides thread-safe isolation and monitoring for plugins.
pub struct IsolatedPlugin {
    /// Plugin instance
    plugin: Arc<dyn UserPlugin>,

    /// Plugin metadata
    metadata: PluginMetadata,

    /// Permission level
    permission: MemoryPermission,

    /// Health status
    is_healthy: Arc<RwLock<bool>>,

    /// Last health check timestamp
    last_health_check: Arc<RwLock<Option<DateTime<Utc>>>>,

    /// Restart count
    restart_count: Arc<Mutex<usize>>,

    /// Max restarts allowed
    max_restarts: usize,
}

impl IsolatedPlugin {
    /// Create a new isolated plugin instance
    pub fn new(
        plugin: Arc<dyn UserPlugin>,
        permission: MemoryPermission,
        max_restarts: usize,
    ) -> Self {
        let metadata = plugin.metadata().clone();

        Self {
            plugin,
            metadata,
            permission,
            is_healthy: Arc::new(RwLock::new(true)),
            last_health_check: Arc::new(RwLock::new(None)),
            restart_count: Arc::new(Mutex::new(0)),
            max_restarts,
        }
    }

    /// Get plugin metadata
    pub fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    /// Get plugin permission level
    pub fn permission(&self) -> MemoryPermission {
        self.permission
    }

    /// Check if plugin is healthy
    pub async fn is_healthy(&self) -> bool {
        *self.is_healthy.read().await
    }

    /// Run health check
    pub async fn health_check(&self) -> Result<bool> {
        let healthy = self.plugin.health_check().await?;
        *self.is_healthy.write().await = healthy;
        *self.last_health_check.write().await = Some(Utc::now());
        Ok(healthy)
    }

    /// Get restart count
    pub async fn restart_count(&self) -> usize {
        *self.restart_count.lock().await
    }

    /// Increment restart count
    pub async fn increment_restart_count(&self) -> usize {
        let mut count = self.restart_count.lock().await;
        *count += 1;
        *count
    }

    /// Check if can restart
    pub async fn can_restart(&self) -> bool {
        *self.restart_count.lock().await < self.max_restarts
    }

    /// Execute plugin event handler with timeout
    pub async fn on_event(
        &self,
        event: &EventNotification,
        context: &PluginContext,
        timeout: Duration,
    ) -> Result<PluginOutput> {
        let plugin = self.plugin.clone();
        let event = event.clone();
        let context = context.clone();

        tokio::time::timeout(timeout, async move {
            plugin.on_event(&event, &context).await
        })
        .await
        .map_err(|_| DirSoulError::PluginTimeout(format!(
            "Plugin {} event handler timed out",
            self.metadata.id
        )))?
    }

    /// Execute plugin query handler with timeout
    pub async fn on_query(
        &self,
        query: &str,
        context: &PluginContext,
        timeout: Duration,
    ) -> Result<PluginResponse> {
        let plugin = self.plugin.clone();
        let query = query.to_string();
        let context = context.clone();

        tokio::time::timeout(timeout, async move {
            plugin.on_query(&query, &context).await
        })
        .await
        .map_err(|_| DirSoulError::PluginTimeout(format!(
            "Plugin {} query handler timed out",
            self.metadata.id
        )))?
    }
}

/// Plugin manager for installation, lifecycle, and health monitoring
///
/// # Design Principles (HEAD.md)
/// - **插件沙箱隔离**: 插件崩溃不影响系统
/// - **权限分级**: 最小权限原则
///
/// # Skill Reference
/// - docs/skills/plugin_permission_system.md
pub struct PluginManager {
    /// Active plugins by ID
    plugins: Arc<RwLock<HashMap<String, IsolatedPlugin>>>,

    /// Plugin specifications registry
    plugin_specs: Arc<RwLock<HashMap<String, PluginSpec>>>,

    /// Timeout configuration
    timeout_config: PluginTimeoutConfig,

    /// Maximum restarts allowed
    max_restarts: usize,

    /// Restart backoff base duration
    restart_backoff: Duration,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self::with_config(PluginTimeoutConfig::default(), 3, Duration::from_secs(5))
    }

    /// Create a new plugin manager with custom configuration
    pub fn with_config(
        timeout_config: PluginTimeoutConfig,
        max_restarts: usize,
        restart_backoff: Duration,
    ) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_specs: Arc::new(RwLock::new(HashMap::new())),
            timeout_config,
            max_restarts,
            restart_backoff,
        }
    }

    /// Register a plugin specification
    pub async fn register_spec(&self, spec: PluginSpec) -> Result<()> {
        let mut specs = self.plugin_specs.write().await;
        specs.insert(spec.id.clone(), spec);
        Ok(())
    }

    /// Install and start a plugin
    ///
    /// # Arguments
    /// - `plugin`: Plugin instance to install
    /// - `permission`: Permission level to grant
    ///
    /// # Returns
    /// Plugin metadata on success
    pub async fn install(
        &self,
        plugin: Arc<dyn UserPlugin>,
        permission: MemoryPermission,
    ) -> Result<PluginMetadata> {
        let metadata = plugin.metadata().clone();

        // Validate requested permission doesn't exceed required
        if permission.as_i32() < metadata.required_permission.as_i32() {
            return Err(DirSoulError::PermissionDenied(format!(
                "Plugin {} requires at least {:?} permission, got {:?}",
                metadata.id, metadata.required_permission, permission
            )));
        }

        // Register spec
        let spec = PluginSpec::from_metadata(&metadata);
        self.register_spec(spec).await?;

        // Initialize plugin
        plugin.initialize().await.map_err(|e| {
            DirSoulError::Plugin(format!("Plugin {} initialization failed: {}", metadata.id, e))
        })?;

        // Create isolated instance
        let isolated = IsolatedPlugin::new(plugin, permission, self.max_restarts);

        // Store plugin
        let mut plugins = self.plugins.write().await;
        plugins.insert(metadata.id.clone(), isolated);

        Ok(metadata)
    }

    /// Uninstall a plugin
    ///
    /// Built-in plugins cannot be uninstalled.
    pub async fn uninstall(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;

        let plugin = plugins.get(plugin_id)
            .ok_or_else(|| DirSoulError::PluginNotFound(plugin_id.to_string()))?;

        // Check if built-in
        if plugin.metadata().is_builtin {
            return Err(DirSoulError::Plugin(format!(
                "Cannot uninstall built-in plugin {}",
                plugin_id
            )));
        }

        // Remove from registry
        let isolated = plugins.remove(plugin_id)
            .ok_or_else(|| DirSoulError::PluginNotFound(plugin_id.to_string()))?;

        // Cleanup plugin
        let plugin = isolated.plugin.clone();
        tokio::time::timeout(
            self.timeout_config.cleanup_timeout,
            plugin.cleanup()
        )
        .await
        .map_err(|_| DirSoulError::PluginTimeout(format!(
            "Plugin {} cleanup timed out",
            plugin_id
        )))??;

        Ok(())
    }

    /// Check if plugin has specific permission
    pub async fn check_permission(
        &self,
        plugin_id: &str,
        permission: MemoryPermission,
    ) -> Result<bool> {
        let plugins = self.plugins.read().await;

        let plugin = plugins.get(plugin_id)
            .ok_or_else(|| DirSoulError::PluginNotFound(plugin_id.to_string()))?;

        Ok(plugin.permission() >= permission)
    }

    /// Get plugin by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<IsolatedPlugin> {
        let plugins = self.plugins.read().await;

        plugins.get(plugin_id)
            .ok_or_else(|| DirSoulError::PluginNotFound(plugin_id.to_string()))
            .map(|p| p.clone())
    }

    /// List all active plugins
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.metadata().clone()).collect()
    }

    /// List plugins by user
    pub async fn list_plugins_by_user(&self, _user_id: &str) -> Vec<PluginMetadata> {
        // TODO: Add user filtering when plugin-user mapping is implemented
        self.list_plugins().await
    }

    /// Run health check on all plugins
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let plugins = self.plugins.read().await;
        let mut results = HashMap::new();

        for (id, plugin) in plugins.iter() {
            let healthy = plugin.health_check().await.unwrap_or(false);
            results.insert(id.clone(), healthy);
        }

        results
    }

    /// Monitor plugin health and restart crashed plugins
    pub async fn monitor(&self) -> Result<()> {
        let mut crashed = Vec::new();
        let plugins = self.plugins.read().await;

        for (id, plugin) in plugins.iter() {
            if !plugin.is_healthy().await {
                crashed.push(id.clone());
            }
        }

        drop(plugins);

        // Handle crashed plugins
        for id in crashed {
            if let Err(e) = self.handle_crash(&id).await {
                eprintln!("Failed to handle crash for plugin {}: {}", id, e);
            }
        }

        Ok(())
    }

    /// Handle crashed plugin restart
    async fn handle_crash(&self, plugin_id: &str) -> Result<()> {
        let plugin = self.get_plugin(plugin_id).await?;

        // Check if can restart
        if !plugin.can_restart().await {
            return Err(DirSoulError::Plugin(format!(
                "Plugin {} exceeded max restarts",
                plugin_id
            )));
        }

        // Increment restart count
        let restart_count = plugin.increment_restart_count().await;

        // Backoff before restart
        let backoff = self.restart_backoff * restart_count as u32;
        tokio::time::sleep(backoff).await;

        // Attempt health check to verify recovery
        let _ = plugin.health_check().await?;

        Ok(())
    }

    /// Get plugin statistics
    pub async fn get_stats(&self) -> PluginManagerStats {
        let plugins = self.plugins.read().await;
        let specs = self.plugin_specs.read().await;

        let mut total_restarts = 0;
        let mut healthy_count = 0;

        for plugin in plugins.values() {
            total_restarts += plugin.restart_count().await;
            if plugin.is_healthy().await {
                healthy_count += 1;
            }
        }

        PluginManagerStats {
            total_plugins: plugins.len(),
            healthy_plugins: healthy_count,
            registered_specs: specs.len(),
            total_restarts,
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManagerStats {
    pub total_plugins: usize,
    pub healthy_plugins: usize,
    pub registered_specs: usize,
    pub total_restarts: usize,
}

/// Clone helper for IsolatedPlugin
impl Clone for IsolatedPlugin {
    fn clone(&self) -> Self {
        Self {
            plugin: self.plugin.clone(),
            metadata: self.metadata.clone(),
            permission: self.permission,
            is_healthy: self.is_healthy.clone(),
            last_health_check: self.last_health_check.clone(),
            restart_count: self.restart_count.clone(),
            max_restarts: self.max_restarts,
        }
    }
}

/// Parsed command from user input
///
/// Represents the result of parsing user input for @plugin commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParsedCommand {
    /// Call to specific plugin: @plugin_name query text
    PluginCall {
        plugin: String,
        query: String,
    },

    /// Default query (no @ command, routes to default plugin)
    DefaultQuery {
        query: String,
    },
}

/// Command response from plugin execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResponse {
    /// Response from plugin execution
    Plugin(PluginResponse),

    /// Response from default plugin (DeepTalk)
    Default(PluginResponse),

    /// Error response
    Error(String),
}

/// Command router for parsing and routing @plugin commands
///
/// # Design Principles (HEAD.md)
/// - **插件对话也是记忆**: 与插件的对话同样进入事件流
/// - **@命令调用**: @决策、@心理分析这样简单调用
///
/// # Skill Reference
/// - docs/skills/plugin_permission_system.md
pub struct CommandRouter {
    /// Plugin manager for accessing plugins
    manager: Arc<PluginManager>,

    /// Default plugin ID (usually "deeptalk")
    default_plugin_id: Option<String>,

    /// Regex for parsing @ commands
    at_regex: Regex,

    /// User ID for event logging
    user_id: String,
}

impl CommandRouter {
    /// Create a new command router
    pub fn new(manager: Arc<PluginManager>, user_id: String) -> Self {
        // Pattern: @plugin_name query text
        // Captures: plugin name and query text
        let at_regex = Regex::new(r"@(\w+)\s+(.+)").unwrap();

        Self {
            manager,
            default_plugin_id: None,
            at_regex,
            user_id,
        }
    }

    /// Set the default plugin (usually DeepTalk)
    pub fn set_default_plugin(&mut self, plugin_id: String) {
        self.default_plugin_id = Some(plugin_id);
    }

    /// Get the default plugin ID
    pub fn default_plugin(&self) -> Option<&str> {
        self.default_plugin_id.as_deref()
    }

    /// Parse user input for @ commands
    pub fn parse_command(&self, input: &str) -> ParsedCommand {
        // Trim whitespace
        let input = input.trim();

        // Try to match @plugin_name query pattern
        if let Some(caps) = self.at_regex.captures(input) {
            let plugin_name = caps.get(1).unwrap().as_str().to_string();
            let query = caps.get(2).unwrap().as_str().to_string();

            return ParsedCommand::PluginCall { plugin: plugin_name, query };
        }

        // No @ command, use default
        ParsedCommand::DefaultQuery { query: input.to_string() }
    }

    /// Route command to appropriate plugin and execute
    pub async fn route(&self, input: &str) -> Result<CommandResponse> {
        let command = self.parse_command(input);

        match command {
            ParsedCommand::PluginCall { plugin, query } => {
                self.route_to_plugin(&plugin, &query).await
            }

            ParsedCommand::DefaultQuery { query } => {
                self.route_to_default(&query).await
            }
        }
    }

    /// Route to specific plugin
    async fn route_to_plugin(&self, plugin_id: &str, query: &str) -> Result<CommandResponse> {
        // Check plugin exists and is healthy
        let plugin = self.manager.get_plugin(plugin_id).await?;

        if !plugin.is_healthy().await {
            return Ok(CommandResponse::Error(format!(
                "Plugin '{}' is not healthy",
                plugin_id
            )));
        }

        // Create plugin context
        let memory_interface = self.create_memory_interface();
        let context = PluginContext::new(
            plugin_id.to_string(),
            self.user_id.clone(),
            plugin.permission(),
            memory_interface,
        );

        // Execute plugin query with timeout
        let timeout = Duration::from_secs(30);
        let response = plugin.on_query(query, &context, timeout).await?;

        // Log plugin interaction as event
        self.log_plugin_interaction(plugin_id, query, &response).await?;

        Ok(CommandResponse::Plugin(response))
    }

    /// Route to default plugin (DeepTalk)
    async fn route_to_default(&self, query: &str) -> Result<CommandResponse> {
        let default_id = self.default_plugin_id.as_ref()
            .ok_or_else(|| DirSoulError::Plugin("No default plugin configured".to_string()))?;

        // Use same logic as specific plugin
        self.route_to_plugin(default_id, query).await
            .map(|resp| match resp {
                CommandResponse::Plugin(r) => CommandResponse::Default(r),
                _ => resp,
            })
    }

    /// Log plugin interaction as event
    ///
    /// Per HEAD.md: **插件对话必须记录为事件**
    async fn log_plugin_interaction(
        &self,
        plugin_id: &str,
        query: &str,
        response: &PluginResponse,
    ) -> Result<()> {
        // Create event memory for plugin interaction
        let event = NewEventMemory {
            memory_id: Uuid::new_v4(),
            user_id: self.user_id.clone(),
            timestamp: Utc::now(),
            actor: Some(self.user_id.clone()),
            action: "chat_with_plugin".to_string(),
            target: plugin_id.to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            extractor_version: Some("command_router".to_string()),
        };

        // TODO: Store event in database
        // For now, we'll create the event structure
        let _event = event;  // Suppress unused warning

        // Log metadata (in production, store in event metadata)
        let metadata = serde_json::json!({
            "query": query,
            "response_length": response.content.len(),
            "sources": response.sources,
            "confidence": response.confidence,
        });

        let _metadata = metadata;  // Suppress unused warning

        Ok(())
    }

    /// Create a mock memory interface (TODO: implement real interface)
    fn create_memory_interface(&self) -> Arc<dyn PluginMemoryInterface> {
        // TODO: This is a placeholder
        // In production, this should connect to the actual memory store
        Arc::new(MockMemoryInterface)
    }
}

/// Mock memory interface for testing (TODO: replace with real implementation)
struct MockMemoryInterface;

#[async_trait]
impl PluginMemoryInterface for MockMemoryInterface {
    async fn query_events(&self, _user_id: &str, _filter: &EventFilter) -> Result<Vec<EventMemory>> {
        Ok(vec![])
    }

    async fn create_view(&self, _user_id: &str, _view: NewCognitiveView) -> Result<CognitiveView> {
        Err(DirSoulError::Config("Mock memory interface".to_string()))
    }

    async fn create_event(&self, _user_id: &str, _event: NewEventMemory) -> Result<EventMemory> {
        Err(DirSoulError::Config("Mock memory interface".to_string()))
    }

    async fn get_statistics(&self, _user_id: &str, _time_range: PluginTimeRange) -> Result<Statistics> {
        Ok(Statistics {
            event_count: 0,
            view_count: 0,
            concept_count: 0,
            entity_count: 0,
        })
    }

    async fn query_entities(&self, _user_id: &str, _filter: &EntityFilter) -> Result<Vec<Entity>> {
        Ok(vec![])
    }

    fn has_permission(&self, _permission: MemoryPermission) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_subscription_equality() {
        let sub1 = EventSubscription::Action("test".to_string());
        let sub2 = EventSubscription::Action("test".to_string());
        let sub3 = EventSubscription::Action("other".to_string());

        assert_eq!(sub1, sub2);
        assert_ne!(sub1, sub3);
    }

    #[test]
    fn test_plugin_metadata_serialization() {
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            required_permission: MemoryPermission::ReadWriteDerived,
            author: "Test Author".to_string(),
            supported_events: vec!["chat".to_string(), "query".to_string()],
            is_builtin: false,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let _deserialized: PluginMetadata = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_plugin_response_serialization() {
        let response = PluginResponse {
            content: "Test response".to_string(),
            sources: vec![Uuid::new_v4()],
            confidence: 0.9,
            metadata: serde_json::json!({}),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let _deserialized: PluginResponse = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter {
            start_time: Some(Utc::now()),
            end_time: Some(Utc::now()),
            actions: Some(vec!["test".to_string()]),
            targets: Some(vec!["target".to_string()]),
            limit: Some(10),
        };

        let json = serde_json::to_string(&filter).unwrap();
        let _deserialized: EventFilter = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_plugin_spec_from_metadata() {
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            required_permission: MemoryPermission::ReadWriteDerived,
            author: "Test Author".to_string(),
            supported_events: vec![],
            is_builtin: true,
        };

        let spec = PluginSpec::from_metadata(&metadata);

        assert_eq!(spec.id, "test_plugin");
        assert_eq!(spec.required_permission, 2);
        assert!(spec.is_builtin);
    }

    #[test]
    fn test_time_range() {
        let range = PluginTimeRange {
            start: Utc::now(),
            end: Utc::now(),
        };

        let json = serde_json::to_string(&range).unwrap();
        let _deserialized: PluginTimeRange = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_statistics() {
        let stats = Statistics {
            event_count: 100,
            view_count: 10,
            concept_count: 5,
            entity_count: 20,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let _deserialized: Statistics = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_entity_filter() {
        let filter = EntityFilter {
            entity_types: Some(vec!["person".to_string(), "location".to_string()]),
            min_confidence: Some(0.8),
            limit: Some(50),
        };

        let json = serde_json::to_string(&filter).unwrap();
        let _deserialized: EntityFilter = serde_json::from_str(&json).unwrap();
    }

    /// Mock plugin for testing
    struct MockPlugin {
        metadata: PluginMetadata,
    }

    impl MockPlugin {
        fn new(id: &str, permission: MemoryPermission) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: id.to_string(),
                    name: format!("Mock Plugin {}", id),
                    version: "1.0.0".to_string(),
                    description: "A mock plugin for testing".to_string(),
                    required_permission: permission,
                    author: "Test".to_string(),
                    supported_events: vec![],
                    is_builtin: false,
                },
            }
        }
    }

    #[async_trait]
    impl UserPlugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn on_event(
            &self,
            _event: &EventNotification,
            _context: &PluginContext,
        ) -> Result<PluginOutput> {
            Ok(PluginOutput::AnalysisComplete)
        }

        async fn on_query(&self, _query: &str, _context: &PluginContext) -> Result<PluginResponse> {
            Ok(PluginResponse {
                content: "Mock response".to_string(),
                sources: vec![],
                confidence: 1.0,
                metadata: serde_json::json!({}),
                timestamp: Utc::now(),
            })
        }

        fn subscriptions(&self) -> &[EventSubscription] {
            &[]
        }

        async fn cleanup(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_plugins, 0);
        assert_eq!(stats.healthy_plugins, 0);
    }

    #[tokio::test]
    async fn test_plugin_manager_install() {
        let manager = PluginManager::new();
        let plugin = Arc::new(MockPlugin::new("test1", MemoryPermission::ReadOnly));

        let metadata = manager
            .install(plugin, MemoryPermission::ReadOnly)
            .await
            .unwrap();

        assert_eq!(metadata.id, "test1");

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_plugins, 1);
        assert_eq!(stats.healthy_plugins, 1);
    }

    #[tokio::test]
    async fn test_plugin_manager_install_insufficient_permission() {
        let manager = PluginManager::new();
        let plugin = Arc::new(MockPlugin::new("test2", MemoryPermission::ReadWriteDerived));

        // Try to install with lower permission than required
        let result = manager.install(plugin, MemoryPermission::ReadOnly).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plugin_manager_check_permission() {
        let manager = PluginManager::new();
        let plugin = Arc::new(MockPlugin::new("test3", MemoryPermission::ReadWriteEvents));

        manager
            .install(plugin, MemoryPermission::ReadWriteEvents)
            .await
            .unwrap();

        // Should have ReadWriteEvents permission
        assert!(manager
            .check_permission("test3", MemoryPermission::ReadWriteEvents)
            .await
            .unwrap());

        // Should have ReadWriteDerived permission (lower level)
        assert!(manager
            .check_permission("test3", MemoryPermission::ReadWriteDerived)
            .await
            .unwrap());

        // Should have ReadOnly permission (lowest level)
        assert!(manager
            .check_permission("test3", MemoryPermission::ReadOnly)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_plugin_manager_list_plugins() {
        let manager = PluginManager::new();

        let plugin1 = Arc::new(MockPlugin::new("plugin1", MemoryPermission::ReadOnly));
        let plugin2 = Arc::new(MockPlugin::new("plugin2", MemoryPermission::ReadWriteDerived));

        manager
            .install(plugin1, MemoryPermission::ReadOnly)
            .await
            .unwrap();
        manager
            .install(plugin2, MemoryPermission::ReadWriteDerived)
            .await
            .unwrap();

        let plugins = manager.list_plugins().await;
        assert_eq!(plugins.len(), 2);
    }

    #[tokio::test]
    async fn test_plugin_manager_health_check() {
        let manager = PluginManager::new();
        let plugin = Arc::new(MockPlugin::new("health_test", MemoryPermission::ReadOnly));

        manager
            .install(plugin, MemoryPermission::ReadOnly)
            .await
            .unwrap();

        let results = manager.health_check_all().await;
        assert_eq!(results.len(), 1);
        assert!(results.get("health_test").unwrap_or(&false));
    }

    #[tokio::test]
    async fn test_plugin_timeout_config_default() {
        let config = PluginTimeoutConfig::default();
        assert_eq!(config.default_timeout, Duration::from_secs(30));
        assert_eq!(config.init_timeout, Duration::from_secs(60));
        assert_eq!(config.cleanup_timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_isolated_plugin_metadata() {
        let plugin = Arc::new(MockPlugin::new("meta_test", MemoryPermission::ReadOnly));
        let isolated = IsolatedPlugin::new(plugin, MemoryPermission::ReadOnly, 3);

        assert_eq!(isolated.metadata().id, "meta_test");
        assert_eq!(isolated.permission(), MemoryPermission::ReadOnly);
    }

    #[tokio::test]
    async fn test_isolated_plugin_restart_count() {
        let plugin = Arc::new(MockPlugin::new("restart_test", MemoryPermission::ReadOnly));
        let isolated = IsolatedPlugin::new(plugin, MemoryPermission::ReadOnly, 3);

        assert_eq!(isolated.restart_count().await, 0);
        assert!(isolated.can_restart().await);

        isolated.increment_restart_count().await;
        assert_eq!(isolated.restart_count().await, 1);
    }

    #[tokio::test]
    async fn test_plugin_manager_stats() {
        let manager = PluginManager::new();

        let plugin1 = Arc::new(MockPlugin::new("stats1", MemoryPermission::ReadOnly));
        let plugin2 = Arc::new(MockPlugin::new("stats2", MemoryPermission::ReadWriteDerived));

        manager
            .install(plugin1, MemoryPermission::ReadOnly)
            .await
            .unwrap();
        manager
            .install(plugin2, MemoryPermission::ReadWriteDerived)
            .await
            .unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_plugins, 2);
        assert_eq!(stats.healthy_plugins, 2);
    }

    #[tokio::test]
    async fn test_plugin_manager_register_spec() {
        let manager = PluginManager::new();

        let spec = PluginSpec {
            id: "spec_test".to_string(),
            name: "Spec Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test spec".to_string(),
            required_permission: 1,
            author: "Test".to_string(),
            executable: None,
            is_builtin: false,
        };

        manager.register_spec(spec).await.unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.registered_specs, 1);
    }

    // ========== CommandRouter Tests ==========

    #[test]
    fn test_parse_command_plugin_call() {
        let manager = PluginManager::new();
        let router = CommandRouter::new(Arc::new(manager), "test_user".to_string());

        // Test @plugin_name query pattern
        let cmd = router.parse_command("@decision 我应该怎么选择？");
        assert_eq!(
            cmd,
            ParsedCommand::PluginCall {
                plugin: "decision".to_string(),
                query: "我应该怎么选择？".to_string()
            }
        );
    }

    #[test]
    fn test_parse_command_default_query() {
        let manager = PluginManager::new();
        let router = CommandRouter::new(Arc::new(manager), "test_user".to_string());

        // Test default query (no @ command)
        let cmd = router.parse_command("今天天气怎么样？");
        assert_eq!(
            cmd,
            ParsedCommand::DefaultQuery {
                query: "今天天气怎么样？".to_string()
            }
        );
    }

    #[test]
    fn test_parse_command_whitespace() {
        let manager = PluginManager::new();
        let router = CommandRouter::new(Arc::new(manager), "test_user".to_string());

        // Test with leading/trailing whitespace
        let cmd = router.parse_command("  @心理分析 我最近感觉压力很大  ");
        assert_eq!(
            cmd,
            ParsedCommand::PluginCall {
                plugin: "心理分析".to_string(),
                query: "我最近感觉压力很大".to_string()
            }
        );
    }

    #[test]
    fn test_parse_command_plugin_name_with_underscores() {
        let manager = PluginManager::new();
        let router = CommandRouter::new(Arc::new(manager), "test_user".to_string());

        // Test plugin name with underscores
        let cmd = router.parse_command("@my_custom_plugin test query");
        assert_eq!(
            cmd,
            ParsedCommand::PluginCall {
                plugin: "my_custom_plugin".to_string(),
                query: "test query".to_string()
            }
        );
    }

    #[test]
    fn test_parse_command_empty_query() {
        let manager = PluginManager::new();
        let router = CommandRouter::new(Arc::new(manager), "test_user".to_string());

        // Test default query with no content
        let cmd = router.parse_command("");
        assert_eq!(
            cmd,
            ParsedCommand::DefaultQuery {
                query: "".to_string()
            }
        );
    }

    #[test]
    fn test_parsed_command_equality() {
        let cmd1 = ParsedCommand::PluginCall {
            plugin: "test".to_string(),
            query: "hello".to_string(),
        };
        let cmd2 = ParsedCommand::PluginCall {
            plugin: "test".to_string(),
            query: "hello".to_string(),
        };
        let cmd3 = ParsedCommand::PluginCall {
            plugin: "test".to_string(),
            query: "world".to_string(),
        };

        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }

    #[test]
    fn test_command_response_serialization() {
        let response = PluginResponse {
            content: "Test".to_string(),
            sources: vec![],
            confidence: 0.9,
            metadata: serde_json::json!({}),
            timestamp: Utc::now(),
        };

        let cmd_resp = CommandResponse::Plugin(response.clone());
        let json = serde_json::to_string(&cmd_resp).unwrap();
        let _deserialized: CommandResponse = serde_json::from_str(&json).unwrap();
    }

    #[tokio::test]
    async fn test_command_router_creation() {
        let manager = PluginManager::new();
        let router = CommandRouter::new(Arc::new(manager), "user123".to_string());

        assert!(router.default_plugin().is_none());
        assert_eq!(router.user_id, "user123");
    }

    #[tokio::test]
    async fn test_command_router_set_default() {
        let manager = PluginManager::new();
        let mut router = CommandRouter::new(Arc::new(manager), "user123".to_string());

        router.set_default_plugin("deeptalk".to_string());
        assert_eq!(router.default_plugin(), Some("deeptalk"));
    }

    #[tokio::test]
    async fn test_command_router_parse_and_route() {
        let manager = Arc::new(PluginManager::new());

        // Install a test plugin
        let plugin = Arc::new(MockPlugin::new("test_plugin", MemoryPermission::ReadOnly));
        manager.install(plugin, MemoryPermission::ReadOnly).await.unwrap();

        let mut router = CommandRouter::new(manager.clone(), "user123".to_string());
        router.set_default_plugin("test_plugin".to_string());

        // Test parsing
        let cmd = router.parse_command("@test_plugin hello");
        assert!(matches!(cmd, ParsedCommand::PluginCall { .. }));

        let cmd2 = router.parse_command("default query");
        assert!(matches!(cmd2, ParsedCommand::DefaultQuery { .. }));
    }

    #[tokio::test]
    async fn test_command_response_error_variant() {
        let error_resp = CommandResponse::Error("Plugin not found".to_string());

        match error_resp {
            CommandResponse::Error(msg) => {
                assert_eq!(msg, "Plugin not found");
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[tokio::test]
    async fn test_parsed_command_serialization() {
        let cmd = ParsedCommand::PluginCall {
            plugin: "decision".to_string(),
            query: "help me decide".to_string(),
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: ParsedCommand = serde_json::from_str(&json).unwrap();

        assert_eq!(cmd, deserialized);
    }
}
