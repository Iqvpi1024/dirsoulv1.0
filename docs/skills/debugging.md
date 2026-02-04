# Skill: Debugging

> **Purpose**: Guide AI through systematic debugging: reproduce → trace → fix → verify, with special attention to 8GB memory constraints and HEAD.md compliance.

---

## Debugging Framework

### Systematic Debugging Process

```yaml
debugging_workflow:
  1_reproduce:
    - "获取复现步骤"
    - "确认环境条件（8GB内存）"
    - "记录错误日志"

  2_understand:
    - "追踪完整调用栈"
    - "理解数据流"
    - "检查HEAD.md是否有相关约束"

  3_hypothesize:
    - "列出可能原因"
    - "按可能性排序"
    - "参考相似历史问题"

  4_test:
    - "创建最小复现"
    - "隔离变量"
    - "验证假设"

  5_fix:
    - "实施修复"
    - "不违反HEAD.md原则"
    - "添加防护措施"

  6_verify:
    - "确认问题解决"
    - "无副作用"
    - "添加测试用例"
```

---

## Common Issue Patterns

### Memory Issues (8GB Environment)

```rust
/// Debug memory-related problems
pub struct MemoryDebugger;

impl MemoryDebugger {
    /// Diagnose OOM situations
    pub fn diagnose_oom(&self, error: &Error, context: &DebugContext) -> Diagnosis {
        let mut diagnosis = Diagnosis::new();

        // Check what was consuming memory
        diagnosis.add_check("Check Vec allocations");
        if let Some(vec_info) = self.check_vec_growth(context) {
            diagnosis.add_evidence(format!("Unbounded Vec: {:?}", vec_info));
        }

        diagnosis.add_check("Check Ollama memory usage");
        let ollama_usage = self.check_ollama_memory();
        diagnosis.add_evidence(format!("Ollama: {:.1} GB", ollama_usage));

        diagnosis.add_check("Check PostgreSQL memory");
        let pg_usage = self.check_postgres_memory();
        diagnosis.add_evidence(format!("PostgreSQL: {:.1} GB", pg_usage));

        // Calculate available
        let available = 8.0 - ollama_usage - pg_usage - 1.0;  // 1GB system
        diagnosis.add_evidence(format!("Available: {:.1} GB", available));

        // Determine cause
        if available < 0.5 {
            diagnosis.add_cause("Insufficient memory headroom");
            diagnosis.add_suggestion("Reduce batch sizes or implement streaming");
            diagnosis.add_reference("EstimateResourceUsage skill");
        }

        diagnosis
    }

    fn check_vec_growth(&self, context: &DebugContext) -> Option<VecInfo> {
        // Look for unbounded Vec patterns
        if context.code.contains("Vec::new()") && contains_loop(context.code) {
            Some(VecInfo {
                location: find_location(context.code, "Vec::new()"),
                suggestion: "Use Vec::with_capacity() or process in chunks",
            })
        } else {
            None
        }
    }
}
```

### Async Concurrency Issues

```rust
/// Debug async/await problems
pub struct AsyncDebugger;

impl AsyncDebugger {
    pub fn diagnose_async_issue(&self, error: &Error) -> Diagnosis {
        let mut diagnosis = Diagnosis::new();

        // Check for common async pitfalls
        if error.to_string().contains("Send") {
            diagnosis.add_check("Check for non-Send types in async");
            diagnosis.add_suggestion("Use Arc instead of Rc in async contexts");
            diagnosis.add_reference("RustMemorySafety skill");
        }

        if error.to_string().contains("deadlock") {
            diagnosis.add_check("Check for lock ordering");
            diagnosis.add_check("Check for .await inside lock hold");
            diagnosis.add_suggestion("Use tokio::sync::RwLock instead of std::sync");
        }

        diagnosis
    }
}
```

### Database Connection Issues

```rust
/// Debug database problems
pub struct DatabaseDebugger;

impl DatabaseDebugger {
    pub fn diagnose_db_issue(&self, error: &Error) -> Diagnosis {
        let mut diagnosis = Diagnosis::new();

        let error_msg = error.to_string();

        if error_msg.contains("connection") {
            diagnosis.add_check("Check connection pool size");
            diagnosis.add_evidence("Max connections might be too high for 8GB");
            diagnosis.add_suggestion("Limit to 50 connections");
            diagnosis.add_reference("PostgresSchemaDesign skill");
        }

        if error_msg.contains("timeout") {
            diagnosis.add_check("Check query performance");
            diagnosis.add_check("Review index usage");
            diagnosis.add_suggestion("Run EXPLAIN ANALYZE on slow queries");
        }

        if error_msg.contains("out of memory") {
            diagnosis.add_check("Check PostgreSQL shared_buffers");
            diagnosis.add_evidence("Recommended: 256MB for 8GB system");
            diagnosis.add_suggestion("Adjust postgresql.conf");
        }

        diagnosis
    }
}
```

---

## SLM/Ollama Issues

### AI Integration Debugging

```rust
/// Debug Ollama/Phi-4-mini issues
pub struct OllamaDebugger;

impl OllamaDebugger {
    pub fn diagnose_ollama_issue(&self, error: &Error) -> Diagnosis {
        let mut diagnosis = Diagnosis::new();

        // Check Ollama service
        diagnosis.add_check("Verify Ollama is running");
        if !self.is_ollama_running() {
            diagnosis.add_cause("Ollama service not running");
            diagnosis.add_suggestion("Run: ollama serve");
        }

        // Check model availability
        diagnosis.add_check("Check Phi-4-mini model");
        if !self.model_exists("phi4-mini") {
            diagnosis.add_cause("Model not downloaded");
            diagnosis.add_suggestion("Run: ollama pull phi4-mini");
        }

        // Check memory
        let available = self.get_available_memory();
        if available < 5.0 {
            diagnosis.add_cause("Insufficient memory for Phi-4-mini");
            diagnosis.add_evidence("Phi-4-mini Q4 requires ~5GB");
            diagnosis.add_suggestion("Close other applications or use Q5_QP_K quantization");
        }

        // Check prompt size
        diagnosis.add_check("Check prompt token count");
        let tokens = estimate_tokens(&error.context().prompt);
        if tokens > 4000 {
            diagnosis.add_cause("Prompt too long for model context");
            diagnosis.add_suggestion("Use batch processing or context compression");
            diagnosis.add_reference("OllamaPromptEngineering skill");
        }

        diagnosis
    }

    /// Debug extraction failures
    pub fn diagnose_extraction_failure(&self, prompt: &str, response: &str) -> Diagnosis {
        let mut diagnosis = Diagnosis::new();

        // Check response format
        if !response.contains("{") || !response.contains("}") {
            diagnosis.add_cause("SLM did not return valid JSON");
            diagnosis.add_suggestion("Improve prompt to explicitly require JSON format");
            diagnosis.add_reference("OllamaPromptEngineering skill");
            diagnosis.add_reference("TestExtractionPrompt skill for A/B testing");
        }

        // Check for hallucinations
        if contains_hallucination(response) {
            diagnosis.add_cause("SLM hallucinated content not in input");
            diagnosis.add_suggestion("Add negative examples to prompt");
            diagnosis.add_suggestion("Validate extracted entities against known views");
            diagnosis.add_reference("CognitiveViewGeneration skill");
        }

        diagnosis
    }
}
```

---

## Trace-Based Debugging

### Log Analysis

```rust
/// Analyze logs to find issues
pub struct LogAnalyzer;

impl LogAnalyzer {
    pub fn analyze_logs(&self, log_path: &Path) -> Vec<IssuePattern> {
        let logs = std::fs::read_to_string(log_path).unwrap();
        let mut issues = Vec::new();

        // Find error patterns
        for line in logs.lines() {
            if line.contains("ERROR") {
                if line.contains("memory") {
                    issues.push(IssuePattern {
                        pattern: "Memory Error",
                        frequency: self.count_pattern(&logs, "memory"),
                        likely_cause: "Memory leak or OOM",
                        suggestion: "Check with EstimateResourceUsage skill",
                    });
                }

                if line.contains("llm") || line.contains("ollama") {
                    issues.push(IssuePattern {
                        pattern: "SLM Error",
                        frequency: self.count_pattern(&logs, "llm|ollama"),
                        likely_cause: "Ollama service or model issue",
                        suggestion: "Check with OllamaDebugger",
                    });
                }
            }
        }

        issues.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        issues
    }
}
```

---

## HEAD.md Compliance in Debugging

### Fix Must Not Violate Principles

```rust
/// Ensure fixes comply with HEAD.md
pub struct ComplianceDebugger;

impl ComplianceDebugger {
    pub fn review_fix(&self, fix: &str, original_issue: &str) -> FixReview {
        let mut review = FixReview::new();

        // Check if fix introduces hardcoded rules
        if contains_hardcoded_rules(fix) {
            review.add_violation("Hardcoded rules introduced");
            review.add_suggestion("Maintain AI-Native principle");
            review.add_reference("EventExtractionPatterns skill");
        }

        // Check if fix bypasses Derived Views
        if bypasses_derived_views(fix) {
            review.add_violation("Bypassed Slow Abstraction");
            review.add_suggestion("Use CognitiveViewGeneration skill");
        }

        // Check if fix compromises privacy
        if compromises_privacy(fix) {
            review.add_violation("Privacy compromised");
            review.add_suggestion("Maintain zero-cloud dependency");
            review.add_reference("EncryptionBestPractices skill");
        }

        // Verify fix actually addresses issue
        if !fixes_issue(fix, original_issue) {
            review.add_problem("Fix does not address root cause");
        }

        review
    }
}
```

---

## Error Recovery Patterns

### Retry with Exponential Backoff

```rust
/// Standard retry pattern for transient errors
pub async fn retry_with_logging<T, E, F>(
    operation: F,
    max_retries: usize,
    operation_name: &str
) -> Result<T, E>
where
    F: Fn() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut delay = Duration::from_millis(100);
    let mut attempt = 0;

    loop {
        match operation() {
            Ok(result) => {
                if attempt > 0 {
                    info!("{} succeeded after {} retries", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    error!(
                        operation = operation_name,
                        attempts = attempt,
                        error = %e,
                        "Operation failed after {} attempts",
                        max_retries
                    );
                    return Err(e);
                }

                warn!(
                    operation = operation_name,
                    attempt = attempt,
                    error = %e,
                    next_retry_ms = delay.as_millis(),
                    "Operation failed, retrying"
                );

                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
    }
}
```

---

## Debug Session Template

```markdown
## Debug Session: [Issue Title]

Date: {{date}}
Task: {{task_id}}

### Issue Description
{{what_is_wrong}}

### Reproduction Steps
1. {{step_1}}
2. {{step_2}}
3. {{step_3}}

### Error Output
```
{{error_message}}
```

### Investigation

#### Checks Performed
- [ ] Environment: 8GB memory, Ollama running
- [ ] HEAD.md compliance verified
- [ ] Similar issues searched

#### Findings
- {{finding_1}}
- {{finding_2}}

### Root Cause
{{root_cause_analysis}}

### Proposed Fix
{{fix_description}}

### HEAD.md Compliance Check
- [ ] No hardcoded rules introduced
- [ ] Maintains AI-Native design
- [ ] Privacy preserved
- [ ] Layered architecture respected

### Verification
- [ ] Fix tested
- [ ] No regressions
- [ ] Test case added

### Related Skills Referenced
- {{skill_1}}
- {{skill_2}}
```

---

## Recommended Combinations

Use this skill together with:
- **EstimateResourceUsage**: For memory issues
- **TestingAndDebugging**: For test verification
- **OllamaPromptEngineering**: For SLM-related issues
- **All skills**: For context-aware debugging
