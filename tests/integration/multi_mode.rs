//! Multi-mode reasoning scenario tests.
//!
//! Tests combinations of different reasoning modes in a single workflow.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::{StorageTrait, Thought};
use serial_test::serial;
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
async fn test_linear_then_tree_workflow() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create session for multi-mode workflow
    let session = storage
        .get_or_create_session(Some("multi-mode-1".to_string()))
        .await
        .expect("Failed to create session");

    // Linear reasoning phase
    let linear_thought = Thought::new(
        "linear-1",
        &session.id,
        "linear",
        "Initial linear analysis of the problem",
    );
    storage.save_thought(&linear_thought).await.expect("Failed to save");

    // Tree reasoning phase - branching from linear
    let tree_root = Thought::new(
        "tree-root",
        &session.id,
        "tree",
        "Exploring branches from linear analysis",
    );
    storage.save_thought(&tree_root).await.expect("Failed to save");

    // Tree branches
    for i in 1..=3 {
        let mut branch = Thought::new(
            &format!("tree-branch-{i}"),
            &session.id,
            "tree",
            &format!("Branch {i} exploration"),
        );
        branch.parent_id = Some("tree-root".to_string());
        storage.save_thought(&branch).await.expect("Failed to save");
    }

    // Verify workflow state
    let thoughts = storage.get_thoughts(&session.id).await.expect("Failed to get");
    assert_eq!(thoughts.len(), 5); // 1 linear + 1 root + 3 branches
}

#[tokio::test]
#[serial]
async fn test_divergent_perspectives() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("divergent-test".to_string()))
        .await
        .expect("Failed to create session");

    // Simulate divergent mode with multiple perspectives
    let perspectives = [
        ("optimist", "This approach will succeed because..."),
        ("pessimist", "This approach may fail because..."),
        ("pragmatist", "We should balance considerations..."),
        ("contrarian", "What if we did the opposite..."),
    ];

    for (perspective, content) in perspectives {
        let thought = Thought::new(
            &format!("perspective-{perspective}"),
            &session.id,
            "divergent",
            content,
        );
        storage.save_thought(&thought).await.expect("Failed to save");
    }

    let thoughts = storage.get_thoughts(&session.id).await.expect("Failed to get");
    assert_eq!(thoughts.len(), 4);
}

#[tokio::test]
#[serial]
async fn test_reflection_on_previous_thoughts() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("reflection-test".to_string()))
        .await
        .expect("Failed to create session");

    // Initial thought
    let initial = Thought::new(
        "initial",
        &session.id,
        "linear",
        "The initial hypothesis is X",
    );
    storage.save_thought(&initial).await.expect("Failed to save");

    // Reflection thought that references initial
    let mut reflection = Thought::new(
        "reflection-1",
        &session.id,
        "reflection",
        "Upon reflection, the initial hypothesis has these strengths and weaknesses...",
    );
    reflection.parent_id = Some("initial".to_string());
    storage.save_thought(&reflection).await.expect("Failed to save");

    let thoughts = storage.get_thoughts(&session.id).await.expect("Failed to get");
    assert_eq!(thoughts.len(), 2);
}

#[tokio::test]
#[serial]
async fn test_mixed_mode_session() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("mixed-mode".to_string()))
        .await
        .expect("Failed to create session");

    // Different modes in sequence
    let modes_and_content = [
        ("linear", "Step 1: Identify the problem"),
        ("linear", "Step 2: Gather information"),
        ("tree", "Explore option A"),
        ("tree", "Explore option B"),
        ("divergent", "Alternative perspective"),
        ("reflection", "Meta-analysis of our reasoning"),
        ("linear", "Step 3: Make decision based on analysis"),
    ];

    for (i, (mode, content)) in modes_and_content.iter().enumerate() {
        let thought = Thought::new(&format!("thought-{i}"), &session.id, mode, content);
        storage.save_thought(&thought).await.expect("Failed to save");
    }

    let thoughts = storage.get_thoughts(&session.id).await.expect("Failed to get");
    assert_eq!(thoughts.len(), 7);
}

#[tokio::test]
#[serial]
async fn test_graph_node_connections() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("graph-test".to_string()))
        .await
        .expect("Failed to create session");

    // Create nodes for a graph structure
    let nodes = [
        "Problem definition",
        "Constraint A",
        "Constraint B",
        "Solution 1",
        "Solution 2",
    ];

    for (i, content) in nodes.iter().enumerate() {
        let thought = Thought::new(&format!("node-{i}"), &session.id, "graph", content);
        storage.save_thought(&thought).await.expect("Failed to save");
    }

    // Create edges (stored as separate metadata)
    storage
        .create_edge(&session.id, "node-0", "node-1", "constrains")
        .await
        .expect("Failed to create edge");
    storage
        .create_edge(&session.id, "node-0", "node-2", "constrains")
        .await
        .expect("Failed to create edge");
    storage
        .create_edge(&session.id, "node-1", "node-3", "enables")
        .await
        .expect("Failed to create edge");
    storage
        .create_edge(&session.id, "node-2", "node-4", "enables")
        .await
        .expect("Failed to create edge");

    // Verify structure
    let thoughts = storage.get_thoughts(&session.id).await.expect("Failed to get");
    assert_eq!(thoughts.len(), 5);
}
