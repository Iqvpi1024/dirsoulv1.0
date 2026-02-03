这是一个基于所有历史对话、设计文件以及2026年最新AI趋势生成的全新、详尽的 **DirSoul V1 开发手册**。

这份手册假定这是一个**全新项目**，抛弃旧代码包袱，直接采用最前沿的**“本地隐私记忆 + 混合认知架构”**。

---

# DirSoul 项目开发手册（V1.0 - 2026 旗舰版）

**版本**：V1.0 (New Genesis)
**架构师**：Grok & You
**核心理念**：隐私优先的数字灵魂，具备一生无限记忆、认知演化能力与决策辅助系统。

---

## 目录
1.  **P0: 项目愿景与核心哲学**
2.  **P1: 2026 黄金技术栈**
3.  **P2: 认知架构设计 (The Brain Anatomy)**
4.  **P3: 核心记忆层 (Hippocampus & Storage)**
5.  **P4: 智能与深度沟通 (DeepTalk & SLM)**
6.  **P5: 插件与 SaaS 商店 (Prefrontal Cortex)**
7.  **P6: 交互界面 (ChatGPT-Style UI)**
8.  **P7: 安全与隐私 (The Immune System)**
9.  **P8: 开发路线图 (MVP Execution)**

---

## P0: 项目愿景与核心哲学

### 0.1 什么是 DirSoul？
DirSoul 是一个本地部署的“个人数字大脑”。它不同于传统的聊天机器人（阅后即焚）或大厂AI（云端隐私黑洞）。DirSoul 的核心是**“可计算的经历”**——它记录你的一生，从琐碎的早餐到重大的人生决策，并随时间自动演化出对你的深度理解。

### 0.2 核心原则 (The First Principles)
1.  **隐私绝对主权**：数据 100% 本地存储（Local First），端到端加密。云端仅用于插件授权，绝不触碰记忆数据。
2.  **无限记忆 (Infinite Context)**：突破 Context Window 限制。通过**分层存储**（热/温/冷）与**动态压缩**，支持数十年的数据积累而不崩溃。
3.  **慢抽象 (Slow Abstraction)**：拒绝 AI 随意修改数据库结构。采用 **事件(Fact) -> 视图(Hypothesis) -> 固化(Schema)** 的三层缓冲机制，防止幻觉污染长期记忆。
4.  **深度沟通 (DeepTalk)**：默认模型不仅仅是聊天，而是基于全局记忆的深度洞察。它永远记得你是谁，哪怕是十年前的对话。

---

## P1: 2026 黄金技术栈

为了确保项目在未来 5-10 年不落伍，我们采用**高性能与低代码混合**的策略：

*   **核心逻辑 (Rust)**：内存安全、并发极高。用于处理记忆写入、加密、分层压缩等“重”任务。
*   **数据库 (Postgres + pgvector)**：比 SQLite 更强大，支持 JSONB（灵活结构）和向量检索，支持未来亿级数据分区。
*   **AI 引擎 (Ollama + Phi-3-mini)**：本地运行 SLM（小语言模型）。Phi-3-mini (3.8B) 在逻辑推理上优于 Qwen0.5B，适合深度分析。通过 **Unsloth** 进行微调。
*   **胶水层 (Python 3.12)**：用于 UI (Streamlit)、Agent 编排 (LangChain) 和快速原型开发。
*   **SaaS 后端 (Node.js)**：轻量级云端服务，处理 Stripe 支付和插件密钥分发。

---

## P2: 认知架构设计 (The Brain Anatomy)

我们将系统映射为人类大脑结构，确保模块化：

| 脑区 | 模块名称 | 功能描述 | 数据流向 |
| :--- | :--- | :--- | :--- |
| **感官皮层** | **Input Gateway** | 接收文本、梦境、命理数据，标准化为 Raw Input。 | 用户 -> 记忆层 |
| **海马体** | **Core Memory** | **事件提取**与**短期存储**。将非结构化对话转为结构化 Event。 | 感官 -> 数据库 |
| **大脑皮层** | **Cognitive Layer** | **派生视图**与**长期固化**。从事件中发现模式（如“财务压力”），生成假设。 | 记忆层 -> 认知表 |
| **前额叶** | **Agent/Plugin** | **决策与执行**。调用记忆进行分析（如“买车建议”），运行外部插件。 | 认知层 -> 用户 |
| **脑干** | **System Core** | 存储分层、加密安全、生命周期管理。 | 底层支撑 |

---

## P3: 核心记忆层 (Hippocampus & Storage)

这是 DirSoul 的心脏。必须严格区分**事实**与**观点**。

### 3.1 数据库 Schema (Postgres)
*不要手动建表，使用 Diesel (Rust ORM) 迁移。*

#### A. 事件表 (Immutable Facts)
*原则：Append-Only，永不修改。记录发生了什么。*
```sql
CREATE TABLE events (
    event_id UUID PRIMARY KEY,
    user_id TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL, -- 精确时间
    action TEXT NOT NULL,           -- 行为: "eat", "buy", "dream"
    object TEXT,                    -- 对象: "apple", "car", "flying"
    quantity FLOAT,                 -- 数值: 2.0, 70000.0
    unit TEXT,                      -- 单位: "个", "CNY"
    source_text TEXT,               -- 原始对话
    embedding VECTOR(1536),         -- 语义向量 (用于模糊搜索)
    is_encrypted BOOLEAN DEFAULT TRUE
);
-- 分区策略：按月分区，旧数据自动移入冷存储
```

#### B. 派生视图表 (Temporary Hypotheses)
*原则：可丢弃、可过期。记录 AI 的“猜测”。*
```sql
CREATE TABLE derived_views (
    view_id UUID PRIMARY KEY,
    hypothesis TEXT,                -- 假设: "用户财务压力大"
    confidence FLOAT,               -- 置信度: 0.85
    derived_from UUID[],            -- 来源事件ID数组
    created_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,         -- 过期时间 (默认30天)
    view_type TEXT                  -- 类型: "pattern", "personality"
);
```

### 3.2 存储分层策略 (Infinite Memory)
为了实现“一生记忆”，我们不能把所有数据都放在内存/热库中。

1.  **热存储 (Hot)**：最近 3 个月的数据。存放在 Postgres SSD 分区，全索引，毫秒级响应。
2.  **温存储 (Warm)**：3个月 - 2年。存放在本地大容量磁盘。索引减少。
3.  **冷存储 (Cold)**：2年以上。
    *   **动作**：使用 Rust 的 `zlib` 压缩原始事件。
    *   **摘要**：触发 SLM 生成“年度摘要”（如“2024年主要在还债，情绪焦虑”），存入 `derived_views`。
    *   **归档**：原始数据导出为加密 JSON 文件备份。

---

## P4: 智能与深度沟通 (DeepTalk & SLM)

### 4.1 模型选择：Phi-3-mini (3.8B)
我们放弃 Qwen0.5B，因为它在逻辑推理上太弱。Phi-3-mini 是目前端侧最强模型，支持 128k 上下文，足以处理长记忆聚合。

### 4.2 微调 (Unsloth Fine-tuning)
我们不使用通用模型，而是训练一个专门的 **DeepTalk** 版本。
*   **工具**：Unsloth (在 Colab 上免费运行)。
*   **数据集**：构建 100-500 条“记忆-洞察”对。
    *   *Input*: `[历史事件: 负债7万, 月薪6k] + [用户: 我想买车]`
    *   *Output*: `杰，基于你目前的财务状况（负债是月薪的10倍以上），现在买车风险极高。建议先制定还款计划。`
*   **目标**：让模型学会**主动调用记忆**，而不是泛泛而谈。

### 4.3 全局记忆检索 (Global Recall)
为了解决“新对话失忆”问题：
1.  用户输入时，系统并行执行**混合检索**（向量搜索语义 + SQL搜索时间/实体）。
2.  提取 Top-K 相关事件 + 相关的有效 `derived_views`。
3.  将这些作为 `System Prompt` 的一部分注入当前上下文。
    *   *Prompt*: `你叫DeepTalk。以下是用户的主动记忆：{memories}。请基于此回答，不要表现得像第一次见面。`

---

## P5: 插件与 SaaS 商店 (Prefrontal Cortex)

### 5.1 架构：本地执行，云端授权
插件是 DirSoul 的变现核心。
*   **商店 (Cloud)**：Node.js + Stripe。只负责收钱、管理订阅、发放 JWT 密钥。
*   **运行时 (Local)**：Rust 负责沙箱环境。用户输入密钥 -> 解锁本地插件功能。

### 5.2 核心插件 (MVP)
1.  **认知分析 Agent (Cognitive)**：
    *   *功能*：后台扫描 `events`，寻找重复模式（如“每周五晚情绪低落”）。
    *   *输出*：生成 `derived_views`。
2.  **决策辅助 Agent (Decision)**：
    *   *功能*：当用户提问包含“买”、“去”、“做”时触发。
    *   *逻辑*：检索相关历史（财务、健康、时间） -> SLM 模拟后果 -> 给出建议。

### 5.3 扩展插件 (V2 - 探索性)
*写在文档中作为未来规划，吸引深度用户。*
*   **梦境解析**：输入 `type='dream'` 的事件，关联现实压力。
*   **数字命理**：结合生辰与历史运势周期（SLM 匹配）。
*   **未来概率预测**：基于贝叶斯概率 + SLM，预测行为后果（如“买车后违约概率 70%”）。

---

## P6: 交互界面 (ChatGPT-Style UI)

放弃复杂的仪表盘，回归最自然的对话。

### 6.1 设计风格
*   **工具**：Streamlit (极简、响应式)。
*   **布局**：
    *   中央：纯净的聊天流（`st.chat_message`）。
    *   侧边栏（默认折叠）：
        *   **记忆时间线**：可视化图表（Plotly），展示“人生大事记”。
        *   **插件状态**：显示已激活的 DeepTalk/决策插件。
        *   **隐私仪表盘**：显示“数据已加密”、“本地存储中”。

### 6.2 交互逻辑
*   **输入**：支持文本。预留文件上传按钮（灰度）。
*   **反馈**：AI 回答后，底部带有小字的“记忆来源”（如：*基于2024年3条财务记录分析*），点击可展开查看原始事件。

---

## P7: 安全与隐私 (The Immune System)

### 7.1 零信任架构
*   **加密**：使用 `Fernet` (对称加密) 存储所有 `content` 和 `embedding`。密钥由用户密码派生，系统不存明文密钥。
*   **审计日志**：每一次记忆的读取（无论是你还是插件），都必须写入不可篡改的 `audit_logs` 表（Postgres）。
*   **RBAC**：插件只有在用户明确授权（如“允许读取财务数据”）后，才能获得解密后的数据句柄。

---

## P8: 开发路线图 (MVP Execution)

**总耗时估算**：4-6 周。
**辅助工具**：Claude Code (负责代码生成), Unsloth (负责模型微调)。

| 阶段 | 任务 (Prompt 关键词) | 产出物 |
| :--- | :--- | :--- |
| **Phase 1** <br> (1周) | **基础设施** <br> `Setup Rust/Postgres/Python env`, `Init Git` | Docker Compose 文件 <br> 数据库初始化 SQL |
| **Phase 2** <br> (2周) | **记忆核心** <br> `Implement Event Table`, `Fine-tune Phi-3 for Extraction`, `Rust Insert Logic` | 事件提取准确率 > 90% <br> 数据库成功写入加密数据 |
| **Phase 3** <br> (1周) | **智能层** <br> `Implement Global Recall`, `Integrate DeepTalk Model` | 聊天 Demo：新对话能叫出名字，记得历史 |
| **Phase 4** <br> (1周) | **插件与UI** <br> `Build SaaS Mock Server`, `Streamlit Chat UI`, `Decision Plugin Logic` | 完整的本地界面 <br> 模拟购买插件流程 |
| **Phase 5** <br> (1周) | **发布准备** <br> `Security Audit`, `Write README`, `Record Demo` | GitHub Repo <br> X/Reddit 推广视频 |

---

### 给 Claude Code 的启动指令 (The God Prompt)

*复制以下内容给 Claude，开始你的项目：*

```text
[Start-Session Hook: 项目名 DirSoul。核心目标：本地隐私记忆框架 + SaaS插件商店。技术栈：Rust(Core) + Postgres(DB) + Python(UI) + Phi-3-mini(AI)。]

你现在是 DirSoul 的首席架构师。我们正在从零开始构建 V1 版本。
请忽略所有旧代码。基于《DirSoul 项目开发手册（V1.0 - 2026 旗舰版）》，我们需要执行 Phase 1 的任务。

任务清单：
1. 创建项目目录结构 (src/rust, src/python, migrations, docker)。
2. 编写 docker-compose.yml，包含 Postgres (带 pgvector) 和 Ollama 服务。
3. 编写 Rust 的 Cargo.toml 和 Python 的 requirements.txt (包含 streamlit, langchain, psycopg2, cryptography)。

请一步步思考，确保端口配置正确，数据卷持久化挂载到本地 ./data 目录以保证隐私。
```