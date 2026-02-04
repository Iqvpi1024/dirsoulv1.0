//! Built-in Plugins for DirSoul
//!
//! This module implements built-in plugins that ship with DirSoul:
//! - DecisionPlugin: Decision support and recommendations
//! - PsychologyPlugin: Behavioral pattern analysis and emotional trends
//!
//! # Design Principles (HEAD.md)
//! - **插件对话也是记忆**: 与插件的对话同样进入事件流
//! - **权限分级**: ReadOnly / ReadWriteDerived / ReadWriteEvents
//! - **Prompt外置化**: 从prompts/目录加载模板
//!
//! # Skill Reference
//! - docs/skills/plugin_permission_system.md

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::MemoryPermission;
use crate::deeptalk::{ConversationContext, EmotionalTrend};
use crate::llm_provider::{ChatMessage, ChatResponse, LLMProvider};
use crate::plugin::{
    PluginContext, PluginMetadata, PluginOutput, PluginResponse, UserPlugin,
};
use crate::prompt_manager::PromptManager;
use crate::{DirSoulError, EventNotification, Result};

// ============================================================================
// Decision Plugin
// ============================================================================

/// Decision Helper Plugin
///
/// Provides evidence-based decision support by analyzing past decisions
/// and their outcomes.
pub struct DecisionPlugin {
    /// LLM provider for generating responses
    llm: Arc<dyn LLMProvider>,

    /// Prompt manager for loading prompts
    prompt_manager: Arc<RwLock<PromptManager>>,

    /// Plugin metadata
    metadata: PluginMetadata,

    /// User ID
    user_id: String,
}

impl DecisionPlugin {
    /// Create a new Decision plugin
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        prompt_manager: PromptManager,
        user_id: String,
    ) -> Result<Self> {
        let metadata = PluginMetadata {
            id: "decision".to_string(),
            name: "Decision Helper".to_string(),
            version: "1.0.0".to_string(),
            description: "Decision support plugin based on past experiences".to_string(),
            required_permission: MemoryPermission::ReadWriteDerived,
            author: "DirSoul Team".to_string(),
            supported_events: vec!["decision".to_string(), "choice".to_string()],
            is_builtin: true,
        };

        Ok(Self {
            llm,
            prompt_manager: Arc::new(RwLock::new(prompt_manager)),
            metadata,
            user_id,
        })
    }

    /// Build decision context from user history
    async fn build_decision_context(&self, _query: &str) -> Result<DecisionContext> {
        let mut context = DecisionContext::default();

        // TODO: Implement actual decision pattern retrieval
        context.relevant_events = vec![
            "Last month you chose to join the gym and have been going 3x/week".to_string(),
            "In previous job changes, you prioritized learning opportunities over salary".to_string(),
        ];

        context.beliefs = vec![
            "You value personal growth and learning".to_string(),
            "You tend to prefer well-researched decisions over impulsive ones".to_string(),
        ];

        context.emotional_state = "calm and thoughtful".to_string();

        Ok(context)
    }

    /// Generate decision analysis
    async fn generate_decision_analysis(
        &self,
        query: &str,
        context: &DecisionContext,
    ) -> Result<PluginResponse> {
        // Build prompt
        let prompt = self.build_decision_prompt(query, context).await?;

        // Create messages
        let messages = vec![
            ChatMessage::system(&prompt),
            ChatMessage::user(query),
        ];

        // Call LLM
        let response = self.llm.chat(messages, Some(0.7), None).await?;

        // Extract response text
        let content = match response {
            ChatResponse::Ollama(ollama) => ollama.response,
            ChatResponse::OpenAI(openai) => openai
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default(),
        };

        Ok(PluginResponse {
            content: content.trim().to_string(),
            sources: vec![],
            confidence: 0.75,
            metadata: serde_json::json!({
                "plugin": "decision",
                "model": self.llm.model_name(),
            }),
            timestamp: Utc::now(),
        })
    }

    /// Build decision analysis prompt
    async fn build_decision_prompt(&self, query: &str, context: &DecisionContext) -> Result<String> {
        let mut prompt_mgr = self.prompt_manager.write().await;
        let template = prompt_mgr.load_prompt("decision")?;

        let mut vars = HashMap::new();

        if !context.relevant_events.is_empty() {
            vars.insert("relevant_events".to_string(), context.relevant_events.join("\n"));
        }

        if !context.beliefs.is_empty() {
            vars.insert("beliefs".to_string(), context.beliefs.join("\n"));
        }

        if !context.emotional_state.is_empty() {
            vars.insert("emotional_state".to_string(), context.emotional_state.clone());
        }

        vars.insert("query".to_string(), query.to_string());

        Ok(Self::render_template(&template, vars))
    }

    /// Simple template rendering
    fn render_template(template: &str, vars: HashMap<String, String>) -> String {
        let mut result = template.to_string();

        // Handle conditionals
        for (key, value) in &vars {
            let if_pattern = format!("{{{{#if {}}}}}", key);
            let end_if = "{{/if}}";

            if !value.is_empty() {
                result = result.replace(&if_pattern, "");
                result = result.replace(end_if, "");
            } else {
                // Remove empty conditional blocks
                let start_idx = result.find(&if_pattern);
                if let Some(start) = start_idx {
                    let end_idx = result[start..].find(end_if);
                    if let Some(end) = end_idx {
                        let end_pos = start + end + end_if.len();
                        result.replace_range(start..end_pos, "");
                    }
                }
            }
        }

        // Replace variables
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, &value);
        }

        result
    }
}

/// Decision context for analysis
#[derive(Debug, Clone, Default)]
pub struct DecisionContext {
    /// Relevant past decisions
    pub relevant_events: Vec<String>,

    /// Beliefs and patterns
    pub beliefs: Vec<String>,

    /// Current emotional state
    pub emotional_state: String,
}

#[async_trait]
impl UserPlugin for DecisionPlugin {
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

    async fn on_query(&self, query: &str, _context: &PluginContext) -> Result<PluginResponse> {
        let ctx = self.build_decision_context(query).await?;
        self.generate_decision_analysis(query, &ctx).await
    }

    fn subscriptions(&self) -> &[crate::plugin::EventSubscription] {
        use crate::plugin::EventSubscription;
        // Create a static Vec using Box::leak
        // This is safe because it's only called once and lives for the program duration
        static SUBS: std::sync::OnceLock<Vec<EventSubscription>> = std::sync::OnceLock::new();
        SUBS.get_or_init(|| vec![EventSubscription::Action("decision".to_string())])
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        let mut prompt_mgr = self.prompt_manager.write().await;
        let _ = prompt_mgr.load_prompt("decision")?;
        Ok(())
    }
}

// ============================================================================
// Psychology Plugin
// ============================================================================

/// Psychology Analyzer Plugin
///
/// Analyzes behavioral patterns, emotional trends, and provides
/// psychological insights for self-awareness and growth.
pub struct PsychologyPlugin {
    /// LLM provider
    llm: Arc<dyn LLMProvider>,

    /// Prompt manager
    prompt_manager: Arc<RwLock<PromptManager>>,

    /// Plugin metadata
    metadata: PluginMetadata,

    /// User ID
    user_id: String,
}

impl PsychologyPlugin {
    /// Create a new Psychology plugin
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        prompt_manager: PromptManager,
        user_id: String,
    ) -> Result<Self> {
        let metadata = PluginMetadata {
            id: "psychology".to_string(),
            name: "Psychology Analyzer".to_string(),
            version: "1.0.0".to_string(),
            description: "Behavioral pattern and emotional trend analysis".to_string(),
            required_permission: MemoryPermission::ReadWriteDerived,
            author: "DirSoul Team".to_string(),
            supported_events: vec!["emotion".to_string(), "mood".to_string(), "feeling".to_string()],
            is_builtin: true,
        };

        Ok(Self {
            llm,
            prompt_manager: Arc::new(RwLock::new(prompt_manager)),
            metadata,
            user_id,
        })
    }

    /// Build psychology context
    async fn build_psychology_context(&self, _query: &str) -> Result<PsychologyContext> {
        let mut context = PsychologyContext::default();

        // TODO: Implement actual behavioral pattern retrieval
        context.recent_events = vec![
            "This week you've been going to bed later than usual".to_string(),
            "Exercise frequency has decreased over the past month".to_string(),
        ];

        context.behavioral_patterns = vec![
            "You tend to procrastinate when feeling overwhelmed".to_string(),
            "Social interaction boosts your mood significantly".to_string(),
        ];

        context.beliefs = vec![
            "You value productivity but struggle with work-life balance".to_string(),
        ];

        context.emotional_state = EmotionalTrend::Neutral;

        Ok(context)
    }

    /// Generate psychological analysis
    async fn generate_psychology_analysis(
        &self,
        query: &str,
        context: &PsychologyContext,
    ) -> Result<PluginResponse> {
        let prompt = self.build_psychology_prompt(query, context).await?;

        let messages = vec![
            ChatMessage::system(&prompt),
            ChatMessage::user(query),
        ];

        let response = self.llm.chat(messages, Some(0.7), None).await?;

        let content = match response {
            ChatResponse::Ollama(ollama) => ollama.response,
            ChatResponse::OpenAI(openai) => openai
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default(),
        };

        Ok(PluginResponse {
            content: content.trim().to_string(),
            sources: vec![],
            confidence: 0.7,
            metadata: serde_json::json!({
                "plugin": "psychology",
                "model": self.llm.model_name(),
                "emotional_trend": format!("{:?}", context.emotional_state),
            }),
            timestamp: Utc::now(),
        })
    }

    /// Build psychology analysis prompt
    async fn build_psychology_prompt(
        &self,
        query: &str,
        context: &PsychologyContext,
    ) -> Result<String> {
        let mut prompt_mgr = self.prompt_manager.write().await;
        let template = prompt_mgr.load_prompt("psychology")?;

        let mut vars = HashMap::new();

        let emotional_state = format!(
            "{} ({})",
            context.emotional_state.emoji(),
            context.emotional_state.description()
        );
        vars.insert("emotional_state".to_string(), emotional_state);

        if !context.recent_events.is_empty() {
            vars.insert("recent_events".to_string(), context.recent_events.join("\n"));
        }

        if !context.behavioral_patterns.is_empty() {
            vars.insert("behavioral_patterns".to_string(), context.behavioral_patterns.join("\n"));
        }

        if !context.beliefs.is_empty() {
            vars.insert("beliefs".to_string(), context.beliefs.join("\n"));
        }

        vars.insert("query".to_string(), query.to_string());

        Ok(Self::render_template(&template, vars))
    }

    /// Simple template rendering (same as DecisionPlugin)
    fn render_template(template: &str, vars: HashMap<String, String>) -> String {
        let mut result = template.to_string();

        for (key, value) in &vars {
            let if_pattern = format!("{{{{#if {}}}}}", key);
            let end_if = "{{/if}}";

            if !value.is_empty() {
                result = result.replace(&if_pattern, "");
                result = result.replace(end_if, "");
            } else {
                let start_idx = result.find(&if_pattern);
                if let Some(start) = start_idx {
                    let end_idx = result[start..].find(end_if);
                    if let Some(end) = end_idx {
                        let end_pos = start + end + end_if.len();
                        result.replace_range(start..end_pos, "");
                    }
                }
            }
        }

        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, &value);
        }

        result
    }
}

/// Psychology context for analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PsychologyContext {
    /// Recent events
    pub recent_events: Vec<String>,

    /// Behavioral patterns
    pub behavioral_patterns: Vec<String>,

    /// Stable beliefs
    pub beliefs: Vec<String>,

    /// Current emotional trend
    pub emotional_state: EmotionalTrend,
}

#[async_trait]
impl UserPlugin for PsychologyPlugin {
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

    async fn on_query(&self, query: &str, _context: &PluginContext) -> Result<PluginResponse> {
        let ctx = self.build_psychology_context(query).await?;
        self.generate_psychology_analysis(query, &ctx).await
    }

    fn subscriptions(&self) -> &[crate::plugin::EventSubscription] {
        use crate::plugin::EventSubscription;
        static SUBS: std::sync::OnceLock<Vec<EventSubscription>> = std::sync::OnceLock::new();
        SUBS.get_or_init(|| vec![
            EventSubscription::Action("emotion".to_string()),
            EventSubscription::Action("mood".to_string()),
            EventSubscription::Action("feeling".to_string()),
        ])
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        let mut prompt_mgr = self.prompt_manager.write().await;
        let _ = prompt_mgr.load_prompt("psychology")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_context_default() {
        let ctx = DecisionContext::default();
        assert!(ctx.relevant_events.is_empty());
        assert!(ctx.beliefs.is_empty());
        assert!(ctx.emotional_state.is_empty());
    }

    #[test]
    fn test_psychology_context_default() {
        let ctx = PsychologyContext::default();
        assert!(ctx.recent_events.is_empty());
        assert!(ctx.behavioral_patterns.is_empty());
        assert!(ctx.beliefs.is_empty());
        assert_eq!(ctx.emotional_state, EmotionalTrend::Neutral);
    }

    #[test]
    fn test_render_template_basic() {
        let template = "Hello {{name}}";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let result = DecisionPlugin::render_template(template, vars);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_render_template_with_conditionals() {
        let template = "{{#if show}}Content{{/if}}";
        let mut vars = HashMap::new();
        vars.insert("show".to_string(), "yes".to_string());

        let result = DecisionPlugin::render_template(template, vars);
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_render_template_empty_conditional() {
        let template = "{{#if show}}Content{{/if}}";
        let mut vars = HashMap::new();
        vars.insert("show".to_string(), String::new());

        let result = DecisionPlugin::render_template(template, vars);
        assert!(!result.contains("Content"));
    }

    #[test]
    fn test_psychology_context_serialization() {
        let ctx = PsychologyContext {
            recent_events: vec!["event1".to_string()],
            behavioral_patterns: vec!["pattern1".to_string()],
            beliefs: vec!["belief1".to_string()],
            emotional_state: EmotionalTrend::Positive,
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("event1"));
        assert!(json.contains("Positive"));
    }
}
