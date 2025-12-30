//! Tests for self-improvement storage.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::helpers::{
    parse_action_status, parse_datetime, parse_diagnosis_status, parse_severity, query_error,
};
use super::operations::SelfImprovementStorage;
use super::records::{
    ActionRecord, ConfigOverrideRecord, DiagnosisRecord, InvocationRecord, InvocationStats,
    LearningRecord,
};
use crate::error::StorageError;
use crate::self_improvement::types::{
    ActionStatus, DiagnosisStatus, NormalizedReward, RewardBreakdown, Severity, SuggestedAction,
    TriggerMetric,
};
use crate::storage::SqliteStorage;
use chrono::{Datelike, Utc};
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
        DiagnosisRecord::from_diagnosis(&trigger, "Quality drop 1", None, &action, None).unwrap();
    let record2 =
        DiagnosisRecord::from_diagnosis(&trigger, "Quality drop 2", None, &action, None).unwrap();

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
    let diagnosis =
        DiagnosisRecord::from_diagnosis(&trigger, "Test diagnosis", None, &suggested_action, None)
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
        DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None).unwrap();
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
        DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None).unwrap();
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
        DiagnosisRecord::from_diagnosis(&trigger, "Test 1", None, &suggested_action, None).unwrap();
    let diagnosis2 =
        DiagnosisRecord::from_diagnosis(&trigger, "Test 2", None, &suggested_action, None).unwrap();
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
        DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None).unwrap();
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
        DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None).unwrap();
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
        DiagnosisRecord::from_diagnosis(&trigger, "Test", None, &suggested_action, None).unwrap();
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
