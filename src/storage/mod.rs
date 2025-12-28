//! Storage backend.
//!
//! This module provides:
//! - `SQLite` database implementation
//! - Session CRUD operations
//! - Thought persistence
//! - Graph node and edge management
//!
//! # Architecture
//!
//! The storage layer uses `SQLite` with the `sqlx` crate for async operations.
//! All operations are transactional and support concurrent access.
//!
//! The implementation is split across submodules for maintainability:
//! - `core`: Pool management, migrations, and helper functions
//! - `session`: Session CRUD operations
//! - `thought`: Thought CRUD operations
//! - `branch`: Branch CRUD operations
//! - `checkpoint`: Checkpoint CRUD operations
//! - `graph`: Graph node and edge operations
//! - `metrics`: Metrics collection operations
//! - `actions`: Self-improvement action operations
//! - `trait_impl`: `StorageTrait` implementation
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::doctest_helpers::{MockStorage, block_on};
//! use mcp_reasoning::traits::StorageTrait;
//!
//! // In production, use SqliteStorage::new("./data/reasoning.db").await?
//! // For doctests, we use MockStorage to avoid file I/O
//! let storage = MockStorage::new();
//!
//! block_on(async {
//!     let session = storage.get_or_create_session(None).await.unwrap();
//!     assert!(!session.id.is_empty());
//!
//!     // Retrieve the session by ID
//!     let found = storage.get_session(&session.id).await.unwrap();
//!     assert!(found.is_some());
//! });
//! ```

mod actions;
mod branch;
mod checkpoint;
mod core;
mod graph;
mod metrics;
mod session;
mod thought;
mod trait_impl;
mod types;

pub use self::core::SqliteStorage;
pub use types::{
    ActionStatus, BranchStatus, GraphEdgeType, GraphNodeType, StoredBranch, StoredCheckpoint,
    StoredGraphEdge, StoredGraphNode, StoredMetric, StoredSelfImprovementAction, StoredSession,
    StoredThought,
};
