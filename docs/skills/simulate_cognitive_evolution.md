# Skill: Simulate Cognitive Evolution

> **Purpose**: Simulate time progression to test promotion gate logic, verify concept stability, and validate conflict detection without waiting for real-time passage.

---

## Time Jumping Simulator

### Core Simulation Engine

```rust
use std::collections::HashMap;
use std::time::Duration;

/// Simulated time for cognitive evolution testing
pub struct TimeSimulator {
    simulated_now: DateTime<Utc>,
    speed_multiplier: u32,  // 1 real second = N simulated seconds
    event_log: Vec<TimedEvent>,
    view_log: Vec<TimedView>,
}

#[derive(Debug, Clone)]
pub struct TimedEvent {
    pub event: Event,
    pub simulated_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TimedView {
    pub view: DerivedView,
    pub simulated_created_at: DateTime<Utc>,
    pub simulated_expires_at: DateTime<Utc>,
}

impl TimeSimulator {
    /// Create new simulator starting at current time
    pub fn new() -> Self {
        Self {
            simulated_now: Utc::now(),
            speed_multiplier: 1,
            event_log: Vec::new(),
            view_log: Vec::new(),
        }
    }

    /// Jump forward by specified duration
    pub fn jump(&mut self, duration: Duration) {
        self.simulated_now = self.simulated_now + duration;
    }

    /// Get current simulated time
    pub fn now(&self) -> DateTime<Utc> {
        self.simulated_now
    }

    /// Add event at current simulated time
    pub fn add_event(&mut self, event: Event) {
        self.event_log.push(TimedEvent {
            event,
            simulated_timestamp: self.simulated_now,
        });
    }

    /// Create view at current simulated time
    pub fn create_view(&mut self, view: DerivedView) {
        let expires_at = self.simulated_now + Duration::days(30);

        self.view_log.push(TimedView {
            view,
            simulated_created_at: self.simulated_now,
            simulated_expires_at: expires_at,
        });
    }

    /// Check if view should promote based on simulated time
    pub fn check_promotion(&self, view_id: Uuid) -> bool {
        let timed_view = self.view_log.iter()
            .find(|tv| tv.view.view_id == view_id);

        if let Some(tv) = timed_view {
            // Calculate time span in simulated time
            let time_span = self.simulated_now - tv.simulated_created_at;

            // Use actual promotion logic but with simulated time
            should_promote_with_time(&tv.view, time_span, self.simulated_now)
        } else {
            false
        }
    }

    /// Get all views that should be promoted now
    pub fn get_promotable_views(&self) -> Vec<&DerivedView> {
        self.view_log.iter()
            .filter(|tv| self.check_promotion(tv.view.view_id))
            .map(|tv| &tv.view)
            .collect()
    }

    /// Get expired views based on simulated time
    pub fn get_expired_views(&self) -> Vec<&DerivedView> {
        self.view_log.iter()
            .filter(|tv| tv.simulated_expires_at <= self.simulated_now)
            .map(|tv| &tv.view)
            .collect()
    }
}

/// Promotion check with simulated time
fn should_promote_with_time(
    view: &DerivedView,
    time_span: Duration,
    simulated_now: DateTime<Utc>
) -> bool {
    // Rule 1: High confidence
    if view.confidence <= 0.85 {
        return false;
    }

    // Rule 2: Minimum time span (using simulated time)
    if time_span < Duration::days(30) {
        return false;
    }

    // Rule 3: Minimum validations
    if view.validation_count < 3 {
        return false;
    }

    // Rule 4: No conflicts
    if has_conflicting_views(view) {
        return false;
    }

    // Rule 5: Low counter-evidence
    let counter_ratio = view.counter_evidence.len() as f32
        / view.derived_from.len() as f32;
    if counter_ratio > 0.15 {
        return false;
    }

    true
}
```

---

## Cognitive Evolution Scenarios

### Test Scenarios

```rust
/// Predefined test scenarios for cognitive evolution
pub struct EvolutionScenarios;

impl EvolutionScenarios {
    /// Scenario 1: View promotes after 30 days with enough validation
    pub fn scenario_successful_promotion() -> SimulationScenario {
        SimulationScenario {
            name: "Successful Promotion",
            steps: vec![
                SimulationStep {
                    action: StepAction::AddEvent("eat coffee".to_string()),
                    time_jump: Duration::days(1),
                },
                // Add coffee events daily for 40 days
                // ... (40 events)
                SimulationStep {
                    action: StepAction::CreateTimePassed,
                    time_jump: Duration::days(40),
                },
                SimulationStep {
                    action: StepAction::CheckPromotion,
                    time_jump: Duration::zero(),
                },
            ],
            expected_outcome: ScenarioOutcome::ViewPromoted,
        }
    }

    /// Scenario 2: View expires without promotion (low confidence)
    pub fn scenario_low_confidence_expires() -> SimulationScenario {
        SimulationScenario {
            name: "Low Confidence Expiration",
            steps: vec![
                // Create view with confidence 0.7
                SimulationStep {
                    action: StepAction::CreateView {
                        hypothesis: "用户喜欢喝茶".to_string(),
                        confidence: 0.7,
                        validations: 5,
                    },
                    time_jump: Duration::zero(),
                },
                // Jump 35 days
                SimulationStep {
                    action: StepAction::CreateTimePassed,
                    time_jump: Duration::days(35),
                },
                SimulationStep {
                    action: StepAction::CheckPromotion,
                    time_jump: Duration::zero(),
                },
            ],
            expected_outcome: ScenarioOutcome::ViewExpired,
        }
    }

    /// Scenario 3: Conflicting views prevent promotion
    pub fn scenario_conflict_prevents_promotion() -> SimulationScenario {
        SimulationScenario {
            name: "Conflict Prevents Promotion",
            steps: vec![
                // Create "likes coffee" view
                SimulationStep {
                    action: StepAction::CreateView {
                        hypothesis: "用户喜欢喝咖啡".to_string(),
                        confidence: 0.9,
                        validations: 10,
                    },
                    time_jump: Duration::zero(),
                },
                // Create "allergic to coffee" view
                SimulationStep {
                    action: StepAction::CreateView {
                        hypothesis: "用户对咖啡过敏".to_string(),
                        confidence: 0.9,
                        validations: 10,
                    },
                    time_jump: Duration::zero(),
                },
                // Jump 35 days
                SimulationStep {
                    action: StepAction::CreateTimePassed,
                    time_jump: Duration::days(35),
                },
                SimulationStep {
                    action: StepAction::CheckPromotion,
                    time_jump: Duration::zero(),
                },
            ],
            expected_outcome: ScenarioOutcome::NoPromotionConflict,
        }
    }

    /// Scenario 4: View promotes after counter-evidence threshold exceeded
    pub fn scenario_counter_evidence_rejection() -> SimulationScenario {
        SimulationScenario {
            name: "Counter Evidence Rejection",
            steps: vec![
                // Create "vegetarian" view
                SimulationStep {
                    action: StepAction::CreateView {
                        hypothesis: "用户是素食主义者".to_string(),
                        confidence: 0.9,
                        validations: 10,
                    },
                    time_jump: Duration::zero(),
                },
                // Add counter-evidence: eating meat events
                SimulationStep {
                    action: StepAction::AddEvent("eat beef".to_string()),
                    time_jump: Duration::days(5),
                },
                SimulationStep {
                    action: StepAction::AddEvent("eat chicken".to_string()),
                    time_jump: Duration::days(5),
                },
                SimulationStep {
                    action: StepAction::AddEvent("eat pork".to_string()),
                    time_jump: Duration::days(5),
                },
                // Jump to expiration
                SimulationStep {
                    action: StepAction::CreateTimePassed,
                    time_jump: Duration::days(35),
                },
                SimulationStep {
                    action: StepAction::CheckStatus,
                    time_jump: Duration::zero(),
                },
            ],
            expected_outcome: ScenarioOutcome::ViewRejected,
        }
    }
}

#[derive(Debug)]
pub struct SimulationScenario {
    pub name: String,
    pub steps: Vec<SimulationStep>,
    pub expected_outcome: ScenarioOutcome,
}

#[derive(Debug)]
pub struct SimulationStep {
    pub action: StepAction,
    pub time_jump: Duration,
}

#[derive(Debug)]
pub enum StepAction {
    AddEvent(String),
    CreateView { hypothesis: String, confidence: f32, validations: i32 },
    CreateTimePassed,
    CheckPromotion,
    CheckStatus,
}

#[derive(Debug, PartialEq)]
pub enum ScenarioOutcome {
    ViewPromoted,
    ViewExpired,
    ViewRejected,
    NoPromotionConflict,
    NoPromotionLowValidation,
}
```

---

## Scenario Runner

```rust
/// Run simulation scenarios and verify outcomes
pub struct ScenarioRunner {
    simulator: TimeSimulator,
}

impl ScenarioRunner {
    pub fn new() -> Self {
        Self {
            simulator: TimeSimulator::new(),
        }
    }

    /// Run a scenario and verify expected outcome
    pub fn run_scenario(&mut self, scenario: SimulationScenario) -> ScenarioResult {
        println!("Running scenario: {}", scenario.name);

        for (i, step) in scenario.steps.iter().enumerate() {
            println!("  Step {}: {:?}", i + 1, step.action);

            self.execute_step(step);
            self.simulator.jump(step.time_jump);
        }

        let actual_outcome = self.determine_outcome();
        let passed = actual_outcome == scenario.expected_outcome;

        ScenarioResult {
            scenario_name: scenario.name,
            expected: scenario.expected_outcome,
            actual: actual_outcome,
            passed,
            final_state: self.capture_final_state(),
        }
    }

    fn execute_step(&mut self, step: &SimulationStep) {
        match &step.action {
            StepAction::AddEvent(event_desc) => {
                let event = Event {
                    event_id: Uuid::new_v4(),
                    timestamp: self.simulator.now(),
                    action: "do".to_string(),
                    target: event_desc.clone(),
                    quantity: None,
                    unit: None,
                    confidence: 1.0,
                    metadata: json!({}),
                };
                self.simulator.add_event(event);
            }

            StepAction::CreateView { hypothesis, confidence, validations } => {
                let view = DerivedView {
                    view_id: Uuid::new_v4(),
                    hypothesis: hypothesis.clone(),
                    derived_from: vec![],
                    confidence: *confidence,
                    expires_at: self.simulator.now() + Duration::days(30),
                    status: ViewStatus::Active,
                    created_at: self.simulator.now(),
                    validation_count: *validations,
                    view_type: ViewType::Preference,
                    counter_evidence: vec![],
                };
                self.simulator.create_view(view);
            }

            StepAction::CreateTimePassed => {
                // Time jump happens after step execution
            }

            StepAction::CheckPromotion => {
                let promotable = self.simulator.get_promotable_views();
                println!("    Promotable views: {}", promotable.len());
            }

            StepAction::CheckStatus => {
                let expired = self.simulator.get_expired_views();
                println!("    Expired views: {}", expired.len());
            }
        }
    }

    fn determine_outcome(&self) -> ScenarioOutcome {
        let promotable = self.simulator.get_promotable_views();
        let expired = self.simulator.get_expired_views();

        if !promotable.is_empty() {
            ScenarioOutcome::ViewPromoted
        } else if !expired.is_empty() {
            // Check if rejected or just expired
            let view = &expired[0];
            let counter_ratio = view.counter_evidence.len() as f32
                / view.derived_from.len() as f32;

            if counter_ratio > 0.3 {
                ScenarioOutcome::ViewRejected
            } else {
                ScenarioOutcome::ViewExpired
            }
        } else {
            // Check for conflicts
            ScenarioOutcome::NoPromotionConflict
        }
    }

    fn capture_final_state(&self) -> SimulationState {
        SimulationState {
            simulated_time: self.simulator.now(),
            total_events: self.simulator.event_log.len(),
            total_views: self.simulator.view_log.len(),
            promotable_count: self.simulator.get_promotable_views().len(),
            expired_count: self.simulator.get_expired_views().len(),
        }
    }
}

#[derive(Debug)]
pub struct ScenarioResult {
    pub scenario_name: String,
    pub expected: ScenarioOutcome,
    pub actual: ScenarioOutcome,
    pub passed: bool,
    pub final_state: SimulationState,
}

#[derive(Debug)]
pub struct SimulationState {
    pub simulated_time: DateTime<Utc>,
    pub total_events: usize,
    pub total_views: usize,
    pub promotable_count: usize,
    pub expired_count: usize,
}
```

---

## Conflict Detection Testing

```rust
/// Test conflict detection logic with simulated views
pub struct ConflictDetectionTest;

impl ConflictDetectionTest {
    /// Test detection of direct contradictions
    pub fn test_direct_contradiction() -> bool {
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

        let detector = ConflictDetector::new();
        detector.contradicts(&view_a, &view_b)
    }

    /// Test conflict resolution over time
    pub fn test_conflict_resolution() -> ScenarioResult {
        let mut runner = ScenarioRunner::new();

        let scenario = SimulationScenario {
            name: "Conflict Resolution".to_string(),
            steps: vec![
                // Create conflicting views
                SimulationStep {
                    action: StepAction::CreateView {
                        hypothesis: "用户喜欢喝咖啡".to_string(),
                        confidence: 0.9,
                        validations: 10,
                    },
                    time_jump: Duration::zero(),
                },
                SimulationStep {
                    action: StepAction::CreateView {
                        hypothesis: "用户讨厌喝咖啡".to_string(),
                        confidence: 0.9,
                        validations: 10,
                    },
                    time_jump: Duration::zero(),
                },
                // Add evidence supporting first view
                SimulationStep {
                    action: StepAction::AddEvent("drink coffee happily".to_string()),
                    time_jump: Duration::days(5),
                },
                // Jump and check
                SimulationStep {
                    action: StepAction::CheckPromotion,
                    time_jump: Duration::days(35),
                },
            ],
            expected_outcome: ScenarioOutcome::NoPromotionConflict,
        };

        runner.run_scenario(scenario)
    }
}
```

---

## Display and Reporting

```rust
impl std::fmt::Display for ScenarioResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.passed { "✅ PASS" } else { "❌ FAIL" };

        writeln!(f, "{}: {}", status, self.scenario_name)?;
        writeln!(f, "  Expected: {:?}", self.expected)?;
        writeln!(f, "  Actual: {:?}", self.actual)?;
        writeln!(f, "  Final State:")?;
        writeln!(f, "    Simulated Time: {}", self.final_state.simulated_time)?;
        writeln!(f, "    Events: {}", self.final_state.total_events)?;
        writeln!(f, "    Views: {}", self.final_state.total_views)?;
        writeln!(f, "    Promotable: {}", self.final_state.promotable_count)?;
        writeln!(f, "    Expired: {}", self.final_state.expired_count)?;

        Ok(())
    }
}

/// Run all scenarios and generate report
pub async fn run_all_scenarios() -> TestReport {
    let mut runner = ScenarioRunner::new();
    let mut results = Vec::new();

    // Run all scenarios
    results.push(runner.run_scenario(EvolutionScenarios::scenario_successful_promotion()));
    runner = ScenarioRunner::new();  // Reset

    results.push(runner.run_scenario(EvolutionScenarios::scenario_low_confidence_expires()));
    runner = ScenarioRunner::new();

    results.push(runner.run_scenario(EvolutionScenarios::scenario_conflict_prevents_promotion()));
    runner = ScenarioRunner::new();

    results.push(runner.run_scenario(EvolutionScenarios::scenario_counter_evidence_rejection()));

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    TestReport {
        results,
        passed,
        total,
    }
}

#[derive(Debug)]
pub struct TestReport {
    pub results: Vec<ScenarioResult>,
    pub passed: usize,
    pub total: usize,
}

impl std::fmt::Display for TestReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cognitive Evolution Simulation Report")?;
        writeln!(f, "=====================================")?;
        writeln!(f, "Results: {}/{} passed", self.passed, self.total)?;
        writeln!(f)?;

        for result in &self.results {
            writeln!(f, "{}", result)?;
            writeln!(f)?;
        }

        Ok(())
    }
}
```

---

## Recommended Combinations

Use this skill together with:
- **CognitiveViewGeneration**: For testing promotion gate logic
- **TestingAndDebugging**: For integration into test suite
- **EventExtractionPatterns**: For generating test events
