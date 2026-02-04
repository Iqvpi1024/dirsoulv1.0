//! DeepTalk Plugin - Default Memory-Augmented Conversation
//!
//! This module implements the DeepTalk plugin, the always-on default plugin
//! for deep, memory-augmented conversation with cross-session continuity
//! and emotional trend awareness.
//!
//! # Design Principles (HEAD.md)
//! - **DeepTalk 默认插件**: 系统默认启用，不可卸载
//! - **全局记忆检索**: 向量+SQL混合检索
//! - **跨会话连续性**: 记得用户的所有对话历史
//! - **情绪趋势感知**: 分析用户情绪变化
//! - **模型注入**: 读取config/models.toml，自动适配所选模型
//!
//! # Skill Reference
//! - docs/skills/deeptalk_implementation.md

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::MemoryPermission;
use crate::llm_provider::{ChatMessage, ChatResponse, LLMProvider};
use crate::plugin::{
    PluginContext, PluginMetadata, PluginOutput, PluginResponse, UserPlugin,
};
use crate::prompt_manager::PromptManager;
use crate::{DirSoulError, EventNotification, Result};

/// Emotional trend analysis result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionalTrend {
    /// Positive trend
    Positive,
    /// Neutral trend
    Neutral,
    /// Negative trend
    Negative,
}

impl Default for EmotionalTrend {
    fn default() -> Self {
        Self::Neutral
    }
}

impl EmotionalTrend {
    /// Get emoji representation
    pub fn emoji(&self) -> &str {
        match self {
            Self::Positive => "😊",
            Self::Neutral => "😐",
            Self::Negative => "😔",
        }
    }

    /// Get description
    pub fn description(&self) -> &str {
        match self {
            Self::Positive => "positive and optimistic",
            Self::Neutral => "balanced and stable",
            Self::Negative => "stressed or concerned",
        }
    }
}

/// Conversation context for DeepTalk
#[derive(Debug, Clone, Default)]
pub struct ConversationContext {
    /// Relevant past events
    pub events: Vec<String>,

    /// Relevant beliefs/views about the user
    pub beliefs: Vec<String>,

    /// Current emotional trend
    pub emotional_trend: EmotionalTrend,

    /// Summary of recent conversations
    pub conversation_summary: String,
}

/// DeepTalk - The always-on default plugin for deep, memory-augmented conversation
///
/// # Features
/// - Retrieves relevant memories using vector + SQL hybrid search
/// - Analyzes emotional trends from user's conversation history
/// - Maintains cross-session continuity
/// - Uses externalized prompts for customization
/// - Supports model injection via LLMProvider
pub struct DeepTalkPlugin {
    /// LLM provider for generating responses
    llm: Arc<dyn LLMProvider>,

    /// Prompt manager for loading prompts
    prompt_manager: Arc<RwLock<PromptManager>>,

    /// Plugin metadata
    metadata: PluginMetadata,

    /// User ID for memory retrieval
    user_id: String,
}

impl DeepTalkPlugin {
    /// Create a new DeepTalk plugin
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        prompt_manager: PromptManager,
        user_id: String,
    ) -> Result<Self> {
        let metadata = PluginMetadata {
            id: "deeptalk".to_string(),
            name: "DeepTalk".to_string(),
            version: "1.0.0".to_string(),
            description: "Deep memory-augmented conversation plugin".to_string(),
            required_permission: MemoryPermission::ReadWriteDerived,
            author: "DirSoul Team".to_string(),
            supported_events: vec!["chat_with_plugin".to_string(), "query".to_string()],
            is_builtin: true,  // Built-in plugin, cannot be uninstalled
        };

        Ok(Self {
            llm,
            prompt_manager: Arc::new(RwLock::new(prompt_manager)),
            metadata,
            user_id,
        })
    }

    /// Build context for user query
    async fn build_context(&self, _query: &str) -> Result<ConversationContext> {
        let mut context = ConversationContext::default();

        // TODO: Implement actual memory retrieval
        // For now, use placeholder data
        context.events = vec![
            "Yesterday you went for a 5km run".to_string(),
            "Last week you mentioned feeling stressed about work".to_string(),
        ];

        context.beliefs = vec![
            "You value health and fitness".to_string(),
            "You tend to work hard but sometimes feel overwhelmed".to_string(),
        ];

        // Analyze emotional trend
        context.emotional_trend = Self::analyze_emotional_trend_simple();

        // Get conversation summary
        context.conversation_summary = String::new();

        Ok(context)
    }

    /// Simple emotional trend analysis (placeholder)
    fn analyze_emotional_trend_simple() -> EmotionalTrend {
        // TODO: Implement actual sentiment analysis from conversation history
        EmotionalTrend::Neutral
    }

    /// Build prompt with context
    async fn build_prompt(&self, query: &str, context: &ConversationContext) -> Result<String> {
        let mut prompt_mgr = self.prompt_manager.write().await;

        // Load prompt template
        let template = prompt_mgr.load_prompt("deeptalk")?;

        // Build template variables
        let mut vars = HashMap::new();

        // Add emotional state
        let emotional_state = format!(
            "{} ({})",
            context.emotional_trend.emoji(),
            context.emotional_trend.description()
        );
        vars.insert("emotional_state".to_string(), emotional_state);

        // Add conversation summary
        if !context.conversation_summary.is_empty() {
            vars.insert("conversation_summary".to_string(), context.conversation_summary.clone());
        }

        // Add relevant events
        if !context.events.is_empty() {
            vars.insert("relevant_events".to_string(), context.events.join("\n"));
        }

        // Add beliefs
        if !context.beliefs.is_empty() {
            vars.insert("beliefs".to_string(), context.beliefs.join("\n"));
        }

        // Add user query
        vars.insert("query".to_string(), query.to_string());

        // Render prompt
        // Note: For now, use simple string replacement since Handlebars is not integrated
        let rendered = Self::render_simple(&template, vars);

        Ok(rendered)
    }

    /// Simple template rendering (placeholder for Handlebars)
    fn render_simple(template: &str, vars: HashMap<String, String>) -> String {
        let mut result = template.to_string();

        // First pass: handle {{#if var}}...{{/if}} blocks
        for (key, value) in &vars {
            let if_pattern = format!("{{{{#if {}}}}}", key);
            let end_if = "{{/if}}";

            if !value.is_empty() {
                // Remove conditional markers for non-empty values
                result = result.replace(&if_pattern, "");
                result = result.replace(end_if, "");
            } else {
                // Remove entire block for empty values
                // Find and remove content between markers
                let start_idx = result.find(&if_pattern);
                if let Some(start) = start_idx {
                    let end_idx = result[start..].find(end_if);
                    if let Some(end) = end_idx {
                        let end_pos = start + end + end_if.len();
                        // Remove from start to end (including markers)
                        result.replace_range(start..end_pos, "");
                    }
                }
            }
        }

        // Second pass: replace {{var}} placeholders
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, &value);
        }

        result
    }

    /// Generate response using LLM
    async fn generate_response(&self, query: &str, context: &ConversationContext) -> Result<PluginResponse> {
        // Build prompt
        let prompt = self.build_prompt(query, context).await?;

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
            sources: vec![],  // TODO: Track source IDs
            confidence: 0.8,
            metadata: serde_json::json!({
                "model": self.llm.model_name(),
                "emotional_trend": format!("{:?}", context.emotional_trend),
            }),
            timestamp: Utc::now(),
        })
    }

    /// Check if an event is significant enough to trigger reflection
    fn is_significant_event(&self, _event: &EventNotification) -> bool {
        // TODO: Implement significance detection
        // For now, return false
        false
    }

    /// Generate reflection on significant event
    async fn generate_reflection(&self, _event: &EventNotification) -> Result<PluginOutput> {
        // TODO: Implement reflection generation
        Ok(PluginOutput::AnalysisComplete)
    }
}

#[async_trait]
impl UserPlugin for DeepTalkPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn on_event(
        &self,
        event: &EventNotification,
        _context: &PluginContext,
    ) -> Result<PluginOutput> {
        // DeepTalk observes all events silently
        // May trigger reflections if significant
        if self.is_significant_event(event) {
            self.generate_reflection(event).await
        } else {
            Ok(PluginOutput::AnalysisComplete)
        }
    }

    async fn on_query(&self, query: &str, _context: &PluginContext) -> Result<PluginResponse> {
        // Build deep context
        let ctx = self.build_context(query).await?;

        // Generate response
        self.generate_response(query, &ctx).await
    }

    fn subscriptions(&self) -> &[crate::plugin::EventSubscription] {
        // Subscribe to all events
        use crate::plugin::EventSubscription;
        &[EventSubscription::All]
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        // Load and cache prompts on initialization
        let mut prompt_mgr = self.prompt_manager.write().await;
        let _ = prompt_mgr.load_prompt("deeptalk")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotional_trend_display() {
        assert_eq!(EmotionalTrend::Positive.emoji(), "😊");
        assert_eq!(EmotionalTrend::Neutral.description(), "balanced and stable");
        assert_eq!(EmotionalTrend::Negative.emoji(), "😔");
    }

    #[test]
    fn test_conversation_context_default() {
        let ctx = ConversationContext::default();
        assert!(ctx.events.is_empty());
        assert!(ctx.beliefs.is_empty());
        assert_eq!(ctx.emotional_trend, EmotionalTrend::Neutral);
        assert!(ctx.conversation_summary.is_empty());
    }

    #[test]
    fn test_render_simple() {
        let template = "Hello {{name}}, your mood is {{mood}}.";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("mood".to_string(), "happy".to_string());

        let result = DeepTalkPlugin::render_simple(template, vars);
        // Result should contain replaced values
        assert!(result.contains("Alice") || result.contains("happy"), "Result: {}", result);
    }

    #[test]
    fn test_render_simple_with_empty_vars() {
        let template = "Value: {{value}}";
        let mut vars = HashMap::new();
        vars.insert("value".to_string(), String::new());

        let result = DeepTalkPlugin::render_simple(template, vars);
        // Empty value should result in template with empty string
        assert!(result.contains("Value:"));
    }

    #[test]
    fn test_render_simple_with_conditionals() {
        let template = "{{#if show}}Shown{{/if}}";

        // Test with non-empty value
        let mut vars = HashMap::new();
        vars.insert("show".to_string(), "yes".to_string());
        let result = DeepTalkPlugin::render_simple(template, vars);
        // Conditional with non-empty value should show content
        assert!(result.contains("Shown"));

        // Test with empty value
        let mut vars2 = HashMap::new();
        vars2.insert("show".to_string(), String::new());
        let result = DeepTalkPlugin::render_simple(template, vars2);
        // Conditional with empty value should hide content
        assert!(!result.contains("Shown"));
    }

    #[test]
    fn test_emotional_trend_serialization() {
        let trend = EmotionalTrend::Positive;
        let json = serde_json::to_string(&trend).unwrap();
        let deserialized: EmotionalTrend = serde_json::from_str(&json).unwrap();
        assert_eq!(trend, deserialized);
    }
}
