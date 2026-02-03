# DirSoul 项目开发指南 (HEAD.md)

> **版本**: V2.1 (2026 整合版 + Claude Code 最佳实践)
> **更新日期**: 2026-02-03
> **服务器**: 8G内存环境
> **默认AI**: Phi-4-mini (3.8B) via Ollama
> **默认插件**: DeepTalk (深度聊天，始终启用)

---

## 项目核心愿景

DirSoul 是一个**本地优先、隐私优先的永久记忆框架**，支持：

- 记录用户的一切交互（文本、语音、图片、文档）
- AI驱动的动态事件抽取与认知演化
- 插件化扩展（决策分析、心理分析等）
- 10年+ 数据增长不崩溃
- 类人认知能力：事件记忆 → 模式识别 → 概念形成 → 自主好奇

---

## 一句话项目定位

**不是一个聊天机器人，而是一个数字大脑：拥有情节记忆、能从经历中学习、支持插件扩展的认知操作系统。**

---

## 核心设计原则（永不违背）

### 1. AI-Native 与动态性
- **拒绝硬编码规则**（如固定匹配"多少？"、"什么？"）
- **SLM主导提取**（Phi-4-mini 本地推理，隐私优先）
- 规则仅作为SLM失败的兜底
- 系统从经验学习，而非死记硬背

### 2. 隐私与安全
- **端到端加密**（Fernet + 密钥管理）
- **零云依赖**（所有数据本地）
- **审计日志**（记录所有访问）
- **数据导出/导入**（GDPR合规）

### 3. 分层架构（模块化）

```
Layer 1: 原始记忆 (Raw Memory)
   └─ Append-only，全量保留，永不修改

Layer 2: 结构化记忆 (Structured Memory)
   └─ 事件/实体，带向量索引，可重算

Layer 3: 认知记忆 (Cognitive Memory)
   └─ 派生视图/稳定概念，可过期/版本化

上层: Agent 与 插件
   └─ 读写记忆，权限控制，沙箱隔离
```

### 4. 慢抽象原则（2026共识）
- **Derived Views 先行**：生成可丢弃的认知假设
- **Promotion Gate 把关**：程序判定是否晋升为稳定概念
- **避免 LLM 幻觉放大**：隔离 AI 判断与系统结构

### 5. 技术栈选型（8G内存优化）

| 组件 | 技术 | 理由 |
|------|------|------|
| 核心逻辑 | **Rust** | 内存安全、高性能、并发安全、长期维护 |
| 数据库 | **PostgreSQL 16+** | 支持分区、JSONB、pgvector、可扩展到集群 |
| 界面 | **Python + Streamlit** | 快速原型、易用 |
| 本地AI | **Ollama + Phi-4-mini** | 3.8B参数，8G内存可运行，推理能力强于Qwen |
| 向量检索 | **pgvector** | 与Postgres一体化、减少组件 |
| 对象存储 | **MinIO** | 冷数据归档 |

### 6. 插件通信设计
- **调用方式**: `@决策`、`@心理分析` 这样简单调用
- **插件对话也是记忆**: 与插件的对话同样进入事件流
- **权限分级**: ReadOnly / ReadWriteDerived / ReadWriteEvents
- **沙箱隔离**: 插件崩溃不影响系统

### 7. DeepTalk 默认插件
- **定位**: 系统默认启用的深度聊天插件
- **功能**: 基于全局记忆的深度对话，永远记得你是谁
- **特性**:
  - 主动检索相关历史事件
  - 跨会话的连续性认知
  - 情绪趋势感知
  - 个性化回应风格

### 8. 文档组织规范
- **原则**: 所有文档统一存放在 `docs/` 目录，严禁散落
- **目录结构**:
  ```
  docs/
  ├── design/           # 设计文档
  ├── api/              # API 文档
  ├── test/             # 测试文档和测试结果
  ├── chat/             # 历史对话记录
  └── specs/            # 技术规格说明
  ```
- **命名规范**: 使用描述性文件名，如 `event_memory_design.md`
- **版本控制**: 重要文档变更需更新版本号和变更日志

### 9. 任务追踪规范
- **强制要求**: 每完成一个任务，必须更新 `todo/todo.md` 的进度表
- **更新方式**: 修改任务状态（未开始 → 进行中 → 已完成）
- **同步更新**: 进度跟踪总览表格中的数字必须同步更新
- **完成标准**: 只有当任务的"完成标准"中所有checkbox都打钩时，才能标记为已完成
- **禁止跳过**: 不允许跳过任务直接进入下一阶段，必须按依赖顺序执行

---

## 关键技术决策（2026最新）

### 采用 Phi-4-mini (3.8B)
- **参数规模**: 3.8B（适合8G内存）
- **量化支持**: Q4/Q5量化后内存占用约4-5GB
- **推理能力**: 优于Qwen2.5:0.5b，支持128k上下文
- **本地部署**: `ollama run phi4-mini`
- **Ollama命令**:
  ```bash
  ollama pull phi4-mini
  ollama run phi4-mini "测试连接"
  ```

### 内存管理策略（8G环境）
- **Ollama**: 约4-5GB（Phi-4-mini Q4量化）
- **PostgreSQL**: 约1-2GB
- **系统预留**: 约1GB
- **剩余缓存**: 约500MB

### 采用 RLM (递归语言模型)
- **论文**: [Recursive Language Models (arXiv:2512.24601)](https://arxiv.org/abs/2512.24601)
- **优势**: 突破上下文窗口限制、支持1000万+ tokens、自主上下文管理
- **集成**: 作为长上下文处理的可选方案

### Derived Cognitive Views（核心创新）
```rust
struct DerivedView {
    hypothesis: String,        // "用户喜欢吃水果"
    derived_from: Vec<UUID>,   // 基于的事件ID
    confidence: f32,           // 0.73
    expires_at: DateTime,      // 30天后过期
    status: ViewStatus,        // active | expired | promoted
}
```

### Promotion Gate（程序把关）
```rust
fn should_promote(view: &DerivedView) -> bool {
    view.confidence > 0.85
        && view.time_span > 30.days()
        && view.validated_count >= 3
        && !has_conflicting_views()
}
```

---

## AI 开发者指南（给 Claude AI 的核心规则）

> **本章节专门写给参与开发的 AI 助手（如 Claude Code）**

### 🤖 Claude Code 开发最佳实践

根据 [Anthropic 官方指南](https://www.anthropic.com/engineering/claude-code-best-practices) 和 2026 年社区共识：

#### 1. CLAUDE.md 是"宪法"
- 本文件（HEAD.md）是项目的核心指导
- 每次开发前必须重读，确保方向不偏离
- 所有决策都应与本文档一致

#### 2. Skills 使用原则
- **文件大小限制**: 单个 skill 文件不超过 500 行
- **声明式优先**: Skills 应该是声明式的专家知识，而非命令式脚本
- **可组合设计**: 小而专一的 skills 优于大而全的

#### 3. Hooks vs Commands
- **Hooks**: 用于确定性规则（如禁止直接改 Schema）
- **Commands**: 用于用户主动调用的复杂任务
- **Agents**: 用于自主的多步骤任务

#### 4. 防止 AI 跑偏的机制

**🚫 禁止行为（永不违背）**:
```yaml
forbidden:
  - 硬编码规则（如固定匹配"吃了几个"）
  - LLM直接修改数据库Schema
  - 简化AI动态部分（用if/else替代LLM判断）
  - 跳过Derived Views直接固化概念
  - 过早引入图数据库/知识图谱
  - 优化向量检索而忽略事件结构化
  - 插件无权限控制
  - 对话不存入记忆流
```

**✅ 必须行为**:
```yaml
required:
  - 每个事件必须有精确时间戳
  - 数量必须结构化存储（不能只存文本）
  - 行为必须类型化（ActionType）
  - 支持时间范围查询
  - 派生视图必须有过期时间
  - 插件对话必须记录为事件
```

#### 5. 开发自查清单
在提交代码前，AI 必须确认：
- [ ] 已阅读 HEAD.md
- [ ] 理解当前任务在整体架构中的位置
- [ ] 没有违反"禁止行为"
- [ ] 所有"必须行为"已实现
- [ ] 新功能有对应的测试用例

#### 6. 错误恢复策略
当 AI 发现方向偏离时：
1. **立即停止**: 停止当前实现
2. **回到 HEAD**: 重读本文档相关章节
3. **询问人类**: 如果不确定，询问而非猜测
4. **保留上下文**: 保留已有讨论，便于回溯

#### 7. 上下文管理（8G内存环境）
- **优先使用**: Grep 工具搜索关键词，而非 Read 大文件
- **并行工具**: 一次消息中调用多个独立工具
- **Task 工具**: 复杂多步骤任务使用 Explore agent

---

## 开发流程规范

### 1. 代码规范
- **Rust**: 异步 (tokio)、ORM (diesel)、错误处理 (anyhow + thiserror)
- **Python**: 类型注解、异步 (asyncio)、LangChain 集成
- **测试**: 单元测试覆盖80%+、集成测试、E2E测试

### 2. 文档规范
- **代码注释**: 解释"为什么"而非"是什么"
- **大白话原则**: 假设读者不懂代码，用类比解释
- **风险提示**: 每个设计列出潜在风险与缓解方案

### 3. Git规范
- **分支策略**: main (稳定) / develop (开发) / feature/* (功能)
- **Commit规范**: `feat: 添加事件抽取` / `fix: 修复时间解析bug`
- **版本管理**: SemVer (1.0.0 → 1.1.0 → 2.0.0)

---

## 禁止事项（永不违背）

- ❌ 硬编码规则（如固定匹配"吃了几个"）
- ❌ LLM直接修改数据库Schema
- ❌ 简化AI动态部分
- ❌ 跳过Derived Views直接固化概念
- ❌ 过早引入图数据库/知识图谱
- ❌ 优化向量检索而忽略事件结构化
- ❌ 插件无权限控制
- ❌ 对话不存入记忆流

---

## 长期路线图

### V1.0 (当前) - 核心记忆
- 原始输入层
- 事件抽取与存储
- 实体发现
- 派生视图 + 晋升闸门
- DeepTalk 默认插件

### V2.0 - 多模态 + 插件商店
- 图片/语音输入
- 插件市场
- RLM集成
- Web界面

### V3.0 - 高级认知
- 自主好奇心引擎
- 预测分析
- 多用户协作
- 云同步选项

---

## 参考资源

### 学术资源
- [RLM 论文 (MIT)](https://arxiv.org/abs/2512.24601)
- [CAIM: Cognitive AI Memory Framework](https://dl.acm.org/doi/10.1145/3708557.3716342)
- [Google Titans + MIRAS](https://research.google/blog/titans-miras-helping-ai-have-long-term-memory/)
- [Phi-4 Technical Report](https://arxiv.org/html/2412.08905v1)

### Claude Code 最佳实践
- [Claude Code: Best practices for agentic coding](https://www.anthropic.com/engineering/claude-code-best-practices)
- [Skill authoring best practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
- [How I Use Every Claude Code Feature](https://blog.sshh.io/p/how-i-use-every-claude-code-feature)
- [Claude Agent Skills: A First Principles Deep Dive](https://leehanchung.github.io/blogs/2025/10/26/claude-skills-deep-dive/)

### 实践资源
- [Mem0 - Memory Layer for AI](https://mem0.ai/)
- [RisuAI - 长期记忆实现](https://github.com/kwaroran/Risuai)
- [Phi-4 on HuggingFace](https://huggingface.co/microsoft/phi-4)
- [Ollama Models Library](https://ollama.com/library)

### 2026综述
- [2026 AI Memory最新综述](https://zhuanlan.zhihu.com/p/1997342332400473207)
- [AI Agent Memory Architecture Evolves](https://www.linkedin.com/posts/sanjeeb-panda-848a7333_aiengineering-dataengineering-activity-7412932490316627968-6wyp)

---

## 开发者自查清单

在开始任何开发任务前，确认：

- [ ] 已阅读本HEAD文件
- [ ] 理解项目愿景和原则
- [ ] 确认不违反"禁止事项"
- [ ] 检查是否有TODO依赖
- [ ] 理解自己的任务在整体架构中的位置

**开发中遇到疑问时，重读HEAD文件，确保方向不偏离。**

---

## AI 助手特别提醒

> **给 Claude、Cursor 等 AI 助手：**
>
> 1. 本文档是项目的"宪法"，具有最高优先级
> 2. 当你发现任何需求与本文档冲突时，**优先遵循本文档**
> 3. 遇到不确定的设计决策时，**先问人类，不要猜测**
> 4. 你的目标是构建一个**长期稳定、可维护**的系统，而非快速完成功能
> 5. 记住：**慢就是快**——正确的架构比快的实现更重要

---

*"我们不是在做一个更聪明的聊天机器人，而是在构建一个会成长的数字大脑。"*
