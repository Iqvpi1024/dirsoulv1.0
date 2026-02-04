# Skill: Event Extraction Patterns

> **Purpose**: Guide SLM-driven event extraction, avoiding hardcoded rules and providing declarative patterns for Phi-4-mini prompt design.

---

## Core Event Structure (Declarative)

### Event Schema Definition

```rust
/// Structured event extracted from natural language
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedEvent {
    /// The action performed (e.g., "eat", "buy", "go")
    pub action: String,

    /// The object/target of the action (e.g., "apple", "coffee", "gym")
    pub target: String,

    /// Quantity involved (optional, e.g., 3 for "3 apples")
    pub quantity: Option<f32>,

    /// Unit of measurement (e.g., "个", "杯", "次")
    pub unit: Option<String>,

    /// Confidence score [0.0, 1.0] from SLM
    pub confidence: f32,

    /// Timestamp extracted or inferred from context
    pub timestamp: DateTime<Utc>,

    /// Additional context (actor, location, etc.)
    pub context: Option<serde_json::Value>,
}
```

### Semantic Rules for Validation

```rust
/// Declarative validation rules
impl ExtractedEvent {
    /// Valid event must have non-empty action and target
    pub fn is_valid(&self) -> bool {
        !self.action.trim().is_empty()
            && !self.target.trim().is_empty()
            && self.confidence >= 0.0
            && self.confidence <= 1.0
            && self.quantity.map_or(true, |q| q >= 0.0)
    }

    /// High-confidence events for direct storage
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }

    /// Medium-confidence needs verification
    pub fn needs_verification(&self) -> bool {
        self.confidence >= 0.5 && self.confidence < 0.8
    }
}
```

---

## Time Parsing Patterns

### Fuzzy Time to TIMESTAMPTZ Mapping

```rust
use chrono::{Utc, DateTime, Duration, Local};

/// Fuzzy time expression parser (declarative pattern)
pub enum FuzzyTime {
    Now,
    Today,
    Yesterday,
    Tomorrow,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    SpecificDay(i32, u32, u32),  // year, month, day
    DaysAgo(i32),
    WeeksAgo(i32),
}

impl FuzzyTime {
    /// Convert fuzzy time to actual timestamp
    pub fn to_timestamp(&self) -> DateTime<Utc> {
        let now = Utc::now();
        let local_now = Local::now();

        match self {
            FuzzyTime::Now => now,

            FuzzyTime::Today => {
                local_now
                    .with_hour(0).unwrap()
                    .with_minute(0).unwrap()
                    .with_second(0).unwrap()
                    .with_nanosecond(0).unwrap()
                    .into()
            }

            FuzzyTime::Yesterday => {
                (local_now - Duration::days(1))
                    .with_hour(0).unwrap()
                    .with_minute(0).unwrap()
                    .into()
            }

            FuzzyTime::LastWeek => {
                (local_now - Duration::weeks(1))
                    .with_hour(0).unwrap()
                    .into()
            }

            FuzzyTime::DaysAgo(n) => {
                (local_now - Duration::days(*n as i64))
                    .with_hour(12).unwrap()  // Assume noon
                    .into()
            }

            FuzzyTime::SpecificDay(y, m, d) => {
                Local.ymd(*y, *m, d).and_hms(12, 0, 0).into()
            }

            _ => now,  // Default to now for other cases
        }
    }
}

/// Extraction examples
// "今天早上吃了苹果" → Today at 08:00
// "昨天去健身房" → Yesterday at user's typical gym time
// "上周三开会" → Last Wednesday at typical meeting time
```

### Time Context Inference

```rust
/// Context-aware time adjustment
pub fn adjust_time_by_context(
    base_time: DateTime<Utc>,
    action: &str,
    target: &str
) -> DateTime<Utc> {
    match (action, target) {
        // Morning activities
        ("吃", "早饭") | ("喝", "咖啡") if base_time.hour() == 0 => {
            base_time.with_hour(8).unwrap()
        }
        // Lunch
        ("吃", "午饭") if base_time.hour() == 0 => {
            base_time.with_hour(12).unwrap()
        }
        // Evening activities
        ("看", "电影") | ("看", "电视") if base_time.hour() == 0 => {
            base_time.with_hour(20).unwrap()
        }
        _ => base_time,
    }
}
```

---

## SLM Prompt Design

### Phi-4-mini Prompt Template

```text
You are an event extraction system. Extract structured events from natural language.

Input text: "{input_text}"

Extract events in this JSON format:
{{
  "events": [
    {{
      "action": "verb describing what happened",
      "target": "object the action was performed on",
      "quantity": number or null,
      "unit": "measurement unit or null",
      "confidence": 0.0 to 1.0,
      "timestamp_hint": "when this happened (e.g., 'today', 'yesterday', 'last week')",
      "actor": "who performed the action (default: 'user')"
    }}
  ]
}}

Rules:
- Only extract actual events, not opinions or questions
- If no clear event exists, return empty events array
- Confidence should be lower for ambiguous text
- Include quantity and unit only when explicitly mentioned
- Default timestamp_hint to "now" if no time mentioned

Example:
Input: "我今天早上吃了3个苹果"
Output: {{
  "events": [{{
    "action": "吃",
    "target": "苹果",
    "quantity": 3,
    "unit": "个",
    "confidence": 0.95,
    "timestamp_hint": "今天早上",
    "actor": "user"
  }}]
}}
```

### Few-Shot Examples for Robustness

```text
Example 1:
Input: "昨天去健身房练了2小时"
Output: {{
  "events": [{{
    "action": "去",
    "target": "健身房",
    "quantity": 1,
    "unit": "次",
    "confidence": 0.92,
    "timestamp_hint": "昨天",
    "actor": "user"
  }}]
}}

Example 2:
Input: "这周买了好多咖啡豆"
Output: {{
  "events": [{{
    "action": "买",
    "target": "咖啡豆",
    "quantity": null,
    "unit": null,
    "confidence": 0.75,
    "timestamp_hint": "这周",
    "actor": "user"
  }}]
}}

Example 3:
Input: "今天天气真好啊"
Output: {{
  "events": []
}}

Example 4 (multi-event):
Input: "早上喝了咖啡，下午又喝了茶"
Output: {{
  "events": [
    {{
      "action": "喝",
      "target": "咖啡",
      "quantity": 1,
      "unit": "杯",
      "confidence": 0.90,
      "timestamp_hint": "今天早上",
      "actor": "user"
    }},
    {{
      "action": "喝",
      "target": "茶",
      "quantity": 1,
      "unit": "杯",
      "confidence": 0.90,
      "timestamp_hint": "今天下午",
      "actor": "user"
    }}
  ]
}}
```

---

## Fallback Mechanism

### Rule Engine as Safety Net

```rust
/// Regex-based extraction (SLM failure fallback)
use regex::Regex;

pub struct RuleBasedExtractor {
    quantity_pattern: Regex,
    action_target_pattern: Regex,
}

impl RuleBasedExtractor {
    pub fn new() -> Self {
        Self {
            // Match: 数字 + 单位 + 名词
            quantity_pattern: Regex::new(r"(\d+(?:\.\d+)?)\s*(个|杯|次|小时|分钟|天|周|公斤|克|毫升|升)\s*(.+?)").unwrap(),

            // Match: 动词 + 对象
            action_target_pattern: Regex::new(r"(吃|喝|买|去|看|读|写|做|玩|听)\s*(.+?)").unwrap(),
        }
    }

    /// Extract with high confidence on exact patterns
    pub fn extract(&self, text: &str) -> Option<ExtractedEvent> {
        // Try quantity pattern first
        if let Some(caps) = self.quantity_pattern.captures(text) {
            return Some(ExtractedEvent {
                action: "提取".to_string(),
                target: caps.get(3)?.as_str().to_string(),
                quantity: Some(caps.get(1)?.as_str().parse().ok()?),
                unit: Some(caps.get(2)?.as_str().to_string()),
                confidence: 0.6,  // Lower confidence for rule-based
                timestamp: Utc::now(),
                context: None,
            });
        }

        // Fall back to action-target pattern
        if let Some(caps) = self.action_target_pattern.captures(text) {
            return Some(ExtractedEvent {
                action: caps.get(1)?.as_str().to_string(),
                target: caps.get(2)?.as_str().to_string(),
                quantity: None,
                unit: None,
                confidence: 0.5,
                timestamp: Utc::now(),
                context: None,
            });
        }

        None
    }
}
```

### Hybrid Extraction Flow

```rust
use ollama::Ollama;

pub struct EventExtractor {
    ollama: Ollama,
    rule_fallback: RuleBasedExtractor,
}

impl EventExtractor {
    pub async fn extract(&self, text: &str) -> Result<Vec<ExtractedEvent>> {
        // Primary: SLM extraction
        match self.extract_with_slm(text).await {
            Ok(events) if !events.is_empty() => Ok(events),
            _ => {
                // Fallback: Rule-based extraction
                match self.rule_fallback.extract(text) {
                    Some(event) => Ok(vec![event]),
                    None => Ok(vec![]),  // No event found
                }
            }
        }
    }

    async fn extract_with_slm(&self, text: &str) -> Result<Vec<ExtractedEvent>> {
        let prompt = format!("{}\n\nInput: \"{}\"", PROMPT_TEMPLATE, text);

        let response = self.ollama
            .generate(&phi4_mini(), &prompt)
            .await?;

        // Parse JSON response
        let result: SLMResponse = serde_json::from_str(&response.response)?;

        Ok(result.events)
    }
}

const PROMPT_TEMPLATE: &str = include_str!("prompts/event_extraction.txt");
```

---

## Hallucination Mitigation

### Confidence Calibration

```rust
/// Adjust confidence based on extraction quality
pub fn calibrate_confidence(event: &ExtractedEvent, original_text: &str) -> f32 {
    let mut confidence = event.confidence;

    // Penalize if target not found in original text
    if !original_text.contains(&event.target) {
        confidence *= 0.5;
    }

    // Penalize if action is too generic
    if matches!(event.action.as_str(), "做" | "弄" | "搞") {
        confidence *= 0.7;
    }

    // Boost if both action and target clearly present
    if original_text.contains(&event.action) && original_text.contains(&event.target) {
        confidence = (confidence * 1.1).min(1.0);
    }

    confidence
}
```

### Validation Against Derived Views

```rust
/// Use existing knowledge to validate extraction
pub fn validate_with_views(
    event: &ExtractedEvent,
    views: &[DerivedView]
) -> f32 {
    for view in views {
        // Check if event contradicts known patterns
        if view.hypothesis.contains(&event.target) {
            if view.confidence > 0.8 {
                // High confidence view supports extraction
                return event.confidence * 1.05;
            }
        }
    }
    event.confidence
}
```

---

## 8GB Memory Optimization

### Batch Processing

```rust
/// Process multiple texts in a single SLM call
pub async fn extract_batch(&self, texts: &[String]) -> Result<Vec<Vec<ExtractedEvent>>> {
    let batch_prompt = format!(
        "{}\n\nExtract events from each input:\n{}",
        PROMPT_TEMPLATE,
        texts.iter()
            .enumerate()
            .map(|(i, t)| format!("Input {}: \"{}\"", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n\n")
    );

    let response = self.ollama.generate(&phi4_mini(), &batch_prompt).await?;
    // Parse batched response...

    Ok(results)
}
```

---

## Recommended Combinations

Use this skill together with:
- **OllamaPromptEngineering**: For SLM prompt optimization
- **EntityResolution**: For linking extracted targets to entities
- **CognitiveViewGeneration**: For building patterns from events
- **PostgresSchemaDesign**: For event_memories table structure
