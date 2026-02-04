//! Agent Actor Model Implementation
//!
//! This module implements agents using the actix framework for concurrent,
//! message-based processing with proper isolation and error handling.
//!
//! # Design Principles (HEAD.md)
//! - **插件沙箱隔离**: Actor崩溃不影响系统
//! - **闭联回路**: Agent输出存回记忆系统
//!
//! # Skill Reference
//! - docs/skills/plugin_permission_system.md

use actix::prelude::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::agents::{Agent, AgentPermissions, AgentRepository};
use crate::error::{DirSoulError, Result};
use crate::pattern_detector::{DetectionTimeRange, PatternDetector};

/// Actor execution context with database and AI access
#[derive(Clone)]
pub struct ActorContext {
    pub database_url: String,
    pub user_id: String,
}

impl ActorContext {
    pub fn new(database_url: String, user_id: String) -> Self {
        Self {
            database_url,
            user_id,
        }
    }

    pub fn connect(&self) -> Result<PgConnection> {
        PgConnection::establish(&self.database_url)
            .map_err(|e| DirSoulError::DatabaseConnection(e))
    }
}

/// Message sent to agents for querying memory
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "AgentResponse")]
pub struct QueryMessage {
    pub query_id: Uuid,
    pub user_id: String,
    pub query: String,
    pub timestamp: DateTime<Utc>,
}

/// Message sent to agents when new events occur
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "AgentOutput")]
pub struct EventNotification {
    pub event_id: Uuid,
    pub user_id: String,
    pub action: String,
    pub target: String,
    pub timestamp: DateTime<Utc>,
}

/// Response from agents to queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub response_id: Uuid,
    pub query_id: Uuid,
    pub agent_id: Uuid,
    pub content: String,
    pub sources: Vec<Uuid>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// Output from agents after processing events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentOutput {
    /// Agent generated new views
    ViewsCreated(Vec<Uuid>),

    /// Agent logged recommendations
    RecommendationLogged {
        recommendation_id: Uuid,
        content: String,
    },

    /// Agent performed analysis but no output
    AnalysisComplete,

    /// Agent encountered error
    Error(String),
}

/// Pattern analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysisResponse {
    pub request_id: Uuid,
    pub patterns_detected: usize,
    pub views_created: Vec<Uuid>,
    pub analysis_time_ms: i64,
}

/// Statistics from cognitive layer
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CognitiveStatistics {
    pub pattern_count: usize,
    pub view_count: usize,
    pub concept_count: usize,
}

/// Cognitive Assistant Agent
///
/// Analyzes patterns and generates views about user behavior
pub struct CognitiveAssistantAgent {
    agent_id: Uuid,
    user_id: String,
    permissions: AgentPermissions,
    context: ActorContext,
}

impl CognitiveAssistantAgent {
    pub fn new(agent: Agent, context: ActorContext) -> Result<Self> {
        let permissions = agent.get_permissions()?;
        Ok(Self {
            agent_id: agent.agent_id,
            user_id: agent.user_id,
            permissions,
            context,
        })
    }
}

impl Actor for CognitiveAssistantAgent {
    type Context = actix::Context<Self>;
}

impl Handler<QueryMessage> for CognitiveAssistantAgent {
    type Result = MessageResult<QueryMessage>;

    fn handle(&mut self, msg: QueryMessage, _ctx: &mut Self::Context) -> Self::Result {
        // Update last_used_at
        if let Ok(mut conn) = self.context.connect() {
            let _ = AgentRepository::mark_used(&mut conn, self.agent_id);
        }

        // Check if can generate views
        if !self.permissions.can_modify_views {
            return MessageResult(AgentResponse {
                response_id: Uuid::new_v4(),
                query_id: msg.query_id,
                agent_id: self.agent_id,
                content: "I don't have permission to generate views".to_string(),
                sources: vec![],
                confidence: 0.0,
                created_at: Utc::now(),
                metadata: serde_json::json!({"error": "permission_denied"}),
            });
        }

        // Query statistics
        let stats = self.query_statistics().unwrap_or_default();

        let content = format!(
            "Cognitive analysis: Found {} patterns in your data. \
            Total views: {}, Active concepts: {}",
            stats.pattern_count, stats.view_count, stats.concept_count
        );

        MessageResult(AgentResponse {
            response_id: Uuid::new_v4(),
            query_id: msg.query_id,
            agent_id: self.agent_id,
            content,
            sources: vec![],
            confidence: 0.85,
            created_at: Utc::now(),
            metadata: serde_json::json!({
                "agent_type": "cognitive",
                "statistics": stats
            }),
        })
    }
}

impl Handler<EventNotification> for CognitiveAssistantAgent {
    type Result = MessageResult<EventNotification>;

    fn handle(&mut self, _msg: EventNotification, _ctx: &mut Self::Context) -> Self::Result {
        // Cognitive assistant observes events but doesn't create them
        MessageResult(AgentOutput::AnalysisComplete)
    }
}

impl CognitiveAssistantAgent {
    fn query_statistics(&self) -> Result<CognitiveStatistics> {
        let mut conn = self.context.connect()?;

        use crate::schema::cognitive_views::dsl::*;
        use crate::schema::stable_concepts::dsl as concepts;

        let active_status = String::from(crate::cognitive::ViewStatus::Active);

        let view_count: i64 = cognitive_views
            .filter(user_id.eq(&self.user_id))
            .filter(status.eq(active_status))
            .count()
            .get_result(&mut conn)?;

        let concept_count: i64 = concepts::stable_concepts
            .filter(concepts::user_id.eq(&self.user_id))
            .filter(concepts::is_deprecated.eq(false))
            .count()
            .get_result(&mut conn)?;

        Ok(CognitiveStatistics {
            pattern_count: 0,
            view_count: view_count as usize,
            concept_count: concept_count as usize,
        })
    }
}

/// Decision Helper Agent
///
/// Provides decision support and logs recommendations
pub struct DecisionHelperAgent {
    agent_id: Uuid,
    user_id: String,
    permissions: AgentPermissions,
    context: ActorContext,
}

impl DecisionHelperAgent {
    pub fn new(agent: Agent, context: ActorContext) -> Result<Self> {
        let permissions = agent.get_permissions()?;
        Ok(Self {
            agent_id: agent.agent_id,
            user_id: agent.user_id,
            permissions,
            context,
        })
    }
}

impl Actor for DecisionHelperAgent {
    type Context = actix::Context<Self>;
}

impl Handler<QueryMessage> for DecisionHelperAgent {
    type Result = MessageResult<QueryMessage>;

    fn handle(&mut self, msg: QueryMessage, _ctx: &mut Self::Context) -> Self::Result {
        // Update last_used_at
        if let Ok(mut conn) = self.context.connect() {
            let _ = AgentRepository::mark_used(&mut conn, self.agent_id);
        }

        // Check if can create events
        if !self.permissions.can_create_events {
            return MessageResult(AgentResponse {
                response_id: Uuid::new_v4(),
                query_id: msg.query_id,
                agent_id: self.agent_id,
                content: "I don't have permission to log recommendations".to_string(),
                sources: vec![],
                confidence: 0.0,
                created_at: Utc::now(),
                metadata: serde_json::json!({"error": "permission_denied"}),
            });
        }

        let content = format!(
            "Decision analysis for: '{}'\n\nBased on your patterns, I recommend: \
            Consider the long-term impact and align with your values.",
            msg.query
        );

        MessageResult(AgentResponse {
            response_id: Uuid::new_v4(),
            query_id: msg.query_id,
            agent_id: self.agent_id,
            content,
            sources: vec![],
            confidence: 0.75,
            created_at: Utc::now(),
            metadata: serde_json::json!({
                "agent_type": "decision",
                "has_recommendation": false
            }),
        })
    }
}

impl Handler<EventNotification> for DecisionHelperAgent {
    type Result = MessageResult<EventNotification>;

    fn handle(&mut self, _msg: EventNotification, _ctx: &mut Self::Context) -> Self::Result {
        // Decision helper observes events
        MessageResult(AgentOutput::AnalysisComplete)
    }
}

/// Agent Manager - manages all agent actors
///
/// Responsible for spawning agents, routing messages, and monitoring health
pub struct AgentManager {
    database_url: String,
    agent_registry: HashMap<String, Uuid>,
}

impl AgentManager {
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            agent_registry: HashMap::new(),
        }
    }

    pub fn register_agent(&mut self, agent_type: String, agent_id: Uuid) {
        self.agent_registry.insert(agent_type, agent_id);
    }

    fn get_agent_id_by_type(&self, agent_type: &str) -> Option<Uuid> {
        self.agent_registry.get(agent_type).copied()
    }

    pub fn load_user_agents(&mut self, user_id: &str) -> Result<()> {
        let mut conn = PgConnection::establish(&self.database_url)
            .map_err(|e| DirSoulError::DatabaseConnection(e))?;

        let agents_list = AgentRepository::find_active_by_user(&mut conn, user_id)?;

        for agent in agents_list {
            self.register_agent(agent.agent_type.clone(), agent.agent_id);
        }

        Ok(())
    }

    fn create_context(&self, user_id: &str) -> ActorContext {
        ActorContext::new(self.database_url.clone(), user_id.to_string())
    }

    pub async fn route_query(&self, user_id: &str, query: &str) -> Result<AgentResponse> {
        if let Some(cognitive_id) = self.get_agent_id_by_type("cognitive") {
            let mut conn = PgConnection::establish(&self.database_url)
                .map_err(|e| DirSoulError::DatabaseConnection(e))?;

            let agent = AgentRepository::find_by_id(&mut conn, cognitive_id)?;
            let context = self.create_context(user_id);

            let addr = CognitiveAssistantAgent::new(agent, context)?.start();

            let msg = QueryMessage {
                query_id: Uuid::new_v4(),
                user_id: user_id.to_string(),
                query: query.to_string(),
                timestamp: Utc::now(),
            };

            let response = addr.send(msg).await
                .map_err(|e| DirSoulError::ExternalError(e.to_string()))?;

            return Ok(response);
        }

        Err(DirSoulError::NotFound("No suitable agent found".to_string()))
    }

    pub async fn route_decision_query(&self, user_id: &str, query: &str) -> Result<AgentResponse> {
        if let Some(decision_id) = self.get_agent_id_by_type("decision") {
            let mut conn = PgConnection::establish(&self.database_url)
                .map_err(|e| DirSoulError::DatabaseConnection(e))?;

            let agent = AgentRepository::find_by_id(&mut conn, decision_id)?;
            let context = self.create_context(user_id);

            let addr = DecisionHelperAgent::new(agent, context)?.start();

            let msg = QueryMessage {
                query_id: Uuid::new_v4(),
                user_id: user_id.to_string(),
                query: query.to_string(),
                timestamp: Utc::now(),
            };

            let response = addr.send(msg).await
                .map_err(|e| DirSoulError::ExternalError(e.to_string()))?;

            return Ok(response);
        }

        Err(DirSoulError::NotFound("No suitable agent found".to_string()))
    }
}

/// Task for running pattern analysis
pub struct PatternAnalysisTask {
    pub user_id: String,
    pub time_range: DetectionTimeRange,
}

impl Message for PatternAnalysisTask {
    type Result = PatternAnalysisResponse;
}

/// Actor for scheduled pattern analysis
pub struct PatternAnalysisActor {
    context: ActorContext,
}

impl PatternAnalysisActor {
    pub fn new(database_url: String, user_id: String) -> Self {
        Self {
            context: ActorContext::new(database_url, user_id),
        }
    }
}

impl Actor for PatternAnalysisActor {
    type Context = Context<Self>;
}

impl Handler<PatternAnalysisTask> for PatternAnalysisActor {
    type Result = MessageResult<PatternAnalysisTask>;

    fn handle(&mut self, msg: PatternAnalysisTask, _ctx: &mut Self::Context) -> Self::Result {
        let start = Utc::now();

        let mut conn = match self.context.connect() {
            Ok(c) => c,
            Err(_) => {
                return MessageResult(PatternAnalysisResponse {
                    request_id: Uuid::new_v4(),
                    patterns_detected: 0,
                    views_created: vec![],
                    analysis_time_ms: 0,
                })
            }
        };

        let detector = PatternDetector::new();

        let result = match detector.detect_patterns(&mut conn, &msg.user_id, msg.time_range) {
            Ok(r) => r,
            Err(_) => {
                return MessageResult(PatternAnalysisResponse {
                    request_id: Uuid::new_v4(),
                    patterns_detected: 0,
                    views_created: vec![],
                    analysis_time_ms: 0,
                })
            }
        };

        let elapsed = (Utc::now() - start).num_milliseconds();

        MessageResult(PatternAnalysisResponse {
            request_id: Uuid::new_v4(),
            patterns_detected: result.patterns.len(),
            views_created: vec![],
            analysis_time_ms: elapsed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_message_serialization() {
        let msg = QueryMessage {
            query_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            query: "What are my patterns?".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let _deserialized: QueryMessage = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_agent_response_serialization() {
        let response = AgentResponse {
            response_id: Uuid::new_v4(),
            query_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            content: "Test response".to_string(),
            sources: vec![],
            confidence: 0.9,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        let json = serde_json::to_string(&response).unwrap();
        let _deserialized: AgentResponse = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_cognitive_statistics_serialization() {
        let stats = CognitiveStatistics {
            pattern_count: 10,
            view_count: 5,
            concept_count: 3,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let _deserialized: CognitiveStatistics = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_event_notification_serialization() {
        let notification = EventNotification {
            event_id: Uuid::new_v4(),
            user_id: "test_user".to_string(),
            action: "喝".to_string(),
            target: "咖啡".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&notification).unwrap();
        let _deserialized: EventNotification = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_agent_output_variants() {
        let output1 = AgentOutput::ViewsCreated(vec![Uuid::new_v4()]);
        let output2 = AgentOutput::RecommendationLogged {
            recommendation_id: Uuid::new_v4(),
            content: "Test recommendation".to_string(),
        };
        let output3 = AgentOutput::AnalysisComplete;
        let output4 = AgentOutput::Error("Test error".to_string());

        // Verify all variants can be created
        let _ = (output1, output2, output3, output4);
    }

    #[test]
    fn test_actor_context() {
        let ctx = ActorContext::new(
            "postgresql://localhost/test".to_string(),
            "test_user".to_string(),
        );

        assert_eq!(ctx.user_id, "test_user");
    }
}
