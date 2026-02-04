# Skill: Cognitive View Generation

> **Purpose**: Reinforce the "Slow Abstraction Principle," guiding Derived Views generation and Promotion Gate to avoid premature concept solidification and LLM hallucination amplification.

---

## Core Philosophy: Slow Abstraction

### The Principle (2026 Consensus)

```
Events (Fact) → Derived Views (Hypothesis) → Promotion Gate → Stable Concepts (Truth)

Key Rule: LLM generates hypotheses, programs decide truth.
```

**Why?** Prevents AI hallucinations from permanently corrupting the knowledge base.

---

## Derived View Structure

### Declarative Schema

```rust
/// A derived cognitive view - a hypothesis about the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedView {
    /// Unique identifier
    pub view_id: Uuid,

    /// The hypothesis (e.g., "用户喜欢在早上喝咖啡")
    pub hypothesis: String,

    /// Source events that support this view
    pub derived_from: Vec<Uuid>,

    /// Confidence score [0.0, 1.0]
    pub confidence: f32,

    /// When this view expires (default: 30 days)
    pub expires_at: DateTime<Utc>,

    /// View lifecycle status
    pub status: ViewStatus,

    /// When this view was first created
    pub created_at: DateTime<Utc>,

    /// How many times this view was validated
    pub validation_count: i32,

    /// View type (pattern, preference, habit, belief)
    pub view_type: ViewType,

    /// Counter-evidence (events that contradict)
    pub counter_evidence: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewStatus {
    /// Active and being tested
    Active,

    /// Expired without promotion
    Expired,

    /// Promoted to stable concept
    Promoted,

    /// Rejected due to contradictions
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewType {
    /// Behavioral pattern (e.g., "exercises on Mondays")
    Pattern,

    /// Preference (e.g., "prefers coffee over tea")
    Preference,

    /// Habit (e.g., "checks phone first thing in morning")
    Habit,

    /// Belief (e.g., "values work-life balance")
    Belief,
}
```

### Isolation Mechanism

```rust
/// Ensure LLM cannot directly modify system structure
impl DerivedView {
    /// LLM can only suggest hypotheses
    pub fn from_llm_hypothesis(
        hypothesis: String,
        evidence: Vec<Uuid>,
        llm_confidence: f32
    ) -> Self {
        Self {
            view_id: Uuid::new_v4(),
            hypothesis,
            derived_from: evidence,
            // Discount LLM confidence to prevent overconfidence
            confidence: llm_confidence * 0.7,
            expires_at: Utc::now() + Duration::days(30),
            status: ViewStatus::Active,
            created_at: Utc::now(),
            validation_count: 0,
            view_type: ViewType::Pattern,
            counter_evidence: Vec::new(),
        }
    }

    /// Program decides if view becomes stable concept
    pub fn check_should_promote(&self) -> bool {
        should_promote(self)
    }
}
```

---

## Promotion Gate Formula

### Programmatic Truth Detection

```rust
/// Pure programmatic promotion logic - no AI judgment here
pub fn should_promote(view: &DerivedView) -> bool {
    // Rule 1: High confidence threshold
    if view.confidence <= 0.85 {
        return false;
    }

    // Rule 2: Minimum time span (30 days)
    let time_span = Utc::now() - view.created_at;
    if time_span < Duration::days(30) {
        return false;
    }

    // Rule 3: Minimum validations
    if view.validation_count < 3 {
        return false;
    }

    // Rule 4: No conflicting views
    if has_conflicting_views(view) {
        return false;
    }

    // Rule 5: Low counter-evidence ratio
    let counter_ratio = view.counter_evidence.len() as f32
        / view.derived_from.len() as f32;
    if counter_ratio > 0.15 {
        return false;
    }

    true
}

/// Detect conflicting hypotheses
fn has_conflicting_views(view: &DerivedView) -> bool {
    // Implementation would check for contradictions like:
    // - "用户喜欢喝咖啡" vs "用户对咖啡过敏"
    // - "用户是素食主义者" vs "用户经常吃牛肉"

    // This is programmatic keyword matching, not LLM-based
    let contradiction_pairs = vec![
        ("喜欢", "讨厌"),
        ("经常", "很少"),
        ("总是", "从不"),
    ];

    // Check against existing views in database
    // Implementation omitted for brevity

    false
}
```

### Validation Increment

```rust
/// Increment validation when new evidence supports view
pub fn validate_view(view: &mut DerivedView, new_evidence: Uuid) {
    view.derived_from.push(new_evidence);
    view.validation_count += 1;

    // Recalculate confidence based on evidence strength
    recalculate_confidence(view);
}

fn recalculate_confidence(view: &mut DerivedView) {
    // Confidence increases with more supporting evidence
    let evidence_boost = (view.validation_count as f32).log10() * 0.05;

    // Confidence decreases with counter-evidence
    let counter_penalty = (view.counter_evidence.len() as f32) * 0.1;

    view.confidence = (0.5 + evidence_boost - counter_penalty)
        .min(1.0)
        .max(0.0);
}

/// Add counter-evidence when contradiction found
pub fn add_counter_evidence(view: &mut DerivedView, evidence: Uuid) {
    view.counter_evidence.push(evidence);

    // If too much counter-evidence, reject the view
    let counter_ratio = view.counter_evidence.len() as f32
        / view.derived_from.len() as f32;
    if counter_ratio > 0.3 {
        view.status = ViewStatus::Rejected;
    }

    recalculate_confidence(view);
}
```

---

## View Generation Pipeline

### Pattern Detection

```rust
/// Detect patterns from events to generate views
pub struct PatternDetector {
    event_store: EventStore,
    view_store: ViewStore,
}

impl PatternDetector {
    /// Find high-frequency behavioral patterns
    pub async fn detect_patterns(&self, user_id: &str) -> Result<Vec<DerivedView>> {
        let mut views = Vec::new();

        // Pattern 1: Daily behaviors
        let daily = self.find_daily_patterns(user_id).await?;
        views.extend(daily);

        // Pattern 2: Weekly routines
        let weekly = self.find_weekly_patterns(user_id).await?;
        views.extend(weekly);

        // Pattern 3: Preference patterns
        let preferences = self.find_preference_patterns(user_id).await?;
        views.extend(preferences);

        Ok(views)
    }

    async fn find_daily_patterns(&self, user_id: &str) -> Result<Vec<DerivedView>> {
        // Query: actions performed at least 5/7 days at similar time
        let sql = r#"
            SELECT action, target, COUNT(*) as frequency,
                   EXTRACT(HOUR FROM timestamp) as hour
            FROM event_memories
            WHERE user_id = $1
              AND timestamp >= NOW() - INTERVAL '30 days'
            GROUP BY action, target, EXTRACT(HOUR FROM timestamp)
            HAVING COUNT(*) >= 20
            ORDER BY frequency DESC
        "#;

        let patterns = self.event_store.query(sql, &[&user_id]).await?;

        patterns.into_iter().map(|row| {
            Ok(DerivedView {
                view_id: Uuid::new_v4(),
                hypothesis: format!(
                    "用户倾向于在 {} 点{} {}",
                    row.get("hour"),
                    row.get::<String, _>("action"),
                    row.get::<String, _>("target")
                ),
                derived_from: vec![],  // Populated separately
                confidence: 0.6,  // Base confidence, will increase with validation
                expires_at: Utc::now() + Duration::days(30),
                status: ViewStatus::Active,
                created_at: Utc::now(),
                validation_count: row.get("frequency"),
                view_type: ViewType::Pattern,
                counter_evidence: vec![],
            })
        }).collect()
    }

    async fn find_preference_patterns(&self, user_id: &str) -> Result<Vec<DerivedView>> {
        // Query: choices made when alternatives available
        // Example: When buying drinks, chooses coffee over tea
        let sql = r#"
            WITH choices AS (
                SELECT
                    target,
                    COUNT(*) as chosen_count,
                    SUM(COUNT(*)) OVER (PARTITION BY action) as total_count
                FROM event_memories
                WHERE user_id = $1
                  AND action IN ('买', '喝', '吃')
                  AND timestamp >= NOW() - INTERVAL '60 days'
                GROUP BY action, target
            )
            SELECT target, chosen_count, total_count,
                   chosen_count::FLOAT / total_count as preference_ratio
            FROM choices
            WHERE preference_ratio >= 0.7  -- 70%+ preference
              AND chosen_count >= 5
        "#;

        let patterns = self.event_store.query(sql, &[&user_id]).await?;

        patterns.into_iter().map(|row| {
            Ok(DerivedView {
                view_id: Uuid::new_v4(),
                hypothesis: format!(
                    "用户偏好 {}，选择比例 {:.0}%",
                    row.get::<String, _>("target"),
                    row.get::<f64, _>("preference_ratio") * 100.0
                ),
                derived_from: vec![],
                confidence: row.get::<f64, _>("preference_ratio") as f32,
                expires_at: Utc::now() + Duration::days(30),
                status: ViewStatus::Active,
                created_at: Utc::now(),
                validation_count: row.get("chosen_count"),
                view_type: ViewType::Preference,
                counter_evidence: vec![],
            })
        }).collect()
    }
}
```

---

## Conflict Detection Algorithm

```rust
/// Detect contradictions between views
pub struct ConflictDetector {
    view_store: ViewStore,
}

impl ConflictDetector {
    /// Find views that contradict each other
    pub async fn find_conflicts(&self) -> Result<Vec<(Uuid, Uuid)>> {
        let active_views = self.view_store.get_active_views().await?;
        let mut conflicts = Vec::new();

        for (i, view_a) in active_views.iter().enumerate() {
            for view_b in active_views.iter().skip(i + 1) {
                if self.contradicts(view_a, view_b) {
                    conflicts.push((view_a.view_id, view_b.view_id));
                }
            }
        }

        Ok(conflicts)
    }

    /// Programmatic contradiction detection
    fn contradicts(&self, view_a: &DerivedView, view_b: &DerivedView) -> bool {
        // Direct keyword contradictions
        if has_opposing_keywords(&view_a.hypothesis, &view_b.hypothesis) {
            return true;
        }

        // Entity-based contradictions
        if let (Some(entity_a), Some(entity_b)) = (
            extract_entity(&view_a.hypothesis),
            extract_entity(&view_b.hypothesis)
        ) {
            if entity_a == entity_b {
                // Same entity with opposing predicates
                return has_opposing_predicates(&view_a.hypothesis, &view_b.hypothesis);
            }
        }

        false
    }
}

fn has_opposing_keywords(text_a: &str, text_b: &str) -> bool {
    let oppositions = vec![
        ("喜欢", "不喜欢"),
        ("爱", "讨厌"),
        ("经常", "很少"),
        ("总是", "从不"),
        ("是", "不是"),
    ];

    for (pos, neg) in oppositions {
        if text_a.contains(pos) && text_b.contains(neg) {
            return true;
        }
        if text_a.contains(neg) && text_b.contains(pos) {
            return true;
        }
    }

    false
}

fn extract_entity(hypothesis: &str) -> Option<String> {
    // Simple extraction: assume last noun phrase is entity
    // In production, use NLP or entity linking
    let words: Vec<&str> = hypothesis.split_whitespace().collect();
    words.last().map(|s| s.to_string())
}
```

---

## Expiration and Cleanup

```rust
/// Periodic view maintenance
pub async fn maintain_views(store: &ViewStore) -> Result<()> {
    let now = Utc::now();

    // 1. Mark expired views
    let expired = store.find_expired_before(now).await?;

    for mut view in expired {
        match view.status {
            ViewStatus::Active => {
                // Try to promote before expiry
                if should_promote(&view) {
                    view.status = ViewStatus::Promoted;
                    store.update_view(&view).await?;
                } else {
                    view.status = ViewStatus::Expired;
                    store.update_view(&view).await?;
                }
            }
            _ => {}
        }
    }

    // 2. Archive old expired views
    store.archive_before(now - Duration::days(90)).await?;

    Ok(())
}
```

---

## Testing with Time Jumping

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Simulate time passage for testing promotion logic
    struct TimeSimulator {
        current_time: DateTime<Utc>,
        event_store: EventStore,
        view_store: ViewStore,
    }

    impl TimeSimulator {
        fn new() -> Self {
            Self {
                current_time: Utc::now(),
                event_store: EventStore::mock(),
                view_store: ViewStore::mock(),
            }
        }

        /// Jump forward in time
        fn jump(&mut self, duration: Duration) {
            self.current_time = self.current_time + duration;
            self.view_store.set_current_time(self.current_time);
        }

        /// Test promotion after 30 days
        #[test]
        fn test_promotion_after_30_days() {
            let mut sim = TimeSimulator::new();

            // Create initial view
            let view = DerivedView {
                view_id: Uuid::new_v4(),
                hypothesis: "用户喜欢喝咖啡".to_string(),
                derived_from: vec![],
                confidence: 0.9,
                expires_at: sim.current_time + Duration::days(30),
                status: ViewStatus::Active,
                created_at: sim.current_time,
                validation_count: 5,
                view_type: ViewType::Preference,
                counter_evidence: vec![],
            };

            sim.view_store.save_view(&view).unwrap();

            // Before 30 days - should not promote
            sim.jump(Duration::days(29));
            assert!(!sim.view_store.get_view(view.view_id).unwrap().check_should_promote());

            // After 30 days - should promote
            sim.jump(Duration::days(1));
            assert!(sim.view_store.get_view(view.view_id).unwrap().check_should_promote());
        }

        /// Test conflict detection
        #[test]
        fn test_conflict_detection() {
            let detector = ConflictDetector::new();

            let view_a = DerivedView {
                hypothesis: "用户喜欢吃肉".to_string(),
                // ...
                ..Default::default()
            };

            let view_b = DerivedView {
                hypothesis: "用户是素食主义者".to_string(),
                // ...
                ..Default::default()
            };

            assert!(detector.contradicts(&view_a, &view_b));
        }
    }
}
```

---

## Recommended Combinations

Use this skill together with:
- **EventExtractionPatterns**: For source events to derive views from
- **EntityResolution**: For entity-based pattern detection
- **PluginPermissionSystem**: For controlling plugin access to cognitive layer
- **OllamaPromptEngineering**: For hypothesis generation prompts
