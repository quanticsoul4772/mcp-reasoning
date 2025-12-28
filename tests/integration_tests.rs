//! Integration tests for MCP Reasoning Server.
//!
//! These tests verify end-to-end workflows including:
//! - Session lifecycle
//! - Multi-mode reasoning scenarios
//! - Configuration handling

use mcp_reasoning::config::Config;
use mcp_reasoning::error::ConfigError;
use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::{StorageTrait, Thought};
use serial_test::serial;
use tempfile::TempDir;

// ============================================================================
// Test Utilities
// ============================================================================

/// Create a test database in a temporary directory.
async fn create_test_storage() -> (SqliteStorage, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let storage = SqliteStorage::new(db_path.to_str().expect("Invalid path"))
        .await
        .expect("Failed to create storage");
    (storage, temp_dir)
}

/// Helper to create a thought with default confidence.
fn create_thought(id: &str, session_id: &str, mode: &str, content: &str) -> Thought {
    Thought::new(id, session_id, content, mode, 0.8)
}

// ============================================================================
// Session Workflow Tests
// ============================================================================

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
    let thought = create_thought("thought-1", &session.id, "linear", "This is a test thought");
    storage
        .save_thought(&thought)
        .await
        .expect("Failed to save thought");

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
        let thought = create_thought(
            &format!("thought-{i}"),
            &session.id,
            "linear",
            &format!("Thought content {i}"),
        );
        storage
            .save_thought(&thought)
            .await
            .expect("Failed to save thought");
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
    let thought1 = create_thought("t1", &session1.id, "linear", "Session 1 thought");
    let thought2 = create_thought("t2", &session2.id, "linear", "Session 2 thought");

    storage
        .save_thought(&thought1)
        .await
        .expect("Failed to save thought 1");
    storage
        .save_thought(&thought2)
        .await
        .expect("Failed to save thought 2");

    // Verify thoughts are isolated
    let thoughts1 = storage
        .get_thoughts(&session1.id)
        .await
        .expect("Failed to get thoughts");
    let thoughts2 = storage
        .get_thoughts(&session2.id)
        .await
        .expect("Failed to get thoughts");

    assert_eq!(thoughts1.len(), 1);
    assert_eq!(thoughts2.len(), 1);
    assert_eq!(thoughts1[0].content, "Session 1 thought");
    assert_eq!(thoughts2[0].content, "Session 2 thought");
}

// ============================================================================
// Multi-Mode Tests
// ============================================================================

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
    let linear_thought = create_thought(
        "linear-1",
        &session.id,
        "linear",
        "Initial linear analysis of the problem",
    );
    storage
        .save_thought(&linear_thought)
        .await
        .expect("Failed to save");

    // Tree reasoning phase
    let tree_root = create_thought(
        "tree-root",
        &session.id,
        "tree",
        "Exploring branches from linear analysis",
    );
    storage
        .save_thought(&tree_root)
        .await
        .expect("Failed to save");

    // Tree branches
    for i in 1..=3 {
        let branch = create_thought(
            &format!("tree-branch-{i}"),
            &session.id,
            "tree",
            &format!("Branch {i} exploration"),
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
        ("optimist", "This approach will succeed because..."),
        ("pessimist", "This approach may fail because..."),
        ("pragmatist", "We should balance considerations..."),
        ("contrarian", "What if we did the opposite..."),
    ];

    for (perspective, content) in perspectives {
        let thought = create_thought(
            &format!("perspective-{perspective}"),
            &session.id,
            "divergent",
            content,
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
        let thought = create_thought(&format!("thought-{i}"), &session.id, mode, content);
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

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_get_nonexistent_session() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Try to get a session that doesn't exist
    let result = storage.get_session("nonexistent-session-id").await;

    // Should return None, not an error
    assert!(result.is_ok());
    assert!(result.expect("Expected Ok").is_none());
}

#[tokio::test]
#[serial]
async fn test_get_thoughts_empty_session() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create session but don't add thoughts
    let session = storage
        .get_or_create_session(Some("empty-session".to_string()))
        .await
        .expect("Failed to create session");

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get thoughts");

    assert!(thoughts.is_empty());
}

#[tokio::test]
#[serial]
async fn test_special_characters_in_content() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("special-chars".to_string()))
        .await
        .expect("Failed to create session");

    // Content with special characters
    let content = r#"This has "quotes", 'apostrophes', \backslashes\,
newlines,	tabs, and unicode: 日本語 🎉 émoji"#;

    let thought = create_thought("special", &session.id, "linear", content);

    storage
        .save_thought(&thought)
        .await
        .expect("Save should work");

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 1);
    assert_eq!(thoughts[0].content, content);
}

#[tokio::test]
#[serial]
async fn test_very_long_content() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("long-content".to_string()))
        .await
        .expect("Failed to create session");

    // Very long content (50KB)
    let content = "A".repeat(50_000);

    let thought = create_thought("long", &session.id, "linear", &content);

    storage
        .save_thought(&thought)
        .await
        .expect("Save should work");

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 1);
    assert_eq!(thoughts[0].content.len(), 50_000);
}

#[test]
#[serial]
fn test_config_missing_api_key() {
    // Ensure API key is not set
    std::env::remove_var("ANTHROPIC_API_KEY");

    let result = Config::from_env();

    // Should fail with missing API key error
    assert!(
        result.is_err(),
        "Expected error when API key is missing, got: {result:?}"
    );
    if let Err(ConfigError::MissingRequired { var }) = result {
        assert_eq!(var, "ANTHROPIC_API_KEY");
    } else {
        panic!("Expected MissingRequired error, got: {result:?}");
    }
}

#[test]
#[serial]
fn test_config_with_api_key() {
    // Set environment
    std::env::set_var("ANTHROPIC_API_KEY", "test-key-123");

    let result = Config::from_env();

    // Should succeed
    assert!(
        result.is_ok(),
        "Expected success with API key set, got: {result:?}"
    );

    // Clean up
    std::env::remove_var("ANTHROPIC_API_KEY");
}

// ============================================================================
// Checkpoint Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_checkpoint_save_and_retrieve() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("checkpoint-test".to_string()))
        .await
        .expect("Failed to create session");

    // Create a checkpoint
    let checkpoint = mcp_reasoning::storage::StoredCheckpoint::new(
        "checkpoint-1",
        &session.id,
        "checkpoint-name",
        "{}", // state as JSON
    )
    .with_description("Before major change");

    storage
        .save_checkpoint(&checkpoint)
        .await
        .expect("Failed to save checkpoint");

    // Retrieve checkpoint
    let retrieved = storage
        .get_checkpoint("checkpoint-1")
        .await
        .expect("Failed to get checkpoint");

    assert!(retrieved.is_some());
    let cp = retrieved.expect("should exist");
    assert_eq!(cp.id, "checkpoint-1");
    assert_eq!(cp.session_id, session.id);
    assert_eq!(cp.description, Some("Before major change".to_string()));
}

#[tokio::test]
#[serial]
async fn test_checkpoint_list_for_session() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("checkpoint-list-test".to_string()))
        .await
        .expect("Failed to create session");

    // Create multiple checkpoints
    for i in 1..=3 {
        let checkpoint = mcp_reasoning::storage::StoredCheckpoint::new(
            &format!("checkpoint-{i}"),
            &session.id,
            &format!("Checkpoint {i}"),
            "{}",
        );
        storage
            .save_checkpoint(&checkpoint)
            .await
            .expect("Failed to save checkpoint");
    }

    // Get all checkpoints for session
    let checkpoints = storage
        .get_checkpoints(&session.id)
        .await
        .expect("Failed to get checkpoints");

    assert_eq!(checkpoints.len(), 3);
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_concurrent_thought_creation() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("concurrent-test".to_string()))
        .await
        .expect("Failed to create session");

    // Spawn multiple tasks to save thoughts concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let storage_clone = SqliteStorage::new(
            _temp_dir
                .path()
                .join("test.db")
                .to_str()
                .expect("Invalid path"),
        )
        .await
        .expect("Failed to create storage");
        let session_id = session.id.clone();

        handles.push(tokio::spawn(async move {
            let thought = create_thought(
                &format!("concurrent-{i}"),
                &session_id,
                "linear",
                &format!("Concurrent thought {i}"),
            );
            storage_clone
                .save_thought(&thought)
                .await
                .expect("Save should work");
        }));
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    // Verify all thoughts were saved
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 10);
}

// ============================================================================
// In-Memory Storage Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_in_memory_storage() {
    // Test with in-memory database
    let storage = SqliteStorage::new_in_memory()
        .await
        .expect("Failed to create in-memory storage");

    let session = storage
        .get_or_create_session(Some("memory-test".to_string()))
        .await
        .expect("Failed to create session");

    let thought = create_thought("mem-thought", &session.id, "linear", "In-memory thought");
    storage
        .save_thought(&thought)
        .await
        .expect("Failed to save thought");

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get thoughts");

    assert_eq!(thoughts.len(), 1);
}
