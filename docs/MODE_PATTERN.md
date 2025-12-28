# Mode Implementation Pattern

This document establishes the pattern for implementing reasoning modes to stay under the 500-line file limit.

## File Structure Pattern

Each mode should be split into submodules when it exceeds 400 lines:

```
src/modes/
├── mod.rs              # Re-exports and public API
├── {mode}/
│   ├── mod.rs          # Mode struct and public methods (<200 lines)
│   ├── types.rs        # Response types and enums (<200 lines)
│   ├── operations.rs   # Operation implementations (<300 lines)
│   └── tests.rs        # Unit tests (no limit, but separate)
```

## When to Split

1. **Single file is fine** when: Mode has ≤2 operations AND total code <400 lines
2. **Split required** when: Mode has >2 operations OR total code >400 lines

## Example: Simple Mode (Single File)

For modes like `detect` with only 2 operations:

```rust
// src/modes/detect.rs (~300-400 lines)
pub struct DetectMode<S, C> { ... }

impl<S, C> DetectMode<S, C> {
    pub fn biases(&self, content: &str) -> Result<BiasesResponse, ModeError> { ... }
    pub fn fallacies(&self, content: &str) -> Result<FallaciesResponse, ModeError> { ... }
}

#[cfg(test)]
mod tests { ... }
```

## Example: Complex Mode (Submodules)

For modes like `graph` with 8 operations:

```rust
// src/modes/graph/mod.rs (~100 lines)
mod operations;
mod types;

pub use types::{GraphResponse, GraphNode, GraphEdge};
pub struct GraphMode<S, C> { ... }

// src/modes/graph/types.rs (~150 lines)
pub struct GraphResponse { ... }
pub struct GraphNode { ... }
pub struct GraphEdge { ... }

// src/modes/graph/operations.rs (~400 lines)
impl<S, C> GraphMode<S, C> {
    pub fn init(&self, ...) { ... }
    pub fn generate(&self, ...) { ... }
    // ... 6 more operations
}
```

## Test Organization

When tests exceed 200 lines, move them to a separate file:

```rust
// src/modes/graph/tests.rs
#[cfg(test)]
use super::*;

#[tokio::test]
async fn test_init() { ... }
```

And include in mod.rs:
```rust
#[cfg(test)]
mod tests;
```

## Response Type Pattern

Keep response types small and focused:

```rust
// Good: Focused response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BiasesResponse {
    pub biases: Vec<DetectedBias>,
    pub assessment: BiasAssessment,
    pub confidence: f64,
}

// Bad: Kitchen sink response
pub struct AnalysisResponse {
    pub biases: Vec<Bias>,
    pub fallacies: Vec<Fallacy>,
    pub decision_matrix: Matrix,
    pub evidence_scores: Vec<Score>,
    // ... too many concerns
}
```

## Implementation Checklist

When implementing a new mode:

1. [ ] Estimate line count before starting
2. [ ] If >400 lines expected, plan submodule structure
3. [ ] Keep response types in separate file if >5 types
4. [ ] Separate tests if >200 lines
5. [ ] Run `wc -l` check before committing

## Existing Modes Requiring Refactoring

These modes exceed 500 lines and should be refactored when modified:

| Mode | Lines | Priority | Notes |
|------|-------|----------|-------|
| reflection.rs | 1035 | High | Many response types |
| tree.rs | 918 | High | 4 operations |
| checkpoint.rs | 781 | Medium | ~55% tests |
| linear.rs | 760 | Medium | ~50% tests |
| divergent.rs | 749 | Medium | ~50% tests |
| core.rs | 709 | Low | Shared utilities |
| auto.rs | 580 | Low | Close to limit |

## Phase 8 Mode Estimates

| Mode | Operations | Est. Lines | Structure |
|------|------------|------------|-----------|
| graph | 8 | ~800 | Submodules required |
| detect | 2 | ~300 | Single file OK |
| decision | 4 | ~500 | Borderline, watch it |
| evidence | 2 | ~300 | Single file OK |
| timeline | 4 | ~500 | Borderline, watch it |
| mcts | 2 | ~350 | Single file OK |
| counterfactual | 1 | ~250 | Single file OK |

## Summary

- **Prevention**: Plan structure before implementing
- **Target**: <400 lines production code per file
- **Tests**: Separate when >200 lines
- **Types**: Separate when >5 response types
