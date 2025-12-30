//! Metrics storage operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use chrono::{DateTime, Utc};
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::StoredMetric;

impl SqliteStorage {
    /// Save a metric to the database.
    pub async fn save_metric(&self, metric: &StoredMetric) -> Result<i64, StorageError> {
        let created_at_str = metric.created_at.to_rfc3339();
        let success_i32: i32 = i32::from(metric.success);

        let result = sqlx::query(
            "INSERT INTO metrics (mode, tool_name, latency_ms, success, error_message, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&metric.mode)
        .bind(&metric.tool_name)
        .bind(metric.latency_ms)
        .bind(success_i32)
        .bind(&metric.error_message)
        .bind(&created_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("INSERT metrics", format!("{e}")))?;

        Ok(result.last_insert_rowid())
    }

    /// Get metrics by mode.
    pub async fn get_metrics_by_mode(&self, mode: &str) -> Result<Vec<StoredMetric>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, mode, tool_name, latency_ms, success, error_message, created_at
             FROM metrics WHERE mode = ? ORDER BY created_at DESC",
        )
        .bind(mode)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT metrics", format!("{e}")))?;

        let mut metrics = Vec::with_capacity(rows.len());
        for row in &rows {
            metrics.push(Self::row_to_metric(row)?);
        }

        Ok(metrics)
    }

    /// Get metrics within a time range.
    pub async fn get_metrics_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<StoredMetric>, StorageError> {
        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, mode, tool_name, latency_ms, success, error_message, created_at
             FROM metrics WHERE created_at >= ? AND created_at <= ? ORDER BY created_at DESC",
        )
        .bind(&start_str)
        .bind(&end_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT metrics", format!("{e}")))?;

        let mut metrics = Vec::with_capacity(rows.len());
        for row in &rows {
            metrics.push(Self::row_to_metric(row)?);
        }

        Ok(metrics)
    }

    /// Get recent metrics (last N).
    pub async fn get_recent_metrics(&self, limit: u32) -> Result<Vec<StoredMetric>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, mode, tool_name, latency_ms, success, error_message, created_at
             FROM metrics ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT metrics", format!("{e}")))?;

        let mut metrics = Vec::with_capacity(rows.len());
        for row in &rows {
            metrics.push(Self::row_to_metric(row)?);
        }

        Ok(metrics)
    }

    /// Convert a database row to a `StoredMetric`.
    fn row_to_metric(row: &sqlx::sqlite::SqliteRow) -> Result<StoredMetric, StorageError> {
        let id: i64 = row.get("id");
        let mode: String = row.get("mode");
        let tool_name: String = row.get("tool_name");
        let latency_ms: i64 = row.get("latency_ms");
        let success: i32 = row.get("success");
        let error_message: Option<String> = row.get("error_message");
        let created_at_str: String = row.get("created_at");

        let created_at = Self::parse_datetime(&created_at_str)?;

        Ok(StoredMetric {
            id: Some(id),
            mode,
            tool_name,
            latency_ms,
            success: success != 0,
            error_message,
            created_at,
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_save_metric() {
        let storage = test_storage().await;

        let metric = StoredMetric::success("linear", "reasoning_linear", 150);
        let result = storage.save_metric(&metric).await;

        assert!(result.is_ok());
        let id = result.expect("result");
        assert!(id > 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_save_metric_failure() {
        let storage = test_storage().await;

        let metric = StoredMetric::failure("graph", "reasoning_graph", 50, "API error");
        let result = storage.save_metric(&metric).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_metrics_by_mode() {
        let storage = test_storage().await;

        let metric1 = StoredMetric::success("linear", "reasoning_linear", 100);
        let metric2 = StoredMetric::success("linear", "reasoning_linear", 150);
        let metric3 = StoredMetric::success("tree", "reasoning_tree", 200);

        storage.save_metric(&metric1).await.expect("save 1");
        storage.save_metric(&metric2).await.expect("save 2");
        storage.save_metric(&metric3).await.expect("save 3");

        let metrics = storage.get_metrics_by_mode("linear").await;
        assert!(metrics.is_ok());
        let metrics = metrics.expect("metrics");
        assert_eq!(metrics.len(), 2);
        assert!(metrics.iter().all(|m| m.mode == "linear"));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_metrics_in_range() {
        let storage = test_storage().await;

        let metric = StoredMetric::success("linear", "reasoning_linear", 100);
        storage.save_metric(&metric).await.expect("save");

        let start = Utc::now() - chrono::Duration::hours(1);
        let end = Utc::now() + chrono::Duration::hours(1);

        let metrics = storage.get_metrics_in_range(start, end).await;
        assert!(metrics.is_ok());
        let metrics = metrics.expect("metrics");
        assert!(!metrics.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_recent_metrics() {
        let storage = test_storage().await;

        let metric1 = StoredMetric::success("linear", "reasoning_linear", 100);
        let metric2 = StoredMetric::success("tree", "reasoning_tree", 150);
        let metric3 = StoredMetric::success("graph", "reasoning_graph", 200);

        storage.save_metric(&metric1).await.expect("save 1");
        storage.save_metric(&metric2).await.expect("save 2");
        storage.save_metric(&metric3).await.expect("save 3");

        let metrics = storage.get_recent_metrics(2).await;
        assert!(metrics.is_ok());
        let metrics = metrics.expect("metrics");
        assert_eq!(metrics.len(), 2);
    }
}
