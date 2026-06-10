# Implementation Plan: Self-Heal Repair Safety — Attribution & Validation-Invariant Guard

**Branch**: `002-heal-repair-safety` | **Date**: 2026-06-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-heal-repair-safety/spec.md`

## Summary

Two safety refinements to the existing self-heal loop (feature `001-heal-parse-schema`), found by
inducing a real `confidence`-out-of-range schema violation against the running server and tracing what
the propose path would do with it:

1. **Validation-invariant guard (US1, P1)** — a candidate fix must never weaken or remove a
   validation/range/contract check. Add a diff-scanning guard that runs *before* a fix is written, and
   fold its verdict into admissibility so "weakens a protected invariant" disqualifies a proposal
   regardless of a passing reproducing test (Constitution III: acceptance must prove a fix, not a
   weakened oracle).
2. **Attribution before propose (US2, P2)** — a recurring defect becomes propose-eligible only on a
   *stable triggering path* (the same redacted input recurring), is held back when the failure is
   spread across varied inputs (likely input-induced), and routes to drift when it correlates with a
   recorded model-version change. Ambiguous ⇒ not eligible (fail safe).

Both reuse 001's never-merge / operator-review model, its `DefectRecord`/`DefectLog`, its model-version
signal, and its `CommandRunner`-backed repair pipeline.

## Technical Context

**Language/Version**: Rust (edition 2024, MSRV 1.94 — matches the crate).

**Primary Dependencies**: existing only — `serde`, `tokio`, `sqlx` (SQLite), the feature-001 `heal`/
`repair`/`heal_cycle`/`heal_review` modules. No new crates.

**Storage**: SQLite (existing). No new tables expected; `DefectLog` is in-process. A per-input-hash
occurrence map is added in memory.

**Testing**: `cargo test` with `mockall` traits + the scripted `CommandRunner` fake (no real
cargo/git/gh runs); in-memory SQLite for storage.

**Target Platform**: the running MCP server (stdio/http), native.

**Project Type**: single Rust project (the existing crate).

**Performance Goals**: negligible — both checks run only inside an already-gated propose cycle
(default OFF), at most K times per cycle. The diff scan is over a handful of changed files.

**Constraints**: Constitution V (no `unwrap`/`expect` in prod, files ≤500 lines, 95%+ coverage,
fmt/clippy clean). Guards MUST fail safe (block/flag on uncertainty).

**Scale/Scope**: small — two new guard functions + an eligibility predicate + their wiring into
`fix_gen`/`heal_review`/`heal_cycle`/`DefectLog`. Estimated ≤ ~600 net new lines incl. tests.

## Constitution Check

*GATE: must pass before Phase 0. Re-checked after Phase 1.*

| Principle | Status | Note |
|-----------|--------|------|
| I. Measured Improvement Only | ✅ strengthens | Neither guard fabricates a win; both only *block*. They reduce false "wins" by stopping fixes that mask/weaken a check. |
| II. Operational Health Is the Fitness | ✅ aligned | Attribution explicitly separates model-drift (route away) from code defects, reinforcing II. |
| III. Test-Gated Self-Modification | ✅ strengthens | The core of US1: the reproducing-test gate proves a behavior change, not correctness; this adds the missing "a fix may not weaken the oracle/validation" guard. |
| IV. Bounded, Reviewable Autonomy | ✅ strengthens | Attribution narrows what the loop may act on; never-merge/operator review unchanged. |
| V. Engineering Quality Bar | ✅ must hold | Same gates as 001; new files ≤500 lines, 95%+ coverage, no prod unwrap/expect. |

No violations. **Complexity Tracking: none.**

## Project Structure

### Documentation (this feature)

```text
specs/002-heal-repair-safety/
├── plan.md              # This file
├── research.md          # Phase 0 — design decisions D1–D6
├── data-model.md        # Phase 1 — entities + signature/eligibility changes
├── quickstart.md        # Phase 1 — how to verify both guards
├── contracts/
│   └── internal-interfaces.md   # Phase 1 — the guard/eligibility function signatures
└── tasks.md             # Phase 2 (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

Changes land in the existing `src/self_improvement/` tree from feature 001:

```text
src/self_improvement/
├── heal/
│   ├── detect.rs      # DefectLog: track per-input-hash occurrences (US2 eligibility)
│   ├── types.rs       # DefectRecord: per-input-hash counts; eligibility helper
│   ├── plan.rs        # eligibility predicate (stable-path vs varied; drift already handled)
│   └── invariant_guard.rs   # NEW — diff scan for weakened validation/range/contract checks (US1)
├── repair/
│   ├── fix_gen.rs     # run the invariant guard before writing; carry its verdict
│   └── types/...      # GeneratedFix gains an `weakens_invariant` verdict (or via FixProposal)
├── heal_review.rs     # admissibility: block when the invariant guard flagged the fix (US1)
└── heal_cycle.rs      # propose-eligibility gate (US2) before localize/propose
```

**Structure Decision**: extend the existing feature-001 modules in place; add exactly one new file,
`heal/invariant_guard.rs`, for the diff-scanning guard (kept separate so it is independently testable
and ≤500 lines). No new crates, no new DB tables.

## Complexity Tracking

No constitution violations — section intentionally empty.
