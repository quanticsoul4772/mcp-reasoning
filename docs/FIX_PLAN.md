# MCP Reasoning Server - Fix Implementation Plan

**Date**: 2025-12-29
**Status**: Approved for Implementation
**Priority**: P1 (Critical) / P2 (Important) / P3 (Enhancement)

---

## Executive Summary

Tool validation revealed 4 issues requiring fixes. This document provides the implementation plan with root cause analysis, design specifications, and effort estimates.

| Issue | Priority | Effort | Status |
|-------|----------|--------|--------|
| Tree Branch Persistence | P1 | 3-4 hrs | Pending |
| Metrics Recording | P1 | 2 hrs | Pending |
| Graph State Retrieval | P2 | 2-3 hrs | Pending |
| Integration Tests | P3 | 4-6 hrs | Pending |

---

## Issue 1: Tree Branch Persistence

### Problem Statement

The `reasoning_tree` tool's `list` and `focus` operations return empty results because branches are stored in an **in-memory HashMap** that is recreated fresh for each tool invocation.

### Root Cause

**Location**: `src/modes/tree.rs:187-205`

```rust
pub struct TreeMode<S, C> {
    storage: S,
    client: C,
    /// In-memory branch storage (session_id -> branches).
    /// In a real implementation, this would be persisted.  // <-- THE PROBLEM
    branches: std::collections::HashMap<String, Vec<Branch>>,
}
```

The storage layer (`src/storage/branch.rs`) has full SQLite persistence for branches, but `TreeMode` doesn't use it.

### Design Solution

#### Architecture Change

```
BEFORE:
┌──────────────┐    ┌──────────────┐
│  TreeMode    │    │   Storage    │
│  (HashMap)   │    │   (SQLite)   │
│  branches    │    │   branches   │  ← Never used
└──────────────┘    └──────────────┘

AFTER:
┌──────────────┐    ┌──────────────┐
│  TreeMode    │───▶│   Storage    │
│  (no state)  │    │   (SQLite)   │
└──────────────┘    └──────────────┘
```

#### Implementation Steps

1. **Remove in-memory HashMap** from `TreeMode` struct
2. **Add type conversion** between `modes::Branch` and `storage::StoredBranch`
3. **Wire storage calls** in each operation:

| Operation | Storage Calls |
|-----------|---------------|
| `create` | `save_branch()` for each generated branch |
| `list` | `get_branches(session_id)` |
| `focus` | `get_branch(branch_id)` + `update_branch_status()` |
| `complete` | `update_branch_status()` |

#### Code Changes

**File**: `src/modes/tree.rs`

```rust
// REMOVE this field from TreeMode struct:
// branches: std::collections::HashMap<String, Vec<Branch>>,

// In create():
for branch in &branches {
    let stored = StoredBranch::new(&branch.id, &session.id, &branch.content)
        .with_score(branch.score);
    self.storage.save_branch(&stored).await?;
}

// In list():
pub async fn list(&self, session_id: &str) -> Result<TreeResponse, ModeError> {
    let stored_branches = self.storage.get_branches(session_id).await
        .map_err(|e| ModeError::ApiUnavailable { message: e.to_string() })?;

    let branches: Vec<Branch> = stored_branches.into_iter()
        .map(|sb| Branch::new(&sb.id, &sb.content, &sb.content, sb.score)
            .with_status(sb.status.into()))
        .collect();

    // ... rest of implementation
}
```

**File**: `src/storage/types.rs`

```rust
// Add conversion implementations
impl From<StoredBranch> for modes::Branch {
    fn from(sb: StoredBranch) -> Self {
        Branch::new(&sb.id, &sb.content, &sb.content, sb.score)
            .with_status(sb.status.into())
    }
}

impl From<modes::Branch> for StoredBranch {
    fn from(b: modes::Branch) -> Self {
        StoredBranch::new(&b.id, "", &b.content)
            .with_score(b.score)
            .with_status(b.status.into())
    }
}
```

### Test Plan

```rust
#[tokio::test]
async fn test_tree_branch_persistence() {
    // Create branches
    let create_resp = tree_mode.create("Test content", Some("sess-1"), Some(3)).await.unwrap();
    assert_eq!(create_resp.branches.unwrap().len(), 3);

    // Drop and recreate mode (simulates new tool invocation)
    let tree_mode2 = TreeMode::new(storage.clone(), client.clone());

    // List should return persisted branches
    let list_resp = tree_mode2.list("sess-1").await.unwrap();
    assert_eq!(list_resp.branches.unwrap().len(), 3);  // <-- Currently fails
}
```

---

## Issue 2: Metrics Recording Not Happening

### Problem Statement

The `reasoning_metrics` tool shows 0 invocations despite multiple tool calls being made. Metrics are not being recorded.

### Root Cause

**Location**: `src/server/tools.rs` (all tool handlers)

The tool handlers create modes and call them, but **never call `metrics.record()`**. The `MetricsCollector` exists in `AppState` but no tool invokes it.

```rust
// Current pattern (missing metrics):
async fn reasoning_linear(&self, req: LinearRequest) -> LinearResponse {
    let mode = LinearMode::new(...);
    match mode.process(&req.content, ...).await {
        Ok(resp) => { /* return success */ },
        Err(e) => { /* return error */ },
    }
    // <-- No metrics.record() call!
}
```

### Design Solution

#### Metrics Recording Pattern

```rust
async fn reasoning_linear(&self, req: LinearRequest) -> LinearResponse {
    let timer = Timer::start();
    let mode = LinearMode::new(...);

    let (response, success) = match mode.process(&req.content, ...).await {
        Ok(resp) => (/* success response */, true),
        Err(e) => (/* error response */, false),
    };

    // Record metrics
    self.state.metrics.record(
        MetricEvent::new("linear", timer.elapsed_ms(), success)
    );

    response
}
```

#### Macro for Consistency

**File**: `src/server/tools.rs`

```rust
/// Helper macro to record metrics for tool invocations
macro_rules! record_metric {
    ($self:expr, $mode:expr, $timer:expr, $success:expr) => {
        $self.state.metrics.record(
            crate::metrics::MetricEvent::new($mode, $timer.elapsed_ms(), $success)
        );
    };
    ($self:expr, $mode:expr, $op:expr, $timer:expr, $success:expr) => {
        $self.state.metrics.record(
            crate::metrics::MetricEvent::new($mode, $timer.elapsed_ms(), $success)
                .with_operation($op)
        );
    };
}
```

#### Implementation Checklist

Add metrics recording to all 15 tool handlers:

- [ ] `reasoning_linear`
- [ ] `reasoning_tree` (with operation)
- [ ] `reasoning_divergent`
- [ ] `reasoning_reflection` (with operation)
- [ ] `reasoning_checkpoint` (with operation)
- [ ] `reasoning_auto`
- [ ] `reasoning_graph` (with operation)
- [ ] `reasoning_detect` (with type)
- [ ] `reasoning_decision` (with type)
- [ ] `reasoning_evidence` (with type)
- [ ] `reasoning_timeline` (with operation)
- [ ] `reasoning_mcts` (with operation)
- [ ] `reasoning_counterfactual`
- [ ] `reasoning_preset` (with operation)
- [ ] `reasoning_metrics`

### Test Plan

```rust
#[tokio::test]
async fn test_metrics_recorded() {
    let state = setup_test_state().await;

    // Make some tool calls
    reasoning_linear(&state, LinearRequest { content: "test".into(), .. }).await;
    reasoning_linear(&state, LinearRequest { content: "test2".into(), .. }).await;

    // Check metrics
    let summary = state.metrics.summary();
    assert_eq!(summary.total_invocations, 2);
    assert!(summary.by_mode.contains_key("linear"));
}
```

---

## Issue 3: Graph State Retrieval Without Content

### Problem Statement

The `reasoning_graph` tool's `generate` and `state` operations fail with "Missing required field: content" even when `node_id` is provided, because they don't look up stored node content.

### Root Cause

**Location**: `src/modes/graph/mod.rs:142-147`

```rust
pub async fn generate(&self, content: &str, ...) -> Result<GenerateResponse, ModeError> {
    validate_content(content)?;  // <-- Requires content, but should use stored node
    // ...
}
```

The operations require the `content` parameter directly instead of retrieving it from storage when `node_id` is provided.

### Design Solution

#### Modified Function Signatures

```rust
// Before:
pub async fn generate(&self, content: &str, session_id: Option<String>) -> Result<...>

// After:
pub async fn generate(
    &self,
    content: Option<&str>,      // Optional now
    node_id: Option<&str>,      // New parameter
    session_id: Option<String>
) -> Result<...>
```

#### Content Resolution Logic

```rust
pub async fn generate(
    &self,
    content: Option<&str>,
    node_id: Option<&str>,
    session_id: Option<String>,
) -> Result<GenerateResponse, ModeError> {
    // Resolve content from either parameter or storage
    let resolved_content = match (content, node_id) {
        (Some(c), _) if !c.is_empty() => c.to_string(),
        (_, Some(nid)) => {
            let node = self.storage.get_graph_node(nid).await
                .map_err(|e| ModeError::ApiUnavailable { message: e.to_string() })?
                .ok_or_else(|| ModeError::InvalidValue {
                    field: "node_id".to_string(),
                    reason: format!("Node {} not found", nid),
                })?;
            node.content
        },
        _ => return Err(ModeError::MissingField {
            field: "content or node_id".to_string()
        }),
    };

    validate_content(&resolved_content)?;
    // ... rest of implementation
}
```

#### State Operation (No LLM Call Needed)

```rust
pub async fn state(&self, session_id: &str) -> Result<StateResponse, ModeError> {
    // Retrieve all nodes and edges from storage
    let nodes = self.storage.get_graph_nodes(session_id).await
        .map_err(|e| ModeError::ApiUnavailable { message: e.to_string() })?;

    let edges = self.storage.get_graph_edges(session_id).await
        .map_err(|e| ModeError::ApiUnavailable { message: e.to_string() })?;

    // Build graph structure from stored data
    let structure = GraphStructure {
        nodes: nodes.into_iter().map(|n| /* convert */).collect(),
        edges: edges.into_iter().map(|e| /* convert */).collect(),
    };

    Ok(StateResponse::new(session_id, structure))
}
```

#### Tool Handler Update

**File**: `src/server/tools.rs:1316-1330`

```rust
async fn reasoning_graph(&self, req: GraphRequest) -> GraphResponse {
    let mode = GraphMode::new(...);

    match req.operation.as_str() {
        "generate" => {
            // Pass both content and node_id
            mode.generate(
                req.content.as_deref(),
                req.node_id.as_deref(),  // <-- New parameter
                Some(req.session_id.clone())
            ).await
        },
        "state" => {
            // No content needed for state
            mode.state(&req.session_id).await
        },
        // ...
    }
}
```

### Test Plan

```rust
#[tokio::test]
async fn test_graph_state_from_storage() {
    let storage = test_storage().await;
    let mode = GraphMode::new(storage.clone(), mock_client());

    // Init creates and stores root node
    let init_resp = mode.init("Test topic", Some("sess-1".into())).await.unwrap();
    let root_id = init_resp.root.id;

    // State should retrieve from storage (no content needed)
    let state_resp = mode.state("sess-1").await.unwrap();
    assert!(state_resp.structure.nodes.len() >= 1);
}

#[tokio::test]
async fn test_graph_generate_from_node_id() {
    let storage = test_storage().await;
    let mode = GraphMode::new(storage.clone(), mock_client());

    // Init and get root node
    let init_resp = mode.init("Test topic", Some("sess-1".into())).await.unwrap();
    let root_id = init_resp.root.id;

    // Generate using node_id instead of content
    let gen_resp = mode.generate(
        None,                    // No content
        Some(&root_id),          // Use node_id
        Some("sess-1".into())
    ).await.unwrap();

    assert!(!gen_resp.children.is_empty());
}
```

---

## Issue 4: Integration Tests for Multi-Step Workflows

### Problem Statement

Existing tests only cover single operations. No tests verify that multi-step workflows work correctly across tool invocations.

### Design Solution

#### New Test Files

| File | Purpose |
|------|---------|
| `tests/workflow_tree.rs` | Tree: create → list → focus → complete |
| `tests/workflow_graph.rs` | Graph: init → generate → score → aggregate → finalize |
| `tests/workflow_checkpoint.rs` | Checkpoint: create → checkpoint → modify → restore |
| `tests/workflow_preset.rs` | Preset: list → run multi-step preset |

#### Test Structure

```rust
// tests/workflow_tree.rs

use mcp_reasoning::modes::{TreeMode, TreeResponse};
use mcp_reasoning::storage::SqliteStorage;
use serial_test::serial;

async fn setup_test_env() -> (SqliteStorage, impl AnthropicClientTrait) {
    let storage = SqliteStorage::new_in_memory().await.unwrap();
    let client = MockClient::with_tree_responses();
    (storage, client)
}

#[tokio::test]
#[serial]
async fn test_tree_full_workflow() {
    let (storage, client) = setup_test_env().await;

    // Step 1: Create exploration with 3 branches
    let mut mode = TreeMode::new(storage.clone(), client.clone());
    let create_resp = mode.create(
        "Analyze architectural patterns for microservices",
        None,
        Some(3)
    ).await.unwrap();

    let session_id = create_resp.session_id.clone();
    let branches = create_resp.branches.unwrap();
    assert_eq!(branches.len(), 3);
    let branch_id = branches[0].id.clone();

    // Step 2: Simulate new invocation - create fresh mode
    let mode2 = TreeMode::new(storage.clone(), client.clone());

    // Step 3: List branches (should persist across invocations)
    let list_resp = mode2.list(&session_id).await.unwrap();
    let listed_branches = list_resp.branches.unwrap();
    assert_eq!(listed_branches.len(), 3, "Branches should persist");

    // Step 4: Focus on first branch
    let mut mode3 = TreeMode::new(storage.clone(), client.clone());
    let focus_resp = mode3.focus(&session_id, &branch_id).await.unwrap();
    assert!(focus_resp.exploration.is_some());

    // Step 5: Complete the branch
    let complete_resp = mode3.complete(&session_id, &branch_id, true).await.unwrap();
    let completed_branches = complete_resp.branches.unwrap();
    let completed = completed_branches.iter().find(|b| b.id == branch_id).unwrap();
    assert_eq!(completed.status, BranchStatus::Completed);
}

#[tokio::test]
#[serial]
async fn test_tree_abandon_branch() {
    // Similar workflow but tests abandoning a branch
}

#[tokio::test]
#[serial]
async fn test_tree_multiple_sessions() {
    // Verify branches are isolated by session_id
}
```

#### Graph Workflow Test

```rust
// tests/workflow_graph.rs

#[tokio::test]
#[serial]
async fn test_graph_full_workflow() {
    let (storage, client) = setup_test_env().await;

    // Step 1: Initialize graph
    let mode = GraphMode::new(storage.clone(), client.clone());
    let init_resp = mode.init(
        "Design a distributed caching system",
        Some("graph-sess-1".into())
    ).await.unwrap();

    let session_id = init_resp.session_id.clone();
    let root_id = init_resp.root.id.clone();

    // Step 2: Generate children from root (using node_id)
    let gen_resp = mode.generate(
        None,
        Some(&root_id),
        Some(session_id.clone())
    ).await.unwrap();
    assert!(!gen_resp.children.is_empty());

    // Step 3: Score a node
    let child_id = gen_resp.children[0].id.clone();
    let score_resp = mode.score(
        None,
        Some(&child_id),
        Some(session_id.clone())
    ).await.unwrap();
    assert!(score_resp.scores.coherence >= 0.0);

    // Step 4: Get graph state
    let state_resp = mode.state(&session_id).await.unwrap();
    assert!(state_resp.structure.nodes.len() >= 2); // root + children

    // Step 5: Finalize
    let finalize_resp = mode.finalize(
        &session_id,
        vec![child_id.clone()]
    ).await.unwrap();
    assert!(!finalize_resp.conclusions.is_empty());
}
```

### Implementation Checklist

- [ ] Create `tests/workflow_tree.rs`
- [ ] Create `tests/workflow_graph.rs`
- [ ] Create `tests/workflow_checkpoint.rs`
- [ ] Create `tests/workflow_preset.rs`
- [ ] Add mock response fixtures for each workflow
- [ ] Update CI to run workflow tests

---

## Implementation Order

### Phase 1: Critical Fixes (P1)

1. **Metrics Recording** (2 hrs)
   - Add Timer + record() to all 15 handlers
   - Verify with unit test

2. **Tree Branch Persistence** (3-4 hrs)
   - Remove HashMap from TreeMode
   - Wire storage calls
   - Add type conversions
   - Verify with integration test

### Phase 2: Important Fixes (P2)

3. **Graph State Retrieval** (2-3 hrs)
   - Add node_id parameter
   - Implement content resolution
   - Update state() to use storage
   - Verify with integration test

### Phase 3: Quality Enhancements (P3)

4. **Integration Tests** (4-6 hrs)
   - Create 4 workflow test files
   - Add mock fixtures
   - Update CI pipeline

---

## Validation Criteria

### Acceptance Tests

After implementation, re-run the tool validation from the original session:

```
✅ reasoning_tree list: Returns previously created branches
✅ reasoning_tree focus: Can focus on persisted branch
✅ reasoning_graph state: Returns graph structure without content
✅ reasoning_graph generate: Works with node_id parameter
✅ reasoning_metrics summary: Shows recorded invocations
```

### Regression Tests

```bash
# Full test suite must pass
cargo test

# Coverage must remain above 96%
cargo llvm-cov --fail-under 96
```

---

## Appendix: Files Changed

```
src/
├── modes/
│   ├── tree.rs           # Remove HashMap, use storage
│   └── graph/
│       └── mod.rs        # Add node lookup from storage
├── server/
│   └── tools.rs          # Add metrics.record() to all handlers
└── storage/
    └── types.rs          # Add Branch ↔ StoredBranch conversion

tests/
├── workflow_tree.rs      # NEW
├── workflow_graph.rs     # NEW
├── workflow_checkpoint.rs # NEW
└── workflow_preset.rs    # NEW
```
