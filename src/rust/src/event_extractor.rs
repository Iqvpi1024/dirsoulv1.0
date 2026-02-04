//! DirSoul Event Extraction Module
//!
//! 使用规则引擎和 SLM 从文本中提取结构化事件。
//! 遵循 HEAD.md 原则：规则仅作为 SLM 失败时的兜底方案。
//!
//! # 设计原则
//! - AI-Native：优先使用 SLM，规则作为兜底
//! - 每个事件必须有精确时间戳
//! - 数量必须结构化存储
//! - 行为必须类型化
//!
//! # 示例
//! ```no_run
//! use dirsoul::event_extractor::{RuleExtractor, ExtractedEvent};
//!
//! let extractor = RuleExtractor::new();
//! let text = "今天吃了3个苹果";
//! let events = extractor.extract(text).unwrap();
//! assert_eq!(events[0].action, "吃");
//! assert_eq!(events[0].target, "苹果");
//! assert_eq!(events[0].quantity, Some(3.0));
//! ```

use chrono::{DateTime, Datelike, Duration, Local, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::Result;
use crate::prompt_manager::PromptManager;

/// 提取的事件结构
///
/// 从文本中提取的结构化事件，可转换为 NewEventMemory。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEvent {
    /// 动作（行为）
    pub action: String,
    /// 目标对象
    pub target: String,
    /// 数量（可选）
    pub quantity: Option<f64>,
    /// 单位（可选）
    pub unit: Option<String>,
    /// 执行者（可选）
    pub actor: Option<String>,
    /// 置信度 (0-1)
    pub confidence: f64,
    /// 提取方法（rule/slm）
    pub method: String,
}

impl ExtractedEvent {
    /// 创建新事件
    pub fn new(action: String, target: String) -> Self {
        Self {
            action,
            target,
            quantity: None,
            unit: None,
            actor: None,
            confidence: 0.5,
            method: "rule".to_string(),
        }
    }

    /// 设置数量和单位
    pub fn with_quantity(mut self, quantity: f64, unit: String) -> Self {
        self.quantity = Some(quantity);
        self.unit = Some(unit);
        self
    }

    /// 设置执行者
    pub fn with_actor(mut self, actor: String) -> Self {
        self.actor = Some(actor);
        self
    }

    /// 设置置信度
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// 设置提取方法
    pub fn with_method(mut self, method: String) -> Self {
        self.method = method;
        self
    }
}

/// 中文时间范围解析器
///
/// 支持相对时间表达："今天"、"昨天"、"上周三"、"下午3点"等。
pub struct TimeParser {
    /// 当前时间（用于相对时间计算）
    now: DateTime<Utc>,
}

impl TimeParser {
    /// 创建新的时间解析器
    pub fn new() -> Self {
        Self {
            now: Utc::now(),
        }
    }

    /// 使用指定时间创建解析器
    pub fn with_time(now: DateTime<Utc>) -> Self {
        Self { now }
    }

    /// 解析中文时间表达
    ///
    /// # 示例
    /// - "今天" → 今天 00:00:00
    /// - "昨天" → 昨天 00:00:00
    /// - "上周三" → 上周三 00:00:00
    /// - "3天前" → 3天前 00:00:00
    pub fn parse(&self, text: &str) -> Option<DateTime<Utc>> {
        let text = text.trim();

        // 今天
        if text == "今天" {
            return Some(self.with_time_zero(Local::now().date_naive()));
        }

        // 昨天
        if text == "昨天" {
            let yesterday = Local::now().date_naive() - Duration::days(1);
            return Some(self.with_time_zero(yesterday));
        }

        // 前天
        if text == "前天" {
            let day_before_yesterday = Local::now().date_naive() - Duration::days(2);
            return Some(self.with_time_zero(day_before_yesterday));
        }

        // X天前
        let days_ago_re = Regex::new(r"^(\d+)天前$").unwrap();
        if let Some(caps) = days_ago_re.captures(text) {
            if let Ok(days) = caps[1].parse::<i64>() {
                let date = Local::now().date_naive() - Duration::days(days);
                return Some(self.with_time_zero(date));
            }
        }

        // 今天上午/下午
        if text == "今天上午" || text == "今天早上" {
            return Some(self.naive_to_utc(Local::now().date_naive(), 9));
        }
        if text == "今天下午" {
            return Some(self.naive_to_utc(Local::now().date_naive(), 14));
        }
        if text == "今天晚上" || text == "今天夜里" {
            return Some(self.naive_to_utc(Local::now().date_naive(), 20));
        }

        // 本周X
        let weekday_re = Regex::new(r"^(今天|昨天|明天|本周|上周|下周)?周?(一|二|三|四|五|六|日|天)$").unwrap();
        if let Some(caps) = weekday_re.captures(text) {
            let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let day = &caps[2];

            // 获取目标星期几 (1=Monday, 7=Sunday)
            let target_weekday = match day {
                "一" => 1,
                "二" => 2,
                "三" => 3,
                "四" => 4,
                "五" => 5,
                "六" => 6,
                "日" | "天" => 7,
                _ => return None,
            };

            let current_weekday = Local::now().date_naive().weekday().num_days_from_monday() + 1;
            let today = Local::now().date_naive();

            let target_date = match prefix {
                "今天" => {
                    // 今天是周X（就是今天）
                    today
                }
                "昨天" => {
                    // 昨天是周X（不太常用，但支持）
                    today - Duration::days(1)
                }
                "明天" => {
                    // 明天是周X
                    today + Duration::days(1)
                }
                "本周" => {
                    // 本周X
                    let diff = target_weekday as i64 - current_weekday as i64;
                    today + Duration::days(diff)
                }
                "上周" => {
                    // 上周X
                    let diff = target_weekday as i64 - current_weekday as i64 - 7;
                    today + Duration::days(diff)
                }
                "下周" => {
                    // 下周X
                    let diff = target_weekday as i64 - current_weekday as i64 + 7;
                    today + Duration::days(diff)
                }
                "" | _ => {
                    // 默认为本周X（"周三"、"周五"等）
                    let diff = target_weekday as i64 - current_weekday as i64;
                    today + Duration::days(diff)
                }
            };

            return Some(self.with_time_zero(target_date));
        }

        None
    }

    /// 将日期转换为当天的 00:00:00 UTC
    fn with_time_zero(&self, date: chrono::NaiveDate) -> DateTime<Utc> {
        self.naive_to_utc(date, 0)
    }

    /// 将本地日期和小时转换为 UTC
    fn naive_to_utc(&self, date: chrono::NaiveDate, hour: i64) -> DateTime<Utc> {
        let naive = date.and_hms_opt(hour as u32, 0, 0).unwrap();
        DateTime::<Local>::from_naive_utc_and_offset(naive, *Local::now().offset())
            .with_timezone(&Utc)
    }
}

impl Default for TimeParser {
    fn default() -> Self {
        Self::new()
    }
}

/// 规则引擎事件抽取器
///
/// 使用正则表达式从中文文本中快速提取事件。
/// 作为 SLM 抽取失败时的兜底方案。
///
/// # HEAD.md 遵循
/// - 拒绝硬编码规则：本模块仅作为兜底，优先使用 SLM
/// - 数量必须结构化存储：提取 quantity + unit
/// - 行为必须类型化：action 字段存储动词
pub struct RuleExtractor {
    /// 动词-动作映射表
    action_map: HashMap<String, String>,
    /// 中文单位列表
    units: Vec<String>,
    /// 数词-数字映射
    number_map: HashMap<String, f64>,
}

impl RuleExtractor {
    /// 创建新的规则抽取器
    pub fn new() -> Self {
        let mut action_map = HashMap::new();

        // 常见动词映射
        action_map.insert("吃".to_string(), "吃".to_string());
        action_map.insert("喝".to_string(), "喝".to_string());
        action_map.insert("买".to_string(), "购买".to_string());
        action_map.insert("购".to_string(), "购买".to_string());
        action_map.insert("去".to_string(), "去".to_string());
        action_map.insert("来".to_string(), "来".to_string());
        action_map.insert("做".to_string(), "做".to_string());
        action_map.insert("完成".to_string(), "完成".to_string());
        action_map.insert("开始".to_string(), "开始".to_string());
        action_map.insert("结束".to_string(), "结束".to_string());
        action_map.insert("看".to_string(), "看".to_string());
        action_map.insert("读".to_string(), "阅读".to_string());
        action_map.insert("写".to_string(), "写".to_string());
        action_map.insert("听".to_string(), "听".to_string());
        action_map.insert("说".to_string(), "说".to_string());
        action_map.insert("玩".to_string(), "玩".to_string());
        action_map.insert("运动".to_string(), "运动".to_string());
        action_map.insert("跑步".to_string(), "跑步".to_string());
        action_map.insert("睡觉".to_string(), "睡觉".to_string());
        action_map.insert("起床".to_string(), "起床".to_string());
        action_map.insert("工作".to_string(), "工作".to_string());
        action_map.insert("学习".to_string(), "学习".to_string());
        action_map.insert("消费".to_string(), "消费".to_string());
        action_map.insert("支付".to_string(), "支付".to_string());

        let mut number_map = HashMap::new();
        number_map.insert("一".to_string(), 1.0);
        number_map.insert("二".to_string(), 2.0);
        number_map.insert("三".to_string(), 3.0);
        number_map.insert("四".to_string(), 4.0);
        number_map.insert("五".to_string(), 5.0);
        number_map.insert("六".to_string(), 6.0);
        number_map.insert("七".to_string(), 7.0);
        number_map.insert("八".to_string(), 8.0);
        number_map.insert("九".to_string(), 9.0);
        number_map.insert("十".to_string(), 10.0);
        number_map.insert("两".to_string(), 2.0);

        Self {
            action_map,
            units: vec![
                "个".to_string(), "只".to_string(), "件".to_string(), "台".to_string(),
                "本".to_string(), "张".to_string(), "次".to_string(), "分钟".to_string(),
                "小时".to_string(), "天".to_string(), "周".to_string(), "月".to_string(),
                "年".to_string(), "公斤".to_string(), "克".to_string(), "斤".to_string(),
                "两".to_string(), "毫升".to_string(), "升".to_string(), "米".to_string(),
                "公里".to_string(), "元".to_string(), "块".to_string(), "百".to_string(),
                "千".to_string(), "万".to_string(),
            ],
            number_map,
        }
    }

    /// 从文本中提取事件
    ///
    /// # 示例
    /// ```
    /// # use dirsoul::event_extractor::RuleExtractor;
    /// let extractor = RuleExtractor::new();
    /// let events = extractor.extract("今天吃了3个苹果").unwrap();
    /// assert_eq!(events.len(), 1);
    /// assert_eq!(events[0].action, "吃");
    /// ```
    pub fn extract(&self, text: &str) -> Result<Vec<ExtractedEvent>> {
        let mut events = Vec::new();

        // 模式1：动词 + 数量 + 单位 + 名词
        // 例如：吃了3个苹果、买了1本书
        let pattern1 = Regex::new(r"([动词去来吃买做看读写听说玩运动跑睡起工作学习消费支付]+)(了|过)?(\d+|一两二三四五六七八九十百千万)([个只件台本张次分钟小时天周月年公斤克斤两毫升升米公里元块百千万]+)(.+)").unwrap();

        if let Some(caps) = pattern1.captures(text) {
            let action = self.normalize_action(&caps[1]);
            let quantity_str = &caps[3];
            let unit = caps[4].to_string();
            let target = caps[5].trim().to_string();

            let quantity = self.parse_quantity(quantity_str)?;

            events.push(
                ExtractedEvent::new(action, target)
                    .with_quantity(quantity, unit)
                    .with_confidence(0.7) // 规则匹配的置信度
                    .with_method("rule".to_string())
            );
        }

        // 模式2：动词 + 名词（无数量）
        // 例如：吃苹果、去跑步
        let pattern2 = Regex::new(r"(去|来|吃|喝|买|做|看|读|写|听|说|玩|运动|跑|睡|起|工作|学习)(了|过)?(.+)").unwrap();

        if events.is_empty() {
            if let Some(caps) = pattern2.captures(text) {
                let action = self.normalize_action(&caps[1]);
                let target = caps[3].trim().to_string();

                events.push(
                    ExtractedEvent::new(action, target)
                        .with_confidence(0.5) // 无数量的置信度较低
                        .with_method("rule".to_string())
                );
            }
        }

        Ok(events)
    }

    /// 标准化动作（动词规范化）
    fn normalize_action(&self, verb: &str) -> String {
        if let Some(action) = self.action_map.get(verb) {
            action.clone()
        } else {
            verb.to_string()
        }
    }

    /// 解析数量（支持中文数字和阿拉伯数字）
    fn parse_quantity(&self, text: &str) -> Result<f64> {
        // 先尝试阿拉伯数字
        if let Ok(num) = text.parse::<f64>() {
            return Ok(num);
        }

        // 尝试中文数字
        if let Some(&num) = self.number_map.get(text) {
            return Ok(num);
        }

        // 处理"十几"、"几十"等表达
        if text.ends_with("十几") {
            return Ok(10.0 + self.parse_quantity(&text.replace("十几", ""))?);
        }

        Err(crate::DirSoulError::Config(format!(
            "无法解析数量: {}",
            text
        )))
    }

    /// 检查文本是否包含时间信息
    pub fn has_time_info(&self, text: &str) -> bool {
        let time_keywords = [
            "今天", "昨天", "前天", "明天", "后天",
            "上午", "下午", "早上", "晚上", "夜里", "中午",
            "本周", "上周", "下周",
            "天前", "周前", "月前",
        ];

        for keyword in &time_keywords {
            if text.contains(keyword) {
                return true;
            }
        }

        false
    }
}

impl Default for RuleExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Ollama API 响应结构
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
}

/// Ollama 生成请求
#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>, // "json" for JSON output
}

/// SLM 事件抽取器
///
/// 使用 Phi-4-mini 通过 Ollama API 从文本中提取结构化事件。
/// 遵循 HEAD.md 原则：SLM 主导提取，规则仅作为失败时的兜底。
///
/// # 设计原则
/// - AI-Native：优先使用 SLM
/// - 异步处理：使用 tokio 避免阻塞
/// - 失败回退：SLM 失败时回退到规则引擎
/// - 内存优化：批处理限制（8GB 内存环境）
pub struct SlmExtractor {
    /// Ollama 主机地址
    host: String,
    /// 模型名称
    model: String,
    /// HTTP 客户端
    client: reqwest::Client,
    /// 兜底规则抽取器
    rule_fallback: RuleExtractor,
    /// 请求超时时间（秒）
    timeout_secs: u64,
    /// Prompt管理器（使用Mutex支持内部可变性）
    prompt_manager: Arc<Mutex<PromptManager>>,
}

impl SlmExtractor {
    /// 创建新的 SLM 抽取器
    ///
    /// # Arguments
    /// * `host` - Ollama 主机地址（默认：http://127.0.0.1:11434）
    /// * `model` - 模型名称（默认：phi4-mini）
    pub async fn new(host: Option<String>, model: Option<String>) -> Result<Self> {
        let host = host.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
        let model = model.unwrap_or_else(|| "phi4-mini".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| {
                crate::DirSoulError::Config(format!("Failed to create HTTP client: {}", e))
            })?;

        // 初始化PromptManager
        let prompt_manager = Arc::new(Mutex::new(PromptManager::new()?));

        Ok(Self {
            host,
            model,
            client,
            rule_fallback: RuleExtractor::new(),
            timeout_secs: 120,
            prompt_manager,
        })
    }

    /// 使用默认配置创建抽取器
    pub async fn default_config() -> Result<Self> {
        Self::new(None, None).await
    }

    /// 从文本中提取事件（SLM 优先）
    ///
    /// # 流程
    /// 1. 尝试使用 SLM 提取
    /// 2. 如果 SLM 失败，回退到规则引擎
    /// 3. 返回提取的事件列表
    ///
    /// # 示例
    /// ```no_run
    /// # use dirsoul::event_extractor::SlmExtractor;
    /// # async fn example() -> dirsoul::Result<()> {
    /// let extractor = SlmExtractor::default_config().await?;
    /// let events = extractor.extract("今天下午吃了3个苹果").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn extract(&self, text: &str) -> Result<Vec<ExtractedEvent>> {
        // 首先尝试 SLM
        match self.extract_with_slm(text).await {
            Ok(events) => {
                tracing::debug!("SLM extraction succeeded: {} events", events.len());
                Ok(events)
            }
            Err(e) => {
                tracing::warn!("SLM extraction failed: {:?}, falling back to rules", e);
                // 回退到规则引擎
                self.rule_fallback.extract(text)
            }
        }
    }

    /// 批量提取事件
    ///
    /// 8GB 内存优化：限制批处理大小
    pub async fn extract_batch(&self, texts: &[String]) -> Result<Vec<Vec<ExtractedEvent>>> {
        const MAX_BATCH_SIZE: usize = 5; // 限制批处理大小

        let mut results = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(MAX_BATCH_SIZE) {
            for text in chunk {
                let events = self.extract(text).await?;
                results.push(events);
            }
        }

        Ok(results)
    }

    /// 使用 SLM 提取事件（内部方法）
    async fn extract_with_slm(&self, text: &str) -> Result<Vec<ExtractedEvent>> {
        let prompt = self.build_prompt(text);

        let request = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            format: Some("json".to_string()),
        };

        let url = format!("{}/api/generate", self.host);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                crate::DirSoulError::Config(format!("HTTP request failed: {}", e))
            })?
            .error_for_status()
            .map_err(|e| {
                crate::DirSoulError::Config(format!("Ollama API error: {:?}", e.status()))
            })?
            .json::<OllamaResponse>()
            .await
            .map_err(|e| {
                crate::DirSoulError::Config(format!("Failed to parse response: {}", e))
            })?;

        self.parse_slm_response(&response.response)
    }

    /// 构建 Prompt
    fn build_prompt(&self, text: &str) -> String {
        // 使用PromptManager从外部文件加载prompt模板
        let mut vars = HashMap::new();
        vars.insert("text", text);

        match self.prompt_manager.lock() {
            Ok(mut manager) => {
                match manager.render_prompt("event_extraction", vars) {
                    Ok(prompt) => prompt,
                    Err(_) => self.build_fallback_prompt(text),
                }
            }
            Err(_) => self.build_fallback_prompt(text),
        }
    }

    /// 构建兜底 Prompt（当外部文件加载失败时使用）
    fn build_fallback_prompt(&self, text: &str) -> String {
        format!(
            r#"你是 DirSoul 事件抽取系统。从以下文本中提取事件，输出 JSON 格式。

# 规则
1. 每个事件包含：action（行为）、target（对象）
2. 如果有数量，添加 quantity（数字）和 unit（单位）
3. 置信度 confidence（0-1），基于匹配确信度
4. 只输出 JSON，不要其他文字

# 文本
{}

# 输出格式
{{"events": [{{"action": "行为", "target": "对象", "quantity": 数量, "unit": "单位", "confidence": 0.9}}]}}

# 示例
输入: "今天吃了3个苹果"
输出: {{"events": [{{"action": "吃", "target": "苹果", "quantity": 3, "unit": "个", "confidence": 0.9}}]}}

输入: "去跑步"
输出: {{"events": [{{"action": "去", "target": "跑步", "quantity": null, "unit": null, "confidence": 0.7}}]}}"#,
            text
        )
    }

    /// 解析 SLM 响应
    fn parse_slm_response(&self, response: &str) -> Result<Vec<ExtractedEvent>> {
        // 尝试解析 JSON
        #[derive(Deserialize)]
        struct SlmOutput {
            events: Vec<SlmEvent>,
        }

        #[derive(Deserialize)]
        struct SlmEvent {
            action: String,
            target: String,
            quantity: Option<f64>,
            unit: Option<String>,
            confidence: f64,
        }

        let output: SlmOutput = serde_json::from_str(response).map_err(|e| {
            tracing::warn!("Failed to parse SLM JSON response: {}", e);
            tracing::warn!("Response was: {}", response);
            crate::DirSoulError::Config(format!("Invalid JSON from SLM: {}", e))
        })?;

        let mut events = Vec::new();
        for slm_event in output.events {
            let mut event = ExtractedEvent::new(slm_event.action, slm_event.target)
                .with_confidence(slm_event.confidence)
                .with_method("slm".to_string());

            if let (Some(q), Some(u)) = (slm_event.quantity, slm_event.unit) {
                event = event.with_quantity(q, u);
            }

            events.push(event);
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_with_quantity() {
        let extractor = RuleExtractor::new();
        let events = extractor.extract("今天吃了3个苹果").unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "吃");
        assert_eq!(events[0].target, "苹果");
        assert_eq!(events[0].quantity, Some(3.0));
        assert_eq!(events[0].unit, Some("个".to_string()));
    }

    #[test]
    fn test_extract_without_quantity() {
        let extractor = RuleExtractor::new();
        let events = extractor.extract("去跑步").unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "去");
        assert_eq!(events[0].target, "跑步");
        assert!(events[0].quantity.is_none());
    }

    #[test]
    fn test_extract_buy_book() {
        let extractor = RuleExtractor::new();
        let events = extractor.extract("买了1本书").unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "购买");
        assert_eq!(events[0].target, "书");
        assert_eq!(events[0].quantity, Some(1.0));
    }

    #[test]
    fn test_time_parser_today() {
        let parser = TimeParser::new();
        let today = parser.parse("今天").unwrap();

        // 应该是今天的 00:00:00
        let now = Local::now().date_naive();
        let expected = parser.with_time_zero(now);

        assert_eq!(today.date_naive(), expected.date_naive());
    }

    #[test]
    fn test_time_parser_yesterday() {
        let parser = TimeParser::new();
        let yesterday = parser.parse("昨天").unwrap();

        let now = Local::now();
        let expected_date = now.date_naive() - Duration::days(1);

        assert_eq!(yesterday.date_naive(), expected_date);
    }

    #[test]
    fn test_time_parser_days_ago() {
        let parser = TimeParser::new();
        let three_days_ago = parser.parse("3天前").unwrap();

        let now = Local::now();
        let expected_date = now.date_naive() - Duration::days(3);

        assert_eq!(three_days_ago.date_naive(), expected_date);
    }

    #[test]
    fn test_time_parser_this_week() {
        let parser = TimeParser::new();
        // 这个测试依赖当前星期几，只验证不返回None
        let result = parser.parse("周三");
        assert!(result.is_some());
    }

    #[test]
    fn test_time_parser_today_afternoon() {
        let parser = TimeParser::new();
        let afternoon = parser.parse("今天下午").unwrap();

        let now = Local::now().date_naive();
        let expected = parser.naive_to_utc(now, 14);

        assert_eq!(afternoon, expected);
    }

    #[test]
    fn test_has_time_info() {
        let extractor = RuleExtractor::new();

        assert!(extractor.has_time_info("今天吃了苹果"));
        assert!(extractor.has_time_info("昨天去跑步"));
        assert!(extractor.has_time_info("上周三"));
        assert!(!extractor.has_time_info("吃苹果"));
    }

    #[test]
    fn test_extract_chinese_number() {
        let extractor = RuleExtractor::new();
        let qty = extractor.parse_quantity("三").unwrap();

        assert_eq!(qty, 3.0);
    }

    #[test]
    fn test_extract_arabic_number() {
        let extractor = RuleExtractor::new();
        let qty = extractor.parse_quantity("42").unwrap();

        assert_eq!(qty, 42.0);
    }

    #[test]
    fn test_extracted_event_builder() {
        let event = ExtractedEvent::new("吃".to_string(), "苹果".to_string())
            .with_quantity(3.0, "个".to_string())
            .with_confidence(0.8)
            .with_method("test".to_string());

        assert_eq!(event.action, "吃");
        assert_eq!(event.target, "苹果");
        assert_eq!(event.quantity, Some(3.0));
        assert_eq!(event.unit, Some("个".to_string()));
        assert_eq!(event.confidence, 0.8);
        assert_eq!(event.method, "test");
    }
}
