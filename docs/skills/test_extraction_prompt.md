# Skill: Test Extraction Prompt

> **Purpose**: Provide a sandbox environment for testing event extraction prompts with preset inputs, comparing rule engine vs SLM outputs to ensure high confidence and reduced hallucination.

---

## Prompt Testing Framework

### Test Harness

```rust
use std::collections::HashMap;

/// Test harness for event extraction prompts
pub struct ExtractionPromptTester {
    ollama: Ollama,
    rule_engine: RuleBasedExtractor,
    test_cases: Vec<TestCase>,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub input: String,
    pub expected_actions: Vec<String>,
    pub expected_targets: Vec<String>,
    pub expected_quantities: Vec<Option<f32>>,
    pub description: &'static str,
    pub category: TestCategory,
}

#[derive(Debug, Clone)]
pub enum TestCategory {
    Simple,           // "吃了3个苹果"
    FuzzyTime,        // "昨天去健身房"
    MultiEvent,       // "早上喝咖啡，下午喝茶"
    NoEvent,          // "今天天气真好"
    Ambiguous,        // "这周买了咖啡" (missing quantity)
    EdgeCase,         // Boundary conditions
}

impl ExtractionPromptTester {
    pub fn new(ollama: Ollama) -> Self {
        Self {
            ollama,
            rule_engine: RuleBasedExtractor::new(),
            test_cases: Self::load_test_cases(),
        }
    }

    fn load_test_cases() -> Vec<TestCase> {
        vec![
            // Simple cases
            TestCase {
                input: "我今天早上吃了3个苹果".to_string(),
                expected_actions: vec!["吃".to_string()],
                expected_targets: vec!["苹果".to_string()],
                expected_quantities: vec![Some(3.0)],
                description: "Simple event with quantity",
                category: TestCategory::Simple,
            },

            TestCase {
                input: "昨天去健身房了".to_string(),
                expected_actions: vec!["去".to_string()],
                expected_targets: vec!["健身房".to_string()],
                expected_quantities: vec![Some(1.0)],
                description: "Fuzzy time - yesterday",
                category: TestCategory::FuzzyTime,
            },

            // Multi-event
            TestCase {
                input: "早上喝了咖啡，下午又喝了茶".to_string(),
                expected_actions: vec!["喝".to_string(), "喝".to_string()],
                expected_targets: vec!["咖啡".to_string(), "茶".to_string()],
                expected_quantities: vec![Some(1.0), Some(1.0)],
                description: "Multiple events in one sentence",
                category: TestCategory::MultiEvent,
            },

            // No event
            TestCase {
                input: "今天天气真好啊".to_string(),
                expected_actions: vec![],
                expected_targets: vec![],
                expected_quantities: vec![],
                description: "Opinion, not an event",
                category: TestCategory::NoEvent,
            },

            // Ambiguous
            TestCase {
                input: "这周买了咖啡".to_string(),
                expected_actions: vec!["买".to_string()],
                expected_targets: vec!["咖啡".to_string()],
                expected_quantities: vec![None],  // Missing quantity
                description: "Event without quantity",
                category: TestCategory::Ambiguous,
            },

            // More test cases...
        ]
    }

    /// Run all test cases with both SLM and rule engine
    pub async fn run_all_tests(&self) -> TestReport {
        let mut results = Vec::new();

        for case in &self.test_cases {
            let result = self.test_case(case).await;
            results.push(result);
        }

        TestReport {
            results,
            summary: self.generate_summary(&results),
        }
    }

    /// Test a single case
    async fn test_case(&self, case: &TestCase) -> CaseResult {
        // SLM extraction
        let slm_result = self.extract_with_slm(&case.input).await;
        let slm_events = slm_result.as_ref().map(|e| e.as_slice()).unwrap_or(&[]);

        // Rule engine extraction
        let rule_result = self.rule_engine.extract(&case.input);
        let rule_events = rule_result.as_ref().map(|e| std::slice::from_ref(e)).unwrap_or(&[]);

        // Compare results
        let comparison = self.compare_extractions(
            case,
            slm_events,
            rule_events,
        );

        CaseResult {
            case: case.clone(),
            slm_result,
            rule_result,
            comparison,
        }
    }

    async fn extract_with_slm(&self, input: &str) -> Result<Vec<ExtractedEvent>> {
        let prompt = format!(
            "{}\n\nInput: \"{}\"",
            EVENT_EXTRACTION_PROMPT_TEMPLATE,
            input
        );

        let response = self.ollama.generate(&phi4_mini(), &prompt).await?;

        // Parse JSON response
        EventExtractionPrompt::new().parse_response(&response.response)
    }

    fn compare_extractions(
        &self,
        case: &TestCase,
        slm_events: &[ExtractedEvent],
        rule_events: &[ExtractedEvent]
    ) -> ExtractionComparison {
        ExtractionComparison {
            slm_correct: self.verify_events(case, slm_events),
            rule_correct: self.verify_events(case, rule_events),
            slm_vs_rule: self.compare_methods(slm_events, rule_events),
            confidence_analysis: self.analyze_confidence(slm_events),
        }
    }

    fn verify_events(&self, case: &TestCase, events: &[ExtractedEvent]) -> VerificationResult {
        let mut result = VerificationResult {
            actions_match: false,
            targets_match: false,
            quantities_match: false,
            event_count_match: false,
            details: HashMap::new(),
        };

        result.event_count_match = events.len() == case.expected_actions.len();

        if !events.is_empty() {
            result.actions_match = events.iter()
                .zip(case.expected_actions.iter())
                .all(|(e, exp)| &e.action == exp);

            result.targets_match = events.iter()
                .zip(case.expected_targets.iter())
                .all(|(e, exp)| &e.target == exp);

            result.quantities_match = events.iter()
                .zip(case.expected_quantities.iter())
                .all(|(e, exp)| e.quantity == *exp);
        } else if case.expected_actions.is_empty() {
            // Both empty - correct for "no event" cases
            result.actions_match = true;
            result.targets_match = true;
            result.quantities_match = true;
        }

        result
    }

    fn compare_methods(
        &self,
        slm: &[ExtractedEvent],
        rule: &[ExtractedEvent]
    ) -> MethodComparison {
        MethodComparison {
            slm_found_more: slm.len() > rule.len(),
            rule_found_more: rule.len() > slm.len(),
            slm_higher_confidence: self.compare_confidence(slm, rule),
        }
    }

    fn compare_confidence(&self, slm: &[ExtractedEvent], rule: &[ExtractedEvent]) -> bool {
        let slm_avg: f32 = slm.iter().map(|e| e.confidence).sum::<f32>()
            / slm.len().max(1) as f32;

        let rule_avg: f32 = rule.iter().map(|e| e.confidence).sum::<f32>()
            / rule.len().max(1) as f32;

        slm_avg > rule_avg
    }

    fn analyze_confidence(&self, events: &[ExtractedEvent]) -> ConfidenceAnalysis {
        if events.is_empty() {
            return ConfidenceAnalysis {
                min: 0.0,
                max: 0.0,
                avg: 0.0,
                below_threshold: 0,
            };
        }

        let confidences: Vec<f32> = events.iter().map(|e| e.confidence).collect();
        let min = confidences.iter().cloned().reduce(f32::min).unwrap_or(0.0);
        let max = confidences.iter().cloned().reduce(f32::max).unwrap_or(0.0);
        let avg = confidences.iter().sum::<f32>() / confidences.len() as f32;
        let below_threshold = confidences.iter().filter(|&&c| c < 0.8).count();

        ConfidenceAnalysis { min, max, avg, below_threshold }
    }

    fn generate_summary(&self, results: &[CaseResult]) -> TestSummary {
        let total = results.len();
        let slm_correct = results.iter().filter(|r| r.comparison.slm_correct.all_match()).count();
        let rule_correct = results.iter().filter(|r| r.comparison.rule_correct.all_match()).count();

        let by_category = self.summarize_by_category(results);

        TestSummary {
            total_cases: total,
            slm_accuracy: slm_correct as f64 / total as f64,
            rule_accuracy: rule_correct as f64 / total as f64,
            by_category,
        }
    }

    fn summarize_by_category(&self, results: &[CaseResult]) -> HashMap<TestCategory, f64> {
        let mut by_category: HashMap<TestCategory, (usize, usize)> = HashMap::new();

        for result in results {
            let entry = by_category
                .entry(result.case.category.clone())
                .or_insert((0, 0));

            entry.0 += 1;  // Total
            if result.comparison.slm_correct.all_match() {
                entry.1 += 1;  // Correct
            }
        }

        by_category
            .into_iter()
            .map(|(cat, (total, correct))| (cat, correct as f64 / total as f64))
            .collect()
    }
}

#[derive(Debug)]
pub struct TestReport {
    pub results: Vec<CaseResult>,
    pub summary: TestSummary,
}

#[derive(Debug)]
pub struct CaseResult {
    pub case: TestCase,
    pub slm_result: Result<Vec<ExtractedEvent>>,
    pub rule_result: Option<ExtractedEvent>,
    pub comparison: ExtractionComparison,
}

#[derive(Debug)]
pub struct ExtractionComparison {
    pub slm_correct: VerificationResult,
    pub rule_correct: VerificationResult,
    pub slm_vs_rule: MethodComparison,
    pub confidence_analysis: ConfidenceAnalysis,
}

#[derive(Debug)]
pub struct VerificationResult {
    pub actions_match: bool,
    pub targets_match: bool,
    pub quantities_match: bool,
    pub event_count_match: bool,
    pub details: HashMap<String, bool>,
}

impl VerificationResult {
    pub fn all_match(&self) -> bool {
        self.actions_match && self.targets_match && self.quantities_match && self.event_count_match
    }
}

#[derive(Debug)]
pub struct MethodComparison {
    pub slm_found_more: bool,
    pub rule_found_more: bool,
    pub slm_higher_confidence: bool,
}

#[derive(Debug)]
pub struct ConfidenceAnalysis {
    pub min: f32,
    pub max: f32,
    pub avg: f32,
    pub below_threshold: usize,  // Count of events with confidence < 0.8
}

#[derive(Debug)]
pub struct TestSummary {
    pub total_cases: usize,
    pub slm_accuracy: f64,
    pub rule_accuracy: f64,
    pub by_category: HashMap<TestCategory, f64>,
}
```

---

## Sandbox Environment

### Isolated Testing

```rust
/// Isolated test environment that doesn't affect production
pub struct PromptSandbox {
    test_db: TestDatabase,
    mock_ollama: MockOllama,
}

impl PromptSandbox {
    /// Create isolated sandbox
    pub async fn new() -> Result<Self> {
        Ok(Self {
            test_db: TestDatabase::in_memory().await?,
            mock_ollama: MockOllama::new(),
        })
    }

    /// Test prompt with mock response
    pub async fn test_with_mock(
        &self,
        prompt: &str,
        mock_response: &str
    ) -> Result<Vec<ExtractedEvent>> {
        self.mock_ollama.set_response(mock_response);

        let extractor = EventExtractor::new(self.mock_ollama.clone());
        extractor.extract_from_prompt(prompt).await
    }

    /// Test full pipeline without side effects
    pub async fn test_pipeline(&self, input: &str) -> Result<TestPipelineResult> {
        // Extract events
        let events = self.extract_events(input).await?;

        // Verify against test DB (in-memory)
        for event in &events {
            self.test_db.store_event(event).await?;
        }

        // Retrieve and verify
        let retrieved = self.test_db.get_events_by_input(input).await?;

        Ok(TestPipelineResult {
            extracted: events,
            stored: retrieved,
            pipeline_integrity: self.verify_pipeline_integrity(&events, &retrieved),
        })
    }

    fn verify_pipeline_integrity(
        &self,
        extracted: &[ExtractedEvent],
        stored: &[ExtractedEvent]
    ) -> bool {
        // All events were stored correctly
        extracted.len() == stored.len()
    }
}

#[derive(Debug)]
pub struct TestPipelineResult {
    pub extracted: Vec<ExtractedEvent>,
    pub stored: Vec<ExtractedEvent>,
    pub pipeline_integrity: bool,
}
```

---

## Prompt A/B Testing

```rust
/// Compare different prompt versions
pub struct PromptAbTester {
    tester: ExtractionPromptTester,
}

impl PromptAbTester {
    /// Test prompt A vs prompt B
    pub async fn compare_prompts(
        &self,
        prompt_a: &str,
        prompt_b: &str,
        test_cases: &[TestCase]
    ) -> PromptComparison {
        let mut results_a = Vec::new();
        let mut results_b = Vec::new();

        for case in test_cases {
            let result_a = self.test_prompt(prompt_a, case).await;
            let result_b = self.test_prompt(prompt_b, case).await;

            results_a.push(result_a);
            results_b.push(result_b);
        }

        let winner = self.determine_winner(&results_a, &results_b);

        PromptComparison {
            prompt_a_results: results_a,
            prompt_b_results: results_b,
            winner,
        }
    }

    async fn test_prompt(&self, prompt: &str, case: &TestCase) -> f32 {
        // Run test with specific prompt
        // Return accuracy score
        0.0  // Placeholder
    }

    fn determine_winner(&self, results_a: &[f32], results_b: &[f32]) -> PromptWinner {
        let avg_a: f32 = results_a.iter().sum();
        let avg_b: f32 = results_b.iter().sum();

        if avg_a > avg_b + 0.05 {
            PromptWinner::A
        } else if avg_b > avg_a + 0.05 {
            PromptWinner::B
        } else {
            PromptWinner::Tie
        }
    }
}

#[derive(Debug)]
pub struct PromptComparison {
    pub prompt_a_results: Vec<f32>,
    pub prompt_b_results: Vec<f32>,
    pub winner: PromptWinner,
}

#[derive(Debug)]
pub enum PromptWinner {
    A,
    B,
    Tie,
}
```

---

## Display Results

```rust
impl std::fmt::Display for TestReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Event Extraction Test Report")?;
        writeln!(f, "============================")?;
        writeln!(f)?;

        writeln!(f, "Overall Results:")?;
        writeln!(f, "  Total Cases: {}", self.summary.total_cases)?;
        writeln!(f, "  SLM Accuracy: {:.1}%", self.summary.slm_accuracy * 100.0)?;
        writeln!(f, "  Rule Accuracy: {:.1}%", self.summary.rule_accuracy * 100.0)?;
        writeln!(f)?;

        writeln!(f, "Results by Category:")?;
        for (cat, accuracy) in &self.summary.by_category {
            writeln!(f, "  {:?}: {:.1}%", cat, accuracy * 100.0)?;
        }
        writeln!(f)?;

        writeln!(f, "Detailed Results:")?;
        for result in &self.results {
            writeln!(f, "\n  Test: {}", result.case.description)?;
            writeln!(f, "    Input: {}", result.case.input)?;
            writeln!(f, "    SLM: {:?}", result.slm_result)?;
            writeln!(f, "    Rule: {:?}", result.rule_result)?;
            writeln!(f, "    Confidence: min={:.2}, max={:.2}, avg={:.2}",
                result.comparison.confidence_analysis.min,
                result.comparison.confidence_analysis.max,
                result.comparison.confidence_analysis.avg,
            )?;
        }

        Ok(())
    }
}
```

---

## Recommended Combinations

Use this skill together with:
- **EventExtractionPatterns**: For testing extraction quality
- **OllamaPromptEngineering**: For A/B testing prompt variations
- **TestingAndDebugging**: For integration into test suite
