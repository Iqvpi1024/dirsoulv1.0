# Skill: Ollama Prompt Engineering

> **Purpose**: Optimize Phi-4-mini prompts for all AI-driven components (event extraction, summary generation, etc.), ensuring efficient inference within 8GB memory constraints.

---

## Prompt Structure Rules

### Declarative Prompt Template

```text
# Prompt Template for Phi-4-mini

You are {role} for the DirSoul memory system.

## Context
{context_section}

## Task
{task_description}

## Input
{user_input}

## Output Format
{output_format_specification}

## Examples
{few_shot_examples}

## Rules
{behavioral_constraints}

Output:
```

### Core Principles

1. **JSON Output Format**: Always request structured JSON for parsing
2. **Few-Shot Examples**: Include 3-5 examples for complex tasks
3. **Confidence Assessment**: Require explicit confidence scores
4. **Context Compression**: Limit context to 1000-2000 tokens for 8GB efficiency

---

## Event Extraction Prompt

### Production Template

```text
You are an event extraction system for DirSoul. Extract structured events from natural language input.

## Task
Extract events from the input text. An event has: action (what happened), target (what/who involved), quantity (how much), and timestamp (when it happened).

## Input
"{user_input}"

## Output Format
Respond ONLY with valid JSON in this exact format:
{{
  "events": [
    {{
      "action": "verb (e.g., eat, drink, buy)",
      "target": "noun (e.g., apple, coffee)",
      "quantity": number or null,
      "unit": "measurement or null",
      "confidence": 0.0 to 1.0,
      "timestamp_hint": "when (e.g., today, yesterday, last week)",
      "reasoning": "brief explanation"
    }}
  ]
}}

## Rules
- Only extract actual events, not opinions, questions, or greetings
- Return empty array if no clear event exists
- Include quantity/unit only when explicitly stated
- Lower confidence for ambiguous text (< 0.6)
- Default timestamp_hint to "now" if no time mentioned
- Do not hallucinate details not in the text

## Examples

Input: "我今天早上吃了3个苹果"
Output: {{
  "events": [{{
    "action": "吃",
    "target": "苹果",
    "quantity": 3,
    "unit": "个",
    "confidence": 0.95,
    "timestamp_hint": "今天早上",
    "reasoning": "明确的事件陈述，包含具体数量"
  }}]
}}

Input: "昨天去健身房了"
Output: {{
  "events": [{{
    "action": "去",
    "target": "健身房",
    "quantity": 1,
    "unit": "次",
    "confidence": 0.90,
    "timestamp_hint": "昨天",
    "reasoning": "明确的行为和地点"
  }}]
}}

Input: "这周买了咖啡"
Output: {{
  "events": [{{
    "action": "买",
    "target": "咖啡",
    "quantity": null,
    "unit": null,
    "confidence": 0.70,
    "timestamp_hint": "这周",
    "reasoning": "事件明确但缺少具体数量"
  }}]
}}

Input: "今天天气真好"
Output: {{
  "events": []
}}

Input: "我早上喝了咖啡，下午又喝了茶"
Output: {{
  "events": [
    {{
      "action": "喝",
      "target": "咖啡",
      "quantity": 1,
      "unit": "杯",
      "confidence": 0.88,
      "timestamp_hint": "今天早上",
      "reasoning": "明确的事件"
    }},
    {{
      "action": "喝",
      "target": "茶",
      "quantity": 1,
      "unit": "杯",
      "confidence": 0.88,
      "timestamp_hint": "今天下午",
      "reasoning": "明确的事件"
    }}
  ]
}}

Output (for input "{user_input}"):
```

### Response Parsing

```rust
use serde_json::Value;

pub struct EventExtractionPrompt {
    template: String,
}

impl EventExtractionPrompt {
    pub fn format(&self, user_input: &str) -> String {
        self.template.replace("{user_input}", user_input)
    }

    pub fn parse_response(&self, response: &str) -> Result<Vec<ExtractedEvent>> {
        // Extract JSON from response
        let json_str = self.extract_json(response)?;

        let value: Value = serde_json::from_str(json_str)?;

        let events = value["events"]
            .as_array()
            .ok_or_else(|| Error::ParseError)?

        events
            .iter()
            .map(|v| Ok(ExtractedEvent {
                action: v["action"].as_str().ok_or(Error::ParseError)?.to_string(),
                target: v["target"].as_str().ok_or(Error::ParseError)?.to_string(),
                quantity: v["quantity"].as_number().and_then(|n| n.as_f64()),
                unit: v["unit"].as_str().map(|s| s.to_string()),
                confidence: v["confidence"].as_f64().unwrap_or(0.5) as f32,
                timestamp: Utc::now(),  // Parse from timestamp_hint
                context: v["reasoning"].as_str().map(|s| json!(s)),
            }))
            .collect()
    }

    fn extract_json(&self, response: &str) -> Result<&str> {
        // Find JSON object in response
        let start = response.find('{').ok_or(Error::ParseError)?;
        let end = response.rfind('}').ok_or(Error::ParseError)?;
        Ok(&response[start..=end])
    }
}
```

---

## Sentiment Analysis Prompt

```text
You are a sentiment analyzer for DirSoul. Analyze the emotional tone of events.

## Task
Analyze the sentiment of the given event on a scale from -1.0 (very negative) to 1.0 (very positive).

## Input
Event: "{action} {target} {quantity}"

## Output Format
Respond ONLY with a JSON object:
{{
  "sentiment": -1.0 to 1.0,
  "confidence": 0.0 to 1.0,
  "emotion_labels": ["list", "of", "emotions"]
}}

## Emotion Labels
Use these labels when applicable:
- Positive: joy, satisfaction, excitement, pride, relief
- Negative: frustration, sadness, anger, disappointment, anxiety
- Neutral: neutral, routine, observation

## Examples

Input: "eat apple 3"
Output: {{
  "sentiment": 0.1,
  "confidence": 0.6,
  "emotion_labels": ["neutral"]
}}

Input: "finish difficult_project 1"
Output: {{
  "sentiment": 0.7,
  "confidence": 0.8,
  "emotion_labels": ["joy", "pride", "satisfaction"]
}}

Input: "lose wallet 1"
Output: {{
  "sentiment": -0.8,
  "confidence": 0.9,
  "emotion_labels": ["sadness", "anxiety", "frustration"]
}}

Output (for "{action} {target} {quantity}"):
```

---

## Summary Generation Prompt

```text
You are a summary generator for DirSoul. Create concise summaries of event clusters.

## Task
Summarize these events into a brief cognitive insight.

## Events
{events_list}

## Output Format
Respond ONLY with a JSON object:
{{
  "summary": "one sentence summary",
  "key_insights": ["insight1", "insight2"],
  "confidence": 0.0 to 1.0
}}

## Rules
- Keep summary under 20 words
- Focus on patterns, not individual events
- Lower confidence for small sample sizes
- Be factual, avoid speculation

## Examples

Events:
- eat apple 3 times (2026-01-01, 2026-01-03, 2026-01-05)
- buy apple 2kg (2026-01-02)

Output: {{
  "summary": "用户经常食用苹果，显示对水果的偏好",
  "key_insights": ["高频食用", "批量购买"],
  "confidence": 0.75
}}

Events:
- go gym 5 times in past week
- exercise 45min average

Output: {{
  "summary": "用户保持规律的运动习惯，每周健身房访问5次",
  "key_insights": ["规律运动", "平均45分钟"],
  "confidence": 0.85
}}

Output (for given events):
```

---

## Optimization Techniques

### Batch Processing for 8GB Memory

```rust
/// Batch multiple prompts into single inference
pub struct BatchPromptProcessor {
    ollama: Ollama,
    max_batch_size: usize,
}

impl BatchPromptProcessor {
    /// Process multiple inputs in one SLM call
    pub async fn process_batch(
        &self,
        inputs: Vec<String>
    ) -> Result<Vec<Vec<ExtractedEvent>>> {
        if inputs.len() <= 1 {
            // Single input - normal processing
            return self.process_single(&inputs[0]).await.map(|v| vec![v]);
        }

        // Build batch prompt
        let batch_prompt = self.build_batch_prompt(&inputs);

        // Single inference call
        let response = self.ollama.generate(&phi4_mini(), &batch_prompt).await?;

        // Parse batched response
        self.parse_batch_response(&response, inputs.len())
    }

    fn build_batch_prompt(&self, inputs: &[String]) -> String {
        format!(
            r#"Extract events from each input. Return JSON array of arrays.

## Inputs
{}

## Output Format
{{
  "results": [
    {{"events": [...]}}  // Input 1
    {{"events": [...]}}  // Input 2
  ]
}}

Extract events for all inputs:"#,
            inputs.iter()
                .enumerate()
                .map(|(i, txt)| format!("Input {}: \"{}\"", i + 1, txt))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn parse_batch_response(
        &self,
        response: &str,
        expected_count: usize
    ) -> Result<Vec<Vec<ExtractedEvent>>> {
        let value: Value = serde_json::from_str(response)?;

        let results = value["results"]
            .as_array()
            .ok_or(Error::ParseError)?;

        // Ensure we got expected number of results
        if results.len() != expected_count {
            return Err(Error::BatchMismatch);
        }

        results
            .iter()
            .map(|r| self.parse_events_array(r))
            .collect()
    }
}
```

### Context Compression

```rust
/// Compress context to fit within token limits
pub struct ContextCompressor {
    max_tokens: usize,
}

impl ContextCompressor {
    /// Compress event list to essential information
    pub fn compress_events(&self, events: &[Event]) -> String {
        if events.len() <= 5 {
            // No compression needed
            return format_events(events);
        }

        // Group by action/target
        let mut groups: std::collections::HashMap<String, Vec<&Event>> =
            std::collections::HashMap::new();

        for event in events {
            let key = format!("{} {}", event.action, event.target);
            groups.entry(key).or_default().push(event);
        }

        // Create compressed summary
        groups
            .into_iter()
            .map(|(key, evts)| {
                let count = evts.len();
                let date_range = if count > 1 {
                    format!(
                        "{} ~ {}",
                        evts.first().unwrap().timestamp.format("%Y-%m-%d"),
                        evts.last().unwrap().timestamp.format("%Y-%m-%d")
                    )
                } else {
                    evts[0].timestamp.format("%Y-%m-%d").to_string()
                };

                format!("- {} ({}次, {})", key, count, date_range)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn format_events(events: &[Event]) -> String {
    events
        .iter()
        .map(|e| format!("- {} {} ({})", e.action, e.target, e.timestamp.format("%Y-%m-%d")))
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## Hallucination Mitigation

### Isolated Judgment Pattern

```rust
/// Use derived views to validate SLM output
pub struct ValidatedPromptExecutor {
    ollama: Ollama,
    view_store: ViewStore,
}

impl ValidatedPromptExecutor {
    /// Extract events with validation against known views
    pub async fn extract_validated(
        &self,
        input: &str
    ) -> Result<Vec<ExtractedEvent>> {
        // Step 1: Extract events
        let events = self.extract_events(input).await?;

        // Step 2: Validate against derived views
        let validated = events
            .into_iter()
            .filter_map(|e| self.validate_event(e))
            .collect();

        Ok(validated)
    }

    fn validate_event(&self, event: ExtractedEvent) -> Option<ExtractedEvent> {
        // Check for contradictions with known views
        let contradicting_views = self.view_store
            .find_contradicting(&event.action, &event.target);

        if contradicting_views.len() > 0 {
            // Reduce confidence if contradicted
            let mut validated = event;
            validated.confidence *= 0.5;

            if validated.confidence < 0.4 {
                return None;  // Reject low-confidence events
            }

            return Some(validated);
        }

        // Boost confidence if supported by views
        let supporting_views = self.view_store
            .find_supporting(&event.action, &event.target);

        if supporting_views.len() > 0 {
            let mut validated = event;
            validated.confidence = (validated.confidence * 1.1).min(1.0);
            return Some(validated);
        }

        Some(event)
    }
}
```

---

## Token Estimation

```rust
/// Estimate token count for prompts (rough approximation)
pub fn estimate_tokens(text: &str) -> usize {
    // Chinese: ~1.5 chars per token
    // English: ~4 chars per token
    let chinese_chars = text.chars().filter(|c| is_chinese(*c)).count();
    let other_chars = text.len() - chinese_chars;

    (chinese_chars / 2 + other_chars / 4).max(1)
}

fn is_chinese(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF)
}

/// Check if prompt fits within context window
pub fn check_prompt_size(prompt: &str, max_tokens: usize) -> bool {
    estimate_tokens(prompt) < max_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let chinese = "我今天早上吃了三个苹果";
        assert!(estimate_tokens(chinese) < 15);

        let english = "I ate three apples this morning";
        assert!(estimate_tokens(english) < 10);
    }
}
```

---

## Recommended Combinations

Use this skill together with:
- **EventExtractionPatterns**: For event extraction prompts
- **CognitiveViewGeneration**: For hypothesis generation
- **DeepTalkImplementation**: For conversation prompts
- **EntityResolution**: For entity classification prompts
