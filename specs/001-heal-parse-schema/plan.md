# Implementation Plan: Self-Healing of Parse/Schema Failures (Operator-Reviewed)

**Branch**: `001-heal-parse-schema` | **Date**: 2026-06-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-heal-parse-schema/spec.md`

## Summary

Extend the existing self-improvement (MAPE-K) loop so the server detects its own parse/malformed-
output and schema-violation failures, distinguishes recurring defects from transients (and from
model/provider drift), localizes the responsible component, and produces an **operator-reviewable
pull request** containing a candidate fix plus an **execution-grounded reproducing test** (fails on
current code, passes on the fix). Nothing is merged to the live server without operator approval and
a green full test suite. Each accepted defect→fix→test is recorded for reuse. The work adds the
missing **Plan** (prioritization) step and a new propose-PR action; it does not auto-edit a running
service.

## Technical Context

**Language/Version**: Rust (workspace MSRV 1.94, per `Cargo.toml`)

**Primary Dependencies**: existing crate modules — `self_improvement/` (monitor, analyzer,
executor, learner, allowlist, circuit_breaker, sensor), `metrics/`, `modes/` (handlers + parsers),
`server/params.rs`/`requests.rs` (JsonSchema), `eval/` (scorer/runner/stats); Anthropic API
(diagnosis/fix generation), `gh` CLI (PR creation). No new heavy dependencies expected.

**Storage**: SQLite (existing SI storage layer) for Defect Record / Fix Proposal / Knowledge Entry.

**Testing**: `cargo test`; `cargo clippy -- -D warnings`; `cargo fmt --check`; `cargo llvm-cov
--fail-under-lines 95`; rustc compile as the cheapest oracle.

**Target Platform**: stdio/HTTP MCP server (Windows/Linux), operator-run (not 24/7).

**Project Type**: single Rust project (MCP server) — existing layout under `src/`.

**Performance Goals**: detection adds negligible per-call overhead (counter increments on the
existing handler/parse path); proposal generation is bounded per cycle (API cost is a constraint).

**Constraints**: programmatic acceptance only (no LLM self-judgment); fitness/measurement code
outside the patcher's writable surface; PR-for-review default; bounded proposals per cycle.

**Scale/Scope**: v1 covers two failure classes (parse/malformed-output, schema-violation) across the
existing reasoning tools/modes; other classes (timeout, auth/quota, upstream-API) are out of scope.

## Constitution Check

*GATE: Must pass before Phase 0. Re-checked after Phase 1.*

- **I. Measured Improvement Only** — PASS. Acceptance is the test gate + green suite; no fabricated
  value. The acceptance signal (tests/error counters) lives outside the patcher's writable surface
  (FR-009, FR-010). Existing `sensor.rs` + `circuit_breaker` divergence tripwire remain in force.
- **II. Operational Health Is the Fitness** — PASS. The target is the server's own parse/schema
  failures, not model accuracy; drift is classified separately and not patched (FR-012).
- **III. Test-Gated Self-Modification** — PASS. Every proposal carries an execution-grounded
  reproducing test (fails pre, passes post); admissible only with green full suite + rustc + clippy
  (FR-006, FR-008).
- **IV. Bounded, Reviewable Autonomy** — PASS. Propose-PR only; no auto-merge to live server
  (FR-007); `require_approval` already defaults true; new Plan step ranks by freq × severity ×
  confidence; per-cycle proposal cap (FR-013).
- **V. Engineering Quality Bar** — PASS (enforced by the same gates the feature uses; new code held
  to no-unsafe/no-panic/≤500-line/95%-coverage).

No violations → Complexity Tracking left empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-heal-parse-schema/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (internal interfaces)
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── metrics/
│   └── mod.rs                 # ADD: parse-failure, schema-violation, exception counters per tool/mode
├── self_improvement/
│   ├── monitor.rs             # EXTEND: surface the new counters in MonitorResult
│   ├── analyzer.rs            # EXTEND: classify failure (parse|schema|drift), localize component
│   ├── plan.rs                # NEW: prioritization step (freq × severity × confidence) — missing phase
│   ├── executor.rs            # EXTEND: new ProposePR action (generate fix + reproducing test)
│   ├── repair/                # NEW: fix+test generation, reproducing-test grounding, gh PR wrapper
│   │   ├── mod.rs
│   │   ├── test_synth.rs      # generate + execution-ground the reproducing test
│   │   └── pr.rs              # gh PR creation (no auto-merge)
│   ├── types/                 # EXTEND: DefectRecord, FixProposal, KnowledgeEntry, new ActionType
│   ├── storage/               # EXTEND: persist defect/proposal/knowledge
│   ├── allowlist.rs           # EXTEND: bound ProposePR; keep auto-apply classes narrow
│   ├── circuit_breaker.rs     # REUSE: per-cycle proposal cap + divergence tripwire
│   └── sensor.rs              # REUSE: measurement stays outside the patched surface
└── modes/ , server/           # detection hooks at the parse/handler boundary (read-only here)

tests/
├── integration/               # induced parse/schema failure → detection → proposal (no auto-merge)
└── unit/                      # per-module: classifier, plan ranking, test grounding, gating
```

**Structure Decision**: single Rust project; extend the existing `self_improvement/` MAPE-K modules
and `metrics/` rather than introduce a parallel system. New code: a `plan.rs` phase (the missing
Plan step), a `repair/` submodule (fix+test synthesis + `gh` PR), and three new record types.

## Complexity Tracking

> No constitution violations; section intentionally empty.
