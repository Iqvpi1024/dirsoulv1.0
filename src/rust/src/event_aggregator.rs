//! DirSoul Event Aggregation Module
//!
//! 时间范围聚合和统计功能
//! 遵循 HEAD.md 原则：支持时间范围查询、数量结构化存储
//!
//! # 设计原则
//! - 灵活的时间范围查询
//! - 多种聚合类型（SUM/COUNT/AVG）
//! - 高效的数据库查询

use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::Result;
use crate::schema::event_memories;

/// 聚合类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationType {
    /// 求和（用于 quantity）
    Sum,
    /// 计数（事件数量）
    Count,
    /// 平均值（用于 quantity）
    Avg,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeRange {
    /// 今天
    Today,
    /// 昨天
    Yesterday,
    /// 最近 N 天
    LastDays(i64),
    /// 本周
    ThisWeek,
    /// 上周
    LastWeek,
    /// 本月
    ThisMonth,
    /// 上月
    LastMonth,
    /// 自定义范围
    Custom(DateTime<Utc>, DateTime<Utc>),
}

/// 聚合结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    /// 聚合类型
    pub agg_type: AggregationType,
    /// 聚合值
    pub value: f64,
    /// 事件数量
    pub count: i64,
}

/// 事件聚合器
///
/// 提供时间范围聚合和统计功能
pub struct EventAggregator;

impl EventAggregator {
    /// 聚合事件
    ///
    /// # 参数
    /// * `conn` - 数据库连接
    /// * `user_id` - 用户 ID
    /// * `action` - 动作过滤（可选）
    /// * `target` - 目标过滤（可选）
    /// * `time_range` - 时间范围
    /// * `agg_type` - 聚合类型
    ///
    /// # 返回
    /// 聚合结果
    ///
    /// # 示例
    /// ```no_run
    /// # use dirsoul::event_aggregator::{EventAggregator, AggregationType, TimeRange};
    /// # async fn example() -> dirsoul::Result<()> {
    /// # let conn: &mut diesel::PgConnection = unimplemented!();
    /// let result = EventAggregator::aggregate_events(
    ///     conn,
    ///     "user123",
    ///     Some("吃"),
    ///     Some("苹果"),
    ///     &TimeRange::ThisWeek,
    ///     AggregationType::Sum
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn aggregate_events(
        conn: &mut PgConnection,
        user_id: &str,
        action: Option<&str>,
        target: Option<&str>,
        time_range: &TimeRange,
        agg_type: AggregationType,
    ) -> Result<AggregationResult> {
        let (start, end) = Self::parse_time_range(time_range);

        debug!(
            "Aggregating events: user={}, action={:?}, target={:?}, range={:?}:{:?}, agg_type={:?}",
            user_id, action, target, start, end, agg_type
        );

        let mut query = event_memories::table
            .filter(event_memories::user_id.eq(user_id))
            .filter(event_memories::timestamp.ge(start))
            .filter(event_memories::timestamp.le(end))
            .into_boxed();

        if let Some(a) = action {
            query = query.filter(event_memories::action.eq(a));
        }

        if let Some(t) = target {
            query = query.filter(event_memories::target.eq(t));
        }

        match agg_type {
            AggregationType::Count => {
                let count: i64 = query.count().get_result(conn)?;
                Ok(AggregationResult {
                    agg_type,
                    value: count as f64,
                    count,
                })
            }
            AggregationType::Sum => {
                // 使用 diesel 的 sum 函数
                use diesel::dsl::sql;

                // SUM quantity 过滤 NULL 值
                let sum: Option<f64> = query
                    .select(sql::<diesel::sql_types::Float8>(
                        "COALESCE(SUM(quantity), 0)"
                    ))
                    .first(conn)
                    .optional()?;

                // 重新构造查询以获取 count
                let count_query = event_memories::table
                    .filter(event_memories::user_id.eq(user_id))
                    .filter(event_memories::timestamp.ge(start))
                    .filter(event_memories::timestamp.le(end))
                    .into_boxed();

                let mut count_query = if let Some(a) = action {
                    count_query.filter(event_memories::action.eq(a))
                } else {
                    count_query
                };

                let count_query = if let Some(t) = target {
                    count_query.filter(event_memories::target.eq(t))
                } else {
                    count_query
                };

                let count: i64 = count_query.count().get_result(conn)?;

                Ok(AggregationResult {
                    agg_type,
                    value: sum.unwrap_or(0.0),
                    count,
                })
            }
            AggregationType::Avg => {
                // AVG quantity 过滤 NULL 值
                let avg: Option<f64> = query
                    .filter(event_memories::quantity.is_not_null())
                    .select(diesel::dsl::sql::<diesel::sql_types::Float8>(
                        "AVG(quantity)"
                    ))
                    .first(conn)
                    .optional()?;

                // 重新构造查询以获取 count
                let count_query = event_memories::table
                    .filter(event_memories::user_id.eq(user_id))
                    .filter(event_memories::timestamp.ge(start))
                    .filter(event_memories::timestamp.le(end))
                    .filter(event_memories::quantity.is_not_null())
                    .into_boxed();

                let mut count_query = if let Some(a) = action {
                    count_query.filter(event_memories::action.eq(a))
                } else {
                    count_query
                };

                let count_query = if let Some(t) = target {
                    count_query.filter(event_memories::target.eq(t))
                } else {
                    count_query
                };

                let count: i64 = count_query.count().get_result(conn)?;

                Ok(AggregationResult {
                    agg_type,
                    value: avg.unwrap_or(0.0),
                    count,
                })
            }
        }
    }

    /// 解析时间范围为 DateTime 范围
    fn parse_time_range(range: &TimeRange) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Local::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_start = DateTime::<Local>::from_naive_utc_and_offset(
            today_start,
            *now.offset()
        ).with_timezone(&Utc);

        match range {
            TimeRange::Today => {
                let today_end = today_start + Duration::days(1) - Duration::nanoseconds(1);
                (today_start, today_end)
            }
            TimeRange::Yesterday => {
                let yesterday_start = today_start - Duration::days(1);
                let yesterday_end = yesterday_start + Duration::days(1) - Duration::nanoseconds(1);
                (yesterday_start, yesterday_end)
            }
            TimeRange::LastDays(n) => {
                let start = today_start - Duration::days(*n);
                (start, now.with_timezone(&Utc))
            }
            TimeRange::ThisWeek => {
                // 周一到周日
                let weekday = now.weekday().num_days_from_monday();
                let monday = today_start - Duration::days(weekday as i64);
                let sunday = monday + Duration::days(7) - Duration::nanoseconds(1);
                (monday, sunday)
            }
            TimeRange::LastWeek => {
                // 上周一到上周日
                let weekday = now.weekday().num_days_from_monday();
                let this_monday = today_start - Duration::days(weekday as i64);
                let last_monday = this_monday - Duration::days(7);
                let last_sunday = this_monday - Duration::nanoseconds(1);
                (last_monday, last_sunday)
            }
            TimeRange::ThisMonth => {
                // 本月1号到月底
                let year = now.year();
                let month = now.month();
                let month_start = DateTime::<Local>::from_naive_utc_and_offset(
                    chrono::NaiveDate::from_ymd_opt(year, month, 1)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap(),
                    *now.offset()
                ).with_timezone(&Utc);

                let next_month = if month == 12 {
                    chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
                } else {
                    chrono::NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
                };
                let month_end = DateTime::<Local>::from_naive_utc_and_offset(
                    next_month.and_hms_opt(0, 0, 0).unwrap(),
                    *now.offset()
                ).with_timezone(&Utc) - Duration::nanoseconds(1);

                (month_start, month_end)
            }
            TimeRange::LastMonth => {
                // 上月1号到上月月底
                let year = now.year();
                let month = now.month();

                let (last_month_year, last_month) = if month == 1 {
                    (year - 1, 12)
                } else {
                    (year, month - 1)
                };

                let last_month_start = DateTime::<Local>::from_naive_utc_and_offset(
                    chrono::NaiveDate::from_ymd_opt(last_month_year, last_month, 1)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap(),
                    *now.offset()
                ).with_timezone(&Utc);

                let this_month_start = DateTime::<Local>::from_naive_utc_and_offset(
                    chrono::NaiveDate::from_ymd_opt(year, month, 1)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap(),
                    *now.offset()
                ).with_timezone(&Utc) - Duration::nanoseconds(1);

                (last_month_start, this_month_start)
            }
            TimeRange::Custom(start, end) => (*start, *end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_today() {
        let (start, end) = EventAggregator::parse_time_range(&TimeRange::Today);

        // 验证开始是今天 00:00:00
        let now = Local::now();
        let expected_start = DateTime::<Local>::from_naive_utc_and_offset(
            now.date_naive().and_hms_opt(0, 0, 0).unwrap(),
            *now.offset()
        ).with_timezone(&Utc);

        assert_eq!(start.date_naive(), expected_start.date_naive());
        assert_eq!(start.hour(), 0);
        assert_eq!(start.minute(), 0);
        assert_eq!(start.second(), 0);

        // 验证结束是今天 23:59:59.999999999
        assert_eq!(end.date_naive(), expected_start.date_naive());
    }

    #[test]
    fn test_time_range_last_days() {
        let (start, end) = EventAggregator::parse_time_range(&TimeRange::LastDays(7));

        // 验证范围是 7 天
        let duration = end.signed_duration_since(start);
        assert!(duration.num_days() >= 6);
        assert!(duration.num_days() <= 8);
    }

    #[test]
    fn test_time_range_this_week() {
        let (start, end) = EventAggregator::parse_time_range(&TimeRange::ThisWeek);

        // 验证范围大约是 7 天
        let duration = end.signed_duration_since(start);
        assert_eq!(duration.num_days(), 6); // 7 天 = 6 天的差异 + 1
    }

    #[test]
    fn test_aggregation_type_serialization() {
        let agg = AggregationType::Sum;
        let serialized = serde_json::to_string(&agg).unwrap();
        assert_eq!(serialized, "\"Sum\"");
    }
}
