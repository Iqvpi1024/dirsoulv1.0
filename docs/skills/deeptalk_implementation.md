# Skill: DeepTalk Implementation

> **Purpose**: Guide the development of the default DeepTalk plugin, ensuring deep dialogue based on global memory with cross-session continuity and emotional trend awareness.

---

## DeepTalk Core Design

### Plugin Definition

```rust
use crate::plugin::{UserPlugin, PluginMetadata, PluginContext, PluginResponse, PluginOutput};
use crate::memory::{Event, DerivedView, TimeRange};
use ollama::Ollama;

/// DeepTalk - The always-on default plugin for deep, memory-augmented conversation
pub struct DeepTalkPlugin {
    ollama: Ollama,
    memory_interface: Arc<dyn DeepTalkMemoryInterface>,
    metadata: PluginMetadata,
}

impl DeepTalkPlugin {
    pub fn new(
        ollama: Ollama,
        memory_interface: Arc<dyn DeepTalkMemoryInterface>
    ) -> Self {
        Self {
            ollama,
            memory_interface,
            metadata: PluginMetadata {
                id: "deeptalk".to_string(),
                name: "DeepTalk".to_string(),
                version: "1.0.0".to_string(),
                description: "Deep memory-augmented conversation plugin".to_string(),
                required_permission: MemoryPermission::ReadWriteDerived,
                author: "DirSoul Team".to_string(),
            },
        }
    }

    /// Build context for user query
    async fn build_context(&self, user_id: &str, query: &str) -> Result<ConversationContext> {
        let mut context = ConversationContext::default();

        // 1. Retrieve relevant events (vector + SQL hybrid)
        let relevant_events = self.retrieve_relevant_events(user_id, query).await?;
        context.events = relevant_events;

        // 2. Retrieve relevant derived views
        let relevant_views = self.retrieve_relevant_views(user_id, query).await?;
        context.views = relevant_views;

        // 3. Analyze emotional trends
        let emotion_trend = self.analyze_emotional_trend(user_id).await?;
        context.emotion_trend = emotion_trend;

        // 4. Get conversation summary
        let conversation_summary = self.get_conversation_summary(user_id).await?;
        context.conversation_summary = conversation_summary;

        Ok(context)
    }

    /// Hybrid retrieval: Vector similarity + SQL filtering
    async fn retrieve_relevant_events(
        &self,
        user_id: &str,
        query: &str
    ) -> Result<Vec<Event>> {
        // Step 1: Generate query embedding
        let query_embedding = self.ollama
            .generate_embedding(query)
            .await?;

        // Step 2: Vector similarity search (min_score_threshold = 0.18)
        let similar_events = self.memory_interface
            .search_similar(user_id, &query_embedding, 0.18, 20)
            .await?;

        // Step 3: SQL filter for recent high-confidence events
        let recent_filter = EventFilter {
            user_id: user_id.to_string(),
            time_range: TimeRange::LastDays(30),
            min_confidence: Some(0.7),
            ..Default::default()
        };

        let recent_events = self.memory_interface
            .query_events(&recent_filter)
            .await?;

        // Step 4: Merge and deduplicate
        let mut merged = std::collections::HashMap::new();
        for event in similar_events.into_iter().chain(recent_events) {
            merged.entry(event.event_id)
                .or_insert(event);
        }

        // Step 5: Re-rank by recency + relevance
        let mut events: Vec<_> = merged.into_values().collect();
        events.sort_by(|a, b| {
            let recency_diff = b.timestamp.cmp(&a.timestamp);
            // In production, add semantic similarity score
            recency_diff
        });

        // Return top 10
        Ok(events.into_iter().take(10).collect())
    }

    /// Retrieve relevant cognitive views
    async fn retrieve_relevant_views(
        &self,
        user_id: &str,
        query: &str
    ) -> Result<Vec<DerivedView>> {
        // Get high-confidence active views
        let views = self.memory_interface
            .get_active_views(user_id, 0.7)
            .await?;

        // Filter by query relevance
        let relevant: Vec<_> = views
            .into_iter()
            .filter(|v| self.view_relevance(query, v) > 0.5)
            .collect();

        Ok(relevant)
    }

    /// Calculate view relevance to query
    fn view_relevance(&self, query: &str, view: &DerivedView) -> f32 {
        // Simple keyword overlap (in production, use embeddings)
        let query_words: std::collections::HashSet<&str> =
            query.split_whitespace().collect();
        let view_words: std::collections::HashSet<&str> =
            view.hypothesis.split_whitespace().collect();

        let intersection = query_words.intersection(&view_words).count();
        let union = query_words.union(&view_words).count();

        if union == 0 { return 0.0; }

        intersection as f32 / union as f32
    }

    /// Analyze emotional trend from events
    async fn analyze_emotional_trend(&self, user_id: &str) -> Result<EmotionalTrend> {
        // Get recent events with emotional content
        let emotion_events = self.memory_interface
            .get_emotional_events(user_id, TimeRange::LastDays(7))
            .await?;

        // Time-series analysis
        let trend = if emotion_events.is_empty() {
            EmotionalTrend::Neutral
        } else {
            // Group by day and analyze sentiment
            let daily_sentiment: std::collections::HashMap<_, _> =
                emotion_events
                    .iter()
                    .filter_map(|e| {
                        e.metadata.get("sentiment")
                            .and_then(|s| s.as_f64())
                            .map(|s| (e.timestamp.date(), s))
                    })
                    .collect();

            // Calculate trend direction
            let sentiments: Vec<_> = daily_sentiment.values().copied().collect();
            let avg = sentiments.iter().sum::<f64>() / sentiments.len() as f64;

            if avg > 0.3 {
                EmotionalTrend::Positive
            } else if avg < -0.3 {
                EmotionalTrend::Negative
            } else {
                EmotionalTrend::Neutral
            }
        };

        Ok(trend)
    }
}

#[async_trait]
impl UserPlugin for DeepTalkPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn on_query(
        &self,
        query: &str,
        context: &PluginContext
    ) -> Result<PluginResponse> {
        // Build deep context
        let ctx = self.build_context(&context.user_id, query).await?;

        // Generate response
        let response = self.generate_response(query, ctx).await?;

        // Save conversation as event
        self.log_conversation(&context.user_id, query, &response).await?;

        Ok(response)
    }

    async fn on_event(&self, event: &Event, context: &PluginContext) -> Result<PluginOutput> {
        // DeepTalk observes all events silently
        // May trigger reflections if significant
        if self.is_significant_event(event) {
            let reflection = self.generate_reflection(event).await?;
            return Ok(PluginOutput::DerivedView(reflection));
        }

        Ok(PluginOutput::Empty)
    }

    fn subscriptions(&self) -> &[EventSubscription] {
        // Subscribe to all events
        &[EventSubscription::All]
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
```

---

## Response Generation

### Personalized Response Style

```rust
impl DeepTalkPlugin {
    /// Generate response using retrieved context
    async fn generate_response(
        &self,
        query: &str,
        context: ConversationContext
    ) -> Result<PluginResponse> {
        // 1. Build prompt with context
        let prompt = self.build_conversation_prompt(query, context)?;

        // 2. Generate with SLM
        let response = self.ollama
            .generate(&phi4_mini(), &prompt)
            .await?;

        // 3. Extract response and sources
        let content = response.response.trim().to_string();

        Ok(PluginResponse {
            content,
            sources: vec![],  // Populated from context
            confidence: 0.8,
        })
    }

    fn build_conversation_prompt(
        &self,
        query: &str,
        context: ConversationContext
    ) -> Result<String> {
        let mut prompt = String::from(
            r#"You are DeepTalk, a deep memory-augmented conversation assistant.
You remember everything about the user and provide personalized, emotionally-aware responses.

Guidelines:
- Reference specific past events when relevant
- Be aware of emotional trends
- Maintain conversation continuity
- Be concise but thoughtful"#
        );

        // Add emotional context
        prompt.push_str(&format!(
            "\n\nUser's recent emotional state: {:?}",
            context.emotion_trend
        ));

        // Add conversation summary
        if !context.conversation_summary.is_empty() {
            prompt.push_str(&format!(
                "\n\nRecent conversation topics:\n{}",
                context.conversation_summary
            ));
        }

        // Add relevant events
        if !context.events.is_empty() {
            prompt.push_str("\n\nRelevant past events:");
            for (i, event) in context.events.iter().take(5).enumerate() {
                prompt.push_str(&format!(
                    "\n{}. {} - {} {}",
                    i + 1,
                    event.timestamp.format("%Y-%m-%d"),
                    event.action,
                    event.target
                ));
            }
        }

        // Add derived views
        if !context.views.is_empty() {
            prompt.push_str("\n\nWhat I know about you:");
            for view in &context.views {
                prompt.push_str(&format!("\n- {}", view.hypothesis));
            }
        }

        prompt.push_str(&format!("\n\nYour query: {}", query));
        prompt.push_str("\n\nResponse:");

        Ok(prompt)
    }
}
```

---

## Emotional Trend Analysis

### Time-Series Sentiment

```rust
#[derive(Debug, Clone, Copy)]
pub enum EmotionalTrend {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug)]
pub struct SentimentPoint {
    pub timestamp: DateTime<Utc>,
    pub sentiment: f64,  // -1.0 to 1.0
    pub confidence: f32,
}

impl DeepTalkPlugin {
    /// Get emotional events with sentiment analysis
    async fn get_emotional_events(
        &self,
        user_id: &str,
        time_range: TimeRange
    ) -> Result<Vec<SentimentPoint>> {
        let events = self.memory_interface
            .get_emotional_events(user_id, time_range)
            .await?;

        // Use SLM to analyze sentiment for events without it
        let mut points = Vec::new();

        for event in events {
            let sentiment = if let Some(s) = event.metadata.get("sentiment") {
                s.as_f64().unwrap_or(0.0)
            } else {
                self.analyze_sentiment(&event).await?
            };

            points.push(SentimentPoint {
                timestamp: event.timestamp,
                sentiment,
                confidence: event.confidence,
            });
        }

        Ok(points)
    }

    /// Analyze sentiment of an event
    async fn analyze_sentiment(&self, event: &Event) -> Result<f64> {
        let prompt = format!(
            "Analyze the emotional sentiment of this event. \
            Return a number from -1.0 (very negative) to 1.0 (very positive). \
            Only return the number.\n\nEvent: {} {} {}",
            event.action, event.target,
            event.quantity.map_or(String::new(), |q| format!("{}", q))
        );

        let response = self.ollama.generate(&phi4_mini(), &prompt).await?;

        response.response
            .trim()
            .parse::<f64>()
            .map_err(|_| Error::ParseError)
    }
}
```

---

## Cross-Session Continuity

### Conversation Summary

```rust
impl DeepTalkPlugin {
    /// Get summary of recent conversations
    async fn get_conversation_summary(&self, user_id: &str) -> Result<String> {
        // Get recent conversation events
        let filter = EventFilter {
            user_id: user_id.to_string(),
            actions: vec!["chat_with_plugin".to_string()],
            time_range: TimeRange::LastDays(7),
            ..Default::default()
        };

        let conversations = self.memory_interface
            .query_events(&filter)
            .await?;

        if conversations.is_empty() {
            return Ok(String::new());
        }

        // Generate summary using SLM
        let summary_prompt = format!(
            "Summarize the main topics from these recent conversations. \
            Keep it under 3 sentences.\n\n{}",
            conversations.iter()
                .map(|c| format!("- {}: {}", c.timestamp.format("%Y-%m-%d"), c.target))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let response = self.ollama.generate(&phi4_mini(), &summary_prompt).await?;

        Ok(response.response)
    }

    /// Log conversation as event
    async fn log_conversation(
        &self,
        user_id: &str,
        query: &str,
        response: &PluginResponse
    ) -> Result<()> {
        let event = Event {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: Some(user_id.to_string()),
            action: "chat_with_plugin".to_string(),
            target: "deeptalk".to_string(),
            quantity: None,
            unit: None,
            confidence: 1.0,
            metadata: json!({
                "query": query,
                "response_length": response.content.len(),
                "sources": response.sources,
            }),
        };

        self.memory_interface.create_event(event).await
    }
}
```

---

## Significant Event Detection

```rust
impl DeepTalkPlugin {
    /// Detect events that may trigger reflections
    fn is_significant_event(&self, event: &Event) -> bool {
        // High-confidence rare events
        if event.confidence > 0.9 {
            // Check if this is unusual
            if matches!(event.action.as_str(), "买" | "决定" | "开始" | "结束") {
                return true;
            }
        }

        // Emotional events
        if event.metadata.get("sentiment")
            .and_then(|s| s.as_f64())
            .map_or(false, |s| s.abs() > 0.7)
        {
            return true;
        }

        false
    }

    /// Generate reflection on significant event
    async fn generate_reflection(&self, event: &Event) -> Result<DerivedView> {
        let prompt = format!(
            r#"Based on this event, generate a brief hypothesis about the user.
            Return as a single sentence starting with "用户可能" (User might).

            Event: {} {} {}

            Hypothesis:"#,
            event.action,
            event.target,
            event.quantity.map_or(String::new(), |q| format!("{}", q))
        );

        let response = self.ollama.generate(&phi4_mini(), &prompt).await?;

        Ok(DerivedView {
            view_id: Uuid::new_v4(),
            hypothesis: response.response.trim().to_string(),
            derived_from: vec![event.event_id],
            confidence: 0.5,  // Low initial confidence
            expires_at: Utc::now() + Duration::days(30),
            status: ViewStatus::Active,
            created_at: Utc::now(),
            validation_count: 1,
            view_type: ViewType::Belief,
            counter_evidence: vec![],
        })
    }
}
```

---

## Memory Interface for DeepTalk

```rust
#[async_trait]
pub trait DeepTalkMemoryInterface: Send + Sync {
    /// Search events by vector similarity
    async fn search_similar(
        &self,
        user_id: &str,
        embedding: &[f32],
        min_score: f32,
        limit: usize
    ) -> Result<Vec<Event>>;

    /// Query events with filters
    async fn query_events(&self, filter: &EventFilter) -> Result<Vec<Event>>;

    /// Get active derived views
    async fn get_active_views(
        &self,
        user_id: &str,
        min_confidence: f32
    ) -> Result<Vec<DerivedView>>;

    /// Get events with emotional content
    async fn get_emotional_events(
        &self,
        user_id: &str,
        time_range: TimeRange
    ) -> Result<Vec<Event>>;

    /// Create new event
    async fn create_event(&self, event: Event) -> Result<()>;

    /// Store derived view
    async fn store_view(&self, view: DerivedView) -> Result<()>;
}
```

---

## Recommended Combinations

Use this skill together with:
- **OllamaPromptEngineering**: For prompt optimization
- **CognitiveViewGeneration**: For creating derived views from conversations
- **EventExtractionPatterns**: For logging conversations as events
- **PluginPermissionSystem**: For permission configuration (ReadWriteDerived)
