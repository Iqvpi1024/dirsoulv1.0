//! Agent and Plugin Management Module
//!
//! This module handles agent/plugin registration and permission validation.
//!
//! # Design Principles (HEAD.md)
//! - **插件沙箱隔离**: 插件崩溃不影响系统
//! - **权限分级**: ReadOnly / ReadWriteDerived / ReadWriteEvents
//! - **最小权限原则**: 只授予必要的访问权限
//!
//! # Skill Reference
//! - docs/skills/plugin_permission_system.md

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{DirSoulError, Result};
use crate::schema::agents;

/// Memory permission levels for agents/plugins
///
/// # Hierarchy
/// - **ReadOnly (1)**: Can only query aggregated statistics
/// - **ReadWriteDerived (2)**: Can read/write cognitive views
/// - **ReadWriteEvents (3)**: Full access including event creation
///
/// # Example
/// ```ignore
/// let permission = MemoryPermission::ReadWriteDerived;
/// assert_eq!(permission.as_i32(), 2);
/// assert!(permission.can_modify_views());
/// assert!(!permission.can_create_events());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryPermission {
    /// Level 1: Read-only access to aggregated statistics
    ReadOnly = 1,

    /// Level 2: Can read and write derived views (cognitive_views table)
    ReadWriteDerived = 2,

    /// Level 3: Full access including event creation
    ReadWriteEvents = 3,
}

impl MemoryPermission {
    /// Convert from integer
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::ReadOnly),
            2 => Some(Self::ReadWriteDerived),
            3 => Some(Self::ReadWriteEvents),
            _ => None,
        }
    }

    /// Convert to integer
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    /// Check if this permission allows reading statistics
    pub fn can_read_stats(self) -> bool {
        matches!(self, Self::ReadOnly | Self::ReadWriteDerived | Self::ReadWriteEvents)
    }

    /// Check if this permission allows modifying views
    pub fn can_modify_views(self) -> bool {
        matches!(self, Self::ReadWriteDerived | Self::ReadWriteEvents)
    }

    /// Check if this permission allows creating events
    pub fn can_create_events(self) -> bool {
        matches!(self, Self::ReadWriteEvents)
    }

    /// Check if this permission allows reading entities
    pub fn can_read_entities(self) -> bool {
        matches!(self, Self::ReadWriteDerived | Self::ReadWriteEvents)
    }
}

/// Permission configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPermissions {
    /// Memory access level (1-3)
    pub memory_level: i32,

    /// Whether this agent can create new event memories
    pub can_create_events: bool,

    /// Whether this agent can modify cognitive views
    pub can_modify_views: bool,

    /// Whether this agent can read entity information
    pub can_read_entities: bool,

    /// Specific operations this agent is allowed to perform
    pub allowed_operations: Vec<String>,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            memory_level: 1, // ReadOnly by default
            can_create_events: false,
            can_modify_views: false,
            can_read_entities: false,
            allowed_operations: vec!["query_stats".to_string()],
        }
    }
}

impl AgentPermissions {
    /// Parse from JSONB value
    pub fn from_jsonb(value: &serde_json::Value) -> Result<Self> {
        serde_json::from_value(value.clone())
            .map_err(DirSoulError::from)
    }

    /// Convert to JSONB value
    pub fn to_jsonb(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self)
            .map_err(DirSoulError::from)
    }

    /// Get memory permission level
    pub fn memory_permission(&self) -> Option<MemoryPermission> {
        MemoryPermission::from_i32(self.memory_level)
    }

    /// Validate that requested operation is allowed
    pub fn validate_operation(&self, operation: &str) -> Result<()> {
        if !self.allowed_operations.contains(&operation.to_string()) {
            return Err(DirSoulError::Config(format!(
                "Operation '{}' not allowed for this agent",
                operation
            )));
        }
        Ok(())
    }
}

/// Agent or Plugin model
///
/// Represents both built-in system agents and user-created plugins.
#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
pub struct Agent {
    /// Primary key
    pub agent_id: Uuid,

    /// User who owns this agent
    pub user_id: String,

    /// Display name
    pub name: String,

    /// Agent type (e.g., "cognitive", "decision", "custom")
    pub agent_type: String,

    /// Version string
    pub version: String,

    /// Human-readable description
    pub description: Option<String>,

    /// Author (user or "system")
    pub author: String,

    /// Permission configuration (JSONB)
    pub permissions: serde_json::Value,

    /// Whether this agent is active
    pub is_active: bool,

    /// Whether this is a built-in system agent
    pub is_builtin: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Last used timestamp
    pub last_used_at: Option<DateTime<Utc>>,

    /// Additional metadata (JSONB)
    pub metadata: Option<serde_json::Value>,

    /// Tags (JSONB array)
    pub tags: Option<serde_json::Value>,
}

impl Agent {
    /// Get parsed permissions
    pub fn get_permissions(&self) -> Result<AgentPermissions> {
        AgentPermissions::from_jsonb(&self.permissions)
    }

    /// Check if agent has specific permission level
    pub fn has_memory_level(&self, level: MemoryPermission) -> bool {
        if let Ok(perms) = self.get_permissions() {
            perms.memory_level >= level.as_i32()
        } else {
            false
        }
    }

    /// Validate that agent can perform operation
    pub fn can_perform(&self, operation: &str) -> Result<()> {
        let perms = self.get_permissions()?;
        perms.validate_operation(operation)
    }

    /// Update last_used_at timestamp
    pub fn mark_used(&self, conn: &mut PgConnection) -> Result<()> {
        diesel::update(agents::table.find(self.agent_id))
            .set(agents::last_used_at.eq(Utc::now()))
            .execute(conn)?;
        Ok(())
    }

    /// Check if agent is active
    pub fn is_available(&self) -> bool {
        self.is_active
    }
}

/// New agent for insertion
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = agents)]
pub struct NewAgent {
    pub user_id: String,
    pub name: String,
    pub agent_type: String,
    pub version: String,
    pub description: Option<String>,
    pub author: String,
    pub permissions: serde_json::Value,
    pub is_active: bool,
    pub is_builtin: bool,
    pub metadata: Option<serde_json::Value>,
    pub tags: Option<serde_json::Value>,
}

impl NewAgent {
    /// Create a new built-in system agent
    pub fn new_builtin(
        user_id: &str,
        name: &str,
        agent_type: &str,
        permissions: AgentPermissions,
    ) -> Result<Self> {
        Ok(Self {
            user_id: user_id.to_string(),
            name: name.to_string(),
            agent_type: agent_type.to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: "system".to_string(),
            permissions: permissions.to_jsonb()?,
            is_active: true,
            is_builtin: true,
            metadata: Some(serde_json::json!({"system_agent": true})),
            tags: None,
        })
    }

    /// Create a new user plugin
    pub fn new_plugin(
        user_id: &str,
        name: &str,
        agent_type: &str,
        permissions: AgentPermissions,
        author: &str,
    ) -> Result<Self> {
        Ok(Self {
            user_id: user_id.to_string(),
            name: name.to_string(),
            agent_type: agent_type.to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: author.to_string(),
            permissions: permissions.to_jsonb()?,
            is_active: true,
            is_builtin: false,
            metadata: None,
            tags: None,
        })
    }
}

/// Agent update structure
#[derive(Debug, Clone, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = agents)]
pub struct AgentUpdate {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<serde_json::Value>,
    pub is_active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub tags: Option<serde_json::Value>,
}

/// Agent repository for database operations
pub struct AgentRepository;

impl AgentRepository {
    /// Create a new agent
    pub fn create(conn: &mut PgConnection, new_agent: &NewAgent) -> Result<Agent> {
        let agent = diesel::insert_into(agents::table)
            .values(new_agent)
            .get_result(conn)?;
        Ok(agent)
    }

    /// Get agent by ID
    pub fn find_by_id(conn: &mut PgConnection, agent_id: Uuid) -> Result<Agent> {
        let agent = agents::table.find(agent_id).first(conn)?;
        Ok(agent)
    }

    /// Get agent by user and type
    pub fn find_by_user_and_type(
        conn: &mut PgConnection,
        user_id: &str,
        agent_type: &str,
    ) -> Result<Agent> {
        let agent = agents::table
            .filter(agents::user_id.eq(user_id))
            .filter(agents::agent_type.eq(agent_type))
            .first(conn)?;
        Ok(agent)
    }

    /// Get all agents for a user
    pub fn find_by_user(conn: &mut PgConnection, user_id: &str) -> Result<Vec<Agent>> {
        let agents_list = agents::table
            .filter(agents::user_id.eq(user_id))
            .order(agents::created_at.desc())
            .load(conn)?;
        Ok(agents_list)
    }

    /// Get active agents for a user
    pub fn find_active_by_user(conn: &mut PgConnection, user_id: &str) -> Result<Vec<Agent>> {
        let agents_list = agents::table
            .filter(agents::user_id.eq(user_id))
            .filter(agents::is_active.eq(true))
            .order(agents::last_used_at.desc().nulls_last())
            .load(conn)?;
        Ok(agents_list)
    }

    /// Get built-in agents
    pub fn find_builtin(conn: &mut PgConnection) -> Result<Vec<Agent>> {
        let agents_list = agents::table
            .filter(agents::is_builtin.eq(true))
            .load(conn)?;
        Ok(agents_list)
    }

    /// Update agent
    pub fn update(conn: &mut PgConnection, agent_id: Uuid, update: &AgentUpdate) -> Result<Agent> {
        let agent = diesel::update(agents::table.find(agent_id))
            .set(update)
            .get_result(conn)?;
        Ok(agent)
    }

    /// Delete agent (only non-built-in)
    pub fn delete(conn: &mut PgConnection, agent_id: Uuid) -> Result<()> {
        let agent = Self::find_by_id(conn, agent_id)?;

        if agent.is_builtin {
            return Err(DirSoulError::Config(
                "Cannot delete built-in system agents".to_string(),
            ));
        }

        diesel::delete(agents::table.find(agent_id)).execute(conn)?;
        Ok(())
    }

    /// Activate or deactivate agent
    pub fn set_active(conn: &mut PgConnection, agent_id: Uuid, active: bool) -> Result<Agent> {
        let agent = diesel::update(agents::table.find(agent_id))
            .set(agents::is_active.eq(active))
            .get_result(conn)?;
        Ok(agent)
    }

    /// Update last_used_at
    pub fn mark_used(conn: &mut PgConnection, agent_id: Uuid) -> Result<()> {
        diesel::update(agents::table.find(agent_id))
            .set(agents::last_used_at.eq(Utc::now()))
            .execute(conn)?;
        Ok(())
    }

    /// Get or create built-in agent for user
    pub fn get_or_create_builtin(
        conn: &mut PgConnection,
        user_id: &str,
        agent_type: &str,
        default_permissions: AgentPermissions,
    ) -> Result<Agent> {
        // Try to find existing
        if let Ok(agent) = Self::find_by_user_and_type(conn, user_id, agent_type) {
            return Ok(agent);
        }

        // Create new built-in agent
        let name = match agent_type {
            "cognitive" => "Cognitive Assistant",
            "decision" => "Decision Helper",
            _ => "System Agent",
        };

        let new_agent = NewAgent::new_builtin(user_id, name, agent_type, default_permissions)?;
        Self::create(conn, &new_agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_permission_values() {
        assert_eq!(MemoryPermission::ReadOnly.as_i32(), 1);
        assert_eq!(MemoryPermission::ReadWriteDerived.as_i32(), 2);
        assert_eq!(MemoryPermission::ReadWriteEvents.as_i32(), 3);
    }

    #[test]
    fn test_memory_permission_from_i32() {
        assert_eq!(MemoryPermission::from_i32(1), Some(MemoryPermission::ReadOnly));
        assert_eq!(MemoryPermission::from_i32(2), Some(MemoryPermission::ReadWriteDerived));
        assert_eq!(MemoryPermission::from_i32(3), Some(MemoryPermission::ReadWriteEvents));
        assert_eq!(MemoryPermission::from_i32(4), None);
        assert_eq!(MemoryPermission::from_i32(0), None);
    }

    #[test]
    fn test_readonly_permissions() {
        let perm = MemoryPermission::ReadOnly;
        assert!(perm.can_read_stats());
        assert!(!perm.can_modify_views());
        assert!(!perm.can_create_events());
        assert!(!perm.can_read_entities());
    }

    #[test]
    fn test_readwrite_derived_permissions() {
        let perm = MemoryPermission::ReadWriteDerived;
        assert!(perm.can_read_stats());
        assert!(perm.can_modify_views());
        assert!(!perm.can_create_events());
        assert!(perm.can_read_entities());
    }

    #[test]
    fn test_readwrite_events_permissions() {
        let perm = MemoryPermission::ReadWriteEvents;
        assert!(perm.can_read_stats());
        assert!(perm.can_modify_views());
        assert!(perm.can_create_events());
        assert!(perm.can_read_entities());
    }

    #[test]
    fn test_agent_permissions_default() {
        let perms = AgentPermissions::default();
        assert_eq!(perms.memory_level, 1);
        assert!(!perms.can_create_events);
        assert!(!perms.can_modify_views);
        assert!(!perms.can_read_entities);
        assert_eq!(perms.allowed_operations, vec!["query_stats"]);
    }

    #[test]
    fn test_agent_permissions_serialization() {
        let perms = AgentPermissions {
            memory_level: 2,
            can_create_events: false,
            can_modify_views: true,
            can_read_entities: true,
            allowed_operations: vec!["query_stats".to_string(), "generate_view".to_string()],
        };

        let json = perms.to_jsonb().unwrap();
        let deserialized = AgentPermissions::from_jsonb(&json).unwrap();

        assert_eq!(deserialized.memory_level, 2);
        assert!(deserialized.can_modify_views);
        assert!(deserialized.allowed_operations.contains(&"generate_view".to_string()));
    }

    #[test]
    fn test_agent_permissions_validate_operation() {
        let mut perms = AgentPermissions::default();
        perms.allowed_operations = vec!["query_stats".to_string()];

        assert!(perms.validate_operation("query_stats").is_ok());
        assert!(perms.validate_operation("generate_view").is_err());
    }

    #[test]
    fn test_new_agent_builtin() {
        let perms = AgentPermissions {
            memory_level: 2,
            can_create_events: false,
            can_modify_views: true,
            can_read_entities: true,
            allowed_operations: vec!["query_stats".to_string()],
        };

        let agent = NewAgent::new_builtin("user1", "Test Agent", "test", perms).unwrap();

        assert_eq!(agent.user_id, "user1");
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.author, "system");
        assert!(agent.is_builtin);
        assert!(agent.is_active);
    }

    #[test]
    fn test_new_agent_plugin() {
        let perms = AgentPermissions::default();
        let agent = NewAgent::new_plugin("user1", "My Plugin", "custom", perms, "user1").unwrap();

        assert_eq!(agent.user_id, "user1");
        assert_eq!(agent.author, "user1");
        assert!(!agent.is_builtin);
        assert!(agent.is_active);
    }

    #[test]
    fn test_memory_permission_hierarchy() {
        let readonly = MemoryPermission::ReadOnly;
        let derived = MemoryPermission::ReadWriteDerived;
        let events = MemoryPermission::ReadWriteEvents;

        // Test level comparison
        assert!(derived.as_i32() > readonly.as_i32());
        assert!(events.as_i32() > derived.as_i32());
    }
}
