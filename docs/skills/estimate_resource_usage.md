# Skill: Estimate Resource Usage

> **Purpose**: Analyze code for memory allocation patterns and predict token/memory consumption for 8GB environment constraints.

---

## Memory Allocation Analysis

### Rust Memory Pattern Detection

```rust
use syn::{Item, ItemFn, Expr, Type};
use std::collections::HashMap;

pub struct MemoryAnalyzer {
    allocations: Vec<Allocation>,
    total_heap: usize,
    stack_size: usize,
}

#[derive(Debug)]
pub struct Allocation {
    location: String,
    allocation_type: AllocType,
    estimated_size: MemorySize,
    risk_level: RiskLevel,
}

#[derive(Debug, Clone)]
pub enum AllocType {
    Vec,
    HashMap,
    Box,
    String,
    Arc,
    Unknown,
}

#[derive(Debug)]
pub enum MemorySize {
    Known(usize),
    Estimated(usize),
    Unbounded,
}

#[derive(Debug, PartialEq)]
pub enum RiskLevel {
    Safe,
    Moderate,
    High,
    Critical,  // May cause OOM in 8GB
}

impl MemoryAnalyzer {
    /// Analyze Rust code for memory patterns
    pub fn analyze_rust_code(&mut self, code: &str) -> Result<MemoryReport> {
        let ast = syn::parse_file(code)?;

        // Find all Vec allocations
        self.find_vec_allocations(&ast)?;

        // Find HashMap allocations
        self.find_hashmap_allocations(&ast)?;

        // Check for large stack allocations
        self.check_stack_usage(&ast)?;

        Ok(MemoryReport {
            total_heap_estimated: self.total_heap,
            stack_size: self.stack_size,
            allocations: self.allocations.clone(),
            recommendations: self.generate_recommendations(),
        })
    }

    fn find_vec_allocations(&mut self, ast: &syn::File) -> Result<()> {
        // Pattern matching for Vec creation
        // Vec::new(), Vec::with_capacity(), vec![]
        // Identify potential unbounded growth

        // Example detection:
        // "let mut all = Vec::new();" in a loop
        // "Vec<u8>" reading entire files

        Ok(())
    }

    fn find_hashmap_allocations(&mut self, ast: &syn::File) -> Result<()> {
        // HashMap allocations
        // Check for capacity planning
        Ok(())
    }

    fn check_stack_usage(&mut self, ast: &syn::File) -> Result<()> {
        // Large arrays on stack
        // Recursive function depth
        Ok(())
    }

    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        for alloc in &self.allocations {
            match alloc.risk_level {
                RiskLevel::Critical => {
                    recommendations.push(format!(
                        "CRITICAL: {:?} at {} may cause OOM. Consider streaming or chunking.",
                        alloc.allocation_type, alloc.location
                    ));
                }
                RiskLevel::High => {
                    recommendations.push(format!(
                        "HIGH: {:?} at {} should have capacity pre-allocated.",
                        alloc.allocation_type, alloc.location
                    ));
                }
                _ => {}
            }
        }

        recommendations
    }
}

#[derive(Debug)]
pub struct MemoryReport {
    pub total_heap_estimated: usize,
    pub stack_size: usize,
    pub allocations: Vec<Allocation>,
    pub recommendations: Vec<String>,
}

impl MemoryReport {
    /// Check if memory usage fits in 8GB environment
    pub fn fits_8gb(&self) -> bool {
        const GB: usize = 1024 * 1024 * 1024;
        const AVAILABLE: usize = 8 * GB;
        const SYSTEM_RESERVED: usize = 1 * GB;  // OS + overhead
        const OLLAMA_RESERVED: usize = 5 * GB;  // Phi-4-mini
        const POSTGRES_RESERVED: usize = 1 * GB;  // Database

        let available_for_app = AVAILABLE - SYSTEM_RESERVED - OLLAMA_RESERVED - POSTGRES_RESERVED;

        self.total_heap_estimated + self.stack_size < available_for_app
    }

    pub fn display(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Memory Usage Report:\n"));
        output.push_str(&format!("  Heap: {:.2} MB\n", self.total_heap_estimated as f64 / 1024.0 / 1024.0));
        output.push_str(&format!("  Stack: {:.2} MB\n", self.stack_size as f64 / 1024.0 / 1024.0));
        output.push_str(&format!("  Fits in 8GB: {}\n\n", self.fits_8gb()));

        if !self.recommendations.is_empty() {
            output.push_str("Recommendations:\n");
            for rec in &self.recommendations {
                output.push_str(&format!("  - {}\n", rec));
            }
        }

        output
    }
}
```

---

## Common Memory Anti-Patterns

### Detection and Fixes

```rust
/// Detect and report memory anti-patterns
pub struct AntiPatternDetector;

impl AntiPatternDetector {
    /// Check for unbounded Vec in loops
    pub fn detect_unbounded_vec(code: &str) -> Vec<Warning> {
        let mut warnings = Vec::new();

        // Pattern: Vec::new() + loop + push without capacity
        if code.contains("Vec::new()") && code.contains(".push(") {
            warnings.push(Warning {
                pattern: "Unbounded Vec growth",
                severity: Severity::High,
                suggestion: "Use Vec::with_capacity() or process in chunks",
            });
        }

        // Pattern: collect() on large iterator
        if code.contains(".collect::<Vec<_>>()") || code.contains(".collect()") {
            warnings.push(Warning {
                pattern: "Full collect of iterator",
                severity: Severity::Moderate,
                suggestion: "Consider streaming or batch processing",
            });
        }

        warnings
    }

    /// Check for unnecessary cloning
    pub fn detect_unnecessary_clone(code: &str) -> Vec<Warning> {
        let mut warnings = Vec::new();

        // Pattern: .clone() when borrow would work
        if code.matches(".clone()").count() > 5 {
            warnings.push(Warning {
                pattern: "Excessive cloning",
                severity: Severity::Moderate,
                suggestion: "Use references (&) instead of cloning",
            });
        }

        warnings
    }
}

#[derive(Debug)]
pub struct Warning {
    pub pattern: &'static str,
    pub severity: Severity,
    pub suggestion: &'static str,
}
```

---

## Token Consumption Estimation

### Phi-4-mini Prompt Analysis

```rust
pub struct TokenEstimator {
    chinese_chars_per_token: f32,
    english_chars_per_token: f32,
}

impl TokenEstimator {
    pub fn new() -> Self {
        Self {
            chinese_chars_per_token: 0.7,  // ~1.4 chars per token
            english_chars_per_token: 4.0,  // ~0.25 chars per token
        }
    }

    /// Estimate token count for text
    pub fn estimate_tokens(&self, text: &str) -> usize {
        let chinese_count = text.chars().filter(|c| is_chinese(*c)).count();
        let other_count = text.len() - chinese_count;

        let chinese_tokens = (chinese_count as f32 / self.chinese_chars_per_token).ceil() as usize;
        let other_tokens = (other_count as f32 / self.english_chars_per_token).ceil() as usize;

        chinese_tokens + other_tokens
    }

    /// Estimate peak memory for inference
    pub fn estimate_inference_memory(&self, prompt_tokens: usize) -> usize {
        // Phi-4-mini Q4: ~4-5GB base
        // Add ~0.5GB per 1K tokens for context
        const BASE_MEMORY: usize = 5 * 1024 * 1024 * 1024;  // 5GB
        const TOKENS_PER_GB: usize = 2000;

        let context_memory = (prompt_tokens / TOKENS_PER_GB) * 1024 * 1024 * 1024;

        BASE_MEMORY + context_memory
    }

    /// Check if prompt fits in memory
    pub fn fits_memory(&self, prompt: &str, available_memory: usize) -> bool {
        let tokens = self.estimate_tokens(prompt);
        let required = self.estimate_inference_memory(tokens);

        required < available_memory
    }
}

fn is_chinese(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let estimator = TokenEstimator::new();

        // Pure Chinese
        let chinese = "我今天早上吃了三个苹果";
        assert!(estimator.estimate_tokens(chinese) < 20);

        // Pure English
        let english = "I ate three apples this morning";
        assert!(estimator.estimate_tokens(english) < 15);

        // Mixed
        let mixed = "我今天 ate 3 apples";
        assert!(estimator.estimate_tokens(mixed) < 20);
    }

    #[test]
    fn test_memory_estimation() {
        let estimator = TokenEstimator::new();

        // 1K tokens
        let memory = estimator.estimate_inference_memory(1000);
        assert!(memory < 6 * 1024 * 1024 * 1024);  // < 6GB
    }
}
```

---

## Batch Processing Optimization

### Memory-Aware Batch Size

```rust
pub struct BatchOptimizer {
    target_memory_per_batch: usize,
    max_tokens_per_batch: usize,
}

impl BatchOptimizer {
    /// Calculate optimal batch size for memory constraints
    pub fn calculate_batch_size(
        &self,
        item_size_bytes: usize,
        available_memory: usize
    ) -> usize {
        // Reserve 20% overhead
        let usable_memory = (available_memory as f64 * 0.8) as usize;

        // Calculate based on item size
        let size_based = usable_memory / item_size_bytes;

        // Cap at reasonable maximum
        size_based.min(self.max_tokens_per_batch).min(1000)
    }

    /// Split work into memory-safe batches
    pub fn create_batches<T>(&self, items: Vec<T>) -> Vec<Vec<T>> {
        let size_hint = std::mem::size_of::<T>();
        let batch_size = self.calculate_batch_size(size_hint, 512 * 1024 * 1024);  // 512MB

        items.chunks(batch_size)
            .map(|c| c.to_vec())
            .collect()
    }
}

/// Usage example for event extraction batching
pub async fn extract_events_batch(
    inputs: Vec<String>
) -> Result<Vec<ExtractedEvent>> {
    let optimizer = BatchOptimizer {
        target_memory_per_batch: 100 * 1024 * 1024,  // 100MB
        max_tokens_per_batch: 4000,  // Phi-4-mini context
    };

    let batches = optimizer.create_batches(inputs);
    let mut all_results = Vec::new();

    for batch in batches {
        let batch_prompt = build_batch_prompt(&batch);
        let tokens = estimate_tokens(&batch_prompt);

        // Check memory before inference
        if !fits_in_memory(&batch_prompt, 6 * 1024 * 1024 * 1024) {
            warn!("Batch exceeds memory limit, reducing size");
            continue;
        }

        let results = ollama_generate(&batch_prompt).await?;
        all_results.extend(results);
    }

    Ok(all_results)
}
```

---

## Connection Pool Analysis

```rust
/// Check database connection pool configuration for 8GB
pub struct ConnectionPoolAnalyzer {
    max_connections: usize,
    estimated_connection_memory: usize,
}

impl ConnectionPoolAnalyzer {
    /// Analyze if pool configuration is memory-safe
    pub fn analyze_pool(&self) -> PoolReport {
        let total_memory = self.max_connections * self.estimated_connection_memory;

        // Each connection: ~2-5MB depending on prepared statements
        const SAFE_LIMIT: usize = 50 * 5 * 1024 * 1024;  // 50 connections * 5MB

        PoolReport {
            total_memory_mb: total_memory / 1024 / 1024,
            is_safe: total_memory < SAFE_LIMIT,
            recommended_max: SAFE_LIMIT / self.estimated_connection_memory,
        }
    }
}

#[derive(Debug)]
pub struct PoolReport {
    pub total_memory_mb: usize,
    pub is_safe: bool,
    pub recommended_max: usize,
}
```

---

## Pre-Commit Memory Check

```rust
/// Run before commit to ensure memory safety
pub async fn memory_pre_commit_check() -> Result<()> {
    let mut analyzer = MemoryAnalyzer::default();

    // Get modified Rust files
    let rust_files = get_modified_rust_files().await?;

    for file in rust_files {
        let code = tokio::fs::read_to_string(&file).await?;
        let report = analyzer.analyze_rust_code(&code)?;

        if !report.fits_8gb() {
            error!("Memory safety check failed for {}:\n{}", file.display(), report.display());
            return Err(Error::MemoryLimitExceeded);
        }
    }

    // Check Python files for heavy dependencies
    let python_files = get_modified_python_files().await?;
    check_python_memory(&python_files).await?;

    println!("✅ Memory safety check passed");
    Ok(())
}

async fn check_python_memory(files: &[PathBuf]) -> Result<()> {
    // Check for memory-heavy patterns
    for file in files {
        let content = tokio::fs::read_to_string(file).await?;

        // Warn about loading entire datasets
        if content.contains("pd.read_csv") && !content.contains("chunksize") {
            warn!("{} may load entire CSV into memory", file.display());
        }

        // Check for tensor operations without size limits
        if content.contains("torch") && !content.contains(".to('cpu')") {
            warn!("{} may keep tensors on GPU", file.display());
        }
    }

    Ok(())
}
```

---

## Recommended Combinations

Use this skill together with:
- **RustMemorySafety**: For detailed memory pattern analysis
- **OllamaPromptEngineering**: For token optimization
- **TestingAndDebugging**: For pre-commit validation
- **PostgresSchemaDesign**: For connection pool analysis
