//! Tree mode workflow integration tests.
//!
//! Tests the tree mode branching workflow using storage-only operations.
//! Note: Full workflow tests with wiremock require careful node ID coordination
//! which is tested in the unit tests for `TreeMode`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::{StorageTrait, Thought};
use serial_test::serial;

// ============================================================================
// Storage-Based Workflow Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_tree_storage_workflow() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Create session
    let session = storage
        .get_or_create_session(Some("tree-workflow".to_string()))
        .await
        .expect("Create session");

    // Simulate tree workflow by storing thoughts
    let root = Thought::new(
        "tree-root",
        &session.id,
        "Root node of exploration",
        "tree",
        0.9,
    );
    storage.save_thought(&root).await.expect("Save root");

    // Add branches
    let branch1 = Thought::new("branch-1", &session.id, "Branch 1: Option A", "tree", 0.85);
    let branch2 = Thought::new("branch-2", &session.id, "Branch 2: Option B", "tree", 0.80);
    let branch3 = Thought::new("branch-3", &session.id, "Branch 3: Option C", "tree", 0.75);

    storage.save_thought(&branch1).await.expect("Save branch 1");
    storage.save_thought(&branch2).await.expect("Save branch 2");
    storage.save_thought(&branch3).await.expect("Save branch 3");

    // Verify all thoughts stored
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get thoughts");

    assert_eq!(thoughts.len(), 4, "Should have root + 3 branches");

    // Verify content preserved
    let root_thought = thoughts.iter().find(|t| t.id == "tree-root");
    assert!(root_thought.is_some());
    assert_eq!(root_thought.unwrap().content, "Root node of exploration");
}

#[tokio::test]
#[serial]
async fn test_tree_session_isolation() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Create two sessions
    let session1 = storage
        .get_or_create_session(Some("tree-sess-1".to_string()))
        .await
        .expect("Create session 1");

    let session2 = storage
        .get_or_create_session(Some("tree-sess-2".to_string()))
        .await
        .expect("Create session 2");

    // Add thoughts to each session
    let t1 = Thought::new("t1", &session1.id, "Session 1 tree", "tree", 0.85);
    let t2 = Thought::new("t2", &session2.id, "Session 2 tree", "tree", 0.85);

    storage.save_thought(&t1).await.expect("Save t1");
    storage.save_thought(&t2).await.expect("Save t2");

    // Verify isolation
    let thoughts1 = storage.get_thoughts(&session1.id).await.unwrap();
    let thoughts2 = storage.get_thoughts(&session2.id).await.unwrap();

    assert_eq!(thoughts1.len(), 1);
    assert_eq!(thoughts2.len(), 1);
    assert_eq!(thoughts1[0].content, "Session 1 tree");
    assert_eq!(thoughts2[0].content, "Session 2 tree");
}

#[tokio::test]
#[serial]
async fn test_tree_list_nonexistent_session() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Get thoughts for nonexistent session
    let thoughts = storage
        .get_thoughts("nonexistent-session")
        .await
        .expect("Get should work");

    assert!(thoughts.is_empty());
}

#[tokio::test]
#[serial]
async fn test_tree_branch_modes_tracked() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    let session = storage
        .get_or_create_session(Some("tree-modes".to_string()))
        .await
        .expect("Create session");

    // Mix of tree and linear thoughts
    let linear = Thought::new("linear-1", &session.id, "Linear thought", "linear", 0.90);
    let tree = Thought::new("tree-1", &session.id, "Tree thought", "tree", 0.85);

    storage.save_thought(&linear).await.expect("Save linear");
    storage.save_thought(&tree).await.expect("Save tree");

    let thoughts = storage.get_thoughts(&session.id).await.unwrap();

    // Verify mode is preserved
    let linear_t = thoughts.iter().find(|t| t.id == "linear-1").unwrap();
    let tree_t = thoughts.iter().find(|t| t.id == "tree-1").unwrap();

    assert_eq!(linear_t.mode, "linear");
    assert_eq!(tree_t.mode, "tree");
}
