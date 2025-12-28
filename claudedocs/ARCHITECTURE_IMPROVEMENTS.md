# Architecture Improvements Implementation Plan

**Status**: Design Complete
**Priority**: Critical → High → Medium
**Estimated Items**: 7 tasks across 3 priority levels

---

## Executive Summary

This plan addresses architecture issues identified in the self-improvement module analysis. Two items from the original list are already complete:
- **Item 6** (DateTime parsing helper): Already implemented in `storage.rs:28-33`
- **Item 2** (RFC3339 consistency): Already verified - all datetime operations use RFC3339

Remaining items are organized by priority with clear implementation specifications.

---

## Critical Priority (Before Deployment)

### C1. Prompt Escaping Verification

**Status**: ✅ Already Implemented
**Location**: `src/self_improvement/anthropic_calls.rs:38-80`

**Evidence**:
```rust
// Line 50-60: escape_for_prompt() - escapes {} and truncates
fn escape_for_prompt(content: &str) -> String {
    let mut escaped = content.replace('{', "{{").replace('}', "}}");
    if escaped.len() > MAX_PROMPT_CONTENT_LEN {
        escaped.truncate(MAX_PROMPT_CONTENT_LEN);
        escaped.push_str("...[truncated]");
    }
    escaped
}

// Line 72-80: sanitize_multiline() - neutralizes injection patterns
fn sanitize_multiline(content: &str) -> String {
    // Replaces ---, ===, ### patterns
}
```

**Verification**: Review all `format!()` calls in `anthropic_calls.rs` to ensure user-controlled content passes through escaping functions.

**Action**: Add test to verify escaping is applied to all user inputs.

---

## High Priority

### H1. JSON Size Limits Enhancement

**Status**: ⚠️ Partial - Needs Review
**Location**: `src/self_improvement/anthropic_calls.rs:32, 553-627`

**Current Implementation**:
```rust
const MAX_JSON_SIZE: usize = 100_000;  // 100KB - Line 32

fn extract_json(text: &str) -> Result<String, ModeError> {
    if text.len() > MAX_JSON_SIZE { /* rejects */ }
    // ...
}
```

**Issue**: 100KB limit exists but analysis suggested 1MB. Need to verify if 100KB is intentionally conservative or should be increased.

**Action**:
1. Review `MAX_JSON_SIZE` constant - 100KB is appropriately conservative
2. Add documentation explaining the limit choice
3. No code change needed - current limit is more secure than suggested 1MB

---

### H2. Add Send + Sync Bounds on AnthropicCalls

**Status**: 🔴 Needs Implementation
**Location**: `src/self_improvement/anthropic_calls.rs:212-217`

**Current Code**:
```rust
pub struct AnthropicCalls<C: AnthropicClientTrait> {
    client: Arc<C>,
    max_tokens: u32,
}

impl<C: AnthropicClientTrait> AnthropicCalls<C> { ... }
```

**Problem**: Without `Send + Sync` bounds, `AnthropicCalls` cannot be safely shared across threads, limiting async executor compatibility.

**Solution**:
```rust
pub struct AnthropicCalls<C: AnthropicClientTrait + Send + Sync> {
    client: Arc<C>,
    max_tokens: u32,
}

impl<C: AnthropicClientTrait + Send + Sync> AnthropicCalls<C> { ... }
```

**Files to Modify**:
1. `src/self_improvement/anthropic_calls.rs` - struct and impl blocks
2. `src/traits/mod.rs` - Add `Send + Sync` to `AnthropicClientTrait` supertraits

**Implementation Steps**:
1. Add bounds to `AnthropicClientTrait`: `pub trait AnthropicClientTrait: Send + Sync`
2. Update `AnthropicCalls<C>` struct declaration
3. Update all `impl<C: AnthropicClientTrait>` blocks
4. Verify MockAnthropicClientTrait automatically implements Send + Sync
5. Run tests to confirm no regressions

---

### H3. Add Unique Constraint on diagnosis_id in si_actions

**Status**: 🔴 Needs Implementation
**Location**: `migrations/001_initial_schema.sql:118-129`

**Current Schema**:
```sql
CREATE TABLE IF NOT EXISTS si_actions (
    id TEXT PRIMARY KEY,
    diagnosis_id TEXT NOT NULL REFERENCES diagnoses(id),
    -- ...
);
CREATE INDEX IF NOT EXISTS idx_si_actions_diagnosis ON si_actions(diagnosis_id);
```

**Problem**: No unique constraint prevents multiple actions per diagnosis. This could lead to:
- Duplicate action execution for same diagnosis
- Inconsistent state in learning records
- Data integrity issues

**Solution**: Add migration file for unique index.

**New File**: `migrations/002_si_actions_unique_diagnosis.sql`
```sql
-- Migration: Add unique constraint on diagnosis_id in si_actions
-- This ensures one action per diagnosis (1:1 relationship)
-- Version: 2

-- Drop existing non-unique index
DROP INDEX IF EXISTS idx_si_actions_diagnosis;

-- Create unique index
CREATE UNIQUE INDEX IF NOT EXISTS idx_si_actions_diagnosis_unique
ON si_actions(diagnosis_id);
```

**Implementation Steps**:
1. Create `migrations/002_si_actions_unique_diagnosis.sql`
2. Update `storage/core.rs` to run all migrations in order
3. Add conflict handling in `SelfImprovementStorage::insert_action()`
4. Add test for duplicate diagnosis_id rejection

---

## Medium Priority

### M1. ConfigScope Mode Validation

**Status**: 🔴 Needs Implementation
**Location**: `src/self_improvement/types.rs:263-277`, `anthropic_calls.rs:661-680`

**Current Code**:
```rust
pub enum ConfigScope {
    Global,
    Mode(String),   // Unvalidated string
    Tool(String),   // Unvalidated string
}
```

**Problem**: `ConfigScope::Mode(String)` accepts any string without validating against known reasoning modes.

**Solution**: Add validation against `ReasoningMode` enum.

**Implementation**:

1. **Add validation module** in `src/self_improvement/types.rs`:
```rust
use crate::prompts::ReasoningMode;

impl ConfigScope {
    /// Validate that Mode variant contains a known reasoning mode.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Mode(mode_str) => {
                // Convert to lowercase for case-insensitive comparison
                let normalized = mode_str.to_lowercase();
                let valid_modes = [
                    "linear", "tree", "divergent", "reflection", "checkpoint",
                    "auto", "graph", "detect", "decision", "evidence",
                    "timeline", "mcts", "counterfactual"
                ];
                if valid_modes.contains(&normalized.as_str()) {
                    Ok(())
                } else {
                    Err(format!("Unknown mode: {mode_str}"))
                }
            }
            Self::Tool(tool_str) => {
                // Validate tool names follow pattern: reasoning_<mode>
                if tool_str.starts_with("reasoning_") {
                    Ok(())
                } else {
                    Err(format!("Invalid tool format: {tool_str}"))
                }
            }
            Self::Global => Ok(()),
        }
    }
}
```

2. **Update parse_scope()** in `anthropic_calls.rs`:
```rust
fn parse_scope(scope: Option<&String>) -> Result<ConfigScope, ModeError> {
    let config_scope = /* existing parsing logic */;

    // Validate before returning
    config_scope.validate().map_err(|reason| ModeError::InvalidValue {
        field: "scope".into(),
        reason,
    })?;

    Ok(config_scope)
}
```

**Files to Modify**:
1. `src/self_improvement/types.rs` - Add `validate()` method
2. `src/self_improvement/anthropic_calls.rs` - Call validation in `parse_scope()`

---

### M2. Batch Database Insert Methods

**Status**: 🔴 Needs Implementation
**Location**: `src/self_improvement/storage.rs`

**Problem**: Current implementation inserts records one at a time, which is inefficient for bulk loads.

**Solution**: Add batch insert methods using SQLite's multi-value INSERT.

**Implementation**:

```rust
impl SelfImprovementStorage {
    /// Batch insert invocation records.
    pub async fn batch_insert_invocations(
        &self,
        records: &[InvocationRecord],
    ) -> Result<u64, StorageError> {
        if records.is_empty() {
            return Ok(0);
        }

        // SQLite supports up to 999 variables per statement
        // Each record uses 5 bind variables, so batch size = 999 / 5 = 199
        const BATCH_SIZE: usize = 199;
        let mut total_inserted = 0u64;

        for chunk in records.chunks(BATCH_SIZE) {
            let placeholders: String = chunk
                .iter()
                .map(|_| "(?, ?, ?, ?, ?)")
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "INSERT INTO invocations (id, tool_name, latency_ms, success, created_at) VALUES {}",
                placeholders
            );

            let mut query = sqlx::query(&sql);
            for record in chunk {
                query = query
                    .bind(&record.id)
                    .bind(&record.tool_name)
                    .bind(record.latency_ms)
                    .bind(record.success)
                    .bind(record.created_at.to_rfc3339());
            }

            let result = query.execute(&self.pool).await.map_err(|e| {
                query_error(format!("Batch insert failed: {e}"))
            })?;

            total_inserted += result.rows_affected();
        }

        Ok(total_inserted)
    }

    /// Batch insert learning records.
    pub async fn batch_insert_learnings(
        &self,
        records: &[LearningRecord],
    ) -> Result<u64, StorageError> {
        // Similar implementation with appropriate columns
    }
}
```

**Files to Modify**:
1. `src/self_improvement/storage.rs` - Add batch methods
2. Add tests for batch operations with various sizes

---

### M3. Integration Tests for Database Operations

**Status**: 🔴 Needs Implementation
**Location**: `tests/integration/` (new directory)

**Implementation**:

**New File**: `tests/integration/self_improvement_db_tests.rs`
```rust
//! Integration tests for self-improvement database operations.

use mcp_reasoning::self_improvement::{
    SelfImprovementStorage, InvocationRecord, DiagnosisRecord,
    ActionRecord, LearningRecord
};
use mcp_reasoning::storage::SqliteStorage;

#[tokio::test]
async fn test_full_self_improvement_cycle() {
    // Setup in-memory database
    let storage = SqliteStorage::new_memory().await.unwrap();
    let si_storage = SelfImprovementStorage::new(storage.pool().clone());

    // 1. Insert invocation records
    let invocation = InvocationRecord::new("reasoning_linear", 100, true, Some(0.9));
    si_storage.insert_invocation(&invocation).await.unwrap();

    // 2. Verify retrieval
    let retrieved = si_storage.get_invocations_since(
        chrono::Utc::now() - chrono::Duration::hours(1)
    ).await.unwrap();
    assert_eq!(retrieved.len(), 1);

    // 3. Test diagnosis → action → learning chain
    // ...
}

#[tokio::test]
async fn test_batch_insert_performance() {
    let storage = SqliteStorage::new_memory().await.unwrap();
    let si_storage = SelfImprovementStorage::new(storage.pool().clone());

    // Create 1000 records
    let records: Vec<InvocationRecord> = (0..1000)
        .map(|i| InvocationRecord::new(
            format!("tool_{}", i % 10),
            100 + (i as i64),
            i % 5 != 0,  // 80% success rate
            Some(0.8),
        ))
        .collect();

    let inserted = si_storage.batch_insert_invocations(&records).await.unwrap();
    assert_eq!(inserted, 1000);
}

#[tokio::test]
async fn test_unique_diagnosis_constraint() {
    let storage = SqliteStorage::new_memory().await.unwrap();
    let si_storage = SelfImprovementStorage::new(storage.pool().clone());

    // Insert diagnosis
    // Insert action for diagnosis
    // Attempt duplicate action for same diagnosis - should fail
}
```

**Files to Create**:
1. `tests/integration/mod.rs`
2. `tests/integration/self_improvement_db_tests.rs`

---

## Implementation Order

```
Phase 1 (Critical - Day 1):
├── C1. Verify prompt escaping coverage [1 hour]
└── Review existing security measures

Phase 2 (High Priority - Day 1-2):
├── H1. Document JSON size limit rationale [30 min]
├── H2. Add Send + Sync bounds [2 hours]
└── H3. Add unique diagnosis_id constraint [2 hours]

Phase 3 (Medium Priority - Day 2-3):
├── M1. ConfigScope validation [2 hours]
├── M2. Batch insert methods [3 hours]
└── M3. Integration tests [4 hours]
```

---

## Verification Checklist

After implementation, verify:

- [ ] All tests pass: `cargo test`
- [ ] Clippy clean: `cargo clippy -- -D warnings`
- [ ] Format check: `cargo fmt --check`
- [ ] Coverage maintained: `cargo llvm-cov`
- [ ] Documentation updated
- [ ] Integration tests cover all new functionality
- [ ] Migration runs successfully on existing databases

---

## Risk Assessment

| Item | Risk | Mitigation |
|------|------|------------|
| H2. Send+Sync bounds | May break existing code | Run full test suite before commit |
| H3. Unique constraint | May fail on existing data | Check for duplicates before migration |
| M2. Batch inserts | SQLite variable limit | Chunk batches to 199 records |
| M3. Integration tests | Test isolation | Use in-memory databases |

---

## Notes

### Already Complete
- DateTime parsing helper (M2 from original list) - implemented in storage.rs:28-33
- RFC3339 consistency (C2 from original list) - verified in storage operations

### Design Decisions
- **100KB JSON limit** preferred over 1MB for security (DoS prevention)
- **Validation approach** for ConfigScope uses string matching rather than parsing to ReasoningMode to avoid circular dependencies
- **Batch size of 199** chosen based on SQLite's 999 variable limit (5 vars per record)
