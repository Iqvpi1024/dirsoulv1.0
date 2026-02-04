# Skill: Frontend Design

> **Purpose**: Design UI/UX for Streamlit interface, ensuring privacy-first design, personalized experience through DeepTalk integration, and accessibility.

---

## Design Principles

### DirSoul-Specific Guidelines

```yaml
design_principles:
  privacy_first:
    - "默认本地存储，不发送数据到云端"
    - "清晰显示加密状态"
    - "导出/删除数据功能明显"
    - "无第三方追踪"

  personalization:
    - "基于记忆的个性化界面"
    - "DeepTalk风格一致性"
    - "用户偏好记忆（深色/浅色模式）"

  simplicity:
    - "渐进式信息展示"
    - "避免认知过载"
    - "移动端友好"

  feedback:
    - "操作状态清晰可见"
    - "错误信息友好"
    - "进度指示器"
```

---

## Streamlit App Structure

### Main Layout

```python
"""
DirSoul Streamlit Interface

设计理念：简洁但强大
- 顶部：全局导航和状态
- 左侧：记忆搜索和过滤
- 中间：主要交互区域
- 右侧：AI洞察和DeepTalk

隐私优先：
- 所有数据本地处理
- 加密状态可视化
- 数据控制权在用户
"""

import streamlit as st
from streamlit_option_menu import option_menu
import datetime

def main():
    # Page config - 隐私友好的标题
    st.set_page_config(
        page_title="DirSoul",
        page_icon="🧠",
        layout="wide",
        initial_sidebar_state="expanded"
    )

    # Custom CSS - DirSoul主题
    apply_dirsoul_theme()

    # Header with encryption status
    render_header()

    # Main navigation
    render_navigation()

    # Route to page
    page = st.session_state.get("current_page", "chat")
    if page == "chat":
        render_chat_page()
    elif page == "timeline":
        render_timeline_page()
    elif page == "insights":
        render_insights_page()
    elif page == "settings":
        render_settings_page()


def apply_dirsoul_theme():
    """
    DirSoul主题：冷静、专业、隐私感

    颜色选择：
    - 主色：深蓝（信任、专业）
    - 强调色：绿色（隐私、安全）
    - 警告色：橙色（注意）
    - 错误色：红色（危险）
    """
    st.markdown("""
        <style>
        :root {
            --dirsoul-primary: #1e3a5f;
            --dirsoul-secondary: #2d5f87;
            --dirsoul-accent: #27ae60;
            --dirsoul-warning: #f39c12;
            --dirsoul-error: #e74c3c;
            --dirsoul-bg: #0e1117;
            --dirsoul-text: #fafafa;
        }

        /* 主容器样式 */
        .main {
            background-color: var(--dirsoul-bg);
            color: var(--dirsoul-text);
        }

        /* 加密状态指示器 */
        .encrypted-badge {
            background-color: var(--dirsoul-accent);
            color: white;
            padding: 4px 12px;
            border-radius: 16px;
            font-size: 12px;
            font-weight: 600;
        }

        /* 记忆卡片 */
        .memory-card {
            background-color: #1a1d24;
            border-left: 3px solid var(--dirsoul-primary);
            padding: 16px;
            margin: 8px 0;
            border-radius: 8px;
            transition: all 0.2s;
        }

        .memory-card:hover {
            border-left-color: var(--dirsoul-accent);
            transform: translateX(4px);
        }

        /* 深度对话气泡 */
        .deeptalk-bubble {
            background: linear-gradient(135deg, var(--dirsoul-primary), var(--dirsoul-secondary));
            color: white;
            padding: 16px;
            border-radius: 18px 18px 4px 18px;
            margin: 12px 0;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }

        /* 时间线样式 */
        .timeline-item {
            position: relative;
            padding-left: 32px;
            margin: 16px 0;
        }

        .timeline-item::before {
            content: '';
            position: absolute;
            left: 0;
            top: 8px;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background-color: var(--dirsoul-accent);
        }

        .timeline-item::after {
            content: '';
            position: absolute;
            left: 5px;
            top: 20px;
            width: 2px;
            height: calc(100% + 8px);
            background-color: var(--dirsoul-secondary);
        }
        </style>
    """, unsafe_allow_html=True)


def render_header():
    """
    顶部导航栏

    设计考虑：
    - 左侧：Logo和名称
    - 中间：加密状态（隐私优先）
    - 右侧：用户设置和帮助
    """
    col1, col2, col3 = st.columns([2, 1, 1])

    with col1:
        st.markdown("""
            <h1 style="margin: 0;">
                🧠 DirSoul
                <span style="font-size: 14px; font-weight: normal; opacity: 0.7;">
                    你的数字大脑
                </span>
            </h1>
        """, unsafe_allow_html=True)

    with col2:
        # 加密状态 - 核心隐私功能
        if check_encryption_enabled():
            st.markdown("""
                <div class="encrypted-badge">
                    🔒 数据已加密
                </div>
            """, unsafe_allow_html=True)
        else:
            st.warning("⚠️ 未启用加密")

    with col3:
        # 快速操作
        if st.button("⚙️ 设置"):
            st.session_state.current_page = "settings"
        if st.button("❓ 帮助"):
            st.session_state.show_help = True


def render_navigation():
    """
    侧边栏导航

    页面组织：
    - 💬 对话：与DeepTalk的主要交互
    - 📅 时间线：按时间查看事件
    - 💡 洞察：AI发现的模式和趋势
    - ⚙️ 设置：数据管理
    """
    with st.sidebar:
        st.markdown("### 导航")

        selected = option_menu(
            menu_title=None,
            options=["对话", "时间线", "洞察", "设置"],
            icons=["chat", "calendar", "lightbulb", "gear"],
            menu_icon="cast",
            default_index=0,
            orientation="vertical"
        )

        # Map selection to page
        page_map = {
            "对话": "chat",
            "时间线": "timeline",
            "洞察": "insights",
            "设置": "settings"
        }
        st.session_state.current_page = page_map.get(selected, "chat")

        st.markdown("---")

        # 统计信息
        render_stats()


def render_stats():
    """
    侧边栏统计

    显示：
    - 总事件数
    - 记忆跨度
    - AI派生视图数

    目的：让用户了解数据规模，增强信任
    """
    stats = get_user_stats()

    st.markdown("### 📊 你的数据")
    st.metric("事件总数", f"{stats['total_events']:,}")
    st.metric("记忆跨度", stats['memory_span'])
    st.metric("派生视图", f"{stats['derived_views']:,}")
```

---

## Chat Page (DeepTalk Interface)

### Main Interaction Area

```python
def render_chat_page():
    """
    DeepTalk对话界面

    设计目标：
    - 像聊天一样自然
    - 显示AI使用的记忆
    - 情绪感知的回应
    """
    st.title("💬 与你的数字大脑对话")

    # 显示对话历史
    render_conversation_history()

    # 输入区域
    user_input = st.chat_input("输入消息...")

    if user_input:
        handle_user_input(user_input)


def render_conversation_history():
    """
    显示对话历史

    设计：
    - 用户消息：右对齐，蓝色
    - DeepTalk回应：左对齐，渐变背景
    - 显示使用的记忆引用
    """
    if "messages" not in st.session_state:
        st.session_state.messages = []

    for msg in st.session_state.messages:
        with st.chat_message(msg["role"]):
            st.markdown(msg["content"])

            # DeepTalk的特殊处理
            if msg["role"] == "assistant":
                # 显示记忆引用
                if "sources" in msg and msg["sources"]:
                    with st.expander("🧠 使用的记忆"):
                        for source in msg["sources"]:
                            st.markdown(f"- {source}")

                # 显示置信度
                if "confidence" in msg:
                    confidence = msg["confidence"]
                    color = "🟢" if confidence > 0.8 else "🟡" if confidence > 0.6 else "🔴"
                    st.caption(f"{color} 置信度: {confidence:.0%}")


def handle_user_input(user_input: str):
    """
    处理用户输入

    流程：
    1. 显示用户消息
    2. 检索相关记忆（向量 + SQL）
    3. DeepTalk生成回应
    4. 存储对话作为事件
    5. 显示AI回应
    """
    # 1. 显示用户消息
    with st.chat_message("user"):
        st.markdown(user_input)
    st.session_state.messages.append({
        "role": "user",
        "content": user_input
    })

    # 2. 检索相关记忆
    with st.spinner("🧠 检索记忆..."):
        relevant_memories = retrieve_memories(user_input)

    # 3. 生成回应
    with st.spinner("💭 思考中..."):
        response = deeptalk_generate(user_input, relevant_memories)

    # 4. 存储对话
    store_conversation_event(user_input, response)

    # 5. 显示回应
    with st.chat_message("assistant"):
        st.markdown(response["content"])

        # 显示使用的记忆
        if response["sources"]:
            with st.expander("🧠 使用的记忆"):
                for source in response["sources"][:3]:  # 最多显示3个
                    st.markdown(f"- {source}")

        # 显示情绪趋势（如果适用）
        if response.get("emotion_trend"):
            trend = response["emotion_trend"]
            emoji = "😊" if trend == "positive" else "😐" if trend == "neutral" else "😔"
            st.caption(f"{emoji} 情绪趋势: {trend}")

    st.session_state.messages.append(response)


@st.cache_data(ttl=300)  # 缓存5分钟
def retrieve_memories(query: str, limit: int = 10) -> list:
    """
    检索相关记忆

    混合检索策略：
    1. 向量相似度搜索（语义）
    2. SQL过滤（时间、置信度）
    3. 合并和排序
    """
    # 调用后端API
    return backend_api.search_memories(query, limit)
```

---

## Timeline Page

### Event Visualization

```python
def render_timeline_page():
    """
    时间线页面

    设计：
    - 按日期分组显示事件
    - 可折叠的日期组
    - 筛选器（动作类型、时间范围）
    """
    st.title("📅 你的记忆时间线")

    # 筛选器
    col1, col2, col3 = st.columns(3)

    with col1:
        date_range = st.date_input(
            "日期范围",
            value=(datetime.date.today() - datetime.timedelta(days=30),
                   datetime.date.today())
        )

    with col2:
        action_filter = st.multiselect(
            "动作类型",
            ["吃", "喝", "买", "去", "运动"],
            default=[]
        )

    with col3:
        view_mode = st.radio("视图", ["卡片", "列表"], horizontal=True)

    # 获取事件
    events = get_timeline_events(date_range, action_filter)

    # 按日期分组
    events_by_date = group_by_date(events)

    # 渲染
    for date, day_events in events_by_date.items():
        with st.expander(f"📆 {date} ({len(day_events)} 个事件)", expanded=False):
            for event in day_events:
                if view_mode == "卡片":
                    render_event_card(event)
                else:
                    render_event_list_item(event)


def render_event_card(event: dict):
    """
    事件卡片

    显示：
    - 动作 + 对象
    - 时间
    - 置信度
    - 相关实体（如果有）
    """
    col1, col2 = st.columns([4, 1])

    with col1:
        st.markdown(f"""
            <div class="memory-card">
                <strong>{event['action']}</strong> {event['target']}
                <br><small style="opacity: 0.7;">
                    🕐 {event['timestamp'].strftime('%H:%M')}
                </small>
            </div>
        """, unsafe_allow_html=True)

    with col2:
        # 置信度指示
        confidence = event['confidence']
        emoji = "🟢" if confidence > 0.8 else "🟡"
        st.markdown(f"<center>{emoji}</center>", unsafe_allow_html=True)
```

---

## Insights Page

### AI-Generated Visualizations

```python
def render_insights_page():
    """
    洞察页面

    显示AI发现的：
    - 行为模式
    - 情绪趋势
    - 派生视图
    """
    st.title("💡 AI洞察")

    tab1, tab2, tab3 = st.tabs(["行为模式", "情绪趋势", "派生视图"])

    with tab1:
        render_behavior_patterns()

    with tab2:
        render_emotion_trends()

    with tab3:
        render_derived_views()


def render_behavior_patterns():
    """
    行为模式可视化

    显示：
    - 频率图（柱状图）
    - 时间热力图
    - 关系网络图
    """
    st.subheader("📊 行为频率")

    # 获取模式数据
    patterns = backend_api.get_behavior_patterns(days=30)

    # 频率柱状图
    import plotly.graph_objects as go

    fig = go.Figure(data=[
        go.Bar(
            x=[p['label'] for p in patterns],
            y=[p['count'] for p in patterns],
            marker_color='#27ae60'
        )
    ])

    fig.update_layout(
        xaxis_title="行为",
        yaxis_title="频率",
        hovermode='x unified'
    )

    st.plotly_chart(fig, use_container_width=True)

    # 模式详情表格
    st.subheader("模式详情")
    for pattern in patterns:
        with st.expander(f"🔍 {pattern['label']}"):
            st.write(f"**频率**: {pattern['count']} 次/30天")
            st.write(f"**置信度**: {pattern['confidence']:.0%}")
            st.write(f"**首次观察到**: {pattern['first_seen']}")
            st.write(f"**相关事件**: {pattern['example']}")


def render_emotion_trends():
    """
    情绪趋势可视化

    显示：
    - 时间序列图
    - 情绪分布
    - 情绪相关事件
    """
    st.subheader("😊 情绪趋势")

    # 获取情绪数据
    emotions = backend_api.get_emotion_timeline(days=7)

    # 时间序列图
    import plotly.express as px

    df = pd.DataFrame(emotions)
    fig = px.line(df, x='timestamp', y='sentiment',
                  title='情绪趋势（7天）',
                  labels={'sentiment': '情绪得分', 'timestamp': '时间'})

    # 添加颜色区域
    fig.add_hrect(y0=0.3, y1=1.0, fillcolor="green", opacity=0.1,
                  annotation_text="积极")
    fig.add_hrect(y0=-0.3, y1=0.3, fillcolor="gray", opacity=0.1,
                  annotation_text="中性")
    fig.add_hrect(y0=-1.0, y1=-0.3, fillcolor="red", opacity=0.1,
                  annotation_text="消极")

    st.plotly_chart(fig, use_container_width=True)

    # 情绪统计
    col1, col2, col3 = st.columns(3)
    with col1:
        st.metric("平均情绪", f"{emotions['sentiment'].mean():.2f}")
    with col2:
        st.metric("最积极", f"{emotions['sentiment'].max():.2f}")
    with col3:
        st.metric("最消极", f"{emotions['sentiment'].min():.2f}")
```

---

## Settings Page

### Data Management

```python
def render_settings_page():
    """
    设置页面

    关键功能（隐私优先）：
    - 加密管理
    - 数据导出
    - 数据删除
    """
    st.title("⚙️ 设置")

    tab1, tab2, tab3 = st.tabs("🔒 加密", "📤 数据", "🗑️ 隐私")

    with tab1:
        render_encryption_settings()

    with tab2:
        render_data_export()

    with tab3:
        render_privacy_settings()


def render_encryption_settings():
    """
    加密设置

    显示：
    - 当前状态
    - 密钥管理
    - 重新加密选项
    """
    st.subheader("加密状态")

    is_encrypted = check_encryption_enabled()

    if is_encrypted:
        st.success("✅ 你的数据已加密")

        col1, col2 = st.columns(2)

        with col1:
            st.info("💡 备份提醒")
            st.write("请确保你的加密密钥已安全备份。密钥丢失将无法恢复数据。")

            if st.button("📋 复制备份说明"):
                st.code("""
# DirSoul 加密密钥备份

1. 找到密钥文件: ~/.dirsoul/.encryption_key
2. 将文件复制到安全位置（U盘、密码管理器等）
3. 不要将密钥文件上传到云端
4. 定期测试密钥恢复流程
                """)
    else:
        st.warning("⚠️ 数据未加密")

        st.info("启用加密可以保护你的隐私。加密后，没有密钥无法读取数据。")

        if st.button("🔒 启用加密", type="primary"):
            with st.spinner("生成加密密钥并加密数据..."):
                enable_encryption()
            st.success("✅ 加密已启用")
            st.rerun()


def render_data_export():
    """
    数据导出（GDPR合规）

    提供：
    - 全部数据导出（加密）
    - 特定时间范围导出
    - 格式选择
    """
    st.subheader("📤 导出数据")

    col1, col2 = st.columns(2)

    with col1:
        export_format = st.selectbox(
            "导出格式",
            ["JSON", "CSV", "SQLite"]
        )

        date_range = st.date_input(
            "时间范围（留空导出全部）",
            value=(None, None)
        )

    with col2:
        include_encrypted = st.checkbox(
            "包含加密的原始数据",
            value=False,
            help="如果启用，导出文件将包含敏感内容，请妥善保管"
        )

    if st.button("📥 导出数据", type="primary"):
        with st.spinner("准备导出..."):
            export_data(export_format, date_range, include_encrypted)

        st.success("✅ 导出完成！请检查下载文件夹。")

        st.info("💡 导出的数据包含你的个人记忆，请妥善保管。")


def render_privacy_settings():
    """
    隐私设置

    功能：
    - 数据删除
    - 隐私政策
    - 数据保留策略
    """
    st.subheader("🗑️ 删除数据")

    st.warning("""
    ⚠️ **危险操作**

    删除数据后无法恢复！请在删除前导出备份。
    """)

    delete_option = st.radio(
        "删除范围",
        [
            "删除最近30天的数据",
            "删除最近1年的数据",
            "删除全部数据"
        ]
    )

    st.info(f"你选择了: {delete_option}")

    # 二次确认
    confirm = st.text_input(
        "输入 'DELETE' 以确认删除",
        placeholder="DELETE"
    )

    if confirm == "DELETE" and st.button("🗑️ 确认删除", type="primary"):
        with st.spinner("删除数据..."):
            delete_data(delete_option)
        st.success("✅ 数据已删除")
        st.info("如需恢复，请从备份导入。")
```

---

## Responsive Design

### Mobile Optimization

```python
def apply_mobile_optimizations():
    """
    移动端优化

    考虑：
    - 触摸目标大小（最小44x44px）
    - 简化导航
    - 减少横向滚动
    """
    st.markdown("""
        <style>
        @media (max-width: 768px) {
            /* 移动端字体调整 */
            .main h1 {
                font-size: 1.5rem;
            }

            /* 卡片间距 */
            .memory-card {
                margin: 4px 0;
                padding: 12px;
            }

            /* 隐藏侧边栏（使用汉堡菜单） */
            .css-1d391kg {
                display: none;
            }

            /* 按钮全宽 */
            .stButton > button {
                width: 100%;
            }
        }
        </style>
    """, unsafe_allow_html=True)
```

---

## Accessibility

### WCAG Compliance

```python
def ensure_accessibility():
    """
    无障碍设计

    功能：
    - 键盘导航支持
    - 屏幕阅读器友好
    - 高对比度模式
    - 字体大小调整
    """
    # 键盘快捷键
    st.markdown("""
        <style>
        /* 焦点可见 */
        *:focus {
            outline: 2px solid var(--dirsoul-accent);
            outline-offset: 2px;
        }

        /* 跳过导航链接 */
        .skip-link {
            position: absolute;
            top: -40px;
            left: 0;
            background: var(--dirsoul-accent);
            color: white;
            padding: 8px;
            z-index: 100;
        }

        .skip-link:focus {
            top: 0;
        }
        </style>
    """, unsafe_allow_html=True)

    # 字体大小调整
    font_size = st.slider(
        "字体大小",
        min_value=12,
        max_value=24,
        value=16,
        help="调整界面文字大小"
    )

    st.markdown(f"""
        <style>
        html {{
            font-size: {font_size}px;
        }}
        </style>
    """, unsafe_allow_html=True)
```

---

## Recommended Combinations

Use this skill together with:
- **DeepTalkImplementation**: For personalized UI integration
- **PluginPermissionSystem**: For settings access control
- **Documentation**: For UI/UX documentation
