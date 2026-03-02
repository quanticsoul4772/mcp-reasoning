//! Integration tests for MCP Reasoning Server.
//!
//! These tests verify end-to-end workflows including:
//! - Session lifecycle
//! - Multi-mode reasoning scenarios
//! - Error recovery paths
//! - Self-improvement system integration
//! - Multi-step workflow validation (tree, graph, checkpoint, preset)
//! - Agent system (registry, teams, coordination)
//! - Skill system (registry, executor, discovery)

mod error_recovery;
mod multi_mode;
mod session_workflow;
mod workflow_agent;
mod workflow_checkpoint;
mod workflow_graph;
mod workflow_preset;
mod workflow_skill;
mod workflow_tree;
