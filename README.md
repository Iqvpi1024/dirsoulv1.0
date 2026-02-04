<div align="center">

# 🧠 DirSoul

### **Give your Local LLM Long-term Memory.** AI-Native permanent memory framework.

[![Rust](https://img.shields.io/badge/Rust-1.70-orange)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.12-blue)](https://www.python.org)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/iqvpi1024/dirsoulv1.0?style=social)](https://github.com/iqvpi1024/dirsoulv1.0/stargazers)
[![Sponsor](https://img.shields.io/badge/GitHub-Sponsor-FF69B4?logo=githubsponsors&style=social)](https://github.com/sponsors/iqvpi1024)

**"26岁运营，0代码基础，用SLM给本地LLM装上了永久记忆。"**

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

---

## 🔒 本地私有化部署 - 你的数据，你的掌控

**为什么选择本地私有化部署？**

```
❌ ChatGPT/Claude (云端AI):
   - 你的对话存储在他们的服务器
   - 他们可以用来训练模型
   - 隐私政策随时可能变
   - 需要网络连接
   - 每月订阅费用

✅ DirSoul (本地私有化):
   - 所有数据存储在你自己的服务器
   - 你拥有完全控制权
   - 端到端加密，即使数据库被盗也无法读取
   - 离线也能用
   - 一次部署，终身免费
```

### 🏠 完全私有化的优势

| 特性 | 说明 |
|------|------|
| **🔐 零云依赖** | 所有数据存储在你自己的服务器 |
| **🔒 端到端加密** | Fernet加密，即使数据库被盗也安全 |
| **📡 无需联网** | 离线环境也能用 |
| **👤 完全匿名** | 不上传任何用户信息 |
| **💰 成本固定** | 一次部署，终身免费 |
| **🎯 数据主权** | 你是数据的唯一主人 |

### 🏢 企业场景

**适合这些场景：**
- 📋 **内部知识管理** - 公司文档、决策记录
- 🏥 **医疗/法律** - 客户记录、案例库（敏感数据）
- 💼 **个人助理** - 日记、想法、项目笔记
- 🔬 **研究笔记** - 实验记录、文献阅读
- 🎨 **创意工作** - 灵感收集、素材管理

**部署方式：**
```bash
# 1. 本地部署 (单机)
docker-compose up -d

# 2. 局域网部署 (NAS/家庭服务器)
# 修改 docker-compose.yml 中的端口映射
ports:
  - "8080:8080"  # 内网访问

# 3. 离线环境
# 完全不需要网络，所有AI本地运行
```

### 🎬 Visual Impact (V2.0 规划中)

**"Graph Porn" - 知识图谱可视化** 🔜 *Coming Soon*

> **这是DirSoul V2.0的核心功能，目前正在开发中**

```
你 (中心节点)
  ├── 初恋 (人名) ───> 2019年夏天 (时间) ──> 分手 (事件)
  ├── 马自达CX-5 (车) ──> 操控好 (属性) ──> 油耗纠结 (情绪)
  └── 电商运营 (职业) ──> 26岁 (年龄) ──> 想转行 (目标)
```

**未来功能展示：**
- 🔍 当你搜索"前任"时，所有相关节点瞬间亮起
- 📊 右侧自动生成3年时间线
- 🎨 动态星空图（D3.js/Echarts实现）
- 🧠 实时知识演化动画

**当前V1.0已实现：**
- ✅ 后端：实体链接、关系抽取（PostgreSQL存储）
- ✅ API：查询所有实体和关系
- ⏳ 前端：图谱可视化（V2.0开发中）

**技术栈（规划）：**
- D3.js 或 Echarts - 图谱渲染
- WebGPU - 大规模数据优化（10000+节点）
- Force-directed graph - 力导向布局
- 实时更新 - WebSocket推送

**想提前体验？**
- 查看 [docs/PROMOTION.md](docs/PROMOTION.md) 了解参与开发

---

## 🚀 Quick Start

### Docker (Recommended)

```bash
git clone https://github.com/iqvpi1024/dirsoulv1.0.git
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

## 💰 Sponsor

**觉得 DirSoul 有用？考虑赞助支持！**

<div align="center">

### GitHub Sponsors
[![Sponsor](https://img.shields.io/badge/GitHub-Sponsor-FF69B4?logo=githubsponsors&style=social)](https://github.com/sponsors/iqvpi1024)

### 扫码赞助
<img src="docs/assets/wechat-pay.jpg" alt="微信赞赏" width="150"/>
<img src="docs/assets/alipay.jpg" alt="支付宝" width="150"/>

**任意金额，感谢支持！** 🙏

### 详见
[docs/Sponsors.md](docs/Sponsors.md) - 赞助档位、回报、企业合作

</div>

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

### V2.0 (Q2 2026) - 知识图谱可视化
- ⏳ **Graph visualization** (Echarts/D3.js) - 核心功能
  - 节点：人名、地点、事件、情绪
  - 边：关系强度、时间流向
  - 搜索：实时高亮相关节点
  - 时间线：自动生成事件时间线
- ⏳ Telegram Bot integration
- ⏳ Mobile app (Tauri)
- ⏳ Multi-user support

**技术方案：**
```rust
// 后端已有实体链接
Entity {
    name: "赵杰"
    type: Person
    attributes: { age: 26, job: "电商运营" }
    relations: [
        { target: "马自达CX-5", type: "想买", strength: 0.8 },
        { target: "初恋", type: "前任", strength: 0.9 }
    ]
}

// V2 前端渲染
// 使用 D3.js force-directed graph
```

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

- **GitHub Issues**: [Bug reports & feature requests](https://github.com/iqvpi1024/dirsoulv1.0/issues)
- **Discussions**: [Q&A & show-and-tell](https://github.com/iqvpi1024/dirsoulv1.0/discussions)
- **Twitter**: [@iqvpi1024](https://twitter.com/iqvpi1024)

---

## 💰 Sponsor

**觉得 DirSoul 有用？考虑赞助支持！**

- **GitHub Sponsors**: [https://github.com/sponsors/iqvpi1024](https://github.com/sponsors/iqvpi1024)
- **详见**: [docs/Sponsors.md](docs/Sponsors.md)

您的赞助将帮助：
- 💻 服务器维护
- 🧠 AI模型优化
- 📚 文档完善
- 🚀 新功能开发

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

[![Star History Chart](https://api.star-history.com/svg?repos=iqvpi1024/dirsoulv1.0&type=Date)](https://star-history.com/#iqvpi1024/dirsoulv1.0&Date)

---

<div align="center">

**"We're not building a smarter chatbot. We're building a digital brain that grows."**

**Made with ❤️ by [Jie Zhao](https://github.com/iqvpi1024)**

*26岁电商运营 → 0代码基础 → Rust开发者 → AI时代的钢铁侠*

</div>
