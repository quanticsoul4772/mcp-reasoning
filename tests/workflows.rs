//! Workflow integration tests entry point.
//!
//! This module includes all workflow-related integration tests:
//! - Tree: create → list → focus → complete
//! - Graph: init → generate → score → aggregate → finalize
//! - Checkpoint: create → list → restore
//! - Preset: list → run multi-step workflows

mod integration;
