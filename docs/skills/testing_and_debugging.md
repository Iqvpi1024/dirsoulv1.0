# Skill: Testing and Debugging

> **Purpose**: Guide self-check and testing processes, ensuring task completion standards (80%+ coverage), preventing deviation from HEAD.md, and maintaining code quality.

---

## Testing Strategy Overview

### Coverage Requirements

| Test Type | Minimum Coverage | Priority |
|-----------|------------------|----------|
| Unit Tests | 80%+ | Critical |
| Integration Tests | 70%+ | High |
| E2E Tests | Key user flows | High |
| Performance Tests | 10-year data simulation | Medium |

---

## Unit Testing Patterns

### Rust Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Test event extraction
    #[test]
    fn test_extract_simple_event() {
        let input = "今天吃了3个苹果";
        let events = extract_events(input).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "吃");
        assert_eq!(events[0].target, "苹果");
        assert_eq!(events[0].quantity, Some(3.0));
        assert!(events[0].confidence > 0.8);
    }

    /// Test fuzzy time parsing
    #[test]
    fn test_parse_yesterday() {
        let fuzzy = FuzzyTime::Yesterday;
        let timestamp = fuzzy.to_timestamp();

        let expected = Utc::now() - Duration::days(1);
        let diff = (timestamp - expected).abs();

        assert!(diff.num_hours() < 24);
    }

    /// Test encryption/decryption roundtrip
    #[test]
    fn test_encryption_roundtrip() {
        let encryption = EncryptionManager::generate_for_test().unwrap();
        let original = "sensitive data";

        let encrypted = encryption.encrypt_string(original).unwrap();
        let decrypted = encryption.decrypt_string(&encrypted).unwrap();

        assert_eq!(original, decrypted);
    }

    /// Test promotion gate logic
    #[test]
    fn test_promotion_gate_rejects_low_confidence() {
        let view = DerivedView {
            confidence: 0.7,  // Below 0.85 threshold
            validation_count: 10,
            created_at: Utc::now() - Duration::days(35),
            counter_evidence: vec![],
            ..Default::default()
        };

        assert!(!should_promote(&view));
    }

    /// Test promotion gate accepts qualified view
    #[test]
    fn test_promotion_gate_accepts_qualified() {
        let view = DerivedView {
            confidence: 0.9,
            validation_count: 5,
            created_at: Utc::now() - Duration::days(35),
            counter_evidence: vec![],
            ..Default::default()
        };

        assert!(should_promote(&view));
    }
}
```

### Property-Based Testing

```rust
use proptest::prelude::*;

/// Property: Event extraction never panics on valid UTF-8
proptest! {
    #[test]
    fn fn_extract_doesnt_panic(input in "[\\u{0}-\\u{7FF}]*") {
        let _ = extract_events(&input);
    }
}

/// Property: Encryption roundtrip preserves data
proptest! {
    #[test]
    fn fn encryption_roundtrip(data in ".{0,1000}") {
        let encryption = EncryptionManager::generate_for_test().unwrap();

        let encrypted = encryption.encrypt_string(&data).unwrap();
        let decrypted = encryption.decrypt_string(&encrypted).unwrap();

        prop_assert_eq!(data, decrypted);
    }
}

/// Property: Confidence always in [0, 1]
proptest! {
    #[test]
    fn fn_confidence_in_range(input in "[\\u{0}-\\u{7FF}]*") {
        if let Ok(events) = extract_events(&input) {
            for event in events {
                prop_assert!(event.confidence >= 0.0 && event.confidence <= 1.0);
            }
        }
    }
}
```

---

## Integration Testing

### Database Integration

```rust
#[tokio::test]
async fn test_store_and_retrieve_memory() {
    // Setup test database
    let pool = setup_test_db().await;

    // Create memory
    let memory = RawMemory::new_encrypted(
        "test_user".to_string(),
        "text".to_string(),
        "test content",
        &encryption()
    ).unwrap();

    // Store
    let stored = store_memory(&pool, memory).await.unwrap();

    // Retrieve
    let retrieved = get_memory(&pool, stored.id).await.unwrap();

    // Verify
    assert_eq!(stored.id, retrieved.id);
    assert_eq!(stored.user_id, retrieved.user_id);

    let content = retrieved.get_content(&encryption()).unwrap();
    assert_eq!(content, "test content");
}

#[tokio::test]
async fn test_event_extraction_pipeline() {
    let pool = setup_test_db().await;
    let extractor = EventExtractor::new(ollama());

    let input = "昨天去健身房练了2小时";

    // Full pipeline
    let events = extractor.extract(&input).await.unwrap();
    assert!(!events.is_empty());

    // Store events
    for event in events {
        store_event(&pool, event).await.unwrap();
    }

    // Verify retrieval
    let stored = get_user_events(&pool, "test_user").await.unwrap();
    assert!(!stored.is_empty());
}
```

### Time Jumping Tests

```rust
/// Test promotion logic without waiting 30 days
#[tokio::test]
async fn test_promotion_after_time_jump() {
    let mut time_sim = TimeSimulator::new();
    let store = time_sim.view_store();

    // Create view
    let view = DerivedView {
        confidence: 0.9,
        validation_count: 5,
        created_at: time_sim.now(),
        ..Default::default()
    };
    store.save_view(&view).await.unwrap();

    // Initially not promoted
    assert!(!should_promote(&store.get_view(view.view_id).await.unwrap()));

    // Jump 35 days forward
    time_sim.jump(Duration::days(35));

    // Now should promote
    assert!(should_promote(&store.get_view(view.view_id).await.unwrap()));
}
```

---

## Self-Check Checklist

### Pre-Commit Validation

```rust
/// Run all checks before considering task complete
pub async fn pre_commit_checks() -> Result<()> {
    println!("Running pre-commit checks...\n");

    // 1. Read HEAD.md
    check_head_md_consistency().await?;

    // 2. Run all tests
    check_tests_pass().await?;

    // 3. Check coverage
    check_coverage_threshold().await?;

    // 4. Verify no forbidden patterns
    check_forbidden_patterns().await?;

    // 5. Clippy lints
    check_clippy().await?;

    // 6. Format check
    check_formatting().await?;

    println!("\n✅ All checks passed!");
    Ok(())
}

async fn check_head_md_consistency() -> Result<()> {
    println!("Checking HEAD.md consistency...");

    let head = tokio::fs::read_to_string("todo/head.md").await?;

    // Verify core principles understood
    assert!(head.contains("AI-Native"), "HEAD.md missing AI-Native principle");
    assert!(head.contains("隐私优先"), "HEAD.md missing privacy principle");

    // Check against forbidden patterns
    let source_files = get_rust_sources().await?;
    for file in source_files {
        let content = tokio::fs::read_to_string(&file).await?;

        // No hardcoded rules
        if content.contains("\"吃了几个\"") || content.contains("\"多少\"") {
            return Err(Error::ForbiddenPattern("Hardcoded rule detected".to_string()));
        }

        // No direct schema modifications from LLM
        if content.contains("ALTER TABLE") && content.contains("llm_generated") {
            return Err(Error::ForbiddenPattern("LLM direct schema modification".to_string()));
        }
    }

    println!("  ✅ HEAD.md consistency verified");
    Ok(())
}

async fn check_tests_pass() -> Result<()> {
    println!("Running tests...");

    let output = tokio::process::Command::new("cargo")
        .args(&["test", "--all"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::TestsFailed(stderr.to_string()));
    }

    println!("  ✅ All tests pass");
    Ok(())
}

async fn check_coverage_threshold() -> Result<()> {
    println!("Checking coverage...");

    // Use tarpaulin or similar
    let output = tokio::process::Command::new("cargo")
        .args(&["tarpaulin", "--out", "Json"])
        .output()
        .await?;

    // Parse coverage JSON
    let coverage: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let total_coverage = coverage["percent"]
        .as_f64()
        .ok_or(Error::ParseError)?;

    if total_coverage < 80.0 {
        return Err(Error::LowCoverage(total_coverage));
    }

    println!("  ✅ Coverage: {:.1}%", total_coverage);
    Ok(())
}

async fn check_forbidden_patterns() -> Result<()> {
    println!("Checking for forbidden patterns...");

    let forbidden = vec![
        ("unwrap()", "Prefer proper error handling"),
        ("expect(", "Prefer proper error handling"),
        ("panic!", "Use Result instead"),
        ("TODO", "Complete implementation"),
    ];

    let source_files = get_rust_sources().await?;
    for file in source_files {
        let content = tokio::fs::read_to_string(&file).await?;

        for (pattern, suggestion) in &forbidden {
            if content.contains(pattern) {
                println!("  ⚠️  {} contains {} - {}", file.display(), pattern, suggestion);
            }
        }
    }

    println!("  ✅ Forbidden patterns check complete");
    Ok(())
}

async fn check_clippy() -> Result<()> {
    println!("Running Clippy...");

    let output = tokio::process::Command::new("cargo")
        .args(&["clippy", "--", "-D", "warnings"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::ClippyFailed(stderr.to_string()));
    }

    println!("  ✅ No Clippy warnings");
    Ok(())
}

async fn check_formatting() -> Result<()> {
    println!("Checking formatting...");

    let output = tokio::process::Command::new("cargo")
        .args(&["fmt", "--", "--check"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(Error::FormattingFailed);
    }

    println!("  ✅ Code formatted correctly");
    Ok(())
}
```

---

## Debugging Techniques

### Logging Levels

```rust
use tracing::{info, warn, error, debug};

/// Structured logging with context
#[tracing::instrument(skip(self))]
pub async fn process_input(&self, input: &str) -> Result<Vec<Event>> {
    debug!(input = %input, "Processing input");

    let events = self.extract_events(input).await?;

    info!(count = events.len(), "Extracted {} events", events.len());

    if events.is_empty() {
        debug!("No events found in input");
    }

    Ok(events)
}

/// Error with context
pub fn extract_events(input: &str) -> Result<Vec<Event>> {
    let events = parse_events(input)?;

    if events.len() > 10 {
        warn!(
            input_len = input.len(),
            event_count = events.len(),
            "Unusually high event count"
        );
    }

    Ok(events)
}
```

### Error Retry with Exponential Backoff

```rust
/// Retry operation with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    max_retries: usize,
    initial_delay: Duration
) -> Result<T, E>
where
    F: Fn() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut delay = initial_delay;
    let mut attempt = 0;

    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    error!(
                        attempts = attempt,
                        error = %e,
                        "Operation failed after {} attempts",
                        max_retries
                    );
                    return Err(e);
                }

                warn!(
                    attempt = attempt,
                    error = %e,
                    "Operation failed, retrying in {:?}",
                    delay
                );

                tokio::time::sleep(delay).await;
                delay *= 2;  // Exponential backoff
            }
        }
    }
}

/// Usage
async fn call_ollama_with_retry(prompt: &str) -> Result<String> {
    retry_with_backoff(
        || ollama_generate(prompt),
        3,
        Duration::from_millis(100)
    ).await
}
```

---

## Edge Case Testing

### Time Edge Cases

```rust
#[test]
fn test_midnight_transition() {
    let time = NaiveTime::from_hms(23, 59, 59);
    let next = time + Duration::seconds(2);

    assert_eq!(next.hour(), 0);
    assert_eq!(next.minute(), 0);
    assert_eq!(next.second(), 1);
}

#[test]
fn test_month_boundary() {
    let date = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
    let next = date.succ_opt().unwrap();

    assert_eq!(next.month(), 2);
    assert_eq!(next.day(), 1);
}

#[test]
fn test_leap_year() {
    let date = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();
    let next = date.succ_opt().unwrap();

    assert_eq!(next.year(), 2024);
    assert_eq!(next.month(), 2);
    assert_eq!(next.day(), 29);  // Leap day
}
```

### Multi-Event Parsing

```rust
#[test]
fn test_multiple_events_in_sentence() {
    let input = "早上喝咖啡，中午吃三明治，晚上喝啤酒";
    let events = extract_events(input).unwrap();

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].target, "咖啡");
    assert_eq!(events[1].target, "三明治");
    assert_eq!(events[2].target, "啤酒");
}

#[test]
fn test_no_event_in_opinion() {
    let input = "我觉得咖啡比茶好喝";
    let events = extract_events(input).unwrap();

    assert_eq!(events.len(), 0);  // Opinion, not event
}
```

---

## Completion Standards

### Task Completion Checklist

```rust
/// Verify task is truly complete
pub async fn verify_task_complete(task_id: &str) -> Result<bool> {
    // 1. Read HEAD.md - understood requirements
    check_head_md_read().await?;

    // 2. No forbidden violations
    check_no_violations().await?;

    // 3. All tests pass
    check_tests_pass().await?;

    // 4. Coverage >= 80%
    check_coverage().await?;

    // 5. Documentation updated
    check_documentation().await?;

    // 6. TODO.md updated
    check_todo_updated(task_id).await?;

    Ok(true)
}
```

---

## Recommended Combinations

Use this skill together with:
- **All other skills**: As a post-development validation step
- **RustMemorySafety**: For memory leak detection tests
- **EncryptionBestPractices**: For encryption verification tests
- **PostgresSchemaDesign**: For schema migration testing
