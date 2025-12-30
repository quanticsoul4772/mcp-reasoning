//! Checkpoint mode workflow integration tests.
//!
//! Tests the checkpoint workflow using storage-only operations.
//! Note: Full workflow tests with wiremock require careful coordination
//! which is tested in the unit tests for `CheckpointMode`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::{StorageTrait, Thought};
use serial_test::serial;

// ============================================================================
// Storage-Based Workflow Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_checkpoint_storage_workflow() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Create session
    let session = storage
        .get_or_create_session(Some("checkpoint-workflow".to_string()))
        .await
        .expect("Create session");

    // Simulate reasoning before checkpoint
    let thought1 = Thought::new("t1", &session.id, "Initial analysis", "linear", 0.85);
    let thought2 = Thought::new("t2", &session.id, "Follow-up analysis", "linear", 0.90);

    storage.save_thought(&thought1).await.expect("Save t1");
    storage.save_thought(&thought2).await.expect("Save t2");

    // Simulate checkpoint by storing checkpoint thought
    let checkpoint = Thought::new(
        "checkpoint-1",
        &session.id,
        "Checkpoint: After initial analysis",
        "checkpoint",
        1.0,
    );
    storage
        .save_thought(&checkpoint)
        .await
        .expect("Save checkpoint");

    // Continue reasoning after checkpoint
    let thought3 = Thought::new(
        "t3",
        &session.id,
        "Post-checkpoint analysis",
        "linear",
        0.88,
    );
    storage.save_thought(&thought3).await.expect("Save t3");

    // Verify all thoughts stored
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get thoughts");

    assert_eq!(thoughts.len(), 4, "Should have 3 analyses + 1 checkpoint");

    // Verify checkpoint is tracked
    let cp = thoughts.iter().find(|t| t.mode == "checkpoint");
    assert!(cp.is_some());
}

#[tokio::test]
#[serial]
async fn test_checkpoint_session_isolation() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Create two sessions
    let session1 = storage
        .get_or_create_session(Some("cp-sess-1".to_string()))
        .await
        .expect("Create session 1");

    let session2 = storage
        .get_or_create_session(Some("cp-sess-2".to_string()))
        .await
        .expect("Create session 2");

    // Add checkpoints to each session
    let cp1 = Thought::new(
        "cp1",
        &session1.id,
        "Session 1 checkpoint",
        "checkpoint",
        1.0,
    );
    let cp2 = Thought::new(
        "cp2",
        &session2.id,
        "Session 2 checkpoint",
        "checkpoint",
        1.0,
    );

    storage.save_thought(&cp1).await.expect("Save cp1");
    storage.save_thought(&cp2).await.expect("Save cp2");

    // Verify isolation
    let thoughts1 = storage.get_thoughts(&session1.id).await.unwrap();
    let thoughts2 = storage.get_thoughts(&session2.id).await.unwrap();

    assert_eq!(thoughts1.len(), 1);
    assert_eq!(thoughts2.len(), 1);
    assert_eq!(thoughts1[0].content, "Session 1 checkpoint");
    assert_eq!(thoughts2[0].content, "Session 2 checkpoint");
}

#[tokio::test]
#[serial]
async fn test_checkpoint_empty_session() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Create session without any thoughts
    let session = storage
        .get_or_create_session(Some("empty-cp-sess".to_string()))
        .await
        .expect("Create session");

    // Get thoughts for empty session
    let thoughts = storage.get_thoughts(&session.id).await.unwrap();

    assert!(thoughts.is_empty(), "Should have no checkpoints");
}

#[tokio::test]
#[serial]
async fn test_checkpoint_multiple_per_session() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    let session = storage
        .get_or_create_session(Some("multi-cp-sess".to_string()))
        .await
        .expect("Create session");

    // Create multiple checkpoints
    for i in 1..=3 {
        let cp = Thought::new(
            format!("checkpoint-{i}"),
            &session.id,
            format!("Checkpoint at phase {i}"),
            "checkpoint",
            1.0,
        );
        storage.save_thought(&cp).await.expect("Save checkpoint");
    }

    let thoughts = storage.get_thoughts(&session.id).await.unwrap();

    // Verify all checkpoints stored
    let checkpoints: Vec<_> = thoughts.iter().filter(|t| t.mode == "checkpoint").collect();
    assert_eq!(checkpoints.len(), 3, "Should have 3 checkpoints");
}

#[tokio::test]
#[serial]
async fn test_checkpoint_persists_across_queries() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    let session = storage
        .get_or_create_session(Some("persist-test-sess".to_string()))
        .await
        .expect("Create session");

    // Create checkpoint
    let cp = Thought::new(
        "persist-cp",
        &session.id,
        "Test checkpoint for persistence",
        "checkpoint",
        1.0,
    );
    storage.save_thought(&cp).await.expect("Save checkpoint");

    // Query multiple times to verify persistence
    for _ in 0..3 {
        let thoughts = storage.get_thoughts(&session.id).await.unwrap();
        assert_eq!(thoughts.len(), 1);
        assert_eq!(thoughts[0].id, "persist-cp");
    }
}
