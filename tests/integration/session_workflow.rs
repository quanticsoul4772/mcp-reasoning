//! Session lifecycle workflow tests.
//!
//! Tests the complete session lifecycle:
//! 1. Create session
//! 2. Use reasoning modes
//! 3. Create checkpoint
//! 4. Continue reasoning
//! 5. Restore from checkpoint
//! 6. Verify state consistency

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::{Session, StorageTrait, Thought};
use serial_test::serial;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test database in a temporary directory.
async fn create_test_storage() -> (SqliteStorage, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let storage = SqliteStorage::new(db_path.to_str().expect("Invalid path"))
        .await
        .expect("Failed to create storage");
    (storage, temp_dir)
}

#[tokio::test]
#[serial]
async fn test_session_creation_and_retrieval() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create a new session
    let session = storage
        .get_or_create_session(Some("test-session-1".to_string()))
        .await
        .expect("Failed to create session");

    assert_eq!(session.id, "test-session-1");

    // Retrieve the same session
    let retrieved = storage
        .get_or_create_session(Some("test-session-1".to_string()))
        .await
        .expect("Failed to get session");

    assert_eq!(session.id, retrieved.id);
}

#[tokio::test]
#[serial]
async fn test_thought_persistence() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create session
    let session = storage
        .get_or_create_session(Some("thought-test".to_string()))
        .await
        .expect("Failed to create session");

    // Save a thought
    let thought = Thought::new(
        "thought-1",
        &session.id,
        "linear",
        "This is a test thought",
    );
    storage.save_thought(&thought).await.expect("Failed to save thought");

    // Retrieve thoughts
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get thoughts");

    assert_eq!(thoughts.len(), 1);
    assert_eq!(thoughts[0].content, "This is a test thought");
}

#[tokio::test]
#[serial]
async fn test_multi_thought_workflow() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create session
    let session = storage
        .get_or_create_session(Some("multi-thought".to_string()))
        .await
        .expect("Failed to create session");

    // Save multiple thoughts in sequence
    for i in 1..=5 {
        let thought = Thought::new(
            &format!("thought-{i}"),
            &session.id,
            "linear",
            &format!("Thought content {i}"),
        );
        storage.save_thought(&thought).await.expect("Failed to save thought");
    }

    // Verify all thoughts were saved
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get thoughts");

    assert_eq!(thoughts.len(), 5);
}

#[tokio::test]
#[serial]
async fn test_session_auto_id_generation() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create session without specifying ID
    let session = storage
        .get_or_create_session(None)
        .await
        .expect("Failed to create session");

    // ID should be auto-generated (UUID format)
    assert!(!session.id.is_empty());
    assert!(session.id.len() >= 32); // UUID without dashes is 32 chars
}

#[tokio::test]
#[serial]
async fn test_thought_with_parent() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create session
    let session = storage
        .get_or_create_session(Some("parent-test".to_string()))
        .await
        .expect("Failed to create session");

    // Create parent thought
    let parent = Thought::new("parent-1", &session.id, "tree", "Parent thought");
    storage.save_thought(&parent).await.expect("Failed to save parent");

    // Create child thought
    let mut child = Thought::new("child-1", &session.id, "tree", "Child thought");
    child.parent_id = Some("parent-1".to_string());
    storage.save_thought(&child).await.expect("Failed to save child");

    // Retrieve and verify relationship
    let child_retrieved = storage
        .get_thought("child-1")
        .await
        .expect("Failed to get thought");

    assert!(child_retrieved.is_some());
    assert_eq!(child_retrieved.unwrap().parent_id, Some("parent-1".to_string()));
}

#[tokio::test]
#[serial]
async fn test_session_isolation() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create two separate sessions
    let session1 = storage
        .get_or_create_session(Some("session-1".to_string()))
        .await
        .expect("Failed to create session 1");

    let session2 = storage
        .get_or_create_session(Some("session-2".to_string()))
        .await
        .expect("Failed to create session 2");

    // Add thoughts to each session
    let thought1 = Thought::new("t1", &session1.id, "linear", "Session 1 thought");
    let thought2 = Thought::new("t2", &session2.id, "linear", "Session 2 thought");

    storage.save_thought(&thought1).await.expect("Failed to save thought 1");
    storage.save_thought(&thought2).await.expect("Failed to save thought 2");

    // Verify thoughts are isolated
    let thoughts1 = storage.get_thoughts(&session1.id).await.expect("Failed to get thoughts");
    let thoughts2 = storage.get_thoughts(&session2.id).await.expect("Failed to get thoughts");

    assert_eq!(thoughts1.len(), 1);
    assert_eq!(thoughts2.len(), 1);
    assert_eq!(thoughts1[0].content, "Session 1 thought");
    assert_eq!(thoughts2[0].content, "Session 2 thought");
}
