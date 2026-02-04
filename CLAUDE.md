# Claude Code Configuration for DirSoul

> **Version**: V1.0
> **Last Updated**: 2026-02-03

This file configures Claude Code (or compatible AI assistants) for the DirSoul project.

---

## Supreme Authority: HEAD.md

```
todo/head.md is the PROJECT CONSTITUTION with supreme authority.

All skills MUST defer to HEAD.md. If any skill conflicts with HEAD.md,
follow HEAD.md and document the deviation with reasoning.
```

Before any development:
1. Read `todo/head.md`
2. Understand the task context
3. Check relevant skills for guidance
4. Verify no "禁止事项" (forbidden items) are violated

---

## Available Skills

All skills are located in `docs/skills/`. Use them as declarative reference
knowledge, not as command scripts to be executed.

### Core Architecture Skills (Priority 1)

| Skill | File | When to Use |
|-------|------|-------------|
| CheckArchitectureCompliance | `docs/skills/check_architecture_compliance.md` | **Before any code changes** - verify HEAD.md compliance |
| RustMemorySafety | `docs/skills/rust_memory_safety.md` | Writing Rust code, especially for memory management |
| PostgresSchemaDesign | `docs/skills/postgres_schema_design.md` | Database schema design, migrations, indexing |
| EncryptionBestPractices | `docs/skills/encryption_best_practices.md` | Any data encryption, key management |
| PluginPermissionSystem | `docs/skills/plugin_permission_system.md` | Plugin development, permission checks |

### AI Integration Skills (Priority 2)

| Skill | File | When to Use |
|-------|------|-------------|
| EventExtractionPatterns | `docs/skills/event_extraction_patterns.md` | Event extraction, time parsing |
| OllamaPromptEngineering | `docs/skills/ollama_prompt_engineering.md` | Writing prompts for Phi-4-mini |
| EntityResolution | `docs/skills/entity_resolution.md` | Entity disambiguation, attribute growth |
| CognitiveViewGeneration | `docs/skills/cognitive_view_generation.md` | Derived views, promotion gate |
| DeepTalkImplementation | `docs/skills/deeptalk_implementation.md` | Default plugin development |

### Development Workflow Skills (Priority 3)

| Skill | File | When to Use |
|-------|------|-------------|
| TestingAndDebugging | `docs/skills/testing_and_debugging.md` | **After completing code** - before committing |
| CodeReview | `docs/skills/code_review.md` | Reviewing code changes |
| Debugging | `docs/skills/debugging.md` | Investigating bugs or issues |
| ProjectMemory | `docs/skills/project_memory.md` | **Before/after significant work** - record decisions |
| Documentation | `docs/skills/documentation.md` | Writing or updating documentation |

### Specialized Testing Skills (Priority 4)

| Skill | File | When to Use |
|-------|------|-------------|
| EstimateResourceUsage | `docs/skills/estimate_resource_usage.md` | **Before deploying** - check 8GB memory usage |
| TestExtractionPrompt | `docs/skills/test_extraction_prompt.md` | Testing event extraction prompts |
| SimulateCognitiveEvolution | `docs/skills/simulate_cognitive_evolution.md` | Testing promotion gate logic |

### UI/UX Skills (Priority 5)

| Skill | File | When to Use |
|-------|------|-------------|
| FrontendDesign | `docs/skills/frontend_design.md` | Streamlit interface development |

---

## How to Use Skills

### Explicit Invocation

When you want Claude to use specific knowledge:

```
"Use Skill: RustMemorySafety when implementing the RawMemory struct"
"Check with CheckArchitectureCompliance before modifying the schema"
"Apply OllamaPromptEngineering patterns for the event extraction prompt"
```

### Automatic Suggestion

Claude should automatically reference relevant skills:

- When writing Rust → RustMemorySafety
- When designing database → PostgresSchemaDesign
- When encrypting data → EncryptionBestPractices
- When testing → TestingAndDebugging

### Skill Combinations

Multiple skills can be combined:

```
"Use Skills: RustMemorySafety + EncryptionBestPractices for encrypted BYTEA handling"
"Apply CheckArchitectureCompliance + CodeReview before committing"
"Reference EventExtractionPatterns + OllamaPromptEngineering for AI integration"
```

---

## Development Workflow

### Before Starting a Task

```yaml
steps:
  1. Read: "todo/head.md"
  2. Read: "todo/todo.md" - understand task dependencies
  3. Identify: Which skills are relevant?
  4. Plan: Approach that respects HEAD.md principles
```

### During Development

```yaml
checks:
  architecture:
    - "Does this align with AI-Native principle?"
    - "Am I introducing hardcoded rules?"
    - "Am I bypassing Derived Views?"

  memory:
    - "Will this fit in 8GB?"
    - "Am I using Vec::with_capacity()?"
    - "Can I use iterators instead?"

  privacy:
    - "Is data encrypted?"
    - "Are permissions checked?"
    - "No cloud dependencies?"

  quality:
    - "Are tests passing?"
    - "Is coverage > 80%?"
    - "Is documentation updated?"
```

### After Completing a Task

```yaml
steps:
  1. Test: "Run cargo test"
  2. Review: "Use CodeReview skill"
  3. Compliance: "Use CheckArchitectureCompliance"
  4. Document: "Update relevant docs"
  5. Memory: "Use ProjectMemory to record decisions"
  6. Update: "Mark task complete in todo.md"
```

---

## Forbidden Behaviors (Never Violate)

From HEAD.md - these are NEVER acceptable:

```yaml
forbidden:
  - "硬编码规则 (hardcoded rules)"
  - "LLM直接修改Schema (LLM directly modifies schema)"
  - "简化AI动态部分 (simplify AI components)"
  - "跳过Derived Views (skip derived views)"
  - "过早引入图数据库 (premature graph DB)"
  - "优化向量而忽略事件 (optimize vectors, ignore events)"
  - "插件无权限控制 (plugins without permissions)"
  - "对话不存记忆 (conversations not logged)"

# If any skill suggests these, IGNORE THE SKILL and follow HEAD.md
```

---

## Required Behaviors (Must Implement)

```yaml
required:
  - "每个事件必须有精确时间戳 (precise timestamps)"
  - "数量必须结构化存储 (structured quantities)"
  - "行为必须类型化 (typed actions)"
  - "支持时间范围查询 (time range queries)"
  - "派生视图必须有过期时间 (view expiration)"
  - "插件对话必须记录为事件 (log plugin conversations)"
```

---

## Skill Hierarchy

```
1. HEAD.md (todo/head.md)
   ↓ Supreme authority, cannot be overridden
   ↓
2. Project-Specific Skills (docs/skills/*.md)
   ↓ Adapted for DirSoul, respect HEAD.md
   ↓
3. General Best Practices
   ↓ Rust, SQL, Python conventions
   ↓
4. AI Suggestions
   ↓ Must be validated against above layers
```

---

## Error Recovery

If Claude detects a deviation from HEAD.md:

```yaml
recovery:
  1. "Stop immediately"
  2. "Re-read todo/head.md"
  3. "Identify the deviation"
  4. "Ask human for clarification if uncertain"
  5. "Preserve context for traceability"
```

---

## Quick Reference

### Common Task → Skills Mapping

| Task | Primary Skills | Secondary Skills |
|------|----------------|------------------|
| Add event extraction | EventExtractionPatterns, OllamaPromptEngineering | TestExtractionPrompt |
| Create database migration | PostgresSchemaDesign, CheckArchitectureCompliance | TestingAndDebugging |
| Implement encryption | EncryptionBestPractices, RustMemorySafety | TestingAndDebugging |
| Build DeepTalk plugin | DeepTalkImplementation, PluginPermissionSystem | OllamaPromptEngineering |
| Add derived views | CognitiveViewGeneration, EntityResolution | SimulateCognitiveEvolution |
| Write Rust code | RustMemorySafety, CheckArchitectureCompliance | TestingAndDebugging |
| Create Streamlit UI | FrontendDesign, DeepTalkImplementation | Documentation |
| Debug issue | Debugging, EstimateResourceUsage | ProjectMemory |
| Review PR | CodeReview, CheckArchitectureCompliance | TestingAndDebugging |

### Phase-Specific Skills

| Phase | Core Skills |
|-------|-------------|
| Phase 1-2: Environment + Raw Memory | PostgresSchemaDesign, RustMemorySafety, EncryptionBestPractices |
| Phase 3: Event Memory | EventExtractionPatterns, OllamaPromptEngineering, PostgresSchemaDesign |
| Phase 4: Entity Memory | EntityResolution, PostgresSchemaDesign |
| Phase 5: Cognitive Memory | CognitiveViewGeneration, SimulateCognitiveEvolution |
| Phase 6: Plugins | PluginPermissionSystem, DeepTalkImplementation |
| Phase 7: Storage | EncryptionBestPractices, PostgresSchemaDesign, EstimateResourceUsage |
| Phase 8: UI/Testing | FrontendDesign, TestingAndDebugging, Documentation |

---

## Environment Constraints

Always remember:

```yaml
memory: "8GB total"
allocation:
  ollama: "4-5GB (Phi-4-mini Q4)"
  postgresql: "1-2GB"
  system: "1GB"
  cache: "~500MB"

implications:
  - "Batch AI operations"
  - "Limit database connections"
  - "Use streaming for large data"
  - "Monitor with EstimateResourceUsage"
```

---

## Integration with Git Hooks

Pre-commit checks (optional but recommended):

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run architecture compliance
cargo run --bin check_compliance || exit 1

# Run tests
cargo test || exit 1

# Check coverage
cargo tarpaulin --out Xml --exclude-files '*/tests/*' || exit 1

# Check formatting
cargo fmt -- --check || exit 1

# Run clippy
cargo clippy -- -D warnings || exit 1
```

---

## Claude Code Commands Reference

When working with Claude Code CLI:

```bash
# View all skills
claude skills list

# Use specific skill
claude run --skill RustMemorySafety "implement the RawMemory struct"

# Check compliance
claude run --skill CheckArchitectureCompliance "review this code"

# Generate documentation
claude run --skill Documentation "document the Event API"
```

---

## Troubleshooting

### If Claude isn't using skills:

1. **Check skill exists**: `ls docs/skills/`
2. **Verify reference**: Skill name matches filename (case-insensitive)
3. **Explicit invoke**: Use "Use Skill: SkillName" in prompt
4. **Check HEAD.md**: Ensure skill doesn't conflict with supreme authority

### If skill suggests violating HEAD.md:

1. **Stop the implementation**
2. **Re-read HEAD.md**
3. **Document the conflict**
4. **Follow HEAD.md, not the skill**
5. **Report the skill issue** (skills should respect HEAD.md)

---

## Updates and Maintenance

### When to update this file:

- Adding new skills
- Changing skill priorities
- Updating workflow
- Adding new constraints

### Update process:

1. Modify this file
2. Update `docs/skills/` directory
3. Update todo/head.md if architecture changes
4. Commit with clear message

---

## Summary

**Remember**: HEAD.md is the constitution. Skills are reference guides.
When in doubt, prioritize in this order:

1. HEAD.md (todo/head.md) - Supreme
2. Task requirements (todo/todo.md)
3. Relevant skills (docs/skills/*.md)
4. General best practices
5. AI suggestions (last, must validate)

**Goal**: Build a long-term, maintainable digital brain that respects
privacy, uses AI-native design, and can grow with the user over 10+ years.
