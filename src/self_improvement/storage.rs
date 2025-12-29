//! Database operations for self-improvement system.
//!
//! Provides CRUD operations for:
//! - Invocation records (from Monitor)
//! - Diagnosis records (from Analyzer)
//! - Action records (from Executor)
//! - Learning records (from Learner)
//! - Config overrides (applied by Executor)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use super::types::{
    ActionStatus, DiagnosisId, DiagnosisStatus, NormalizedReward, Severity, SuggestedAction,
    TriggerMetric,
};
use crate::error::StorageError;

/// Helper to create a QueryFailed error.
fn query_error(message: impl Into<String>) -> StorageError {
    StorageError::QueryFailed {
        query: "self_improvement".to_string(),
        message: message.into(),
    }
}

/// Parse an RFC3339 timestamp string into DateTime<Utc>.
fn parse_datetime(s: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| query_error(format!("Invalid datetime '{s}': {e}")))
}

// ============================================================================
// Invocation Records
// ============================================================================

/// Invocation event record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationRecord {
    /// Unique identifier.
    pub id: String,
    /// Tool name that was invoked.
    pub tool_name: String,
    /// Latency in milliseconds.
    pub latency_ms: i64,
    /// Whether the invocation succeeded.
    pub success: bool,
    /// Optional quality score (0.0 to 1.0).
    pub quality_score: Option<f64>,
    /// When the invocation occurred.
    pub created_at: DateTime<Utc>,
}

impl InvocationRecord {
    /// Create a new invocation record.
    #[must_use]
    pub fn new(
        tool_name: impl Into<String>,
        latency_ms: i64,
        success: bool,
        quality_score: Option<f64>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.into(),
            latency_ms,
            success,
            quality_score,
            created_at: Utc::now(),
        }
    }
}

// ============================================================================
// Diagnosis Records
// ============================================================================

/// Diagnosis record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisRecord {
    /// Unique identifier.
    pub id: DiagnosisId,
    /// Type of trigger (error_rate, latency, quality_score).
    pub trigger_type: String,
    /// JSON-serialized trigger data.
    pub trigger_json: String,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable description.
    pub description: String,
    /// Suspected root cause (LLM-generated).
    pub suspected_cause: Option<String>,
    /// JSON-serialized suggested action.
    pub suggested_action_json: String,
    /// Rationale for the action (LLM-generated).
    pub action_rationale: Option<String>,
    /// Current status.
    pub status: DiagnosisStatus,
    /// When the diagnosis was created.
    pub created_at: DateTime<Utc>,
}

impl DiagnosisRecord {
    /// Create from a trigger metric and suggested action.
    pub fn from_diagnosis(
        trigger: &TriggerMetric,
        description: impl Into<String>,
        suspected_cause: Option<String>,
        suggested_action: &SuggestedAction,
        action_rationale: Option<String>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger_type: trigger.metric_type().to_string(),
            trigger_json: serde_json::to_string(trigger)?,
            severity: trigger.severity(),
            description: description.into(),
            suspected_cause,
            suggested_action_json: serde_json::to_string(suggested_action)?,
            action_rationale,
            status: DiagnosisStatus::Pending,
            created_at: Utc::now(),
        })
    }
}

// ============================================================================
// Action Records
// ============================================================================

/// Action execution record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Unique identifier.
    pub id: String,
    /// Associated diagnosis ID.
    pub diagnosis_id: DiagnosisId,
    /// Type of action (adjust_param, scale_resource, no_op).
    pub action_type: String,
    /// JSON-serialized action data.
    pub action_json: String,
    /// Outcome of execution.
    pub outcome: ActionStatus,
    /// JSON-serialized pre-execution metrics.
    pub pre_metrics_json: String,
    /// JSON-serialized post-execution metrics (if available).
    pub post_metrics_json: Option<String>,
    /// Execution time in milliseconds.
    pub execution_time_ms: i64,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// When the action was executed.
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Learning Records
// ============================================================================

/// Learning record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningRecord {
    /// Unique identifier.
    pub id: String,
    /// Associated action ID.
    pub action_id: String,
    /// Reward value (-1.0 to 1.0).
    pub reward_value: f64,
    /// JSON-serialized reward breakdown.
    pub reward_breakdown_json: String,
    /// Confidence level (0.0 to 1.0).
    pub confidence: f64,
    /// JSON-serialized lessons learned.
    pub lessons_json: Option<String>,
    /// JSON-serialized future recommendations.
    pub recommendations_json: Option<String>,
    /// When the learning was recorded.
    pub created_at: DateTime<Utc>,
}

impl LearningRecord {
    /// Create from a normalized reward.
    pub fn from_reward(
        action_id: impl Into<String>,
        reward: &NormalizedReward,
        lessons: Option<Vec<String>>,
        recommendations: Option<Vec<String>>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            action_id: action_id.into(),
            reward_value: reward.value,
            reward_breakdown_json: serde_json::to_string(&reward.breakdown)?,
            confidence: reward.confidence,
            lessons_json: lessons.map(|l| serde_json::to_string(&l)).transpose()?,
            recommendations_json: recommendations
                .map(|r| serde_json::to_string(&r))
                .transpose()?,
            created_at: Utc::now(),
        })
    }
}

// ============================================================================
// Config Override Records
// ============================================================================

/// Config override record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOverrideRecord {
    /// Configuration key.
    pub key: String,
    /// JSON-serialized value.
    pub value_json: String,
    /// Action ID that applied this override.
    pub applied_by_action: Option<String>,
    /// When the override was last updated.
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Self-Improvement Storage
// ============================================================================

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

/// Invocation statistics summary.
#[derive(Debug, Clone, Default)]
pub struct InvocationStats {
    /// Total number of invocations.
    pub total_count: u64,
    /// Number of successful invocations.
    pub success_count: u64,
    /// Error rate (0.0 to 1.0).
    pub error_rate: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Average quality score (if available).
    pub avg_quality_score: Option<f64>,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "info" => Severity::Info,
        "warning" => Severity::Warning,
        "high" => Severity::High,
        "critical" => Severity::Critical,
        _ => Severity::Info,
    }
}

fn parse_diagnosis_status(s: &str) -> DiagnosisStatus {
    match s.to_lowercase().as_str() {
        "pending" => DiagnosisStatus::Pending,
        "approved" => DiagnosisStatus::Approved,
        "rejected" => DiagnosisStatus::Rejected,
        "executed" => DiagnosisStatus::Executed,
        "failed" => DiagnosisStatus::Failed,
        "rolled_back" | "rolledback" => DiagnosisStatus::RolledBack,
        _ => DiagnosisStatus::Pending,
    }
}

fn parse_action_status(s: &str) -> ActionStatus {
    match s.to_lowercase().as_str() {
        "proposed" => ActionStatus::Proposed,
        "approved" => ActionStatus::Approved,
        "executed" | "completed" => ActionStatus::Completed,
        "failed" => ActionStatus::Failed,
        "rolled_back" | "rolledback" => ActionStatus::RolledBack,
        _ => ActionStatus::Proposed,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::self_improvement::types::{NormalizedReward, RewardBreakdown};
    use crate::storage::SqliteStorage;
    use serial_test::serial;
    use std::time::Duration;

    // Helper to create test storage with self-improvement tables
    async fn test_storage() -> SelfImprovementStorage {
        let sqlite_storage = SqliteStorage::new_in_memory()
            .await
            .expect("Failed to create test storage");
        SelfImprovementStorage::new(sqlite_storage.pool.clone())
    }

    #[test]
    fn test_invocation_record_creation() {
        let record = InvocationRecord::new("reasoning_linear", 100, true, Some(0.95));
        assert_eq!(record.tool_name, "reasoning_linear");
        assert_eq!(record.latency_ms, 100);
        assert!(record.success);
        assert_eq!(record.quality_score, Some(0.95));
        assert!(!record.id.is_empty());
    }

    #[test]
    fn test_invocation_record_creation_no_quality() {
        let record = InvocationRecord::new("reasoning_tree", 200, false, None);
        assert_eq!(record.tool_name, "reasoning_tree");
        assert_eq!(record.latency_ms, 200);
        assert!(!record.success);
        assert_eq!(record.quality_score, None);
    }

    #[test]
    fn test_parse_severity() {
        assert_eq!(parse_severity("info"), Severity::Info);
        assert_eq!(parse_severity("WARNING"), Severity::Warning);
        assert_eq!(parse_severity("High"), Severity::High);
        assert_eq!(parse_severity("CRITICAL"), Severity::Critical);
        assert_eq!(parse_severity("unknown"), Severity::Info);
    }

    #[test]
    fn test_parse_diagnosis_status() {
        assert_eq!(parse_diagnosis_status("pending"), DiagnosisStatus::Pending);
        assert_eq!(
            parse_diagnosis_status("APPROVED"),
            DiagnosisStatus::Approved
        );
        assert_eq!(
            parse_diagnosis_status("rolled_back"),
            DiagnosisStatus::RolledBack
        );
        assert_eq!(
            parse_diagnosis_status("rolledback"),
            DiagnosisStatus::RolledBack
        );
        assert_eq!(
            parse_diagnosis_status("rejected"),
            DiagnosisStatus::Rejected
        );
        assert_eq!(
            parse_diagnosis_status("executed"),
            DiagnosisStatus::Executed
        );
        assert_eq!(parse_diagnosis_status("failed"), DiagnosisStatus::Failed);
        assert_eq!(parse_diagnosis_status("unknown"), DiagnosisStatus::Pending);
    }

    #[test]
    fn test_parse_action_status() {
        assert_eq!(parse_action_status("proposed"), ActionStatus::Proposed);
        assert_eq!(parse_action_status("approved"), ActionStatus::Approved);
        assert_eq!(parse_action_status("EXECUTED"), ActionStatus::Completed);
        assert_eq!(parse_action_status("completed"), ActionStatus::Completed);
        assert_eq!(parse_action_status("failed"), ActionStatus::Failed);
        assert_eq!(parse_action_status("rolled_back"), ActionStatus::RolledBack);
        assert_eq!(parse_action_status("rolledback"), ActionStatus::RolledBack);
        assert_eq!(parse_action_status("unknown"), ActionStatus::Proposed);
    }

    #[test]
    fn test_query_error() {
        let err = query_error("test error");
        match err {
            StorageError::QueryFailed { query, message } => {
                assert_eq!(query, "self_improvement");
                assert_eq!(message, "test error");
            }
            _ => panic!("Expected QueryFailed error"),
        }
    }

    #[test]
    fn test_parse_datetime_valid() {
        let result = parse_datetime("2024-01-15T10:30:00Z");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let result = parse_datetime("invalid-date");
        assert!(result.is_err());
        match result {
            Err(StorageError::QueryFailed { message, .. }) => {
                assert!(message.contains("Invalid datetime"));
            }
            _ => panic!("Expected QueryFailed error"),
        }
    }

    #[test]
    fn test_diagnosis_record_from_diagnosis() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.15,
            baseline: 0.05,
            threshold: 0.10,
        };
        let action = SuggestedAction::no_op("wait and see", Duration::from_secs(300));

        let result = DiagnosisRecord::from_diagnosis(
            &trigger,
            "High error rate detected",
            Some("API overload".to_string()),
            &action,
            Some("Monitor and wait".to_string()),
        );

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.trigger_type, "error_rate");
        assert_eq!(record.description, "High error rate detected");
        assert_eq!(record.suspected_cause, Some("API overload".to_string()));
        assert_eq!(
            record.action_rationale,
            Some("Monitor and wait".to_string())
        );
        assert_eq!(record.status, DiagnosisStatus::Pending);
        assert!(!record.id.is_empty());
    }

    #[test]
    fn test_learning_record_from_reward() {
        let reward = NormalizedReward::new(0.5, RewardBreakdown::new(0.3, 0.5, 0.7), 0.85);

        let result = LearningRecord::from_reward(
            "action-123",
            &reward,
            Some(vec!["lesson 1".to_string()]),
            Some(vec!["recommendation 1".to_string()]),
        );

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.action_id, "action-123");
        assert_eq!(record.reward_value, 0.5);
        assert_eq!(record.confidence, 0.85);
        assert!(record.lessons_json.is_some());
        assert!(record.recommendations_json.is_some());
    }

    #[test]
    fn test_learning_record_from_reward_no_optional() {
        let reward = NormalizedReward::new(0.0, RewardBreakdown::default(), 0.5);

        let result = LearningRecord::from_reward("action-456", &reward, None, None);

        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.action_id, "action-456");
        assert!(record.lessons_json.is_none());
        assert!(record.recommendations_json.is_none());
    }

    #[test]
    fn test_invocation_stats_default() {
        let stats = InvocationStats::default();
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.error_rate, 0.0);
        assert_eq!(stats.avg_latency_ms, 0.0);
        assert_eq!(stats.avg_quality_score, None);
    }

    // ========== Async Database Tests ==========

    #[tokio::test]
    #[serial]
    async fn test_insert_and_get_invocation() {
        let storage = test_storage().await;
        let record = InvocationRecord::new("reasoning_linear", 150, true, Some(0.9));

        // Insert
        let result = storage.insert_invocation(&record).await;
        assert!(result.is_ok());

        // Get recent
        let records = storage.get_recent_invocations(10).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, record.id);
        assert_eq!(records[0].tool_name, "reasoning_linear");
        assert_eq!(records[0].latency_ms, 150);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_invocations_by_tool() {
        let storage = test_storage().await;

        // Insert multiple records for different tools
        let record1 = InvocationRecord::new("reasoning_linear", 100, true, Some(0.8));
        let record2 = InvocationRecord::new("reasoning_tree", 200, true, Some(0.7));
        let record3 = InvocationRecord::new("reasoning_linear", 150, true, Some(0.9));

        storage.insert_invocation(&record1).await.unwrap();
        storage.insert_invocation(&record2).await.unwrap();
        storage.insert_invocation(&record3).await.unwrap();

        // Get by tool
        let linear_records = storage
            .get_invocations_by_tool("reasoning_linear", 10)
            .await
            .unwrap();
        assert_eq!(linear_records.len(), 2);

        let tree_records = storage
            .get_invocations_by_tool("reasoning_tree", 10)
            .await
            .unwrap();
        assert_eq!(tree_records.len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_insert_and_get_diagnosis() {
        let storage = test_storage().await;

        let trigger = TriggerMetric::Latency {
            observed_p95_ms: 500,
            baseline_ms: 200,
            threshold_ms: 400,
        };
        let action = SuggestedAction::no_op("wait", Duration::from_secs(60));

        let record = DiagnosisRecord::from_diagnosis(
            &trigger,
            "High latency detected",
            Some("Network congestion".to_string()),
            &action,
            Some("Monitor traffic".to_string()),
        )
        .unwrap();

        let record_id = record.id.clone();

        // Insert
        storage.insert_diagnosis(&record).await.unwrap();

        // Get by ID
        let retrieved = storage.get_diagnosis(&record_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, record_id);
        assert_eq!(retrieved.trigger_type, "latency");
        assert_eq!(retrieved.description, "High latency detected");
        assert_eq!(retrieved.status, DiagnosisStatus::Pending);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_diagnosis_not_found() {
        let storage = test_storage().await;
        let result = storage.get_diagnosis("nonexistent-id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_update_diagnosis_status() {
        let storage = test_storage().await;

        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.05,
            threshold: 0.1,
        };
        let action = SuggestedAction::no_op("retry", Duration::from_secs(30));

        let record =
            DiagnosisRecord::from_diagnosis(&trigger, "Error spike", None, &action, None).unwrap();
        let record_id = record.id.clone();

        storage.insert_diagnosis(&record).await.unwrap();

        // Update status
        storage
            .update_diagnosis_status(&record_id, DiagnosisStatus::Approved)
            .await
            .unwrap();

        // Verify
        let retrieved = storage.get_diagnosis(&record_id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, DiagnosisStatus::Approved);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_pending_diagnoses() {
        let storage = test_storage().await;

        let trigger = TriggerMetric::QualityScore {
            observed: 0.5,
            baseline: 0.8,
            minimum: 0.7,
        };
        let action = SuggestedAction::no_op("review", Duration::from_secs(120));

        let record1 =
            DiagnosisRecord::from_diagnosis(&trigger, "Quality drop 1", None, &action, None)
                .unwrap();
        let record2 =
            DiagnosisRecord::from_diagnosis(&trigger, "Quality drop 2", None, &action, None)
                .unwrap();

        storage.insert_diagnosis(&record1).await.unwrap();
        storage.insert_diagnosis(&record2).await.unwrap();

        // Update one to approved
        storage
            .update_diagnosis_status(&record1.id, DiagnosisStatus::Approved)
            .await
            .unwrap();

        // Get pending only
        let pending = storage.get_pending_diagnoses().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, record2.id);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_diagnoses_by_status() {
        let storage = test_storage().await;

        let trigger = TriggerMetric::ErrorRate {
            observed: 0.3,
            baseline: 0.1,
            threshold: 0.2,
        };
        let action = SuggestedAction::no_op("wait", Duration::from_secs(60));

        let record1 =
            DiagnosisRecord::from_diagnosis(&trigger, "Error 1", None, &action, None).unwrap();
        let record2 =
            DiagnosisRecord::from_diagnosis(&trigger, "Error 2", None, &action, None).unwrap();

        storage.insert_diagnosis(&record1).await.unwrap();
        storage.insert_diagnosis(&record2).await.unwrap();

        // Get by status
        let pending = storage
            .get_diagnoses_by_status(DiagnosisStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        let approved = storage
            .get_diagnoses_by_status(DiagnosisStatus::Approved)
            .await
            .unwrap();
        assert!(approved.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_insert_and_get_action() {
        let storage = test_storage().await;

        // First insert a diagnosis (foreign key constraint)
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.15,
            baseline: 0.05,
            threshold: 0.1,
        };
        let suggested_action = SuggestedAction::no_op("wait", Duration::from_secs(60));
        let diagnosis = DiagnosisRecord::from_diagnosis(
            &trigger,
            "Test diagnosis",
            None,
            &suggested_action,
            None,
        )
        .unwrap();
        storage.insert_diagnosis(&diagnosis).await.unwrap();

        // Now insert action
        let action_record = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis.id.clone(),
            action_type: "no_op".to_string(),
            action_json: r#"{"action":"no_op","reason":"wait"}"#.to_string(),
            outcome: ActionStatus::Proposed,
            pre_metrics_json: r#"{"error_rate":0.15}"#.to_string(),
            post_metrics_json: None,
            execution_time_ms: 0,
            error_message: None,
            created_at: Utc::now(),
        };

        let action_id = action_record.id.clone();
        storage.insert_action(&action_record).await.unwrap();

        // Get by ID
        let retrieved = storage.get_action(&action_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, action_id);
        assert_eq!(retrieved.action_type, "no_op");
        assert_eq!(retrieved.outcome, ActionStatus::Proposed);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_action_not_found() {
        let storage = test_storage().await;
        let result = storage.get_action("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_update_action_outcome() {
        let storage = test_storage().await;

        // Setup diagnosis and action
        let trigger = TriggerMetric::Latency {
            observed_p95_ms: 600,
            baseline_ms: 200,
            threshold_ms: 400,
        };
        let suggested_action = SuggestedAction::no_op("wait", Duration::from_secs(60));
        let diagnosis =
            DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None)
                .unwrap();
        storage.insert_diagnosis(&diagnosis).await.unwrap();

        let action_record = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis.id.clone(),
            action_type: "no_op".to_string(),
            action_json: "{}".to_string(),
            outcome: ActionStatus::Proposed,
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: None,
            execution_time_ms: 0,
            error_message: None,
            created_at: Utc::now(),
        };

        let action_id = action_record.id.clone();
        storage.insert_action(&action_record).await.unwrap();

        // Update outcome
        storage
            .update_action_outcome(
                &action_id,
                ActionStatus::Completed,
                Some(r#"{"error_rate":0.05}"#),
                None,
            )
            .await
            .unwrap();

        // Verify
        let retrieved = storage.get_action(&action_id).await.unwrap().unwrap();
        assert_eq!(retrieved.outcome, ActionStatus::Completed);
        assert!(retrieved.post_metrics_json.is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_update_action_outcome_with_error() {
        let storage = test_storage().await;

        // Setup
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.05,
            threshold: 0.1,
        };
        let suggested_action = SuggestedAction::no_op("wait", Duration::from_secs(30));
        let diagnosis =
            DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None)
                .unwrap();
        storage.insert_diagnosis(&diagnosis).await.unwrap();

        let action_record = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis.id.clone(),
            action_type: "no_op".to_string(),
            action_json: "{}".to_string(),
            outcome: ActionStatus::Proposed,
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: None,
            execution_time_ms: 0,
            error_message: None,
            created_at: Utc::now(),
        };

        let action_id = action_record.id.clone();
        storage.insert_action(&action_record).await.unwrap();

        // Update with error
        storage
            .update_action_outcome(
                &action_id,
                ActionStatus::Failed,
                None,
                Some("Execution failed: timeout"),
            )
            .await
            .unwrap();

        let retrieved = storage.get_action(&action_id).await.unwrap().unwrap();
        assert_eq!(retrieved.outcome, ActionStatus::Failed);
        assert_eq!(
            retrieved.error_message,
            Some("Execution failed: timeout".to_string())
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_actions_by_outcome() {
        let storage = test_storage().await;

        // Setup two diagnoses (each action needs unique diagnosis due to unique constraint)
        let trigger = TriggerMetric::QualityScore {
            observed: 0.6,
            baseline: 0.8,
            minimum: 0.7,
        };
        let suggested_action = SuggestedAction::no_op("wait", Duration::from_secs(60));
        let diagnosis1 =
            DiagnosisRecord::from_diagnosis(&trigger, "Test 1", None, &suggested_action, None)
                .unwrap();
        let diagnosis2 =
            DiagnosisRecord::from_diagnosis(&trigger, "Test 2", None, &suggested_action, None)
                .unwrap();
        storage.insert_diagnosis(&diagnosis1).await.unwrap();
        storage.insert_diagnosis(&diagnosis2).await.unwrap();

        // Insert actions with different diagnoses (unique constraint on diagnosis_id)
        let action1 = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis1.id.clone(),
            action_type: "no_op".to_string(),
            action_json: "{}".to_string(),
            outcome: ActionStatus::Completed,
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: None,
            execution_time_ms: 100,
            error_message: None,
            created_at: Utc::now(),
        };

        let action2 = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis2.id.clone(),
            action_type: "no_op".to_string(),
            action_json: "{}".to_string(),
            outcome: ActionStatus::Failed,
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: None,
            execution_time_ms: 50,
            error_message: Some("error".to_string()),
            created_at: Utc::now(),
        };

        storage.insert_action(&action1).await.unwrap();
        storage.insert_action(&action2).await.unwrap();

        // Get by outcome
        let completed = storage
            .get_actions_by_outcome(ActionStatus::Completed, 10)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);

        let failed = storage
            .get_actions_by_outcome(ActionStatus::Failed, 10)
            .await
            .unwrap();
        assert_eq!(failed.len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_insert_and_get_learning() {
        let storage = test_storage().await;

        // Setup diagnosis and action
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.1,
            baseline: 0.05,
            threshold: 0.08,
        };
        let suggested_action = SuggestedAction::no_op("wait", Duration::from_secs(60));
        let diagnosis =
            DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None)
                .unwrap();
        storage.insert_diagnosis(&diagnosis).await.unwrap();

        let action_record = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis.id.clone(),
            action_type: "no_op".to_string(),
            action_json: "{}".to_string(),
            outcome: ActionStatus::Completed,
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: Some("{}".to_string()),
            execution_time_ms: 100,
            error_message: None,
            created_at: Utc::now(),
        };
        let action_id = action_record.id.clone();
        storage.insert_action(&action_record).await.unwrap();

        // Insert learning
        let reward = NormalizedReward::new(0.6, RewardBreakdown::new(0.5, 0.6, 0.7), 0.9);
        let learning = LearningRecord::from_reward(
            &action_id,
            &reward,
            Some(vec!["Lesson learned".to_string()]),
            Some(vec!["Try faster".to_string()]),
        )
        .unwrap();

        storage.insert_learning(&learning).await.unwrap();

        // Get by action ID
        let retrieved = storage.get_learning_by_action(&action_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.action_id, action_id);
        assert_eq!(retrieved.reward_value, 0.6);
        assert_eq!(retrieved.confidence, 0.9);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_learning_not_found() {
        let storage = test_storage().await;
        let result = storage.get_learning_by_action("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_config_override_crud() {
        let storage = test_storage().await;

        // Insert
        let config = ConfigOverrideRecord {
            key: "max_retries".to_string(),
            value_json: r#"{"type":"integer","value":5}"#.to_string(),
            applied_by_action: None,
            updated_at: Utc::now(),
        };

        storage.upsert_config_override(&config).await.unwrap();

        // Get
        let retrieved = storage.get_config_override("max_retries").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.key, "max_retries");

        // Update (upsert) - without applied_by_action to avoid FK constraint
        let updated_config = ConfigOverrideRecord {
            key: "max_retries".to_string(),
            value_json: r#"{"type":"integer","value":10}"#.to_string(),
            applied_by_action: None,
            updated_at: Utc::now(),
        };
        storage
            .upsert_config_override(&updated_config)
            .await
            .unwrap();

        let retrieved = storage
            .get_config_override("max_retries")
            .await
            .unwrap()
            .unwrap();
        assert!(retrieved.value_json.contains("10"));

        // Delete
        let deleted = storage.delete_config_override("max_retries").await.unwrap();
        assert!(deleted);

        // Verify deleted
        let retrieved = storage.get_config_override("max_retries").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_config_override_with_action_reference() {
        let storage = test_storage().await;

        // Setup a valid action first
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.1,
            baseline: 0.05,
            threshold: 0.08,
        };
        let suggested_action = SuggestedAction::no_op("wait", Duration::from_secs(60));
        let diagnosis =
            DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None)
                .unwrap();
        storage.insert_diagnosis(&diagnosis).await.unwrap();

        let action_record = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis.id.clone(),
            action_type: "no_op".to_string(),
            action_json: "{}".to_string(),
            outcome: ActionStatus::Completed,
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: None,
            execution_time_ms: 100,
            error_message: None,
            created_at: Utc::now(),
        };
        let action_id = action_record.id.clone();
        storage.insert_action(&action_record).await.unwrap();

        // Now create config override referencing the action
        let config = ConfigOverrideRecord {
            key: "timeout_ms".to_string(),
            value_json: r#"{"type":"integer","value":5000}"#.to_string(),
            applied_by_action: Some(action_id.clone()),
            updated_at: Utc::now(),
        };

        storage.upsert_config_override(&config).await.unwrap();

        let retrieved = storage
            .get_config_override("timeout_ms")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.applied_by_action, Some(action_id));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_config_override_not_found() {
        let storage = test_storage().await;
        let result = storage.get_config_override("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_config_override_not_found() {
        let storage = test_storage().await;
        let deleted = storage.delete_config_override("nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_all_config_overrides() {
        let storage = test_storage().await;

        let config1 = ConfigOverrideRecord {
            key: "alpha".to_string(),
            value_json: r#"{"value":1}"#.to_string(),
            applied_by_action: None,
            updated_at: Utc::now(),
        };
        let config2 = ConfigOverrideRecord {
            key: "beta".to_string(),
            value_json: r#"{"value":2}"#.to_string(),
            applied_by_action: None,
            updated_at: Utc::now(),
        };

        storage.upsert_config_override(&config1).await.unwrap();
        storage.upsert_config_override(&config2).await.unwrap();

        let all = storage.get_all_config_overrides().await.unwrap();
        assert_eq!(all.len(), 2);
        // Ordered by key
        assert_eq!(all[0].key, "alpha");
        assert_eq!(all[1].key, "beta");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_invocation_stats() {
        let storage = test_storage().await;

        // Insert test data
        let record1 = InvocationRecord::new("tool1", 100, true, Some(0.8));
        let record2 = InvocationRecord::new("tool2", 200, true, Some(0.9));
        let record3 = InvocationRecord::new("tool1", 150, false, None);

        storage.insert_invocation(&record1).await.unwrap();
        storage.insert_invocation(&record2).await.unwrap();
        storage.insert_invocation(&record3).await.unwrap();

        // Get stats from past hour
        let since = Utc::now() - chrono::Duration::hours(1);
        let stats = storage.get_invocation_stats(since).await.unwrap();

        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.success_count, 2);
        // Error rate should be 1/3 ≈ 0.333
        assert!((stats.error_rate - 0.333).abs() < 0.01);
        // Avg latency should be (100+200+150)/3 = 150
        assert!((stats.avg_latency_ms - 150.0).abs() < 1.0);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_invocation_stats_empty() {
        let storage = test_storage().await;

        let since = Utc::now() - chrono::Duration::hours(1);
        let stats = storage.get_invocation_stats(since).await.unwrap();

        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.error_rate, 0.0);
        assert_eq!(stats.avg_latency_ms, 0.0);
    }

    #[tokio::test]
    #[serial]
    async fn test_storage_new() {
        let sqlite_storage = SqliteStorage::new_in_memory().await.unwrap();
        let si_storage = SelfImprovementStorage::new(sqlite_storage.pool.clone());
        // Just verify it was created without error
        let _ = si_storage.get_recent_invocations(1).await;
    }

    // Test edge case: invocation with all fields
    #[tokio::test]
    #[serial]
    async fn test_invocation_full_roundtrip() {
        let storage = test_storage().await;

        let record = InvocationRecord::new("reasoning_graph", 999, true, Some(0.999));
        let id = record.id.clone();

        storage.insert_invocation(&record).await.unwrap();

        let retrieved = storage.get_recent_invocations(1).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].id, id);
        assert_eq!(retrieved[0].latency_ms, 999);
        assert_eq!(retrieved[0].quality_score, Some(0.999));
    }

    // Test parsing edge cases with chrono
    #[test]
    fn test_parse_datetime_with_offset() {
        let result = parse_datetime("2024-06-15T14:30:00+05:30");
        assert!(result.is_ok());
    }

    // Batch insert tests
    #[tokio::test]
    #[serial]
    async fn test_batch_insert_invocations_empty() {
        let storage = test_storage().await;
        let count = storage.batch_insert_invocations(&[]).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_insert_invocations_single() {
        let storage = test_storage().await;
        let records = vec![InvocationRecord::new("tool1", 100, true, Some(0.9))];
        let count = storage.batch_insert_invocations(&records).await.unwrap();
        assert_eq!(count, 1);

        let retrieved = storage.get_recent_invocations(10).await.unwrap();
        assert_eq!(retrieved.len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_insert_invocations_multiple() {
        let storage = test_storage().await;
        let records: Vec<InvocationRecord> = (0..50)
            .map(|i| InvocationRecord::new(format!("tool_{}", i % 5), 100 + i, i % 3 != 0, None))
            .collect();

        let count = storage.batch_insert_invocations(&records).await.unwrap();
        assert_eq!(count, 50);

        let retrieved = storage.get_recent_invocations(100).await.unwrap();
        assert_eq!(retrieved.len(), 50);
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_insert_invocations_exceeds_batch_size() {
        let storage = test_storage().await;
        // Create more than BATCH_SIZE (166) records to test chunking
        let records: Vec<InvocationRecord> = (0..200)
            .map(|i| InvocationRecord::new(format!("tool_{}", i % 10), i, true, Some(0.8)))
            .collect();

        let count = storage.batch_insert_invocations(&records).await.unwrap();
        assert_eq!(count, 200);

        let retrieved = storage.get_recent_invocations(250).await.unwrap();
        assert_eq!(retrieved.len(), 200);
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_insert_learnings_empty() {
        let storage = test_storage().await;
        let count = storage.batch_insert_learnings(&[]).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_insert_learnings_multiple() {
        let storage = test_storage().await;

        // Setup required foreign key references
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.1,
            baseline: 0.05,
            threshold: 0.08,
        };
        let suggested_action = SuggestedAction::no_op("wait", Duration::from_secs(60));
        let diagnosis =
            DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None)
                .unwrap();
        storage.insert_diagnosis(&diagnosis).await.unwrap();

        let action_record = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            diagnosis_id: diagnosis.id.clone(),
            action_type: "no_op".to_string(),
            action_json: "{}".to_string(),
            outcome: ActionStatus::Completed,
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: None,
            execution_time_ms: 100,
            error_message: None,
            created_at: Utc::now(),
        };
        let action_id = action_record.id.clone();
        storage.insert_action(&action_record).await.unwrap();

        // Create learning records
        let reward = NormalizedReward::new(0.5, RewardBreakdown::new(0.3, 0.5, 0.7), 0.85);
        let records: Vec<LearningRecord> = (0..10)
            .map(|_| {
                LearningRecord::from_reward(&action_id, &reward, Some(vec!["lesson".into()]), None)
                    .unwrap()
            })
            .collect();

        let count = storage.batch_insert_learnings(&records).await.unwrap();
        assert_eq!(count, 10);
    }

    use chrono::Datelike;
}
