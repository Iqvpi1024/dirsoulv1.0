# Skill: Documentation

> **Purpose**: Generate and maintain project documentation following "大白话原则" (explain why not what), ensuring clarity and maintainability.

---

## Documentation Standards

### Core Principles

```yaml
documentation_principles:
  大白话原则:
    - "假设读者不懂代码"
    - "用类比解释复杂概念"
    - "优先解释'为什么'而非'是什么'"

  accuracy:
    - "与代码同步更新"
    - "版本控制所有文档"
    - "记录设计权衡"

  risk_transparency:
    - "明确列出潜在风险"
    - "提供缓解方案"
    - "标注已知限制"
```

---

## File Organization

### Structure (per HEAD.md)

```
docs/
├── design/           # 设计文档
│   ├── event_memory_design.md
│   ├── cognitive_architecture.md
│   └── plugin_system.md
├── api/              # API 文档
│   ├── events_api.md
│   ├── memory_api.md
│   └── plugin_api.md
├── test/             # 测试文档和结果
│   ├── test_report_2026-02-03.md
│   └── coverage_report.md
├── chat/             # 历史对话记录
│   ├── memory.yaml
│   └── session_*.yaml
└── specs/            # 技术规格说明
    ├── event_format_spec.md
    └── view_promotion_spec.md
```

---

## Code Documentation

### Rust Comments

```rust
/// 处理原始输入并提取事件
///
/// 这个函数是DirSoul的核心入口点。当用户输入任何内容（文本、语音等），
/// 它首先被转换为RawMemory并存储（Layer 1），然后使用AI提取结构化事件（Layer 2）。
///
/// # 为什么需要分层存储？
///
/// 想象你在写日记：你会先记下发生了什么（Raw Memory），事后可能会总结
/// "我最近经常喝咖啡"（Derived View）。但原始日记不能修改——它是真相的来源。
/// 同样，我们保留原始输入，让AI从这些"日记"中学习模式。
///
/// # 处理流程
///
/// 1. **加密存储**：输入被加密后存入数据库（隐私优先）
/// 2. **AI提取**：Phi-4-mini分析输入，提取事件结构
/// 3. **规则兜底**：如果AI失败，使用规则引擎作为fallback
/// 4. **置信度检查**：只有高置信度的事件才会被存储
///
/// # 内存限制
///
/// 在8GB环境中，我们使用批处理来减少AI推理次数。例如：
/// - 单次处理：100 tokens × 10 inputs = 1000 tokens
/// - 批量处理：1次调用处理10 inputs
///
/// # 错误处理
///
/// - 如果加密失败：返回错误，不存储任何数据
/// - 如果AI提取失败：尝试规则引擎
/// - 如果两者都失败：记录原始输入但无事件结构
///
/// # 示例
///
/// ```ignore
/// let input = "我今天早上吃了3个苹果";
/// let events = process_input(input).await?;
/// assert_eq!(events[0].action, "吃");
/// assert_eq!(events[0].target, "苹果");
/// assert_eq!(events[0].quantity, Some(3.0));
/// ```
///
/// # HEAD.md 遵守
///
/// - ✅ 事件有时间戳（精确）
/// - ✅ 数量结构化存储（不是文本）
/// - ✅ 行为类型化（ActionType）
/// - ✅ 规则仅作为fallback
/// - ✅ 端到端加密
pub async fn process_input(input: RawInput) -> Result<Vec<Event>> {
    // Implementation...
}

/// 检查派生视图是否应该晋升为稳定概念
///
/// # "慢抽象"原则解释
///
/// 想象你在观察一个朋友的行为：
/// - 第1天：他喝了咖啡 → "他可能喜欢咖啡"（假设，置信度低）
/// - 第30天：他每天喝咖啡 → "他确实喜欢咖啡"（置信度高）
/// - 第60天：仍然每天喝 → 这是稳定的概念了
///
/// 我们的"慢抽象"原则就是：不要急于下结论。让AI提出假设，
/// 但只有经过时间和验证后，才将其视为"真理"。
///
/// # 为什么需要这个闸门？
///
/// AI会"幻觉"——它可能自信地说错话。如果我们直接把AI的输出
/// 当作真理存入数据库，错误会固化并传播。Promotion Gate就像
/// 一个编辑：审查AI的建议，只有足够确信时才发布。
///
/// # 晋升标准
///
/// ```text
/// 置信度 > 0.85      AND
/// 时间跨度 > 30天     AND
/// 验证次数 >= 3       AND
/// 无矛盾观点
/// ```
///
/// # 潜在风险
///
/// - **风险**：真实变化可能被延迟识别（如用户突然不再喝咖啡）
/// - **缓解**：DerivedView有过期时间（30天），过期后需重新验证
///
/// # 示例
///
/// ```ignore
/// let view = DerivedView {
///     hypothesis: "用户喜欢喝咖啡".to_string(),
///     confidence: 0.9,
///     created_at: Utc::now() - Duration::days(35),
///     validation_count: 10,
///     counter_evidence: vec![],
/// };
/// assert!(should_promote(&view));
/// ```
pub fn should_promote(view: &DerivedView) -> bool {
    view.confidence > 0.85
        && (Utc::now() - view.created_at) > Duration::days(30)
        && view.validation_count >= 3
        && !has_conflicting_views(view)
}
```

### Python Docstrings

```python
def retrieve_relevant_events(user_id: str, query: str) -> List[Event]:
    """
    检索与用户查询相关的事件。

    这就像在图书馆找资料：你知道要找什么，但书库太大。
    我们先用"向量搜索"找到可能相关的书（基于语义相似度），
    再用"SQL过滤"精确定位（基于时间、置信度等）。

    为什么用混合搜索？
    - 纯向量搜索：可能找到语义相关但时间久远的事件
    - 纯SQL搜索：可能错过语义相关但用词不同的事件
    - 混合搜索：兼顾语义理解和精确匹配

    Args:
        user_id: 用户ID，用于隔离数据
        query: 用户的查询文本，如"我最近运动了吗？"

    Returns:
        最多10个最相关的事件，按相关性排序

    示例:
        >>> events = retrieve_relevant_events("user123", "运动")
        >>> print([e.action for e in events])
        ['去', '运动', '跑步']
    """
    pass
```

---

## API Documentation

### Endpoint Documentation Template

```markdown
# Event Memory API

## 概述

事件记忆API允许你存储和检索用户的结构化事件。这是DirSoul的Layer 2——
在原始记忆之上构建的结构化数据层。

**类比**：原始记忆是"日记"，事件记忆是"日记索引"。当你问"我上周吃了几次苹果？"
系统会搜索事件记忆而不是重读所有日记。

---

## POST /events

### 描述
创建新事件。事件必须从原始输入通过SLM提取创建，不能手动构造。

### 为什么不能直接创建事件？

这违反了AI-Native原则。如果允许手动创建事件：
1. 你可能会硬编码规则（"if input contains '苹果', create event..."）
2. 无法保证置信度评估
3. 无法追踪事件来源

正确做法：调用 `/process` 端点，让SLM提取事件。

### 请求
```json
{
  "events": [
    {
      "action": "吃",
      "target": "苹果",
      "quantity": 3,
      "unit": "个",
      "confidence": 0.95,
      "timestamp": "2026-02-03T10:30:00Z",
      "source_memory_id": "uuid-from-raw-memory"
    }
  ]
}
```

### 风险与限制
- **风险**：低置信度事件可能污染认知层
- **缓解**：confidence < 0.5 的事件被拒绝
- **限制**：每次最多100个事件（防止内存溢出）

### 响应
- `201 Created`: 事件已存储
- `400 Bad Request`: 置信度过低或格式错误
- `409 Conflict`: 事件与已知观点矛盾

---

## GET /events

### 描述
查询用户的事件，支持时间范围、动作、对象等过滤。

### 查询参数
| 参数 | 类型 | 描述 | 示例 |
|------|------|------|------|
| user_id | string | 必需，用户ID | user123 |
| start_time | datetime | 开始时间（ISO 8601） | 2026-01-01T00:00:00Z |
| end_time | datetime | 结束时间 | 2026-02-01T00:00:00Z |
| action | string | 行为过滤（可重复） | action=吃&action=喝 |
| min_confidence | float | 最低置信度 | 0.8 |

### 示例
```bash
# 获取上周所有饮食相关事件
GET /events?user_id=user123&action=吃&action=喝&start_time=2026-01-27T00:00:00Z
```

### 性能考虑
- 使用复合索引 `(user_id, timestamp DESC)`
- 查询30天以上数据可能较慢
- 8GB环境建议限制返回1000条以内
```

---

## Design Documentation

### Architecture Decision Records (ADR)

```markdown
# ADR-001: 使用PostgreSQL作为主数据库

## 状态
已接受 (2026-01-15)

## 背景
DirSoul需要一个数据库来存储10年+的记忆数据。主要挑战：
- 内存限制：8GB环境
- 隐私要求：零云依赖
- 扩展性：支持向量检索

## 决策
使用 **PostgreSQL 16+** 作为主数据库，配合以下扩展：
- `pgvector`: 向量相似度搜索
- 分区表：按月分区，支持长期存储

## 为什么选PostgreSQL？

### 1. 内存效率
PostgreSQL的shared_buffers配置可控：
- 8GB环境：256MB shared_buffers
- 相比Neo4j（默认2GB+）更节省内存

### 2. 零云依赖
- 完全开源，可离线部署
- 不依赖外部API（如Pinecone）
- 符合隐私优先原则

### 3. pgvector集成
- 向量检索与SQL在同一数据库
- 减少组件复杂度
- 支持HNSW索引（高性能）

### 4. 分区支持
- 按月分区支持10年+数据
- 自动归档旧数据
- 查询性能稳定

## 考虑过的替代方案

| 方案 | 优点 | 缺点 | 为什么不选 |
|------|------|------|-----------|
| Pinecone | 易用 | 外部服务，隐私风险 | 违反零云依赖 |
| Milvus | 向量专用 | 独立服务，内存开销大 | 8GB环境紧张 |
| Neo4j | 图数据库优势 | 内存占用高，向量需额外方案 | 过度设计 |

## 风险与缓解

### 风险1：向量检索性能可能不如专业向量DB
**缓解**：
- 使用HNSW索引（比IVFFlat快）
- 设置合理的相似度阈值（0.18）
- 结合SQL过滤减少搜索空间

### 风险2：分区表管理复杂
**缓解**：
- 自动化分区创建（pg_cron）
- 迁移脚本模板化
- 监控和告警

## 后果
- 正面：单数据库简化架构，内存可控
- 负面：向量检索性能需持续优化
- 需要定期维护分区

## 参考
- [PostgreSQL pgvector](https://github.com/pgvector/pgvector)
- HEAD.md Section: 技术栈选型
```

---

## README.md Structure

```markdown
# DirSoul

> 一个本地优先、隐私优先的永久记忆框架

一句话描述：**DirSoul不是聊天机器人，而是一个会成长的数字大脑。**

## 什么是DirSoul？

想象你有一个朋友，他记得你做过的每一件事：
- 你昨天吃了什么
- 你上周去了哪里
- 你最近在关注什么

但DirSoul不止是"记忆"——它能从这些经历中学习：
- 发现模式（"你最近经常喝咖啡"）
- 形成概念（"你喜欢咖啡"）
- 提供洞察（"你的咖啡摄入在增加"）

**关键区别**：DirSoul不会"编造"知识。它只从你的真实经历中学习。

## 核心特性

### 1. 隐私优先
- 所有数据本地存储
- 端到端加密
- 零云依赖
- 你拥有你的数据

### 2. AI-Native设计
- 使用Phi-4-mini本地推理
- 不硬编码规则
- 从经验学习，而非死记硬背

### 3. 慢抽象原则
- AI提出假设（"你可能喜欢咖啡"）
- 程序验证（30天后确认）
- 防止AI幻觉固化

### 4. 长期存储
- 支持十年+数据增长
- 自动归档和压缩
- 8GB内存优化

## 快速开始

### 安装
\`\`\`bash
# 克隆仓库
git clone https://github.com/yourname/dirsoul.git
cd dirsoul

# 安装Rust和PostgreSQL（见 docs/installation.md）
# 配置数据库
createdb dirsoul_db

# 运行迁移
diesel migration run

# 启动Ollama并拉取模型
ollama serve
ollama pull phi4-mini

# 运行DirSoul
cargo run
\`\`\`

### 第一次使用
\`\`\`bash
# 输入你的第一条记忆
echo "我今天开始使用DirSoul了" | cargo run --bin process

# 查看提取的事件
cargo run --bin query -- --last 24h
\`\`\`

## 架构概览

```
┌─────────────────────────────────────────────────────┐
│                    输入层                            │
│  文本 | 语音 | 图片 | 文档  →  RawMemory (加密)        │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│                  AI处理层                            │
│  Phi-4-mini: 事件提取 + 实体识别 + 情感分析           │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│              Layer 2: 事件记忆                        │
│  结构化事件 {action, target, quantity, confidence}   │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│              Layer 3: 结构化记忆                      │
│  实体 + 关系 + 属性（动态增长）                       │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│              Layer 4: 认知记忆                        │
│  DerivedViews → PromotionGate → StableConcepts      │
└─────────────────────────────────────────────────────┘
```

## 项目状态

当前版本：v1.0.0 (开发中)

| Phase | 进度 | 说明 |
|-------|------|------|
| Phase 1: 环境 | 60% | Rust, PostgreSQL, Ollama配置完成 |
| Phase 2: 原始记忆 | 0% | 待开始 |
| Phase 3: 事件记忆 | 0% | 待开始 |

## 文档

- [设计文档](docs/design/) - 架构和设计决策
- [API文档](docs/api/) - 接口规格
- [开发指南](todo/head.md) - 给AI开发者的指南
- [任务追踪](todo/todo.md) - 当前开发进度

## 贡献

欢迎贡献！请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md)

## 许可证

MIT License - 见 [LICENSE](LICENSE)
```

---

## Document Maintenance

### When to Update Docs

```yaml
update_triggers:
  code_changes:
    - "新增公共函数 → 更新API文档"
    - "架构变更 → 更新设计文档"
    - "新增配置 → 更新README"

  decisions:
    - "技术选型 → 创建ADR"
    - "重大设计变更 → 更新架构文档"

  regular:
    - "每周 → 更新开发进度"
    - "每个Phase → 更新README状态表"
```

### Version Control

```markdown
<!-- docs/design/ADR.md 前言 -->
---

**文档版本**: v2.1
**最后更新**: 2026-02-03
**变更历史**:
- v2.1 (2026-02-03): 添加RLM集成说明
- v2.0 (2026-01-28): 重构认知层设计
- v1.0 (2026-01-15): 初始版本

---
```

---

## Recommended Combinations

Use this skill together with:
- **ProjectMemory**: For documenting decisions
- **CodeReview**: For code documentation standards
- **TestingAndDebugging**: For test documentation
