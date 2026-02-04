//! HTTP API Server for Python/Rust Bridge
//!
//! This module provides a simple HTTP server that allows the Python Streamlit
//! interface to communicate with the Rust core.
//!
//! # Design Principles (HEAD.md)
//! - **V1策略**: subprocess简单调用
//! - **快速原型**: Streamlit界面 + Rust core
//!
//! # Example
//! ```text
//! use dirsoul::http_api::HttpServer;
//!
//! let server = HttpServer::new("127.0.0.1:8080".to_string(), "postgresql://localhost/dirsoul".to_string())?;
//! server.start().await?;
//! ```

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;

use crate::audit::ThreadSafeAuditLogger;
use crate::error::{DirSoulError, Result};
use crate::llm_provider::ChatMessage;
use crate::models::{EventMemory, Entity, RawMemory, NewRawMemory};
use crate::schema::{event_memories, entities, raw_memories};

/// Chat request from Python
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// User input
    pub message: String,

    /// User ID
    pub user_id: String,

    /// Conversation history
    pub history: Vec<ChatMessage>,

    /// Optional context
    pub context: Option<serde_json::Value>,
}

/// Chat response to Python (renamed to avoid conflict with llm_provider::ChatResponse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChatResponse {
    /// Response text
    pub response: String,

    /// Updated conversation history
    pub history: Vec<ChatMessage>,

    /// Memories recorded (IDs)
    pub recorded_memory_ids: Vec<String>,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Timeline request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineRequest {
    /// User ID
    pub user_id: String,

    /// Start date (ISO format)
    pub start_date: String,

    /// End date (ISO format)
    pub end_date: String,

    /// Filters
    pub filters: Option<TimelineFilters>,
}

/// Timeline filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineFilters {
    /// Filter by entity
    pub entities: Option<Vec<String>>,

    /// Filter by event type
    pub event_types: Option<Vec<String>>,

    /// Minimum confidence
    pub min_confidence: Option<f64>,
}

/// Timeline event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Event ID
    pub event_id: String,

    /// Timestamp
    pub timestamp: String,

    /// Actor
    pub actor: Option<String>,

    /// Action
    pub action: String,

    /// Target
    pub target: String,

    /// Quantity
    pub quantity: Option<f64>,

    /// Unit
    pub unit: Option<String>,

    /// Confidence
    pub confidence: f64,

    /// Related entities
    pub entities: Vec<String>,
}

/// Timeline response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineResponse {
    /// Events grouped by date
    pub events_by_date: HashMap<String, Vec<TimelineEvent>>,

    /// Total event count
    pub total_events: usize,

    /// Summary statistics
    pub summary: TimelineSummary,
}

/// Timeline summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSummary {
    /// Total days in range
    pub total_days: i64,

    /// Average events per day
    pub avg_events_per_day: f64,

    /// Most active date
    pub most_active_date: String,

    /// Top entities
    pub top_entities: Vec<String>,
}

/// Statistics request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsRequest {
    /// User ID
    pub user_id: String,

    /// Time range: "7d", "30d", "90d", "all"
    pub time_range: String,
}

/// Statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResponse {
    /// Total memories
    pub total_memories: usize,

    /// Total events
    pub total_events: usize,

    /// Total entities
    pub total_entities: usize,

    /// Events per day
    pub events_per_day: HashMap<String, i64>,

    /// Event type distribution
    pub event_types: HashMap<String, i64>,

    /// Entity frequency
    pub entities: Vec<EntityStat>,

    /// Time range stats
    pub time_range: TimeRangeStats,
}

/// Entity statistic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStat {
    /// Entity name
    pub name: String,

    /// Frequency
    pub frequency: i64,

    /// First seen
    pub first_seen: String,

    /// Last seen
    pub last_seen: String,
}

/// Time range statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRangeStats {
    /// Start date
    pub start_date: String,

    /// End date
    pub end_date: String,

    /// Total days
    pub total_days: i64,

    /// Most active day
    pub most_active_day: String,

    /// Least active day
    pub least_active_day: String,
}

/// HTTP API server
pub struct HttpServer {
    /// Bind address
    bind_address: String,
    /// Database URL
    database_url: String,
    /// In-memory data store (for demo purposes)
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// Audit logger for recording all operations
    audit_logger: Arc<ThreadSafeAuditLogger>,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(bind_address: String, database_url: String) -> Result<Self> {
        let audit_logger = Arc::new(ThreadSafeAuditLogger::new(database_url.clone()));

        Ok(Self {
            bind_address,
            database_url,
            data: Arc::new(RwLock::new(HashMap::new())),
            audit_logger,
        })
    }

    /// Process chat message - V3 Simplified (Client-side history)
    /// Uses client-provided history and calls LLM for semantic understanding
    fn process_chat(&self, req: ChatRequest) -> Result<ApiChatResponse> {
        let start = std::time::Instant::now();

        // Build LLM prompt - 只包含年龄计算的few-shot
        let mut conversation = String::from(r#"今年25→明年26。今年30→明年31。
"#);

        // 只发送最近2轮对话（4条消息）- 倒序排列
        let recent_count = 4;
        let start_idx = if req.history.len() > recent_count {
            req.history.len() - recent_count
        } else {
            0
        };

        // 收集最近的对话
        let recent_messages: Vec<_> = req.history.iter().skip(start_idx).collect();

        // 倒序添加历史（最新的在最前面）
        for msg in recent_messages.iter().rev() {
            conversation.push_str(&format!("{}: {}\n",
                if msg.role == "user" { "用户" } else { "助手" },
                msg.content
            ));
        }

        // 添加最新消息
        conversation.push_str(&format!("用户: {}\n", req.message));

        // 添加简短提示
        conversation.push_str("回答（10字内）：\n");

        // Call LLM - 使用qwen2:0.5b
        let ollama_url = format!("{}/api/generate", "http://localhost:11434");
        let ollama_request = serde_json::json!({
            "model": "qwen2:0.5b",
            "prompt": conversation,
            "stream": false,
            "options": {
                "num_predict": 30,
                "temperature": 0.7
            }
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| DirSoulError::Config(format!("Failed to create HTTP client: {}", e)))?;

        let response_text = match client
            .post(&ollama_url)
            .header("Content-Type", "application/json")
            .body(ollama_request.to_string())
            .send() {
            Ok(resp) => {
                eprintln!("HTTP status: {}", resp.status());
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>() {
                        Ok(json) => {
                            eprintln!("JSON response: {:?}", json);
                            let raw = json["response"].as_str()
                                .unwrap_or("").trim();
                            if raw.is_empty() {
                                "我收到你的消息了。".to_string()
                            } else {
                                raw.to_string()
                            }
                        }
                        Err(e) => {
                            eprintln!("JSON error: {:?}", e);
                            "我收到你的消息了。".to_string()
                        }
                    }
                } else {
                    let body = resp.text().unwrap_or_default();
                    eprintln!("HTTP error, body: {}", body);
                    "我收到你的消息了。".to_string()
                }
            }
            Err(e) => {
                eprintln!("Request error: {:?}", e);
                "我收到你的消息了。".to_string()
            }
        };

        // Update history
        let mut updated_history = req.history.clone();
        updated_history.push(ChatMessage {
            role: "user".to_string(),
            content: req.message.clone(),
        });
        updated_history.push(ChatMessage {
            role: "assistant".to_string(),
            content: response_text.clone(),
        });

        Ok(ApiChatResponse {
            response: response_text,
            history: updated_history,
            recorded_memory_ids: vec![],
            processing_time_ms: start.elapsed().as_millis() as u64,
            metadata: Some(serde_json::json!({
                "version": "3.0.0",
                "mode": "client-history+llm",
                "model": "qwen2:0.5b"
            })),
        })
    }

    /// Query timeline events from database
    fn query_timeline(&self, user_id: &str, start_date: &str, end_date: &str) -> Result<Vec<EventMemory>> {
        let mut conn = PgConnection::establish(&self.database_url)?;

        // Parse date strings - support both date (YYYY-MM-DD) and datetime (RFC3339) formats
        let start = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start_date) {
            dt.with_timezone(&chrono::Utc)
        } else {
            // Try parsing as date only
            let date = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
                .map_err(|e| DirSoulError::Config(format!("Invalid start_date: {}", e)))?;
            date.and_hms_opt(0, 0, 0)
                .ok_or_else(|| DirSoulError::Config("Invalid start_date".to_string()))?
                .and_utc()
        };

        let end = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(end_date) {
            dt.with_timezone(&chrono::Utc)
        } else {
            // Try parsing as date only
            let date = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
                .map_err(|e| DirSoulError::Config(format!("Invalid end_date: {}", e)))?;
            date.and_hms_opt(23, 59, 59)
                .ok_or_else(|| DirSoulError::Config("Invalid end_date".to_string()))?
                .and_utc()
        };

        // Query events within time range
        let events = event_memories::table
            .filter(event_memories::user_id.eq(user_id))
            .filter(event_memories::timestamp.ge(start))
            .filter(event_memories::timestamp.le(end))
            .order(event_memories::timestamp.desc())
            .load::<EventMemory>(&mut conn)?;

        Ok(events)
    }

    /// Query statistics from database
    fn query_stats(&self, user_id: &str, time_range: &str) -> Result<StatsResponse> {
        let mut conn = PgConnection::establish(&self.database_url)?;

        // Calculate time range
        let (start, end) = match time_range {
            "7d" => {
                let start = chrono::Utc::now() - chrono::Duration::days(7);
                (start, chrono::Utc::now())
            }
            "30d" => {
                let start = chrono::Utc::now() - chrono::Duration::days(30);
                (start, chrono::Utc::now())
            }
            "90d" => {
                let start = chrono::Utc::now() - chrono::Duration::days(90);
                (start, chrono::Utc::now())
            }
            "all" => {
                // Query all time
                let start = chrono::DateTime::from_timestamp(0, 0).unwrap();
                (start, chrono::Utc::now())
            }
            _ => {
                return Err(DirSoulError::Config(format!(
                    "Invalid time_range: {}. Expected: 7d, 30d, 90d, or all",
                    time_range
                )));
            }
        };

        // Count total events
        let total_events: i64 = event_memories::table
            .filter(event_memories::user_id.eq(user_id))
            .filter(event_memories::timestamp.ge(start))
            .filter(event_memories::timestamp.le(end))
            .count()
            .get_result(&mut conn)?;

        // Count total entities
        let total_entities: i64 = entities::table
            .filter(entities::user_id.eq(user_id))
            .count()
            .get_result(&mut conn)?;

        // Get events per day
        let events: Vec<EventMemory> = event_memories::table
            .filter(event_memories::user_id.eq(user_id))
            .filter(event_memories::timestamp.ge(start))
            .filter(event_memories::timestamp.le(end))
            .load(&mut conn)?;

        let mut events_per_day = HashMap::new();
        for event in &events {
            let date = event.timestamp.format("%Y-%m-%d").to_string();
            *events_per_day.entry(date).or_insert(0) += 1;
        }

        // Get event type distribution
        let mut event_types = HashMap::new();
        for event in &events {
            *event_types.entry(event.action.clone()).or_insert(0) += 1;
        }

        // Calculate time range stats
        let total_days = (end.timestamp() - start.timestamp()) / 86400;
        let avg_events_per_day = if total_days > 0 {
            total_events as f64 / total_days as f64
        } else {
            0.0
        };

        let most_active_day = events_per_day
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(day, _)| day.clone())
            .unwrap_or_default();

        // Get top entities
        let entity_list = entities::table
            .filter(entities::user_id.eq(user_id))
            .limit(10)
            .load::<Entity>(&mut conn)?;

        let entities_stats: Vec<EntityStat> = entity_list
            .into_iter()
            .map(|e| EntityStat {
                name: e.canonical_name,
                frequency: 1, // TODO: calculate actual frequency
                first_seen: e.first_seen.to_rfc3339(),
                last_seen: e.last_seen.to_rfc3339(),
            })
            .collect();

        Ok(StatsResponse {
            total_memories: 0, // TODO: count from raw_memories
            total_events: total_events as usize,
            total_entities: total_entities as usize,
            events_per_day,
            event_types,
            entities: entities_stats,
            time_range: TimeRangeStats {
                start_date: start.format("%Y-%m-%d").to_string(),
                end_date: end.format("%Y-%m-%d").to_string(),
                total_days,
                most_active_day,
                least_active_day: String::new(),
            },
        })
    }

    /// Start the HTTP server (runs forever)
    pub async fn start(self) -> Result<()> {
        // CORS headers
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec![warp::http::Method::GET, warp::http::Method::POST]);

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| {
                warp::reply::json(&serde_json::json!({
                    "status": "healthy",
                    "service": "dirsoul-api",
                    "version": "1.0.0"
                }))
            });

        // Chat endpoint
        let db_url_chat = self.database_url.clone();
        let audit_logger_chat = self.audit_logger.clone();
        let chat = warp::path("api")
            .and(warp::path("chat"))
            .and(warp::post())
            .and(warp::filters::body::json())
            .map(move |req: ChatRequest| {
                let user_id = req.user_id.clone();
                let message_len = req.message.len();

                // Create a temporary server instance for processing
                let server = HttpServer {
                    bind_address: String::new(),
                    database_url: db_url_chat.clone(),
                    data: Arc::new(RwLock::new(HashMap::new())),
                    audit_logger: audit_logger_chat.clone(),
                };

                match server.process_chat(req) {
                    Ok(response) => {
                        // Extract result count before moving response
                        let result_count = response.recorded_memory_ids.len() as i32;

                        // Log the query asynchronously (don't block response)
                        let logger = audit_logger_chat.clone();
                        let user_id_clone = user_id.clone();
                        tokio::spawn(async move {
                            let _ = logger.log_query(
                                &user_id_clone,
                                "chat",
                                true,
                                result_count,
                            ).await;
                        });

                        warp::reply::json(&response)
                    }
                    Err(e) => {
                        // Log the failed query
                        let logger = audit_logger_chat.clone();
                        tokio::spawn(async move {
                            let _ = logger.log_query(&user_id, "chat", false, 0).await;
                        });

                        let error_response = ApiChatResponse {
                            response: format!("Error: {}", e),
                            history: vec![],
                            recorded_memory_ids: vec![],
                            processing_time_ms: 0,
                            metadata: None,
                        };
                        warp::reply::json(&error_response)
                    }
                }
            });

        // Timeline endpoint
        let db_url_timeline = self.database_url.clone();
        let audit_logger_timeline = self.audit_logger.clone();
        let timeline = warp::path("api")
            .and(warp::path("timeline"))
            .and(warp::post())
            .and(warp::filters::body::json())
            .map(move |req: TimelineRequest| {
                let user_id = req.user_id.clone();
                let start_date = req.start_date.clone();
                let end_date = req.end_date.clone();

                let server = HttpServer {
                    bind_address: String::new(),
                    database_url: db_url_timeline.clone(),
                    data: Arc::new(RwLock::new(HashMap::new())),
                    audit_logger: audit_logger_timeline.clone(),
                };

                match server.query_timeline(&req.user_id, &req.start_date, &req.end_date) {
                    Ok(events) => {
                        let result_count = events.len() as i32;

                        // Convert to timeline format
                        let mut events_by_date: HashMap<String, Vec<TimelineEvent>> = HashMap::new();
                        for event in events {
                            let date = event.timestamp.format("%Y-%m-%d").to_string();
                            let timeline_event = TimelineEvent {
                                event_id: event.id().to_string(),
                                timestamp: event.timestamp.to_rfc3339(),
                                actor: event.actor,
                                action: event.action,
                                target: event.target,
                                quantity: event.quantity,
                                unit: event.unit,
                                confidence: event.confidence,
                                entities: vec![], // TODO: query related entities
                            };
                            events_by_date.entry(date).or_insert_with(Vec::new).push(timeline_event);
                        }

                        let total_events = events_by_date.values().map(|v| v.len()).sum();

                        let response = TimelineResponse {
                            events_by_date,
                            total_events,
                            summary: TimelineSummary {
                                total_days: 0, // TODO: calculate from date range
                                avg_events_per_day: 0.0,
                                most_active_date: String::new(),
                                top_entities: vec![],
                            },
                        };

                        // Log the query asynchronously
                        let logger = audit_logger_timeline.clone();
                        tokio::spawn(async move {
                            let _ = logger.log_query(
                                &user_id,
                                &format!("timeline:{}:{}", start_date, end_date),
                                true,
                                result_count,
                            ).await;
                        });

                        warp::reply::json(&response)
                    }
                    Err(e) => {
                        // Log the failed query
                        let logger = audit_logger_timeline.clone();
                        tokio::spawn(async move {
                            let _ = logger.log_query(
                                &user_id,
                                &format!("timeline:{}:{}", start_date, end_date),
                                false,
                                0,
                            ).await;
                        });

                        let error_response = TimelineResponse {
                            events_by_date: HashMap::new(),
                            total_events: 0,
                            summary: TimelineSummary {
                                total_days: 0,
                                avg_events_per_day: 0.0,
                                most_active_date: format!("Error: {}", e),
                                top_entities: vec![],
                            },
                        };
                        warp::reply::json(&error_response)
                    }
                }
            });

        // Statistics endpoint
        let db_url_stats = self.database_url.clone();
        let audit_logger_stats = self.audit_logger.clone();
        let stats = warp::path("api")
            .and(warp::path("stats"))
            .and(warp::post())
            .and(warp::filters::body::json())
            .map(move |req: StatsRequest| {
                let user_id = req.user_id.clone();
                let time_range = req.time_range.clone();

                let server = HttpServer {
                    bind_address: String::new(),
                    database_url: db_url_stats.clone(),
                    data: Arc::new(RwLock::new(HashMap::new())),
                    audit_logger: audit_logger_stats.clone(),
                };

                match server.query_stats(&req.user_id, &req.time_range) {
                    Ok(response) => {
                        let result_count = response.total_events + response.total_memories;

                        // Log the query asynchronously
                        let logger = audit_logger_stats.clone();
                        tokio::spawn(async move {
                            let _ = logger.log_query(
                                &user_id,
                                &format!("stats:{}", time_range),
                                true,
                                result_count as i32,
                            ).await;
                        });

                        warp::reply::json(&response)
                    }
                    Err(e) => {
                        // Log the failed query
                        let logger = audit_logger_stats.clone();
                        tokio::spawn(async move {
                            let _ = logger.log_query(
                                &user_id,
                                &format!("stats:{}", time_range),
                                false,
                                0,
                            ).await;
                        });

                        let error_response = StatsResponse {
                            total_memories: 0,
                            total_events: 0,
                            total_entities: 0,
                            events_per_day: HashMap::new(),
                            event_types: HashMap::new(),
                            entities: vec![],
                            time_range: TimeRangeStats {
                                start_date: String::new(),
                                end_date: String::new(),
                                total_days: 0,
                                most_active_day: format!("Error: {}", e),
                                least_active_day: String::new(),
                            },
                        };
                        warp::reply::json(&error_response)
                    }
                }
            });

        // Combine routes
        let routes = health
            .or(chat)
            .or(timeline)
            .or(stats)
            .with(cors);

        // Start server
        let addr = self.bind_address.clone();
        println!("🚀 DirSoul API Server starting on {}", addr);
        println!("💬 Chat endpoint: http://{}/api/chat", addr);
        println!("📅 Timeline endpoint: http://{}/api/timeline", addr);
        println!("📊 Stats endpoint: http://{}/api/stats", addr);

        // Parse address
        let socket_addr: std::net::SocketAddr = addr.parse()
            .map_err(|e| DirSoulError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;

        // warp::serve().run() runs forever and returns (), so we can't use ?
        // It will only return if there's an error connecting
        warp::serve(routes).run(socket_addr).await;

        // This line is unreachable, but needed for type signature
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest {
            message: "Hello".to_string(),
            user_id: "test_user".to_string(),
            history: vec![],
            context: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        let _deserialized: ChatRequest = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_api_chat_response_serialization() {
        let resp = ApiChatResponse {
            response: "Hi".to_string(),
            history: vec![],
            recorded_memory_ids: vec![],
            processing_time_ms: 100,
            metadata: None,
        };

        let json = serde_json::to_string(&resp).unwrap();
        let _deserialized: ApiChatResponse = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_http_server_creation() {
        let server = HttpServer::new("127.0.0.1:8080".to_string(), "postgresql://localhost/test".to_string()).unwrap();
        assert_eq!(server.bind_address, "127.0.0.1:8080");
    }
}
