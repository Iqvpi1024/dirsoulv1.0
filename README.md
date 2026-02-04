<div align="center">

# 🧠 DirSoul

### **Give your AI a Soul.** The missing long-term memory layer for local LLMs.

[![Rust](https://img.shields.io/badge/Rust-1.70-orange)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.12-blue)](https://www.python.org)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/Iqvpi1024/dirsoulv1.0?style=social)](https://github.com/Iqvpi1024/dirsoulv1.0)

**"我把 DeepSeek 聊崩了，但我把这 3 个月的记忆变成了一片星空。"**

</div>

---

## 📖 Story

> "26岁电商运营，0代码基础，用Claude手搓了一个Rust数据库，给本地LLM装上了海马体。"

### 🎯 The Problem

**DeepSeek-R1 很强，但它只有 7 秒记忆。**

- ❌ 关掉窗口 → 它忘了你是谁
- ❌ 问"我上周说想买啥车？" → "抱歉我不知道"
- ❌ 你的 3 年聊天记录 → 散落在几十个log文件里

**现有"记忆"方案的问题：**
- 🤖 **ChatGPT/Claude**: 数据在云端，隐私裸奔
- 💾 **MemGPT/Mem0**: 太复杂，需要云API key
- 📓 **Obsidian/Roam**: 手动记录，AI无法理解
- 🔧 **RAG框架**: 技术门槛高，普通用户玩不动

### ✨ The Solution

**DirSoul = 给本地LLM插上一根 10TB 的内存条**

- ✅ **完全本地运行** - 隐私优先，零云依赖
- ✅ **AI-Native设计** - 无硬编码规则，SLM主导
- ✅ **持久化记忆** - 事件+实体+关系，10年+不崩溃
- ✅ **一键Docker部署** - 8GB内存即可运行
- ✅ **插件化扩展** - DeepTalk深度对话，决策分析

### 🎬 Visual Impact

**"Graph Porn" - 知识图谱可视化**

```
你 (中心节点)
  ├── 初恋 (人名) ───> 2019年夏天 (时间) ──> 分手 (事件)
  ├── 马自达CX-5 (车) ──> 操控好 (属性) ──> 油耗纠结 (情绪)
  └── 电商运营 (职业) ──> 26岁 (年龄) ──> 想转行 (目标)
```

> **当你搜索"前任"时，所有相关节点瞬间亮起，右侧自动生成3年时间线。**

---

## 🚀 Quick Start

### Docker (Recommended)

```bash
git clone https://github.com/Iqvpi1024/dirsoulv1.0.git
cd dirsoulv1.0
docker-compose up -d
```

Open http://localhost:8501 and start chatting.

### Manual Install

```bash
# Install Ollama & qwen2:0.5b
curl -fsSL https://ollama.com/install.sh | sh
ollama pull qwen2:0.5b

# Setup PostgreSQL
sudo apt install postgresql-16
sudo -u postgres createdb dirsoul

# Run DirSoul
cargo build --release
./target/release/dirsoul

# Run Streamlit UI
cd src/python/streamlit
pip install -r requirements.txt
streamlit run app.py
```

---

## 🏗️ Architecture

### Layered Memory System

```
Layer 4: Agent & Plugins
   └─ DeepTalk (深度对话) | Decision (决策分析) | Psych (心理分析)

Layer 3: Cognitive Memory (认知记忆)
   └─ Derived Views (派生视图) | Stable Concepts (稳定概念)
   └─ Promotion Gate (晋升把关) | Versioning (版本控制)

Layer 2: Structured Memory (结构化记忆)
   └─ Events (事件) | Entities (实体) | Relations (关系)
   └─ Vector Index (向量索引) | Full-text Search (全文搜索)

Layer 1: Raw Memory (原始记忆)
   └─ Append-only Log (只追加日志) | Immutable (不可变)
   └─ Encrypted Storage (加密存储)
```

### Tech Stack

| Component | Tech | Why |
|-----------|------|-----|
| **Core Engine** | Rust | Memory safety, 8GB RAM friendly |
| **UI** | Python/Streamlit | Rapid prototyping, 2026 Dark Glassmorphism |
| **Database** | PostgreSQL 16+ | JSONB, partitioning, pgvector |
| **Local AI** | Ollama + qwen2:0.5b | 352MB, fast, privacy-first |
| **Vector Search** | pgvector | Integrated with Postgres |
| **Container** | Docker | One-click deployment |

---

## 💡 Use Cases

### Before vs After

| Scenario | DeepSeek Alone | DeepSeek + DirSoul |
|----------|----------------|-------------------|
| "我叫什么？" | "我不知道" | "你是赵杰，26岁，电商运营" |
| "我去年想买啥车？" | "抱歉，没有上下文" | "你去年11月提到马自达CX-5，因为操控好，但你在纠结油耗" |
| "我明年多大？" | "我不知道你的年龄" | "你今年26岁，明年27岁" |

### Real Demo

```
User: 我叫赵杰
AI: 好的赵杰，我记住了。

User: 我今年26岁，是一名电商运营
AI: 记住了，26岁电商运营。

User: 我明年多大？
AI: 你明年27岁。

User: 我叫什么？
AI: 你叫赵杰。
```

> **完全本地运行，无API key，无云依赖。**

---

## 🎨 Features

### ✅ AI-Native Design
- No hardcoded rules
- SLM (qwen2:0.5b) does all understanding
- Learns from experience, not rote memorization

### ✅ Privacy First
- End-to-end encryption (Fernet)
- Zero cloud dependency
- All data stored locally

### ✅ Plugin System
- **DeepTalk** - Deep conversation with global memory
- **Decision** - Multi-criteria decision analysis
- **Psych** - Emotional trend analysis

### ✅ 2026 Dark Glassmorphism UI
- Modern dark theme
- Glassmorphism effects
- Bento Box layout
- Micro-animations

---

## 📚 Documentation

- [CLAUDE.md](CLAUDE.md) - AI Developer Configuration
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment Guide
- [docs/skills/](docs/skills/) - 20+ Technical Skills

---

## 🛣️ Roadmap

### V1.0 (Current)
- ✅ Event memory with qwen2:0.5b
- ✅ Entity linking & relation extraction
- ✅ Cognitive view generation
- ✅ DeepTalk plugin
- ✅ Streamlit UI
- ✅ Docker deployment

### V2.0 (Q2 2026)
- ⏳ Graph visualization (Echarts/D3.js)
- ⏳ Telegram Bot integration
- ⏳ Mobile app (Tauri)
- ⏳ Multi-user support

### V3.0 (Q4 2026)
- ⏳ Federated learning
- ⏳ Plugin marketplace
- ⏳ Cloud sync (encrypted)

---

## 🤝 Contributing

**We need your help!**

- 🐛 Bug reports
- 💡 Feature requests
- 📖 Documentation improvements
- 🧪 Test cases

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## 💬 Community

- **GitHub Issues**: [Bug reports & feature requests](https://github.com/Iqvpi1024/dirsoulv1.0/issues)
- **Discussions**: [Q&A & show-and-tell](https://github.com/Iqvpi1024/dirsoulv1.0/discussions)
- **Twitter**: [@Iqvpi1024](https://twitter.com/Iqvpi1024)

---

## 📄 License

MIT License - 详见 [LICENSE](LICENSE)

**Free for personal & commercial use.**

---

## 🙏 Acknowledgments

This project stands on the shoulders of giants:

- [Recursive Language Models (MIT)](https://arxiv.org/abs/2512.24601) - Theoretical foundation
- [Google Titans + MIRAS](https://research.google/blog/titans-miras-helping-ai-have-long-term-memory/) - Neural memory architecture
- [Mem0](https://mem0.ai/) - Inspiration for memory management
- [RisuAI](https://github.com/kwaroran/Risuai) - AI-native design principles

---

## 🌟 Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Iqvpi1024/dirsoulv1.0&type=Date)](https://star-history.com/#Iqvpi1024/dirsoulv1.0&Date)

---

<div align="center">

**"We're not building a smarter chatbot. We're building a digital brain that grows."**

**Made with ❤️ by [Jie Zhao](https://github.com/Iqvpi1024)**

*26岁电商运营 → 0代码基础 → Rust开发者 → AI时代的钢铁侠*

</div>
