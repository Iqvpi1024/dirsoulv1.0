# DirSoul

> **本地优先的永久记忆框架** - 构建你的数字大脑

DirSoul 是一个**本地优先、隐私优先的永久记忆框架**，支持：

- 记录用户的一切交互（文本、语音、图片、文档）
- AI驱动的动态事件抽取与认知演化
- 插件化扩展（决策分析、心理分析等）
- 10年+ 数据增长不崩溃
- 类人认知能力：事件记忆 → 模式识别 → 概念形成 → 自主好奇

## 项目定位

**不是一个聊天机器人，而是一个数字大脑：拥有情节记忆、能从经历中学习、支持插件扩展的认知操作系统。**

## 核心特性

### AI-Native 与动态性
- 拒绝硬编码规则，使用 Phi-4-mini 本地推理
- 系统从经验学习，而非死记硬背

### 分层架构
```
Layer 1: 原始记忆 (Raw Memory)   - Append-only，全量保留，永不修改
Layer 2: 结构化记忆 (Structured)  - 事件/实体，带向量索引，可重算
Layer 3: 认知记忆 (Cognitive)     - 派生视图/稳定概念，可过期/版本化
上层:    Agent 与 插件             - 读写记忆，权限控制，沙箱隔离
```

### 慢抽象原则（2026共识）
- **Derived Views 先行**：生成可丢弃的认知假设
- **Promotion Gate 把关**：程序判定是否晋升为稳定概念
- 避免 LLM 幻觉放大

### DeepTalk 默认插件
- 基于全局记忆的深度对话
- 跨会话连续性认知
- 情绪趋势感知

## 技术栈

| 组件 | 技术 | 理由 |
|------|------|------|
| 核心逻辑 | **Rust** | 内存安全、高性能、并发安全 |
| 数据库 | **PostgreSQL 16+** | 支持分区、JSONB、pgvector |
| 界面 | **Python + Streamlit** | 快速原型、易用 |
| 本地AI | **Ollama + Phi-4-mini** | 3.8B参数，8G内存可运行 |
| 向量检索 | **pgvector** | 与Postgres一体化 |
| 对象存储 | **MinIO** | 冷数据归档 |

## 开发状态

> 当前版本：V2.1 (2026 整合版 + Phi-4-mini优化)

详见 [todo/todo.md](todo/todo.md) 查看完整任务列表。

## 快速开始

### 环境要求
- 8GB RAM
- Rust 1.75+
- PostgreSQL 16+
- Python 3.12+
- Ollama

### 安装

```bash
# 克隆仓库
git clone https://github.com/yourusername/dirsoulv1.git
cd dirsoulv1

# 安装 Phi-4-mini 模型
ollama pull phi4-mini

# 运行（开发中）
cargo run
```

## 文档

- [HEAD.md](todo/head.md) - 项目开发指南（给 AI 开发者）
- [TODO.md](todo/todo.md) - 完整开发任务列表
- [docs/](docs/) - 设计文档、API文档、测试结果等

## 许可证

MIT License - 详见 [LICENSE](LICENSE)

## 致谢

本项目参考了以下前沿研究：
- [Recursive Language Models (MIT)](https://arxiv.org/abs/2512.24601)
- [Google Titans + MIRAS](https://research.google/blog/titans-miras-helping-ai-have-long-term-memory/)
- [Mem0](https://mem0.ai/)
- [RisuAI](https://github.com/kwaroran/Risuai)

---

*"我们不是在做一个更聪明的聊天机器人，而是在构建一个会成长的数字大脑。"*
