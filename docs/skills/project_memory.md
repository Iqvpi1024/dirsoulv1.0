# Skill: Project Memory

> **Purpose**: Help AI remember project history, decisions, and context across sessions. Critical for DirSoul's long-term development with layered memory architecture.

---

## Memory Storage Structure

### Memory Hierarchy (mirrors DirSoul architecture)

```yaml
# docs/chat/memory.yaml (persistent memory file)
project: DirSoul
last_updated: 2026-02-03

# Raw decisions (append-only, like Layer 1)
raw_decisions:
  - timestamp: "2026-01-15T10:00:00Z"
    context: "Initial database schema design"
    decision: "Use PostgreSQL with pgvector instead of separate vector DB"
    reasoning: "Reduces components, better integration in 8GB environment"
    alternatives_rejected:
      - "Pinecone (external service, privacy concern)"
      - "Milvus (separate service, memory overhead)"

  - timestamp: "2026-01-20T14:30:00Z"
    context: "AI model selection"
    decision: "Use Phi-4-mini (3.8B) via Ollama"
    reasoning: "Fits in 8GB RAM, good reasoning能力"
    alternatives_rejected:
      - "Qwen2.5:0.5b (weaker reasoning)"
      - "GPT-4 via API (privacy concern, cost)"

# Structured patterns (like Layer 2)
patterns:
  - name: "Event Extraction Pipeline"
    first_used: "2026-01-25"
    usage_count: 15
    confidence: 0.95
    description: "SLM extraction → Rule fallback → Confidence check"
    last_refined: "2026-02-01"

# Stabilized concepts (like Layer 3)
concepts:
  - name: "Slow Abstraction Principle"
    status: "promoted"
    promoted_date: "2026-01-28"
    validation_count: 8
    description: "Derived Views → Promotion Gate → Stable Concepts"
```

---

## Memory Update Patterns

### When to Update Memory

```rust
/// Update memory after significant decisions
pub async fn record_decision(
    context: &str,
    decision: &str,
    reasoning: &str,
    alternatives: &[&str]
) -> Result<()> {
    let entry = DecisionEntry {
        timestamp: Utc::now(),
        context: context.to_string(),
        decision: decision.to_string(),
        reasoning: reasoning.to_string(),
        alternatives_rejected: alternatives.iter().map(|s| s.to_string()).collect(),
    };

    append_to_memory_file("raw_decisions", entry).await?;

    // Also add to HEAD.md changelog if major
    if is_major_decision(decision) {
        update_head_changelog(decition).await?;
    }

    Ok(())
}
```

### Decision Categorization

```yaml
# Decision impact levels
major_decisions:
  - "Technology stack changes"
  - "Architecture modifications"
  - "Breaking API changes"

minor_decisions:
  - "Code style preferences"
  - "Variable naming"
  - "Minor refactors"

# Only major decisions go to HEAD.md
```

---

## Memory Retrieval Patterns

### Context-Aware Lookup

```rust
/// Find relevant decisions for current context
pub async fn find_relevant_decisions(
    current_context: &str
) -> Result<Vec<DecisionEntry>> {
    // 1. Keyword matching
    let keywords = extract_keywords(current_context);

    // 2. Search raw_decisions
    let relevant = search_memory("raw_decisions", &keywords).await?;

    // 3. Check related patterns
    let patterns = find_related_patterns(current_context).await?;

    // 4. Include stabilized concepts
    let concepts = get_relevant_concepts(current_context).await?;

    Ok(combine_results(relevant, patterns, concepts))
}
```

---

## HEAD.md Compliance

### Memory Must Respect Core Principles

```yaml
# All memory updates must follow HEAD.md rules
compliance_rules:
  - "Never record hardcoded rules in memory"
  - "Document AI-Native decisions (SLM优先)"
  - "Note privacy-first choices (零云依赖)"
  - "Record architectural boundaries (分层架构)"
  - "Include reasoning (explaining 'why' not 'what')"

# Forbidden in memory:
forbidden_entries:
  - "今天用if/else替换了事件抽取" # 违反AI-Native
  - "为了方便直接修改了Schema"    # 违反流程
  - "简化了AI部分"               # 违反核心设计
```

---

## Cross-Session Context

### Session Summary Template

```markdown
## Session Summary: YYYY-MM-DD

### Work Completed
- [ ] Task 1.1完成
- [ ] Task 2.3进行中

### Key Decisions Made
- **Decision**: X
  - **Reasoning**: Y
  - **Alternatives rejected**: Z

### Dependencies Created
- Task 3.1 depends on Task 2.3 completion

### HEAD.md Verification
- [x] Checked HEAD.md before starting
- [x] No forbidden patterns violated
- [x] All required behaviors implemented

### Next Session Priorities
1. Complete Task 2.3
2. Start Task 3.1
3. Review and update HEAD.md if needed
```

---

## Integration with Development

### Auto-Save Triggers

```rust
/// Automatically save context at key points
pub struct MemoryHooks;

impl MemoryHooks {
    pub async fn on_task_complete(task_id: &str) -> Result<()> {
        // Save task completion
        let summary = generate_task_summary(task_id).await?;
        append_session_log("task_completion", summary).await?;

        // Update todo.md progress
        update_todo_progress(task_id).await?;

        Ok(())
    }

    pub async fn on_decision_made(decision: &Decision) -> Result<()> {
        // Check HEAD.md compliance first
        if !verify_head_compliance(decision).await? {
            warn!("Decision may violate HEAD.md principles");
            return Err(Error::HeadViolation);
        }

        // Record decision
        record_decision(
            &decision.context,
            &decision.text,
            &decision.reasoning,
            &decision.alternatives
        ).await?;

        Ok(())
    }

    pub async fn on_error_encountered(error: &Error) -> Result<()> {
        // Record error and solution
        let entry = ErrorEntry {
            timestamp: Utc::now(),
            error_type: std::any::type_name::<Error>(),
            message: error.to_string(),
            solution: extract_solution(error)?,
        };

        append_session_log("errors_and_solutions", entry).await?;
        Ok(())
    }
}
```

---

## Quick Reference

### Memory File Locations

```
docs/
├── chat/
│   ├── memory.yaml              # Main project memory
│   ├── session_2026-02-03.yaml  # Daily session logs
│   └── decisions/               # Major decision records
│       ├── 001-postgres-choice.md
│       ├── 002-phi4-mini-choice.md
│       └── ...
```

### Memory Commands

```bash
# View recent decisions
cat docs/chat/memory.yaml | grep -A 5 "raw_decisions"

# Find decisions about specific topic
grep -r "pgvector" docs/chat/decisions/

# Update memory after significant work
# (AI will prompt for this automatically)
```

---

## HEAD.md Integration

```markdown
## 在 CLAUDE.md 中引用

在开发开始时，AI应该：

1. 读取 HEAD.md 确保方向不偏离
2. 读取 docs/chat/memory.yaml 了解历史决策
3. 检查是否有与当前任务相关的记录决策
4. 完成任务后更新 memory.yaml

示例提示词：
"Use Skill: ProjectMemory to check if we've made decisions about database schema before proceeding with Task 2.1"
```

---

## Compliance Checklist

Before recording any decision to memory:

- [ ] Does this align with HEAD.md core principles?
- [ ] Am I recording "why" not just "what"?
- [ ] Are there alternatives considered and documented?
- [ ] Does this violate any "禁止事项"?
- [ ] Is this properly categorized (raw/pattern/concept)?

---

## Recommended Combinations

Use this skill together with:
- **TestingAndDebugging**: For recording error solutions
- **CheckArchitectureCompliance**: For verifying decisions before recording
- **All other skills**: For context preservation across sessions
