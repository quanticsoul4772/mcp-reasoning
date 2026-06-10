//! Database operations for self-improvement system.
//!
//! This module contains the `SelfImprovementStorage` struct and its implementation
//! for CRUD operations on self-improvement records.

use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use super::helpers::{parse_action_status, parse_datetime, parse_diagnosis_status, parse_severity};
use super::records::{
    ActionRecord, ActionTypeStatRecord, ConfigOverrideRecord, DiagnosisRecord, InvocationRecord,
    InvocationStats,
};
use crate::error::StorageError;
use crate::self_improvement::heal::{FixProposal, KnowledgeEntry, ProposalReview};
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

            // Safe: only `?` placeholders are interpolated; values are bound below.
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
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

    // ------------------------------------------------------------------------
    // Action-Type Learning Stats Operations (Migration 008)
    // ------------------------------------------------------------------------

    /// Upsert the persisted learning stats for one action type.
    ///
    /// Keyed on `action_type`, so repeated calls overwrite the row with the
    /// latest aggregates rather than accumulating duplicates.
    pub async fn upsert_action_type_stats(
        &self,
        record: &ActionTypeStatRecord,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO si_action_type_stats (
                action_type, total_executions, successful, avg_reward,
                total_expected, total_actual, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(action_type) DO UPDATE SET
                total_executions = excluded.total_executions,
                successful       = excluded.successful,
                avg_reward       = excluded.avg_reward,
                total_expected   = excluded.total_expected,
                total_actual     = excluded.total_actual,
                updated_at       = excluded.updated_at
            ",
        )
        .bind(&record.action_type)
        .bind(record.total_executions)
        .bind(record.successful)
        .bind(record.avg_reward)
        .bind(record.total_expected)
        .bind(record.total_actual)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Load all persisted action-type learning stats (to seed the Learner on
    /// startup).
    pub async fn get_all_action_type_stats(
        &self,
    ) -> Result<Vec<ActionTypeStatRecord>, StorageError> {
        let rows = sqlx::query(
            r"
            SELECT action_type, total_executions, successful, avg_reward,
                   total_expected, total_actual
            FROM si_action_type_stats
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| ActionTypeStatRecord {
                action_type: row.get("action_type"),
                total_executions: row.get("total_executions"),
                successful: row.get("successful"),
                avg_reward: row.get("avg_reward"),
                total_expected: row.get("total_expected"),
                total_actual: row.get("total_actual"),
            })
            .collect())
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
    // Heal Knowledge Operations (Migration 009, spec 001 FR-011)
    // ------------------------------------------------------------------------

    /// Upsert a self-heal knowledge entry.
    ///
    /// Keyed on `failure_signature`, so re-accepting a fix for the same
    /// `(component, failure_class)` class overwrites the prior mapping rather
    /// than accumulating duplicates.
    pub async fn upsert_knowledge_entry(&self, entry: &KnowledgeEntry) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO heal_knowledge_entries (
                id, failure_signature, fix_summary, test_ref, accepted_at
            )
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(failure_signature) DO UPDATE SET
                id          = excluded.id,
                fix_summary = excluded.fix_summary,
                test_ref    = excluded.test_ref,
                accepted_at = excluded.accepted_at
            ",
        )
        .bind(&entry.id)
        .bind(&entry.failure_signature)
        .bind(&entry.fix_summary)
        .bind(&entry.test_ref)
        .bind(entry.accepted_at)
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Look up an accepted knowledge entry by its failure signature, so a
    /// recurring defect of a previously-fixed class can be recognized and
    /// skip re-diagnosis (FR-011, SC-006).
    pub async fn get_knowledge_by_signature(
        &self,
        signature: &str,
    ) -> Result<Option<KnowledgeEntry>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, failure_signature, fix_summary, test_ref, accepted_at
            FROM heal_knowledge_entries
            WHERE failure_signature = ?
            ",
        )
        .bind(signature)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(row.map(|row| KnowledgeEntry {
            id: row.get("id"),
            failure_signature: row.get("failure_signature"),
            fix_summary: row.get("fix_summary"),
            test_ref: row.get("test_ref"),
            accepted_at: row.get("accepted_at"),
        }))
    }

    // ------------------------------------------------------------------------
    // Heal Fix-Proposal Operations (Migration 010, spec 001 US3)
    // ------------------------------------------------------------------------

    /// Upsert a fix proposal (keyed on `id`), so re-running the propose pipeline
    /// for the same proposal updates its verdicts/PR URL rather than duplicating.
    pub async fn upsert_fix_proposal(&self, p: &FixProposal) -> Result<(), StorageError> {
        sqlx::query(
            r"
            INSERT INTO heal_fix_proposals (
                id, defect_id, failure_signature, branch, change_summary, reproducing_test_ref,
                grounded, suite_green, quality_green, pr_url, review_status, created_at,
                weakens_invariant, block_reason
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                defect_id            = excluded.defect_id,
                failure_signature    = excluded.failure_signature,
                branch               = excluded.branch,
                change_summary       = excluded.change_summary,
                reproducing_test_ref = excluded.reproducing_test_ref,
                grounded             = excluded.grounded,
                suite_green          = excluded.suite_green,
                quality_green        = excluded.quality_green,
                pr_url               = excluded.pr_url,
                review_status        = excluded.review_status,
                weakens_invariant    = excluded.weakens_invariant,
                block_reason         = excluded.block_reason
            ",
        )
        .bind(&p.id)
        .bind(&p.defect_id)
        .bind(&p.failure_signature)
        .bind(&p.branch)
        .bind(&p.change_summary)
        .bind(&p.reproducing_test_ref)
        .bind(i64::from(p.grounded))
        .bind(i64::from(p.suite_green))
        .bind(i64::from(p.quality_green))
        .bind(&p.pr_url)
        .bind(p.review_status.as_str())
        .bind(Utc::now().timestamp_millis())
        .bind(i64::from(p.weakens_invariant))
        .bind(&p.block_reason)
        .execute(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(())
    }

    /// Load a fix proposal by id.
    pub async fn get_fix_proposal(&self, id: &str) -> Result<Option<FixProposal>, StorageError> {
        let row = sqlx::query(
            r"
            SELECT id, defect_id, failure_signature, branch, change_summary, reproducing_test_ref,
                   grounded, suite_green, quality_green, pr_url, review_status,
                   weakens_invariant, block_reason
            FROM heal_fix_proposals
            WHERE id = ?
            ",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| query_error(e.to_string()))?;

        Ok(row.map(|row| {
            let review_status: String = row.get("review_status");
            FixProposal {
                id: row.get("id"),
                defect_id: row.get("defect_id"),
                failure_signature: row.get("failure_signature"),
                branch: row.get("branch"),
                change_summary: row.get("change_summary"),
                reproducing_test_ref: row.get("reproducing_test_ref"),
                grounded: row.get::<i64, _>("grounded") != 0,
                suite_green: row.get::<i64, _>("suite_green") != 0,
                quality_green: row.get::<i64, _>("quality_green") != 0,
                pr_url: row.get("pr_url"),
                review_status: ProposalReview::from_db(&review_status),
                weakens_invariant: row.get::<i64, _>("weakens_invariant") != 0,
                block_reason: row.get("block_reason"),
            }
        }))
    }

    /// Record an operator review decision for a proposal. The loop never calls
    /// this with `Approved` for itself — only an operator override does (US3).
    pub async fn update_proposal_review(
        &self,
        id: &str,
        review: ProposalReview,
    ) -> Result<(), StorageError> {
        sqlx::query("UPDATE heal_fix_proposals SET review_status = ? WHERE id = ?")
            .bind(review.as_str())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| query_error(e.to_string()))?;

        Ok(())
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
