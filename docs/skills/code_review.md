# Skill: Code Review

> **Purpose**: Automated code review that checks style, consistency, security, and scalability. Aligned with HEAD.md principles and DirSoul architecture.

---

## Review Framework

### Review Categories

```yaml
review_categories:
  architecture:
    - "分层架构遵守"
    - "模块化原则"
    - "AI-Native设计"

  rust_specific:
    - "内存安全"
    - "所有权规则"
    - "错误处理"

  database:
    - "Schema设计"
    - "查询优化"
    - "分区策略"

  ai_integration:
    - "SLM使用正确"
    - "Prompt质量"
    - "幻觉防护"

  security:
    - "加密实现"
    - "权限控制"
    - "隐私保护"

  compliance:
    - "HEAD.md遵守"
    - "禁止事项检查"
    - "必须行为验证"
```

---

## Automated Checks

### HEAD.md Compliance (Critical)

```rust
pub struct HeadComplianceChecker;

impl HeadComplianceChecker {
    /// Verify code doesn't violate HEAD.md forbidden items
    pub fn check_forbidden_patterns(&self, code: &str) -> Vec<ComplianceIssue> {
        let mut issues = Vec::new();

        // Forbidden: Hardcoded rules
        if contains_hardcoded_rules(code) {
            issues.push(ComplianceIssue {
                severity: Severity::Critical,
                category: "AI-Native Violation",
                message: "检测到硬编码规则。事件提取应由SLM完成。",
                location: find_pattern_location(code, r"(match|if).*["']吃了|几个["']"),
                suggestion: "使用 EventExtractionPatterns skill，通过SLM提取事件",
            });
        }

        // Forbidden: LLM directly modifies schema
        if contains_llm_schema_mod(code) {
            issues.push(ComplianceIssue {
                severity: Severity::Critical,
                category: "Architecture Violation",
                message: "LLM不应直接修改Schema",
                location: find_pattern_location(code, r"ALTER.*llm|ai.*generated"),
                suggestion: "所有Schema变更必须通过migration文件",
            });
        }

        // Forbidden: Skipping Derived Views
        if contains_view_skip(code) {
            issues.push(ComplianceIssue {
                severity: Severity::Critical,
                category: "Slow Abstraction Violation",
                message: "跳过了Derived Views直接创建稳定概念",
                location: find_pattern_location(code, r"stable.*concept.*direct|bypass.*derived"),
                suggestion: "使用 CognitiveViewGeneration skill，所有认知假设需经过Promotion Gate",
            });
        }

        // Required: Typed actions
        if !contains_typed_actions(code) {
            issues.push(ComplianceIssue {
                severity: Severity::Warning,
                category: "Type Safety",
                message: "行为未使用ActionType类型",
                location: find_pattern_location(code, r"action:\s*String"),
                suggestion: "使用 ActionType enum 而非 String",
            });
        }

        // Required: Timestamps on events
        if !contains_timestamp(code) {
            issues.push(ComplianceIssue {
                severity: Severity::Error,
                category: "Data Integrity",
                message: "事件缺少时间戳",
                location: find_pattern_location(code, r"INSERT.*event(?!.*timestamp)"),
                suggestion: "每个事件必须有精确的TIMESTAMPTZ",
            });
        }

        issues
    }
}
```

### Rust-Specific Checks

```rust
pub struct RustCodeReview;

impl RustCodeReview {
    pub fn review_rust_code(&self, code: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Memory safety checks
        items.extend(self.check_memory_safety(code));

        // Ownership and borrowing
        items.extend(self.check_ownership(code));

        // Error handling
        items.extend(self.check_error_handling(code));

        items
    }

    fn check_memory_safety(&self, code: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Check for unbounded Vec growth
        if code.contains("Vec::new()") && contains_loop_with_push(code) {
            items.push(ReviewItem {
                severity: Severity::Warning,
                message: "Vec可能在循环中无限制增长",
                suggestion: "使用 Vec::with_capacity() 或流式处理",
                reference: "RustMemorySafety skill",
            });
        }

        // Check for unnecessary cloning
        let clone_count = code.matches(".clone()").count();
        if clone_count > 5 {
            items.push(ReviewItem {
                severity: Severity::Info,
                message: format!("检测到 {} 次 .clone() 调用", clone_count),
                suggestion: "考虑使用引用 (&) 而非克隆",
                reference: "RustMemorySafety skill",
            });
        }

        items
    }

    fn check_ownership(&self, code: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Check for Rc in async context (should use Arc)
        if code.contains("Rc<") && code.contains("async") {
            items.push(ReviewItem {
                severity: Severity::Error,
                message: "异步上下文中使用了Rc而非Arc",
                suggestion: "在 async 代码中使用 Arc 以实现 Send + Sync",
                reference: "RustMemorySafety skill",
            });
        }

        items
    }

    fn check_error_handling(&self, code: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Check for unwrap() in production code
        if code.contains(".unwrap()") && !code.contains("#[test]") {
            items.push(ReviewItem {
                severity: Severity::Warning,
                message: "生产代码中使用了 .unwrap()",
                suggestion: "使用 ? 或 .expect() 并提供有意义的错误信息",
                reference: "TestingAndDebugging skill",
            });
        }

        items
    }
}
```

---

## Database Code Review

### SQL and Schema Checks

```rust
pub struct DatabaseReview;

impl DatabaseReview {
    pub fn review_sql(&self, sql: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Check for proper indexing
        if sql.contains("CREATE TABLE") && !sql.contains("CREATE INDEX") {
            items.push(ReviewItem {
                severity: Severity::Info,
                message: "新建表但未定义索引",
                suggestion: "根据查询模式添加适当的索引",
                reference: "PostgresSchemaDesign skill",
            });
        }

        // Check for partitioning on large tables
        if sql.contains("raw_memories") || sql.contains("event_memories") {
            if !sql.contains("PARTITION") && !sql.contains("partition") {
                items.push(ReviewItem {
                    severity: Severity::Warning,
                    message: "大表未配置分区策略",
                    suggestion: "使用按月分区支持10年+数据增长",
                    reference: "PostgresSchemaDesign skill",
                });
            }
        }

        // Check for JSONB usage
        if sql.contains("TEXT") && should_use_jsonb(sql) {
            items.push(ReviewItem {
                severity: Severity::Info,
                message: "应使用 JSONB 而非 TEXT",
                suggestion: "JSONB 提供更好的查询性能和灵活性",
                reference: "PostgresSchemaDesign skill",
            });
        }

        items
    }
}
```

---

## AI Integration Review

### SLM and Prompt Checks

```rust
pub struct AIIntegrationReview;

impl AIIntegrationReview {
    pub fn review_ai_code(&self, code: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Check for hardcoded rules instead of SLM
        if contains_rule_based_extraction(code) {
            items.push(ReviewItem {
                severity: Severity::Critical,
                message: "使用规则引擎而非SLM进行事件提取",
                suggestion: "规则引擎仅作为SLM失败时的fallback",
                reference: "EventExtractionPatterns skill",
            });
        }

        // Check prompt quality
        if let Some(prompt) = extract_prompt(code) {
            items.extend(self.review_prompt(&prompt));
        }

        // Check for hallucination mitigation
        if !contains_confidence_validation(code) {
            items.push(ReviewItem {
                severity: Severity::Warning,
                message: "缺少置信度验证",
                suggestion: "所有SLM输出应包含confidence并验证",
                reference: "OllamaPromptEngineering skill",
            });
        }

        items
    }

    fn review_prompt(&self, prompt: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Check for JSON output format
        if !prompt.contains("JSON") && !prompt.contains("json") {
            items.push(ReviewItem {
                severity: Severity::Warning,
                message: "Prompt未要求JSON输出格式",
                suggestion: "要求结构化JSON输出便于解析",
                reference: "OllamaPromptEngineering skill",
            });
        }

        // Check for few-shot examples
        if !prompt.contains("Example") && !prompt.contains("示例") {
            items.push(ReviewItem {
                severity: Severity::Info,
                message: "Prompt缺少few-shot示例",
                suggestion: "添加3-5个示例提高提取质量",
                reference: "OllamaPromptEngineering skill",
            });
        }

        // Estimate token count
        let tokens = estimate_tokens(prompt);
        if tokens > 2000 {
            items.push(ReviewItem {
                severity: Severity::Info,
                message: format!("Prompt过长 (约{} tokens)", tokens),
                suggestion: "压缩上下文，控制在1000-2000 tokens",
                reference: "EstimateResourceUsage skill",
            });
        }

        items
    }
}
```

---

## Security Review

### Encryption and Permission Checks

```rust
pub struct SecurityReview;

impl SecurityReview {
    pub fn review_security(&self, code: &str) -> Vec<ReviewItem> {
        let mut items = Vec::new();

        // Check for hardcoded keys
        if code.contains("const ENCRYPTION_KEY") || code.contains("\"hardcoded_key\"") {
            items.push(ReviewItem {
                severity: Severity::Critical,
                message: "检测到硬编码加密密钥",
                suggestion: "使用 EncryptionBestPractices skill，密钥应从文件加载",
                reference: "EncryptionBestPractices skill",
            });
        }

        // Check for permission enforcement in plugins
        if code.contains("impl UserPlugin") && !code.contains("permission") {
            items.push(ReviewItem {
                severity: Severity::Critical,
                message: "插件缺少权限控制",
                suggestion: "使用 PluginPermissionSystem skill",
                reference: "PluginPermissionSystem skill",
            });
        }

        // Check for sensitive data logging
        if code.contains("debug!") || code.contains("println!") {
            if contains_sensitive_keywords(code) {
                items.push(ReviewItem {
                    severity: Severity::Warning,
                    message: "可能记录敏感数据到日志",
                    suggestion: "避免记录加密内容或密钥",
                    reference: "EncryptionBestPractices skill",
                });
            }
        }

        items
    }
}
```

---

## Review Output Format

### Structured Report

```rust
#[derive(Debug)]
pub struct CodeReviewReport {
    pub file_path: PathBuf,
    pub overall_score: f32,  // 0.0 - 1.0
    pub items: Vec<ReviewItem>,
    pub summary: ReviewSummary,
}

#[derive(Debug)]
pub struct ReviewItem {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub location: Option<CodeLocation>,
    pub suggestion: String,
    pub reference: &'static str,  // Which skill to reference
}

#[derive(Debug)]
pub struct ReviewSummary {
    pub critical_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub head_compliance: bool,
}

impl std::fmt::Display for CodeReviewReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Code Review Report: {}", self.file_path.display())?;
        writeln!(f, "Overall Score: {:.1}%", self.overall_score * 100.0)?;
        writeln!(f)?;

        writeln!(f, "Summary:")?;
        writeln!(f, "  Critical: {}", self.summary.critical_count)?;
        writeln!(f, "  Errors: {}", self.summary.error_count)?;
        writeln!(f, "  Warnings: {}", self.summary.warning_count)?;
        writeln!(f, "  Info: {}", self.summary.info_count)?;
        writeln!(f, "  HEAD.md Compliant: {}", if self.summary.head_compliance { "✅" } else { "❌" })?;
        writeln!(f)?;

        for item in &self.items {
            let severity = match item.severity {
                Severity::Critical => "🔴 CRITICAL",
                Severity::Error => "❌ ERROR",
                Severity::Warning => "⚠️  WARNING",
                Severity::Info => "ℹ️  INFO",
            };

            writeln!(f, "{}: {}", severity, item.message)?;
            if let Some(loc) = &item.location {
                writeln!(f, "  Location: {}:{}", loc.line, loc.column)?;
            }
            writeln!(f, "  Suggestion: {}", item.suggestion)?;
            writeln!(f, "  Reference: {}", item.reference)?;
            writeln!(f)?;
        }

        Ok(())
    }
}
```

---

## Pre-Commit Integration

```rust
/// Run code review before commit
pub async fn pre_commit_review() -> Result<()> {
    let modified_files = get_staged_files().await?;

    for file in modified_files {
        let content = tokio::fs::read_to_string(&file).await?;

        // Run all checks
        let mut report = CodeReviewReport::new(&file);

        report.items.extend(HeadComplianceChecker.check_forbidden_patterns(&content));
        report.items.extend(RustCodeReview.review_rust_code(&content));
        report.items.extend(DatabaseReview.review_sql(&content));
        report.items.extend(AIIntegrationReview.review_ai_code(&content));
        report.items.extend(SecurityReview.review_security(&content));

        // Calculate score and summary
        report.finalize();

        // Display report
        println!("{}", report);

        // Fail commit if critical issues
        if report.summary.critical_count > 0 {
            return Err(Error::CriticalIssues);
        }

        // Fail if HEAD.md not compliant
        if !report.summary.head_compliance {
            return Err(Error::HeadViolation);
        }
    }

    Ok(())
}
```

---

## HEAD.md as Supreme Authority

```yaml
# All reviews subordinate to HEAD.md
review_hierarchy:
  1: "HEAD.md (宪法，最高优先级)"
  2: "Project-specific skills (DirSoul)"
  3: "General best practices (Rust, SQL, etc.)"

# When in conflict:
conflict_resolution:
  "HEAD.md overrides all other guidelines"
  "If a general best practice contradicts HEAD.md, follow HEAD.md"
  "Document any deviations with reasoning"
```

---

## Recommended Combinations

Use this skill together with:
- **CheckArchitectureCompliance**: For HEAD.md compliance verification
- **TestingAndDebugging**: For pre-commit validation
- **All other skills**: As reference for suggestions
