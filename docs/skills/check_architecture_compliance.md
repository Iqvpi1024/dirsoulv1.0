# Skill: Check Architecture Compliance

> **Purpose**: Serve as the "architecture auditor" that prevents AI from introducing hardcoded rules, violating AI-Native principles, or allowing LLM to directly modify database schemas.

---

## Compliance Guardrails

### Forbidden Pattern Detection

```rust
use grep::regex::Regex;

pub struct ArchitectureComplianceChecker {
    head_md_path: PathBuf,
    forbidden_patterns: Vec<Pattern>,
}

#[derive(Debug)]
struct Pattern {
    name: &'static str,
    regex: Regex,
    severity: Severity,
    explanation: &'static str,
}

#[derive(Debug, PartialEq)]
enum Severity {
    Error,    // Must fix before commit
    Warning,  // Should fix, document exception if keeping
    Info,     // FYI, architectural consideration
}

impl ArchitectureComplianceChecker {
    pub fn new(head_md_path: &Path) -> Self {
        Self {
            head_md_path: head_md_path.to_path_buf(),
            forbidden_patterns: vec![
                // CRITICAL: Hardcoded rules violate AI-Native principle
                Pattern {
                    name: "Hardcoded Rules",
                    regex: Regex::new(r#"(match|if)\s*\([^)]*["'](吃了|喝了|买了|多少|几个|什么)["']"#).unwrap(),
                    severity: Severity::Error,
                    explanation: "禁止硬编码规则！事件提取应由SLM完成，规则引擎仅作为fallback",
                },

                // CRITICAL: LLM direct schema modification
                Pattern {
                    name: "LLM Schema Modification",
                    regex: Regex::new(r"(ALTER|CREATE|DROP)\s+TABLE.*llm|ai.*generated|gpt.*output").unwrap(),
                    severity: Severity::Error,
                    explanation: "禁止LLM直接修改Schema！所有Schema变更必须通过migration",
                },

                // CRITICAL: Simplification of AI components
                Pattern {
                    name: "AI Simplification",
                    regex: Regex::new(r"//\s*TODO:\s*use.*llm|replace.*ai.*with.*if.*else").unwrap(),
                    severity: Severity::Error,
                    explanation: "禁止简化AI动态部分！保持SLM主导提取",
                },

                // IMPORTANT: Skipping Derived Views
                Pattern {
                    name: "Skip Derived Views",
                    regex: Regex::new(r"(stable|concept).*directly.*insert|bypass.*derived.*view").unwrap(),
                    severity: Severity::Error,
                    explanation: "禁止跳过Derived Views！所有认知假设必须经过Promotion Gate",
                },

                // IMPORTANT: Untyped actions
                Pattern {
                    name: "Untyped Action",
                    regex: Regex::new(r#"action:\s*String\s*=.*parse\(|action:\s*str::from"#).unwrap(),
                    severity: Severity::Warning,
                    explanation: "行为必须类型化！使用ActionType enum而非String",
                },

                // IMPORTANT: Missing timestamps
                Pattern {
                    name: "Missing Timestamp",
                    regex: Regex::new(r#"INSERT\s+INTO.*event.*\((?!.*timestamp)"#).unwrap(),
                    severity: Severity::Error,
                    explanation: "每个事件必须有精确时间戳",
                },

                // IMPORTANT: Unstructured quantities
                Pattern {
                    name: "Unstructured Quantity",
                    regex: Regex::new(r#"quantity.*text|quantity.*varchar|数量.*文本"#).unwrap(),
                    severity: Severity::Error,
                    explanation: "数量必须结构化存储（不能只存文本）",
                },

                // CRITICAL: Plugin without permissions
                Pattern {
                    name: "Unpermissioned Plugin",
                    regex: Regex::new(r#"impl\s+UserPlugin.*\n.*\n.*\n(?!.*permission)"#).unwrap(),
                    severity: Severity::Error,
                    explanation: "插件必须有权限控制！",
                },

                // IMPORTANT: Conversation not logged
                Pattern {
                    name: "Unlogged Conversation",
                    regex: Regex::new(r#"fn\s+chat\(.*\)\s*(?!.*log|.*record|.*event)"#).unwrap(),
                    severity: Severity::Warning,
                    explanation: "插件对话必须记录为事件",
                },
            ],
        }
    }

    /// Check code files for compliance violations
    pub async fn check_compliance(&self, files: &[PathBuf]) -> Result<ComplianceReport> {
        let mut report = ComplianceReport::default();

        for file in files {
            let content = tokio::fs::read_to_string(file).await?;

            for pattern in &self.forbidden_patterns {
                if let Some(mat) = pattern.regex.find(&content) {
                    report.violations.push(Violation {
                        file: file.clone(),
                        pattern: pattern.name,
                        severity: pattern.severity.clone(),
                        location: Location {
                            line: content[..mat.start()].lines().count() + 1,
                            context: self.extract_context(&content, mat.start(), mat.end()),
                        },
                        explanation: pattern.explanation,
                    });
                }
            }
        }

        Ok(report)
    }

    fn extract_context(&self, content: &str, start: usize, end: usize) -> String {
        const CONTEXT_LINES: usize = 2;

        let start_line = content[..start].lines().count();
        let lines: Vec<&str> = content.lines().collect();

        let from = start_line.saturating_sub(CONTEXT_LINES);
        let to = (start_line + CONTEXT_LINES + 1).min(lines.len());

        lines[from..to].join("\n")
    }

    /// Verify ActionType is used for action fields
    pub async fn check_action_typing(&self, file: &Path) -> Result<bool> {
        let content = tokio::fs::read_to_string(file).await?;

        // Check if ActionType enum exists
        let has_action_type = content.contains("pub enum ActionType")
            || content.contains("enum ActionType");

        // Check if actions use the type
        let uses_action_type = content.contains(": ActionType")
            || content.contains("action: ActionType");

        Ok(has_action_type && uses_action_type)
    }

    /// Run before any Rust struct or SQL migration modification
    pub async fn pre_modify_check(&self, modified_files: &[PathBuf]) -> Result<()> {
        // Read HEAD.md
        let head = tokio::fs::read_to_string(&self.head_md_path).await?;

        // Check for schema changes
        for file in modified_files {
            if file.extension().map_or(false, |e| e == "sql") {
                self.check_migration_compliance(file, &head).await?;
            }

            if file.extension().map_or(false, |e| e == "rs") {
                self.check_rust_compliance(file, &head).await?;
            }
        }

        Ok(())
    }

    async fn check_migration_compliance(&self, file: &Path, head: &str) -> Result<()> {
        let content = tokio::fs::read_to_string(file).await?;

        // Verify no AI-generated content
        if content.contains("llm") || content.contains("ai_generated") {
            return Err(Error::MigrationViolation(
                "Migration contains AI-generated references".to_string()
            ));
        }

        // Verify idempotency
        if !content.contains("IF NOT EXISTS") && !content.contains("CREATE OR REPLACE") {
            warn!("Migration {} may not be idempotent", file.display());
        }

        Ok(())
    }

    async fn check_rust_compliance(&self, file: &Path, head: &str) -> Result<()> {
        let content = tokio::fs::read_to_string(file).await?;

        // Check against HEAD.md forbidden items
        for forbidden_item &[
            "硬编码规则", "固定匹配", "直接修改", "跳过Derived",
        ] {
            if content.contains(forbidden_item) {
                return Err(Error::ArchitectureViolation(format!(
                    "Code contains forbidden item: {}",
                    forbidden_item
                )));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ComplianceReport {
    pub violations: Vec<Violation>,
}

impl ComplianceReport {
    pub fn has_errors(&self) -> bool {
        self.violations.iter().any(|v| v.severity == Severity::Error)
    }

    pub fn display(&self) -> String {
        let mut output = String::new();

        for v in &self.violations {
            let severity = match v.severity {
                Severity::Error => "❌ ERROR",
                Severity::Warning => "⚠️  WARNING",
                Severity::Info => "ℹ️  INFO",
            };

            output.push_str(&format!(
                "{}: {} in {}:{}\n{}\nReason: {}\n\n",
                severity,
                v.pattern,
                v.file.display(),
                v.location.line,
                v.location.context,
                v.explanation
            ));
        }

        output
    }
}

#[derive(Debug)]
pub struct Violation {
    pub file: PathBuf,
    pub pattern: &'static str,
    pub severity: Severity,
    pub location: Location,
    pub explanation: &'static str,
}

#[derive(Debug)]
pub struct Location {
    pub line: usize,
    pub context: String,
}
```

---

## HEAD.md Consistency Verification

```rust
impl ArchitectureComplianceChecker {
    /// Verify all HEAD.md "禁止事项" are not violated
    pub async fn verify_head_compliance(&self, files: &[PathBuf]) -> Result<bool> {
        let head_content = tokio::fs::read_to_string(&self.head_md_path).await?;

        let forbidden_items = self.extract_forbidden_items(&head_content);

        for file in files {
            let content = tokio::fs::read_to_string(file).await?;

            for item in &forbidden_items {
                if content.contains(item) {
                    error!(
                        "File {} contains forbidden item from HEAD.md: {}",
                        file.display(),
                        item
                    );
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    fn extract_forbidden_items(&self, head: &str) -> Vec<String> {
        let mut items = Vec::new();

        // Parse the "禁止事项（永不违背）" section
        if let Some(start) = head.find("禁止事项") {
            let section = &head[start..];
            let end = section.find('\n').unwrap_or(section.len());
            let section_content = &section[..end];

            // Extract bullet points
            for line in section_content.lines() {
                if line.trim().starts_with("- ") || line.trim().starts_with("❌") {
                    items.push(line.trim().to_string());
                }
            }
        }

        items
    }
}
```

---

## Required Behavior Verification

```rust
impl ArchitectureComplianceChecker {
    /// Verify all "必须行为" are implemented
    pub async fn verify_required_behaviors(&self, files: &[PathBuf]) -> Result<bool> {
        let head_content = tokio::fs::read_to_string(&self.head_md_path).await?;

        let required = self.extract_required_behaviors(&head_content);

        for (behavior, description) in required {
            let mut found = false;

            for file in files {
                let content = tokio::fs::read_to_string(file).await?;
                if content.contains(&behavior) {
                    found = true;
                    break;
                }
            }

            if !found {
                warn!(
                    "Required behavior not found: {} - {}",
                    behavior, description
                );
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn extract_required_behaviors(&self, head: &str) -> Vec<(String, String)> {
        let mut items = Vec::new();

        // Parse the "必须行为" section
        // Extract behaviors and their descriptions
        items.push((
            "TIMESTAMPTZ".to_string(),
            "每个事件必须有精确时间戳".to_string()
        ));
        items.push((
            "quantity: Option<f32>".to_string(),
            "数量必须结构化存储".to_string()
        ));
        items.push((
            "ActionType".to_string(),
            "行为必须类型化".to_string()
        ));
        items.push((
            "expires_at: DateTime".to_string(),
            "派生视图必须有过期时间".to_string()
        ));

        items
    }
}
```

---

## Integration with Git Hooks

```rust
/// Git pre-commit hook for architecture compliance
pub async fn pre_commit_hook() -> Result<()> {
    let checker = ArchitectureComplianceChecker::new(Path::new("todo/head.md"));

    // Get staged files
    let staged_files = get_staged_files().await?;

    // Run compliance check
    let report = checker.check_compliance(&staged_files).await?;

    if report.has_errors() {
        eprintln!("Architecture compliance check failed:\n");
        eprintln!("{}", report.display());
        eprintln!("Please fix violations before committing.");
        std::process::exit(1);
    }

    Ok(())
}
```

---

## Recommended Combinations

Use this skill together with:
- **All skills**: As a pre-commit validation step
- **TestingAndDebugging**: For comprehensive pre-commit checks
- **OllamaPromptEngineering**: To ensure AI components remain AI-Native
