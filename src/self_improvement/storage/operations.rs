//! Database operations for self-improvement system.
//!
//! This module contains the `SelfImprovementStorage` struct and its implementation
//! for CRUD operations on self-improvement records.

use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use super::helpers::{parse_action_status, parse_datetime, parse_diagnosis_status, parse_severity};
use super::records::{
    ActionRecord, ConfigOverrideRecord, DiagnosisRecord, InvocationRecord, InvocationStats,
    LearningRecord,
};
use crate::error::StorageError;
use crate::self_improvement::types::{ActionStatus, DiagnosisStatus};

/// Helper to create a QueryFailed error.
fn query_error(message: impl Into<String>) -> StorageError {
    StorageError::QueryFailed {
        query: "self_improvement".to_string(),
        message: message.into(),
    }
}

/// Storage interface for self-improvement system.
pub struct SelfImprovementStorage {
    pool: SqlitePool,
}

impl SelfImprovementStorage {
    /// Create a new storage instance.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------------
    // Invocation Operations
    // ------------------------------------------------------------------------

    /// Insert an invocation record.
    pub async fn insert_invocation(&self, record: &InvocationRecord) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO invocations (id, tool_name, latency_ms, success, quality_score, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&record.id)
        .bind(&record.tool_name)
        .bind(record.latency_ms)
        .bind(record.success)
        .bind(record.quality_score)
        .bind(record.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Batch insert invocation records.
    ///
    /// Uses chunked inserts to respect SQLite's 999 variable limit.
    /// Returns the total number of records inserted.
    pub async fn batch_insert_invocations(
        &self,
        records: &[InvocationRecord],
    ) -> Result<u64, StorageError> {
        // SQLite supports up to 999 variables per statement
        // Each record uses 6 bind variables: id, tool_name, latency_ms, success, quality_score, created_at
        const VARS_PER_RECORD: usize = 6;
        const MAX_VARS: usize = 999;
        const BATCH_SIZE: usize = MAX_VARS / VARS_PER_RECORD; // 166

        if records.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0u64;

        for chunk in records.chunks(BATCH_SIZE) {
            let placeholders: String = chunk
                .iter()
                .map(|_| "(?, ?, ?, ?, ?, ?)")
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "INSERT INTO invocations (id, tool_name, latency_ms, success, quality_score, created_at) VALUES {placeholders}"
            );

            let mut query = sqlx::query(&sql);
            for record in chunk {
                query = query
                    .bind(&record.id)
                    .bind(&record.tool_name)
                    .bind(record.latency_ms)
                    .bind(record.success)
                    .bind(record.quality_score)
                    .bind(record.created_at.to_rfc3339());
            }

            let result = query
                .execute(&self.pool)
                .await
                .map_err(|e| query_error(format!("Batch insert failed: {e}")))?;

            total_inserted += result.rows_affected();
        }

        Ok(total_inserted)
    }

    /// Get recent invocations.
    pub async fn get_recent_invocations(
        &self,
        limit: i64,
    ) -> Result<Vec<InvocationRecord>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT id, tool_name, latency_ms, success, quality_score, created_at
            FROM invocations
            ORDER BY created_at DESC
            LIMIT ?
            ",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let created_at_str: String = row.get("created_at");
            records.push(InvocationRecord {
                id: row.get("id"),
                tool_name: row.get("tool_name"),
                latency_ms: row.get("latency_ms"),
                success: row.get("success"),
                quality_score: row.get("quality_score"),
                created_at: parse_datetime(&created_at_str)?,
            });
        }

        Ok(records)
    }

    /// Get invocations for a specific tool.
    pub async fn get_invocations_by_tool(
        &self,
        tool_name: &str,
        limit: i64,
    ) -> Result<Vec<InvocationRecord>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT id, tool_name, latency_ms, success, quality_score, created_at
            FROM invocations
            WHERE tool_name = ?
            ORDER BY created_at DESC
            LIMIT ?
            ",
        )
        .bind(tool_name)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let created_at_str: String = row.get("created_at");
            records.push(InvocationRecord {
                id: row.get("id"),
                tool_name: row.get("tool_name"),
                latency_ms: row.get("latency_ms"),
                success: row.get("success"),
                quality_score: row.get("quality_score"),
                created_at: parse_datetime(&created_at_str)?,
            });
        }

        Ok(records)
    }

    // ------------------------------------------------------------------------
    // Diagnosis Operations
    // ------------------------------------------------------------------------

    /// Insert a diagnosis record.
    pub async fn insert_diagnosis(&self, record: &DiagnosisRecord) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO diagnoses (
                id, trigger_type, trigger_json, severity, description,
                suspected_cause, suggested_action_json, action_rationale, status, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&record.id)
        .bind(&record.trigger_type)
        .bind(&record.trigger_json)
        .bind(record.severity.to_string())
        .bind(&record.description)
        .bind(&record.suspected_cause)
        .bind(&record.suggested_action_json)
        .bind(&record.action_rationale)
        .bind(record.status.to_string())
        .bind(record.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Update diagnosis status.
    pub async fn update_diagnosis_status(
        &self,
        id: &str,
        status: DiagnosisStatus,
    ) -> Result<(), StorageError> {
        sqlx::query("UPDATE diagnoses SET status = ? WHERE id = ?")
            .bind(status.to_string())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Get diagnosis by ID.
    pub async fn get_diagnosis(&self, id: &str) -> Result<Option<DiagnosisRecord>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, trigger_type, trigger_json, severity, description,
                   suspected_cause, suggested_action_json, action_rationale, status, created_at
            FROM diagnoses
            WHERE id = ?
            ",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        match row {
            Some(row) => {
                let created_at_str: String = row.get("created_at");
                let severity_str: String = row.get("severity");
                let status_str: String = row.get("status");

                Ok(Some(DiagnosisRecord {
                    id: row.get("id"),
                    trigger_type: row.get("trigger_type"),
                    trigger_json: row.get("trigger_json"),
                    severity: parse_severity(&severity_str),
                    description: row.get("description"),
                    suspected_cause: row.get("suspected_cause"),
                    suggested_action_json: row.get("suggested_action_json"),
                    action_rationale: row.get("action_rationale"),
                    status: parse_diagnosis_status(&status_str),
                    created_at: parse_datetime(&created_at_str)?,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get pending diagnoses.
    pub async fn get_pending_diagnoses(&self) -> Result<Vec<DiagnosisRecord>, StorageError> {
        self.get_diagnoses_by_status(DiagnosisStatus::Pending).await
    }

    /// Get diagnoses by status.
    pub async fn get_diagnoses_by_status(
        &self,
        status: DiagnosisStatus,
    ) -> Result<Vec<DiagnosisRecord>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT id, trigger_type, trigger_json, severity, description,
                   suspected_cause, suggested_action_json, action_rationale, status, created_at
            FROM diagnoses
            WHERE status = ?
            ORDER BY created_at DESC
            ",
        )
        .bind(status.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let created_at_str: String = row.get("created_at");
            let severity_str: String = row.get("severity");
            let status_str: String = row.get("status");

            records.push(DiagnosisRecord {
                id: row.get("id"),
                trigger_type: row.get("trigger_type"),
                trigger_json: row.get("trigger_json"),
                severity: parse_severity(&severity_str),
                description: row.get("description"),
                suspected_cause: row.get("suspected_cause"),
                suggested_action_json: row.get("suggested_action_json"),
                action_rationale: row.get("action_rationale"),
                status: parse_diagnosis_status(&status_str),
                created_at: parse_datetime(&created_at_str)?,
            });
        }

        Ok(records)
    }

    // ------------------------------------------------------------------------
    // Action Operations
    // ------------------------------------------------------------------------

    /// Insert an action record.
    pub async fn insert_action(&self, record: &ActionRecord) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO si_actions (
                id, diagnosis_id, action_type, action_json, outcome,
                pre_metrics_json, post_metrics_json, execution_time_ms, error_message, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&record.id)
        .bind(&record.diagnosis_id)
        .bind(&record.action_type)
        .bind(&record.action_json)
        .bind(record.outcome.to_string())
        .bind(&record.pre_metrics_json)
        .bind(&record.post_metrics_json)
        .bind(record.execution_time_ms)
        .bind(&record.error_message)
        .bind(record.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Update action outcome and post-metrics.
    pub async fn update_action_outcome(
        &self,
        id: &str,
        outcome: ActionStatus,
        post_metrics_json: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r"
            UPDATE si_actions
            SET outcome = ?, post_metrics_json = ?, error_message = ?
            WHERE id = ?
            ",
        )
        .bind(outcome.to_string())
        .bind(post_metrics_json)
        .bind(error_message)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Get action by ID.
    pub async fn get_action(&self, id: &str) -> Result<Option<ActionRecord>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, diagnosis_id, action_type, action_json, outcome,
                   pre_metrics_json, post_metrics_json, execution_time_ms, error_message, created_at
            FROM si_actions
            WHERE id = ?
            ",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        match row {
            Some(row) => {
                let created_at_str: String = row.get("created_at");
                let outcome_str: String = row.get("outcome");

                Ok(Some(ActionRecord {
                    id: row.get("id"),
                    diagnosis_id: row.get("diagnosis_id"),
                    action_type: row.get("action_type"),
                    action_json: row.get("action_json"),
                    outcome: parse_action_status(&outcome_str),
                    pre_metrics_json: row.get("pre_metrics_json"),
                    post_metrics_json: row.get("post_metrics_json"),
                    execution_time_ms: row.get("execution_time_ms"),
                    error_message: row.get("error_message"),
                    created_at: parse_datetime(&created_at_str)?,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get actions by outcome.
    pub async fn get_actions_by_outcome(
        &self,
        outcome: ActionStatus,
        limit: i64,
    ) -> Result<Vec<ActionRecord>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT id, diagnosis_id, action_type, action_json, outcome,
                   pre_metrics_json, post_metrics_json, execution_time_ms, error_message, created_at
            FROM si_actions
            WHERE outcome = ?
            ORDER BY created_at DESC
            LIMIT ?
            ",
        )
        .bind(outcome.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let created_at_str: String = row.get("created_at");
            let outcome_str: String = row.get("outcome");

            records.push(ActionRecord {
                id: row.get("id"),
                diagnosis_id: row.get("diagnosis_id"),
                action_type: row.get("action_type"),
                action_json: row.get("action_json"),
                outcome: parse_action_status(&outcome_str),
                pre_metrics_json: row.get("pre_metrics_json"),
                post_metrics_json: row.get("post_metrics_json"),
                execution_time_ms: row.get("execution_time_ms"),
                error_message: row.get("error_message"),
                created_at: parse_datetime(&created_at_str)?,
            });
        }

        Ok(records)
    }

    // ------------------------------------------------------------------------
    // Learning Operations
    // ------------------------------------------------------------------------

    /// Insert a learning record.
    pub async fn insert_learning(&self, record: &LearningRecord) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO learnings (
                id, action_id, reward_value, reward_breakdown_json,
                confidence, lessons_json, recommendations_json, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&record.id)
        .bind(&record.action_id)
        .bind(record.reward_value)
        .bind(&record.reward_breakdown_json)
        .bind(record.confidence)
        .bind(&record.lessons_json)
        .bind(&record.recommendations_json)
        .bind(record.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Batch insert learning records.
    ///
    /// Uses chunked inserts to respect SQLite's 999 variable limit.
    /// Returns the total number of records inserted.
    pub async fn batch_insert_learnings(
        &self,
        records: &[LearningRecord],
    ) -> Result<u64, StorageError> {
        // Each record uses 8 bind variables
        const VARS_PER_RECORD: usize = 8;
        const MAX_VARS: usize = 999;
        const BATCH_SIZE: usize = MAX_VARS / VARS_PER_RECORD; // 124

        if records.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0u64;

        for chunk in records.chunks(BATCH_SIZE) {
            let placeholders: String = chunk
                .iter()
                .map(|_| "(?, ?, ?, ?, ?, ?, ?, ?)")
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "INSERT INTO learnings (id, action_id, reward_value, reward_breakdown_json, confidence, lessons_json, recommendations_json, created_at) VALUES {placeholders}"
            );

            let mut query = sqlx::query(&sql);
            for record in chunk {
                query = query
                    .bind(&record.id)
                    .bind(&record.action_id)
                    .bind(record.reward_value)
                    .bind(&record.reward_breakdown_json)
                    .bind(record.confidence)
                    .bind(&record.lessons_json)
                    .bind(&record.recommendations_json)
                    .bind(record.created_at.to_rfc3339());
            }

            let result = query
                .execute(&self.pool)
                .await
                .map_err(|e| query_error(format!("Batch insert failed: {e}")))?;

            total_inserted += result.rows_affected();
        }

        Ok(total_inserted)
    }

    /// Get learning by action ID.
    pub async fn get_learning_by_action(
        &self,
        action_id: &str,
    ) -> Result<Option<LearningRecord>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, action_id, reward_value, reward_breakdown_json,
                   confidence, lessons_json, recommendations_json, created_at
            FROM learnings
            WHERE action_id = ?
            ",
        )
        .bind(action_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        match row {
            Some(row) => {
                let created_at_str: String = row.get("created_at");

                Ok(Some(LearningRecord {
                    id: row.get("id"),
                    action_id: row.get("action_id"),
                    reward_value: row.get("reward_value"),
                    reward_breakdown_json: row.get("reward_breakdown_json"),
                    confidence: row.get("confidence"),
                    lessons_json: row.get("lessons_json"),
                    recommendations_json: row.get("recommendations_json"),
                    created_at: parse_datetime(&created_at_str)?,
                }))
            }
            None => Ok(None),
        }
    }

    // ------------------------------------------------------------------------
    // Config Override Operations
    // ------------------------------------------------------------------------

    /// Upsert a config override.
    pub async fn upsert_config_override(
        &self,
        record: &ConfigOverrideRecord,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO config_overrides (key, value_json, applied_by_action, updated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                applied_by_action = excluded.applied_by_action,
                updated_at = excluded.updated_at
            ",
        )
        .bind(&record.key)
        .bind(&record.value_json)
        .bind(&record.applied_by_action)
        .bind(record.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Get a config override by key.
    pub async fn get_config_override(
        &self,
        key: &str,
    ) -> Result<Option<ConfigOverrideRecord>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT key, value_json, applied_by_action, updated_at
            FROM config_overrides
            WHERE key = ?
            ",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        match row {
            Some(row) => {
                let updated_at_str: String = row.get("updated_at");

                Ok(Some(ConfigOverrideRecord {
                    key: row.get("key"),
                    value_json: row.get("value_json"),
                    applied_by_action: row.get("applied_by_action"),
                    updated_at: parse_datetime(&updated_at_str)?,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all config overrides.
    pub async fn get_all_config_overrides(
        &self,
    ) -> Result<Vec<ConfigOverrideRecord>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT key, value_json, applied_by_action, updated_at
            FROM config_overrides
            ORDER BY key
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let updated_at_str: String = row.get("updated_at");

            records.push(ConfigOverrideRecord {
                key: row.get("key"),
                value_json: row.get("value_json"),
                applied_by_action: row.get("applied_by_action"),
                updated_at: parse_datetime(&updated_at_str)?,
            });
        }

        Ok(records)
    }

    /// Delete a config override by key.
    pub async fn delete_config_override(&self, key: &str) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM config_overrides WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| query_error(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ------------------------------------------------------------------------
    // Statistics
    // ------------------------------------------------------------------------

    /// Get invocation statistics for a time range.
    pub async fn get_invocation_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<InvocationStats, StorageError> {
        let row = sqlx::query(
            r"
            SELECT
                COUNT(*) as total_count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                AVG(latency_ms) as avg_latency,
                AVG(quality_score) as avg_quality
            FROM invocations
            WHERE created_at >= ?
            ",
        )
        .bind(since.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        let total_count: i64 = row.get("total_count");
        let success_count: i64 = row.get("success_count");

        // Counts are always non-negative; use max(0) to satisfy clippy
        #[allow(clippy::cast_sign_loss)]
        let total_count_u64 = total_count.max(0) as u64;
        #[allow(clippy::cast_sign_loss)]
        let success_count_u64 = success_count.max(0) as u64;

        Ok(InvocationStats {
            total_count: total_count_u64,
            success_count: success_count_u64,
            error_rate: if total_count > 0 {
                1.0 - (success_count as f64 / total_count as f64)
            } else {
                0.0
            },
            avg_latency_ms: row.get::<Option<f64>, _>("avg_latency").unwrap_or(0.0),
            avg_quality_score: row.get::<Option<f64>, _>("avg_quality"),
        })
    }
}
