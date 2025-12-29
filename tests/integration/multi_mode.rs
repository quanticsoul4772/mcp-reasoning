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
        "Initial linear analysis of the problem",
        "linear",
        0.85,
    );
    storage
        .save_thought(&linear_thought)
        .await
        .expect("Failed to save");

    // Tree reasoning phase - branching from linear
    let tree_root = Thought::new(
        "tree-root",
        &session.id,
        "Exploring branches from linear analysis",
        "tree",
        0.80,
    );
    storage
        .save_thought(&tree_root)
        .await
        .expect("Failed to save");

    // Tree branches
    for i in 1..=3 {
        let branch = Thought::new(
            &format!("tree-branch-{i}"),
            &session.id,
            &format!("Branch {i} exploration"),
            "tree",
            0.75 + (i as f64 * 0.02),
        );
        storage.save_thought(&branch).await.expect("Failed to save");
    }

    // Verify workflow state
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get");
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
        ("optimist", "This approach will succeed because...", 0.82),
        ("pessimist", "This approach may fail because...", 0.78),
        ("pragmatist", "We should balance considerations...", 0.85),
        ("contrarian", "What if we did the opposite...", 0.70),
    ];

    for (perspective, content, confidence) in perspectives {
        let thought = Thought::new(
            &format!("perspective-{perspective}"),
            &session.id,
            content,
            "divergent",
            confidence,
        );
        storage
            .save_thought(&thought)
            .await
            .expect("Failed to save");
    }

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get");
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
        "The initial hypothesis is X",
        "linear",
        0.75,
    );
    storage
        .save_thought(&initial)
        .await
        .expect("Failed to save");

    // Reflection thought that analyzes initial
    let reflection = Thought::new(
        "reflection-1",
        &session.id,
        "Upon reflection, the initial hypothesis has these strengths and weaknesses...",
        "reflection",
        0.82,
    );
    storage
        .save_thought(&reflection)
        .await
        .expect("Failed to save");

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get");
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
        ("linear", "Step 1: Identify the problem", 0.90),
        ("linear", "Step 2: Gather information", 0.88),
        ("tree", "Explore option A", 0.80),
        ("tree", "Explore option B", 0.78),
        ("divergent", "Alternative perspective", 0.75),
        ("reflection", "Meta-analysis of our reasoning", 0.85),
        ("linear", "Step 3: Make decision based on analysis", 0.92),
    ];

    for (i, (mode, content, confidence)) in modes_and_content.iter().enumerate() {
        let thought = Thought::new(
            &format!("thought-{i}"),
            &session.id,
            *content,
            *mode,
            *confidence,
        );
        storage
            .save_thought(&thought)
            .await
            .expect("Failed to save");
    }

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get");
    assert_eq!(thoughts.len(), 7);
}

#[tokio::test]
#[serial]
async fn test_graph_thoughts() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("graph-test".to_string()))
        .await
        .expect("Failed to create session");

    // Create nodes for a graph structure
    let nodes = [
        ("Problem definition", 0.90),
        ("Constraint A", 0.85),
        ("Constraint B", 0.82),
        ("Solution 1", 0.78),
        ("Solution 2", 0.80),
    ];

    for (i, (content, confidence)) in nodes.iter().enumerate() {
        let thought = Thought::new(
            &format!("node-{i}"),
            &session.id,
            *content,
            "graph",
            *confidence,
        );
        storage
            .save_thought(&thought)
            .await
            .expect("Failed to save");
    }

    // Verify structure
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get");
    assert_eq!(thoughts.len(), 5);
}
