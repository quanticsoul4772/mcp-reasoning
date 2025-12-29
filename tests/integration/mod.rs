//! Integration tests for MCP Reasoning Server.
//!
//! These tests verify end-to-end workflows including:
//! - Session lifecycle
//! - Multi-mode reasoning scenarios
//! - Error recovery paths
//! - Self-improvement system integration
//! - Multi-step workflow validation (tree, graph, checkpoint, preset)

mod error_recovery;
mod multi_mode;
mod session_workflow;
mod workflow_checkpoint;
mod workflow_graph;
mod workflow_preset;
mod workflow_tree;
