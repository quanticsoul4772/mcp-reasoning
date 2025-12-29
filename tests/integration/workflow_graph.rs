//! Graph mode workflow integration tests.
//!
//! Tests the graph-of-thoughts workflow using storage-only operations.
//! Note: Full workflow tests with wiremock require careful node ID coordination
//! which is tested in the unit tests for GraphMode.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::{StorageTrait, Thought};
use serial_test::serial;

// ============================================================================
// Storage-Based Workflow Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_graph_storage_workflow() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Create session
    let session = storage
        .get_or_create_session(Some("graph-workflow".to_string()))
        .await
        .expect("Create session");

    // Simulate graph workflow by storing thoughts
    let root = Thought::new(
        "graph-root",
        &session.id,
        "Root: Design distributed cache",
        "graph",
        0.9,
    );
    storage.save_thought(&root).await.expect("Save root");

    // Add child nodes
    let child1 = Thought::new("node-1", &session.id, "Redis-based solution", "graph", 0.85);
    let child2 = Thought::new("node-2", &session.id, "Memcached solution", "graph", 0.80);
    let child3 = Thought::new("node-3", &session.id, "Custom cache", "graph", 0.75);

    storage.save_thought(&child1).await.expect("Save node 1");
    storage.save_thought(&child2).await.expect("Save node 2");
    storage.save_thought(&child3).await.expect("Save node 3");

    // Add aggregation node
    let aggregate = Thought::new(
        "aggregate-1",
        &session.id,
        "Synthesis: Redis recommended",
        "graph_aggregate",
        0.88,
    );
    storage
        .save_thought(&aggregate)
        .await
        .expect("Save aggregate");

    // Verify all thoughts stored
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get thoughts");

    assert_eq!(
        thoughts.len(),
        5,
        "Should have root + 3 children + aggregate"
    );
}

#[tokio::test]
#[serial]
async fn test_graph_session_isolation() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    // Create two sessions
    let session1 = storage
        .get_or_create_session(Some("graph-sess-1".to_string()))
        .await
        .expect("Create session 1");

    let session2 = storage
        .get_or_create_session(Some("graph-sess-2".to_string()))
        .await
        .expect("Create session 2");

    // Add thoughts to each session
    let t1 = Thought::new("g1", &session1.id, "Graph 1 node", "graph", 0.85);
    let t2 = Thought::new("g2", &session2.id, "Graph 2 node", "graph", 0.85);

    storage.save_thought(&t1).await.expect("Save t1");
    storage.save_thought(&t2).await.expect("Save t2");

    // Verify isolation
    let thoughts1 = storage.get_thoughts(&session1.id).await.unwrap();
    let thoughts2 = storage.get_thoughts(&session2.id).await.unwrap();

    assert_eq!(thoughts1.len(), 1);
    assert_eq!(thoughts2.len(), 1);
    assert_eq!(thoughts1[0].content, "Graph 1 node");
    assert_eq!(thoughts2[0].content, "Graph 2 node");
}

#[tokio::test]
#[serial]
async fn test_graph_different_modes_tracked() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    let session = storage
        .get_or_create_session(Some("graph-modes".to_string()))
        .await
        .expect("Create session");

    // Different graph operations
    let init = Thought::new("init-1", &session.id, "Graph init", "graph_init", 0.90);
    let gen = Thought::new(
        "gen-1",
        &session.id,
        "Generated node",
        "graph_generate",
        0.85,
    );
    let score = Thought::new("score-1", &session.id, "Scored node", "graph_score", 0.82);
    let agg = Thought::new("agg-1", &session.id, "Aggregated", "graph_aggregate", 0.88);

    storage.save_thought(&init).await.expect("Save init");
    storage.save_thought(&gen).await.expect("Save gen");
    storage.save_thought(&score).await.expect("Save score");
    storage.save_thought(&agg).await.expect("Save agg");

    let thoughts = storage.get_thoughts(&session.id).await.unwrap();

    // Verify modes are preserved
    assert!(thoughts.iter().any(|t| t.mode == "graph_init"));
    assert!(thoughts.iter().any(|t| t.mode == "graph_generate"));
    assert!(thoughts.iter().any(|t| t.mode == "graph_score"));
    assert!(thoughts.iter().any(|t| t.mode == "graph_aggregate"));
}

#[tokio::test]
#[serial]
async fn test_graph_confidence_scores_preserved() {
    let storage = SqliteStorage::new_in_memory().await.unwrap();

    let session = storage
        .get_or_create_session(Some("graph-confidence".to_string()))
        .await
        .expect("Create session");

    let high = Thought::new("high", &session.id, "High confidence", "graph", 0.95);
    let medium = Thought::new("medium", &session.id, "Medium confidence", "graph", 0.70);
    let low = Thought::new("low", &session.id, "Low confidence", "graph", 0.40);

    storage.save_thought(&high).await.expect("Save high");
    storage.save_thought(&medium).await.expect("Save medium");
    storage.save_thought(&low).await.expect("Save low");

    let thoughts = storage.get_thoughts(&session.id).await.unwrap();

    let high_t = thoughts.iter().find(|t| t.id == "high").unwrap();
    let medium_t = thoughts.iter().find(|t| t.id == "medium").unwrap();
    let low_t = thoughts.iter().find(|t| t.id == "low").unwrap();

    assert!((high_t.confidence - 0.95).abs() < 0.01);
    assert!((medium_t.confidence - 0.70).abs() < 0.01);
    assert!((low_t.confidence - 0.40).abs() < 0.01);
}
