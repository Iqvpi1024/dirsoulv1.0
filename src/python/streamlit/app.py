"""
DirSoul - 数字大脑 UI
2026年最新设计趋势：Dark Glassmorphism + Bento Box Layout
"""

import streamlit as st
from datetime import datetime, timedelta
import sys
from pathlib import Path

# Page configuration
st.set_page_config(
    page_title="DirSoul",
    page_icon="🧠",
    layout="wide",
    initial_sidebar_state="expanded",
)

# 2026 Dark Glassmorphism CSS
st.markdown("""
<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=Space+Grotesk:wght@500;700&display=swap');

/* ========== 全局样式 ========== */
.stApp {
    background: #0a0a0f;
    font-family: 'Inter', -apple-system, sans-serif;
}

/* 隐藏默认元素 */
.stDeployButton, #MainMenu, footer, .stStatusWidget {
    display: none !important;
}

/* ========== 深色玻璃态侧边栏 ========== */
[data-testid="stSidebar"] {
    background: linear-gradient(180deg,
        rgba(20, 20, 30, 0.8) 0%,
        rgba(10, 10, 15, 0.9) 100%);
    backdrop-filter: blur(40px) saturate(180%);
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    padding: 0 !important;
}

[data-testid="stSidebar"]::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background:
        radial-gradient(circle at 20% 30%, rgba(120, 119, 198, 0.15) 0%, transparent 50%),
        radial-gradient(circle at 80% 70%, rgba(78, 56, 163, 0.15) 0%, transparent 50%);
    pointer-events: none;
    z-index: 0;
}

[data-testid="stSidebar"] > div:first-child {
    position: relative;
    z-index: 1;
    padding: 1.5rem;
    background: transparent;
}

/* ========== Logo区域 - 动态发光 ========== */
.logo-section {
    text-align: center;
    padding: 2rem 0;
    margin-bottom: 1.5rem;
    position: relative;
}

.logo-icon {
    font-size: 4.5rem;
    filter: drop-shadow(0 0 30px rgba(139, 92, 246, 0.6));
    animation: pulse-glow 3s ease-in-out infinite;
}

@keyframes pulse-glow {
    0%, 100% { filter: drop-shadow(0 0 30px rgba(139, 92, 246, 0.6)); }
    50% { filter: drop-shadow(0 0 40px rgba(139, 92, 246, 0.9)); }
}

.logo-title {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 2.2rem;
    font-weight: 700;
    background: linear-gradient(135deg, #a78bfa 0%, #818cf8 50%, #6366f1 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    margin-top: 0.5rem;
    letter-spacing: -0.5px;
}

.logo-subtitle {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.45);
    font-weight: 400;
    letter-spacing: 2px;
    text-transform: uppercase;
    margin-top: 0.5rem;
}

/* ========== Bento Box风格统计卡片 ========== */
.bento-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem;
    margin: 1rem 0;
}

.bento-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 1.25rem;
    backdrop-filter: blur(20px);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
}

.bento-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(90deg,
        transparent 0%,
        rgba(167, 139, 250, 0.5) 50%,
        transparent 100%);
}

.bento-card:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(139, 92, 246, 0.3);
    transform: translateY(-2px);
    box-shadow: 0 8px 32px rgba(139, 92, 246, 0.15);
}

.bento-value {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 2rem;
    font-weight: 700;
    background: linear-gradient(135deg, #a78bfa 0%, #818cf8 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    line-height: 1;
}

.bento-label {
    font-size: 0.7rem;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 1.5px;
    margin-top: 0.5rem;
}

/* ========== 导航按钮 ========== */
.nav-item {
    background: rgba(255, 255, 255, 0.02) !important;
    border: 1px solid rgba(255, 255, 255, 0.06) !important;
    border-radius: 14px !important;
    padding: 1rem 1.25rem !important;
    margin: 0.5rem 0 !important;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
    position: relative;
    overflow: hidden;
}

.nav-item::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: 3px;
    height: 100%;
    background: linear-gradient(180deg, #a78bfa 0%, #818cf8 100%);
    transform: scaleY(0);
    transition: transform 0.3s ease;
}

.nav-item:hover {
    background: rgba(139, 92, 246, 0.1) !important;
    border-color: rgba(139, 92, 246, 0.3) !important;
    transform: translateX(4px);
}

.nav-item:hover::before {
    transform: scaleY(1);
}

/* ========== 主内容区域 ========== */
.main .block-container {
    padding-top: 1rem;
    background: transparent;
    max-width: 1400px;
}

/* ========== 页面标题 ========== */
.page-header {
    text-align: center;
    margin-bottom: 3rem;
    position: relative;
}

.page-title {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 3rem;
    font-weight: 700;
    background: linear-gradient(135deg, #e0e7ff 0%, #c7d2fe 50%, #a5b4fc 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    margin-bottom: 0.5rem;
    letter-spacing: -1px;
}

.page-subtitle {
    font-size: 1rem;
    color: rgba(255, 255, 255, 0.4);
    font-weight: 300;
    letter-spacing: 0.5px;
}

/* ========== 聊天容器 - Glassmorphism ========== */
.chat-wrapper {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 24px;
    padding: 1.5rem;
    backdrop-filter: blur(40px) saturate(180%);
    max-width: 1000px;
    margin: 0 auto;
    position: relative;
}

.chat-wrapper::before {
    content: '';
    position: absolute;
    top: -1px;
    left: 50%;
    transform: translateX(-50%);
    width: 60%;
    height: 1px;
    background: linear-gradient(90deg,
        transparent 0%,
        rgba(167, 139, 250, 0.5) 50%,
        transparent 100%);
}

/* ========== 聊天气泡 ========== */
.stChatMessage {
    background: transparent !important;
    border: none !important;
    padding: 1.25rem 0 !important;
}

/* 用户消息 */
.stChatMessage[data-testid="user-message"] {
    flex-direction: row-reverse;
}

.stChatMessage[data-testid="user-message"] > div {
    background: linear-gradient(135deg,
        rgba(139, 92, 246, 0.9) 0%,
        rgba(99, 102, 241, 0.9) 100%) !important;
    border: 1px solid rgba(167, 139, 250, 0.3) !important;
    border-radius: 20px 20px 6px 20px !important;
    padding: 1rem 1.5rem !important;
    box-shadow:
        0 4px 24px rgba(139, 92, 246, 0.25),
        inset 0 1px 0 rgba(255, 255, 255, 0.1);
    max-width: 65%;
    backdrop-filter: blur(10px);
}

/* AI消息 */
.stChatMessage[data-testid="assistant-message"] > div {
    background: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    border-radius: 20px 20px 20px 6px !important;
    padding: 1rem 1.5rem !important;
    backdrop-filter: blur(20px) saturate(180%);
    max-width: 65%;
}

.stChatMessage[data-testid="assistant-message"] p {
    color: rgba(255, 255, 255, 0.9);
}

/* ========== 输入框 ========== */
.stChatInputContainer {
    background: rgba(255, 255, 255, 0.03) !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    border-radius: 20px !important;
    padding: 0.5rem !important;
    backdrop-filter: blur(30px) saturate(180%);
    transition: all 0.3s ease !important;
}

.stChatInputContainer:focus-within {
    border-color: rgba(139, 92, 246, 0.5) !important;
    box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.1) !important;
}

.stChatInputContainer > div {
    background: transparent !important;
}

.stChatInput textarea {
    background: transparent !important;
    color: white !important;
    border: none !important;
    font-size: 0.95rem;
}

.stChatInput textarea::placeholder {
    color: rgba(255, 255, 255, 0.35);
}

/* 发送按钮 */
.stChatInputContainer button {
    background: linear-gradient(135deg, #8b5cf6 0%, #6366f1 100%) !important;
    border: none !important;
    border-radius: 14px !important;
    width: 44px;
    height: 44px;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
}

.stChatInputContainer button:hover {
    transform: scale(1.08);
    box-shadow: 0 4px 20px rgba(139, 92, 246, 0.5);
}

/* ========== Metric卡片 ========== */
[data-testid="stMetricValue"] {
    color: white !important;
    font-family: 'Space Grotesk', sans-serif;
    font-size: 2.5rem !important;
    font-weight: 700 !important;
    background: linear-gradient(135deg, #a78bfa 0%, #818cf8 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
}

[data-testid="stMetricDelta"] {
    font-size: 0.9rem !important;
    color: rgba(255, 255, 255, 0.5) !important;
}

/* ========== 标签和标题 ========== */
label {
    color: rgba(255, 255, 255, 0.7) !important;
    font-weight: 500 !important;
    font-size: 0.9rem;
}

h1, h2, h3 {
    color: white !important;
    font-weight: 600 !important;
}

/* ========== 按钮 ========== */
.stButton > button {
    background: linear-gradient(135deg, #8b5cf6 0%, #6366f1 100%) !important;
    border: none !important;
    border-radius: 14px !important;
    padding: 0.85rem 2rem !important;
    color: white !important;
    font-weight: 600 !important;
    font-size: 0.9rem !important;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
    box-shadow: 0 4px 14px rgba(139, 92, 246, 0.3) !important;
}

.stButton > button:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(139, 92, 246, 0.4) !important;
}

/* ========== Expander ========== */
.streamlit-expanderHeader {
    background: rgba(255, 255, 255, 0.04) !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    border-radius: 16px !important;
    color: white !important;
    transition: all 0.3s ease !important;
}

.streamlit-expanderHeader:hover {
    background: rgba(139, 92, 246, 0.1) !important;
    border-color: rgba(139, 92, 246, 0.3) !important;
}

.streamlit-expanderContent {
    background: rgba(255, 255, 255, 0.02) !important;
    border: 1px solid rgba(255, 255, 255, 0.08) !important;
    border-radius: 16px !important;
}

/* ========== 滚动条 ========== */
::-webkit-scrollbar {
    width: 6px;
}

::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.02);
}

::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, #8b5cf6 0%, #6366f1 100%);
    border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
    background: linear-gradient(180deg, #a78bfa 0%, #818cf8 100%);
}

/* ========== Info卡片 ========== */
.info-box {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 1.5rem;
    margin: 1rem 0;
    backdrop-filter: blur(20px);
}

/* ========== Selectbox ========== */
.stSelectbox > div > div {
    background: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    border-radius: 12px !important;
}

/* ========== TextInput ========== */
.stTextInput > div > div > input {
    background: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    border-radius: 12px !important;
    color: white !important;
}

/* ========== DateInput ========== */
.stDateInput > div > div > input {
    background: rgba(255, 255, 255, 0.05) !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
    border-radius: 12px !important;
    color: white !important;
}
</style>
""", unsafe_allow_html=True)

# Initialize session state
if "messages" not in st.session_state:
    st.session_state.messages = []
if "current_page" not in st.session_state:
    st.session_state.current_page = "chat"

# ========== Sidebar ==========
with st.sidebar:
    # Logo
    st.markdown("""
    <div class="logo-section">
        <div class="logo-icon">🧠</div>
        <div class="logo-title">DirSoul</div>
        <div class="logo-subtitle">数字大脑</div>
    </div>
    """, unsafe_allow_html=True)

    st.markdown("---")

    # Navigation
    st.markdown("### 导航")
    page = st.radio(
        "",
        ["💬 对话", "📅 时间线", "📊 洞察", "⚙️ 设置"],
        label_visibility="collapsed",
    )

    st.markdown("---")

    # Bento Grid Stats
    st.markdown("### 统计")
    st.markdown("""
    <div class="bento-grid">
        <div class="bento-card">
            <div class="bento-value">156</div>
            <div class="bento-label">今日</div>
        </div>
        <div class="bento-card">
            <div class="bento-value">2.3K</div>
            <div class="bento-label">总数</div>
        </div>
    </div>
    """, unsafe_allow_html=True)

    st.markdown("---")

    # System Status
    st.markdown("### 系统")
    st.markdown("""
    <div style="font-size: 0.85rem; color: rgba(255,255,255,0.5); line-height: 1.8;">
    🧠 qwen2:0.5b<br>
    💾 8GB RAM<br>
    ⚡ <span style="color: #a78bfa;">● 运行中</span>
    </div>
    """, unsafe_allow_html=True)

    st.markdown("---")
    st.markdown("""
    <div style="text-align: center; font-size: 0.7rem; color: rgba(255,255,255,0.3); padding: 1rem;">
    🧠 DirSoul v1.0<br>
    本地优先 · 隐私保护
    </div>
    """, unsafe_allow_html=True)

# ========== Main Content ==========
if page == "💬 对话":
    st.markdown("""
    <div class="page-header">
        <div class="page-title">对话记忆</div>
        <div class="page-subtitle">记录想法，构建知识</div>
    </div>
    """, unsafe_allow_html=True)

    # Chat messages (不用wrapper，让Streamlit自然布局)
    for message in st.session_state.messages:
        with st.chat_message(message["role"]):
            st.markdown(message["content"])

    # Chat input (Streamlit会自动放在底部)
    if prompt := st.chat_input("✍️ 输入你的想法..."):
        with st.chat_message("user"):
            st.markdown(prompt)

        try:
            import requests
            api_url = "http://localhost:8080/api/chat"
            payload = {
                "user_id": "streamlit_user",
                "message": prompt,
                "history": [{"role": m["role"], "content": m["content"]}
                           for m in st.session_state.messages if m["role"] in ["user", "assistant"]]
            }

            response = requests.post(api_url, json=payload, timeout=15)

            if response.status_code == 200:
                data = response.json()
                assistant_message = data.get("response", "抱歉，我暂时无法回应。")
                st.session_state.messages = [
                    {"role": m["role"], "content": m["content"]}
                    for m in data.get("history", [])
                ]
            else:
                assistant_message = f"服务不可用 (HTTP {response.status_code})"
                st.session_state.messages.append({"role": "user", "content": prompt})
                st.session_state.messages.append({"role": "assistant", "content": assistant_message})

        except Exception as e:
            assistant_message = f"连接错误: {str(e)}"
            st.session_state.messages.append({"role": "user", "content": prompt})
            st.session_state.messages.append({"role": "assistant", "content": assistant_message})

        with st.chat_message("assistant"):
            st.markdown(assistant_message)

        st.rerun()

elif page == "📅 时间线":
    st.markdown("""
    <div class="page-header">
        <div class="page-title">记忆时间线</div>
        <div class="page-subtitle">回顾经历，发现模式</div>
    </div>
    """, unsafe_allow_html=True)

    col1, col2 = st.columns(2)
    with col1:
        start_date = st.date_input("开始", datetime.now() - timedelta(days=7))
    with col2:
        end_date = st.date_input("结束", datetime.now())

    st.markdown("---")

    dates = [start_date + timedelta(days=i) for i in range((end_date - start_date).days + 1)]

    for date in dates:
        with st.expander(f"📅 {date.strftime('%Y年%m月%d日')}"):
            st.markdown("""
            <div style="padding: 0.5rem 0; border-left: 2px solid rgba(139,92,246,0.3); padding-left: 1rem;">
            <div style="color: rgba(255,255,255,0.4); font-size: 0.8rem;">09:30</div>
            <div style="color: white;">记录想法</div>
            </div>
            """, unsafe_allow_html=True)

elif page == "📊 洞察":
    st.markdown("""
    <div class="page-header">
        <div class="page-title">数据洞察</div>
        <div class="page-subtitle">可视化分析记忆模式</div>
    </div>
    """, unsafe_allow_html=True)

    col1, col2, col3, col4 = st.columns(4)
    with col1:
        st.metric("总记忆", "2,341", "+12")
    with col2:
        st.metric("本周", "156", "8%")
    with col3:
        st.metric("最活跃", "周三")
    with col4:
        st.metric("平均", "5.2/天")

    st.markdown("---")

    col1, col2 = st.columns(2)
    with col1:
        st.markdown("### 📈 每日趋势")
        chart_data = {"一": 4, "二": 6, "三": 8, "四": 5, "五": 7, "六": 3, "日": 2}
        st.bar_chart(chart_data, use_container_width=True, color="#8b5cf6")

    with col2:
        st.markdown("### 🎯 类型分布")
        type_data = {"对话": 45, "想法": 30, "事件": 15, "笔记": 10}
        st.bar_chart(type_data, use_container_width=True, color="#6366f1")

elif page == "⚙️ 设置":
    st.markdown("""
    <div class="page-header">
        <div class="page-title">系统设置</div>
        <div class="page-subtitle">配置你的数字大脑</div>
    </div>
    """, unsafe_allow_html=True)

    st.markdown("---")

    col1, col2 = st.columns(2)

    with col1:
        st.markdown("### 🤖 推理模型")
        st.info("""
        **当前**: qwen2:0.5b

        用于语义理解
        - 大小: 352MB
        - 速度: 快速
        """)
        st.selectbox("切换模型", ["qwen2:0.5b", "phi4-mini"], label_visibility="collapsed")

    with col2:
        st.markdown("### 🔤 向量模型")
        st.info("""
        **当前**: nomic-embed-text

        用于语义搜索
        - 维度: 768
        """)
        st.text_input("Embedding", "nomic-embed-text", disabled=True, label_visibility="collapsed")

    st.markdown("---")
    st.markdown("### 🔧 Ollama")
    st.text_input("地址", "http://localhost:11434")

    st.markdown("---")
    st.markdown("### 💾 数据")
    col1, col2, col3 = st.columns(3)
    with col1:
        st.button("📤 导出", use_container_width=True)
    with col2:
        st.button("📥 导入", use_container_width=True)
    with col3:
        st.button("🗑️ 清除", use_container_width=True)

# Footer
st.markdown("""
<div style="text-align: center; padding: 3rem 0; color: rgba(255,255,255,0.25); font-size: 0.8rem;">
    🧠 DirSoul v1.0 · 本地优先 · 隐私保护 · AI驱动
</div>
""", unsafe_allow_html=True)
