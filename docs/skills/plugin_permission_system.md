# Skill: Plugin Permission System

> **Purpose**: Ensure plugin sandbox isolation and permission hierarchy, enabling safe plugin development that prevents uncontrolled memory access.

---

## Permission Hierarchy

### Declarative Permission Levels

```rust
/// Memory access permission levels (principle of least privilege)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemoryPermission {
    /// Read-only access to aggregated data
    /// - Can query event counts, trends, statistics
    /// - Cannot read individual memory contents
    /// - Example: "How many times did I exercise this week?"
    ReadOnly = 1,

    /// Read + write access to derived views only
    /// - Can read and generate cognitive hypotheses
    /// - Cannot modify raw events or entities
    /// - Example: Psychology plugin analyzing patterns
    ReadWriteDerived = 2,

    /// Read + write access to event layer
    /// - Can create new events from plugin output
    /// - Cannot modify raw memories
    /// - Example: Decision plugin logging recommendations
    ReadWriteEvents = 3,

    /// Full access (system internal only)
    /// - Can access raw memories, encryption keys
    /// - NEVER granted to plugins
    #[doc(hidden)]
    Full = 99,
}

impl MemoryPermission {
    /// Check if this permission allows the requested action
    pub fn can_read_raw(&self) -> bool {
        *self >= Self::ReadWriteEvents
    }

    pub fn can_write_events(&self) -> bool {
        *self >= Self::ReadWriteEvents
    }

    pub fn can_write_views(&self) -> bool {
        *self >= Self::ReadWriteDerived
    }

    pub fn can_modify_entities(&self) -> bool {
        *self >= Self::ReadWriteEvents
    }
}
```

### Permission Matrix

| Action | ReadOnly | ReadWriteDerived | ReadWriteEvents | Full |
|--------|----------|------------------|-----------------|------|
| Query statistics | ✅ | ✅ | ✅ | ✅ |
| Read derived views | ✅ | ✅ | ✅ | ✅ |
| Write derived views | ❌ | ✅ | ✅ | ✅ |
| Read events | ❌ | ❌ | ✅ | ✅ |
| Write events | ❌ | ❌ | ✅ | ✅ |
| Read raw memories | ❌ | ❌ | ❌ | ✅ |
| Modify entities | ❌ | ❌ | ✅ | ✅ |
| Access encryption keys | ❌ | ❌ | ❌ | ✅ |

---

## Plugin Interface Definition

### Core Plugin Trait

```rust
/// Base plugin trait with permission enforcement
#[async_trait]
pub trait UserPlugin: Send + Sync {
    /// Plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Called when new event arrives
    async fn on_event(&self, event: &Event, context: &PluginContext)
        -> Result<PluginOutput>;

    /// Called when user queries the plugin
    async fn on_query(&self, query: &str, context: &PluginContext)
        -> Result<PluginResponse>;

    /// Subscribe to specific event types
    fn subscriptions(&self) -> &[EventSubscription];

    /// Cleanup on unload
    async fn cleanup(&self) -> Result<()>;
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub required_permission: MemoryPermission,
    pub author: String,
}

/// Context passed to plugins with permission-guarded access
pub struct PluginContext {
    pub plugin_id: String,
    pub permission: MemoryPermission,
    memory_interface: Arc<dyn PluginMemoryInterface>,
}

impl PluginContext {
    /// Query with permission check
    pub async fn query_events(
        &self,
        filter: &EventFilter
    ) -> Result<Vec<Event>> {
        if !self.permission.can_read_raw() {
            return Err(Error::PermissionDenied(
                "Plugin does not have permission to read events"
            ));
        }

        self.memory_interface.query_events(filter).await
    }

    /// Create derived view with permission check
    pub async fn create_view(&self, view: DerivedView) -> Result<()> {
        if !self.permission.can_write_views() {
            return Err(Error::PermissionDenied(
                "Plugin does not have permission to write views"
            ));
        }

        self.memory_interface.create_view(view).await
    }
}

/// Memory interface for plugins (permission-guarded proxy)
#[async_trait]
pub trait PluginMemoryInterface: Send + Sync {
    async fn query_events(&self, filter: &EventFilter) -> Result<Vec<Event>>;
    async fn create_view(&self, view: DerivedView) -> Result<()>;
    async fn get_statistics(&self, time_range: TimeRange) -> Result<Statistics>;
}
```

---

## Sandbox Isolation

### Process-Level Isolation

```rust
/// Plugin execution in isolated process
pub struct IsolatedPlugin {
    plugin_id: String,
    child_process: Option<tokio::process::Child>,
    communication: PluginCommunication,
    permission: MemoryPermission,
    timeout: Duration,
}

impl IsolatedPlugin {
    /// Spawn plugin in separate process
    pub async fn spawn(
        plugin_spec: &PluginSpec,
        permission: MemoryPermission
    ) -> Result<Self> {
        let plugin_id = Uuid::new_v4().to_string();

        // Launch plugin as separate process
        let mut child = tokio::process::Command::new(&plugin_spec.executable)
            .arg("--plugin-id")
            .arg(&plugin_id)
            .arg("--permission")
            .arg(format!("{}", permission as i32))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Setup IPC channel
        let communication = PluginCommunication::new(
            child.stdin.take().unwrap(),
            child.stdout.take().unwrap()
        ).await?;

        Ok(Self {
            plugin_id,
            child_process: Some(child),
            communication,
            permission,
            timeout: Duration::from_secs(30),  // 30s timeout
        })
    }

    /// Execute plugin command with timeout
    pub async fn execute(&self, command: PluginCommand) -> Result<PluginResponse> {
        let timeout = tokio::time::timeout(
            self.timeout,
            self.communication.send_command(command)
        );

        timeout.await.map_err(|_| {
            Error::PluginTimeout("Plugin execution timed out".to_string())
        })?
    }

    /// Check if plugin is still alive
    pub async fn is_alive(&self) -> bool {
        if let Some(child) = &self.child_process {
            child.id().is_some()
        } else {
            false
        }
    }

    /// Kill plugin if misbehaving
    pub async fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.child_process.take() {
            child.kill().await?;
            child.wait().await?;
        }
        Ok(())
    }
}
```

### Crash Recovery

```rust
/// Plugin manager with crash recovery
pub struct PluginManager {
    plugins: std::collections::HashMap<String, IsolatedPlugin>,
    plugin_specs: std::collections::HashMap<String, PluginSpec>,
    max_restarts: usize,
    restart_backoff: Duration,
}

impl PluginManager {
    pub async fn start_plugin(
        &mut self,
        plugin_id: &str,
        permission: MemoryPermission
    ) -> Result<()> {
        let spec = self.plugin_specs.get(plugin_id)
            .ok_or_else(|| Error::PluginNotFound(plugin_id.to_string()))?;

        let plugin = IsolatedPlugin::spawn(spec, permission).await?;
        self.plugins.insert(plugin_id.to_string(), plugin);

        Ok(())
    }

    /// Monitor plugin health
    pub async fn monitor(&mut self) {
        let mut crashed = Vec::new();

        for (id, plugin) in &self.plugins {
            if !plugin.is_alive().await {
                crashed.push(id.clone());
            }
        }

        for id in crashed {
            self.handle_crash(&id).await;
        }
    }

    async fn handle_crash(&mut self, plugin_id: &str) {
        let plugin = self.plugins.remove(plugin_id).unwrap();

        // Log crash
        error!("Plugin {} crashed", plugin_id);

        // Get restart count (from persistent storage)
        let restart_count = self.get_restart_count(plugin_id);

        if restart_count < self.max_restarts {
            // Restart with backoff
            tokio::time::sleep(self.restart_backoff * (restart_count as u32)).await;

            let permission = plugin.permission;
            if let Err(e) = self.start_plugin(plugin_id, permission).await {
                error!("Failed to restart plugin {}: {}", plugin_id, e);
            }
        } else {
            error!("Plugin {} exceeded max restarts, giving up", plugin_id);
        }
    }
}
```

---

## @ Command Routing

### Command Parser

```rust
/// Parse and route @plugin commands
pub struct CommandRouter {
    plugins: std::collections::HashMap<String, IsolatedPlugin>,
    default_plugin: Option<String>,  // DeepTalk
}

impl CommandRouter {
    /// Parse user input for @ commands
    pub fn parse_command(&self, input: &str) -> ParsedCommand {
        // Pattern: @plugin_name query text
        let at_regex = regex::Regex::new(r"@(\w+)\s+(.+)").unwrap();

        if let Some(caps) = at_regex.captures(input) {
            let plugin_name = caps.get(1).unwrap().as_str();
            let query = caps.get(2).unwrap().as_str();

            ParsedCommand::PluginCall {
                plugin: plugin_name.to_string(),
                query: query.to_string(),
            }
        } else {
            // No @ command, use default plugin (DeepTalk)
            ParsedCommand::DefaultQuery {
                query: input.to_string(),
            }
        }
    }

    /// Route command to appropriate plugin
    pub async fn route(&self, input: &str, user_id: &str) -> Result<CommandResponse> {
        let command = self.parse_command(input);

        match command {
            ParsedCommand::PluginCall { plugin, query } => {
                // Check plugin exists
                let plugin_instance = self.plugins.get(&plugin)
                    .ok_or_else(|| Error::PluginNotFound(plugin))?;

                // Execute plugin
                let response = plugin_instance.execute(
                    PluginCommand::Query { query, user_id: user_id.to_string() }
                ).await?;

                // Log plugin interaction as event
                self.log_plugin_interaction(user_id, &plugin, &query, &response).await?;

                Ok(CommandResponse::Plugin(response))
            }

            ParsedCommand::DefaultQuery { query } => {
                // Route to DeepTalk
                if let Some(ref deep_talk) = self.default_plugin {
                    let plugin = self.plugins.get(deep_talk).unwrap();
                    let response = plugin.execute(
                        PluginCommand::Query { query, user_id: user_id.to_string() }
                    ).await?;

                    Ok(CommandResponse::Default(response))
                } else {
                    Err(Error::NoDefaultPlugin)
                }
            }
        }
    }

    /// Log plugin interaction as event
    async fn log_plugin_interaction(
        &self,
        user_id: &str,
        plugin: &str,
        query: &str,
        response: &PluginResponse
    ) -> Result<()> {
        let event = Event {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: Some(user_id.to_string()),
            action: "chat_with_plugin".to_string(),
            target: plugin.to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            metadata: json!({
                "query": query,
                "response_length": response.content.len()
            }),
        };

        // Store in event memory
        // ...

        Ok(())
    }
}

#[derive(Debug)]
enum ParsedCommand {
    PluginCall { plugin: String, query: String },
    DefaultQuery { query: String },
}

#[derive(Debug)]
enum CommandResponse {
    Plugin(PluginResponse),
    Default(PluginResponse),
}
```

---

## Permission Enforcement

### Runtime Permission Check

```rust
/// Permission-checked memory access wrapper
pub struct GuardedMemoryInterface {
    inner: Arc<dyn MemoryStore>,
    permission: MemoryPermission,
    plugin_id: String,
}

#[async_trait]
impl PluginMemoryInterface for GuardedMemoryInterface {
    async fn query_events(&self, filter: &EventFilter) -> Result<Vec<Event>> {
        if !self.permission.can_read_raw() {
            warn!(
                "Plugin {} attempted to query events without permission",
                self.plugin_id
            );
            return Err(Error::PermissionDenied("Insufficient permissions"));
        }

        self.inner.query_events(filter).await
    }

    async fn create_view(&self, view: DerivedView) -> Result<()> {
        if !self.permission.can_write_views() {
            warn!(
                "Plugin {} attempted to create view without permission",
                self.plugin_id
            );
            return Err(Error::PermissionDenied("Insufficient permissions"));
        }

        // Validate view doesn't exceed permission
        self.validate_view_permissions(&view)?;

        self.inner.create_view(view).await
    }

    async fn get_statistics(&self, time_range: TimeRange) -> Result<Statistics> {
        // ReadOnly and above can access statistics
        self.inner.get_statistics(time_range).await
    }
}

impl GuardedMemoryInterface {
    /// Validate view content matches permission level
    fn validate_view_permissions(&self, view: &DerivedView) -> Result<()> {
        // ReadOnly plugins cannot create views
        if self.permission == MemoryPermission::ReadOnly {
            return Err(Error::PermissionDenied("ReadOnly plugins cannot create views"));
        }

        // ReadWriteDerived views must not reference raw event details
        if self.permission == MemoryPermission::ReadWriteDerived {
            // Check hypothesis doesn't contain sensitive data
            if view.hypothesis.contains("raw:") || view.hypothesis.contains("encrypted:") {
                return Err(Error::PermissionDenied("View contains restricted data references"));
            }
        }

        Ok(())
    }
}
```

---

## Example: DeepTalk Default Plugin

```rust
/// DeepTalk - always-on default plugin
pub struct DeepTalkPlugin {
    metadata: PluginMetadata,
}

impl DeepTalkPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "deeptalk".to_string(),
                name: "DeepTalk".to_string(),
                version: "1.0.0".to_string(),
                description: "Deep memory-augmented conversation".to_string(),
                required_permission: MemoryPermission::ReadWriteDerived,
                author: "DirSoul Team".to_string(),
            },
        }
    }
}

#[async_trait]
impl UserPlugin for DeepTalkPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn on_query(&self, query: &str, context: &PluginContext) -> Result<PluginResponse> {
        // DeepTalk has ReadWriteDerived permission
        // - Can read derived views
        // - Can generate new views
        // - Cannot access raw memories directly

        // 1. Retrieve relevant context
        let relevant_views = context.memory_interface
            .query_relevant_views(query)
            .await?;

        // 2. Generate response using retrieved context
        let response = self.generate_response(query, &relevant_views).await?;

        Ok(response)
    }

    async fn on_event(&self, _event: &Event, _context: &PluginContext) -> Result<PluginOutput> {
        // DeepTalk observes all events but doesn't modify them
        Ok(PluginOutput::Empty)
    }

    fn subscriptions(&self) -> &[EventSubscription] {
        &[EventSubscription::All]  // Subscribe to all events
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

impl DeepTalkPlugin {
    async fn generate_response(&self, query: &str, views: &[DerivedView]) -> Result<PluginResponse> {
        // Use SLM with retrieved context
        let context_summary: Vec<String> = views.iter()
            .map(|v| v.hypothesis.clone())
            .collect();

        let prompt = format!(
            "User query: {}\n\nRelevant context from memory:\n{}\n\nGenerate a helpful response:",
            query,
            context_summary.join("\n- ")
        );

        // Call to Ollama...
        Ok(PluginResponse {
            content: "Response from SLM".to_string(),
            sources: views.iter().map(|v| v.view_id).collect(),
        })
    }
}
```

---

## Recommended Combinations

Use this skill together with:
- **DeepTalkImplementation**: For default plugin development
- **CognitiveViewGeneration**: For plugin access to cognitive layer
- **EncryptionBestPractices**: For permission-checked encryption key access
- **TestingAndDebugging**: For plugin isolation testing
