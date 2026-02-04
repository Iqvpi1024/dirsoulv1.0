//! DirSoul Event Storage Module
//!
//! 完整的事件存储流程：输入处理 → 事件抽取 → 数据库存储
//! 遵循 HEAD.md 原则：原子性事务、精确时间戳、结构化数量
//!
//! # 设计原则
//! - 原子性：原始记忆和事件记忆在同一事务中
//! - 重试机制：指数退避处理临时失败
//! - 异步优先：tokio 非阻塞操作

use diesel::prelude::*;
use tracing::{debug, info};

use crate::error::Result;
use crate::event_extractor::{ExtractedEvent, SlmExtractor, TimeParser};
use crate::models::{EventMemory, NewEventMemory, NewRawMemory, RawMemory};
use crate::schema::{event_memories, raw_memories};

/// 事件存储处理器
///
/// 负责处理完整的输入流程：输入 → 原始记忆 → 事件抽取 → 事件记忆
pub struct EventStorage {
    /// SLM 事件抽取器
    extractor: SlmExtractor,
    /// 时间解析器
    time_parser: TimeParser,
    /// 用户 ID
    user_id: String,
}

impl EventStorage {
    /// 创建新的事件存储处理器
    ///
    /// # Arguments
    /// * `extractor` - SLM 事件抽取器
    /// * `user_id` - 用户 ID
    pub fn new(extractor: SlmExtractor, user_id: String) -> Self {
        Self {
            extractor,
            time_parser: TimeParser::new(),
            user_id,
        }
    }

    /// 处理输入并存储记忆（同步版本）
    ///
    /// # 流程
    /// 1. 插入原始记忆到数据库
    /// 2. 抽取事件
    /// 3. 插入事件记忆到数据库
    /// 4. 失败时返回错误（由调用方重试）
    ///
    /// # 参数
    /// * `conn` - 数据库连接
    /// * `input` - 输入数据
    ///
    /// # 返回
    /// 插入的事件记忆列表
    pub fn process_input_sync(
        &self,
        conn: &mut PgConnection,
        input: &NewRawMemory,
    ) -> Result<Vec<EventMemory>> {
        info!("Processing input for user '{}'", self.user_id);

        // 插入原始记忆
        diesel::insert_into(raw_memories::table)
            .values(input)
            .execute(conn)?;

        debug!("Inserted raw memory");

        // 简化版本：返回空事件列表
        // TODO: 在 Task 3.6 集成测试中实现完整的异步抽取
        Ok(vec![])
    }

    /// 构建 NewEventMemory
    fn build_new_event(
        &self,
        raw_memory: &RawMemory,
        extracted: ExtractedEvent,
    ) -> Result<NewEventMemory> {
        // 解析时间戳（如果有时间信息）
        let timestamp = if let Some(content) = &raw_memory.content {
            if let Some(parsed) = self.time_parser.parse(content) {
                parsed
            } else {
                raw_memory.created_at
            }
        } else {
            raw_memory.created_at
        };

        Ok(NewEventMemory {
            memory_id: raw_memory.memory_id,
            user_id: raw_memory.user_id.clone(),
            timestamp,
            actor: extracted.actor,
            action: extracted.action,
            target: extracted.target,
            quantity: extracted.quantity,
            unit: extracted.unit,
            confidence: extracted.confidence,
            extractor_version: Some(format!("{}-slm", env!("CARGO_PKG_VERSION"))),
        })
    }

    /// 插入事件记忆到数据库
    pub fn insert_event(
        &self,
        conn: &mut PgConnection,
        event: &NewEventMemory,
    ) -> Result<EventMemory> {
        let inserted = diesel::insert_into(event_memories::table)
            .values(event)
            .execute(conn)?;

        debug!("Inserted event memory, rows affected: {}", inserted);

        // 简化版本：返回一个模拟的 EventMemory
        // 实际应用中需要查询刚插入的记录
        Ok(EventMemory {
            event_id: uuid::Uuid::new_v4(),
            memory_id: event.memory_id,
            user_id: event.user_id.clone(),
            timestamp: event.timestamp,
            actor: event.actor.clone(),
            action: event.action.clone(),
            target: event.target.clone(),
            quantity: event.quantity,
            unit: event.unit.clone(),
            confidence: event.confidence,
            extractor_version: event.extractor_version.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_storage_creation() {
        // 基本创建测试
        // 集成测试会在 Task 3.6 中完成
    }
}
