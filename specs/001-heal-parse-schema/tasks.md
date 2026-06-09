# Tasks: Self-Healing of Parse/Schema Failures (Operator-Reviewed)

**Feature**: `specs/001-heal-parse-schema` | **Branch**: `001-heal-parse-schema`
**Inputs**: spec.md, plan.md, research.md, data-model.md, contracts/internal-interfaces.md, quickstart.md
**Testing**: tests included (Constitution III — Test-Gated Self-Modification; repo TDD standard).

Task format: `- [ ] [ID] [P?] [Story?] Description with file path`. `[P]` = parallelizable
(different files, no incomplete dependency).

## Phase 1: Setup

- [ ] T001 Create module skeletons: `src/self_improvement/plan.rs` and `src/self_improvement/repair/{mod.rs,test_synth.rs,pr.rs}`; declare them in `src/self_improvement/mod.rs`
- [ ] T002 [P] Add `gh` CLI availability check + thin wrapper stub in `src/self_improvement/repair/pr.rs` (no PR yet; returns `not_configured` if `gh` absent)

## Phase 2: Foundational (blocks all user stories)

- [X] T003 [P] Add `DefectRecord` (with `FailureClass` enum Parse|Schema|Drift, `DefectStatus` enum) — implemented in `src/self_improvement/heal/types.rs`
- [X] T004 [P] Add `FixProposal` and `KnowledgeEntry` types — implemented in `src/self_improvement/heal/types.rs`
- [ ] T005 Add `ProposePR` variant to `ActionType` in `src/self_improvement/types/enums.rs` and register it in `src/self_improvement/allowlist.rs` (bounded; NOT auto-apply by default)
- [X] T006 Implement the success-signal integrity guard — implemented in `src/self_improvement/heal/guard.rs`: protected-path set (`tests/`, `src/metrics/`, `src/eval/`, `src/self_improvement/sensor.rs`, `circuit_breaker.rs`, `allowlist.rs`) the repair action may never modify (D6)
- [X] T007 [P] Unit test the integrity guard rejects edits to forbidden paths — in `src/self_improvement/heal/guard.rs` (`#[cfg(test)]`)
- [ ] T008 Extend SI storage to persist/load `DefectRecord`, `FixProposal`, `KnowledgeEntry` in `src/self_improvement/storage/operations.rs` + `records.rs`
- [ ] T009 [P] Unit tests for storage round-trip of the three records in `src/self_improvement/storage/tests.rs`
- [X] T039 Implement the redaction scrubber + content hash — implemented in `src/self_improvement/heal/redact.rs`: strip credential-shaped tokens, cap excerpt length, return `RedactedInput { hash, excerpt }` (FR-016, D8). `DefectRecord::observe` uses it — never the raw input
- [X] T040 [P] Unit test the scrubber removes API-key/credential patterns and bounds the excerpt, and that the hash is stable — in `src/self_improvement/heal/redact.rs` (`#[cfg(test)]`) (FR-016)

> Note: T039/T040 were added during `/speckit-analyze` remediation (finding U2). They execute in
> this Foundational phase (before US1), despite the higher IDs.

**Checkpoint**: types, action, integrity guard, storage, and the redaction scrubber exist and are tested.

## Phase 3: User Story 1 — Detect & record malformed/schema output (P1) — MVP

**Goal**: failures are detected, counted per tool/mode, and recorded — never silently dropped.
**Independent test**: induce non-conforming output → counter increments + DefectRecord created.

- [X] T010 [US1] Add `record_parse_failure(component)` / `record_schema_violation(component)` counters + `parse_failure_count` / `schema_violation_count` getters in `src/metrics/mod.rs`
- [~] T011 [US1] Parse-failure recording at the JSON-extract seam — LIVE for `reasoning_linear`: `DefectSink` (`heal/sink.rs`) wired into `linear.rs` `process` at the `extract_json` Err path (redacted, never raw); `AppState` now holds a shared `DefectLog` (`server/types.rs`) and the linear handler (`handlers_basic.rs`) attaches the sink — so it records in the running server. Proven by `records_parse_failure_via_sink`. REMAINING: the other 12 modes (same `with_defect_sink` one-liner in each handler).
- [~] T012 [US1] Schema-violation recording — LIVE for `reasoning_linear`: missing-field / invalid-confidence paths in `linear.rs` `process` record a schema violation via the sink. REMAINING: other modes' handlers + the server param-validation site (same pattern).
- [X] T013 [US1] Surface parse/schema counts — exposed via `MetricsCollector` getters (`parse_failure_count`/`schema_violation_count` per component + `total_parse_failures`/`total_schema_violations`), which `Monitor::check` already holds. Adding fields to `MonitorResult` itself was deliberately skipped: that struct is constructed at 13 sites (monitor/analyzer/manager, mostly tests) and the ripple isn't worth it vs. the available getters.
- [X] T014 [US1] Implement recurrence tracking (Observed→Recurring at ≥N, FR-003) — implemented as `DefectLog` in `src/self_improvement/heal/detect.rs` (process-scoped recurrence; rolling time-bound window still configurable-pending)
- [X] T015 [P] [US1] Test: induced parse failure increments counter + creates DefectRecord — implemented as unit tests in `src/self_improvement/heal/detect.rs` (end-to-end integration test via the live `extract_json` seam pending T011)
- [X] T016 [P] [US1] Test: induced schema violation recorded distinctly; one-off stays Observed (not Recurring) — implemented as unit tests in `src/self_improvement/heal/detect.rs`

**Checkpoint**: US1 independently testable; operators can see real defect rates.

## Phase 4: User Story 2 — Propose a reviewed fix with a reproducing test (P2)

**Goal**: a recurring defect yields a PR with a fix + grounded reproducing test; nothing auto-merges.
**Independent test**: recurring defect → PR opened with a test that fails on base, passes on fix; no merge.

- [ ] T041 [US2] Record the pinned model identifier per call and emit a model-version-change event when it changes, in `src/anthropic/client.rs` (or `metrics/mod.rs`); store the events for the classifier (FR-017, D3). Runs before T017.
- [ ] T017 [US2] Implement `classify(defect) -> FailureClass` (Parse|Schema|Drift) in `src/self_improvement/analyzer.rs`, consuming the model-version signal from T041 (D3: drift = broad/cross-tool or model-change-correlated)
- [ ] T018 [US2] Implement `localize(defect) -> component + source hint` (LLM diagnosis over error+input) in `src/self_improvement/analyzer.rs`
- [ ] T019 [US2] Implement the Plan step `plan(recurring) -> ranked, capped` in `src/self_improvement/plan.rs`: rank by frequency × severity (FR-014: class-weight × recurrence × blast-radius) × fix-confidence (FR-015: grounded + first-attempt-pass + knowledge-match), all bounded [0,1]; cap ≤K/cycle (D7)
- [ ] T020 [P] [US2] Unit tests for classify + plan ranking/cap in `src/self_improvement/analyzer.rs` and `src/self_improvement/plan.rs`
- [ ] T021 [US2] Implement reproducing-test synthesis + execution-grounding in `src/self_improvement/repair/test_synth.rs`: generate test, run on unpatched tree, REQUIRE fail (`grounded=true`) else abort (D4, FR-006)
- [ ] T022 [US2] Implement fix generation on a branch + validation (reproducing test passes, full `cargo test` green, fmt/clippy/rustc green) in `src/self_improvement/repair/mod.rs` (FR-008)
- [ ] T023 [US2] Implement `gh pr create` (branch → commit fix+test → open PR, set pr_url, review_status=Proposed; NEVER merge) in `src/self_improvement/repair/pr.rs` (D5, FR-007)
- [ ] T024 [US2] Wire `ProposePR` action through `src/self_improvement/executor.rs` (Analyze→Plan→Execute), enforcing the integrity guard from T006
- [ ] T025 [P] [US2] Integration test: recurring defect → FixProposal with `grounded=true`, PR opened, nothing merged in `tests/integration/heal_propose.rs`
- [ ] T026 [P] [US2] Integration test: reproducing test fails on base commit, passes on the fix; a non-grounded test aborts the proposal in `tests/integration/heal_propose.rs`
- [ ] T027 [P] [US2] Integration test: a drift-classified failure does NOT produce a code patch (routes to drift) in `tests/integration/heal_propose.rs`
- [ ] T042 [P] [US2] Integration test (SC-005 timing): once recurrence is met, a ready-for-review proposal is produced within one improvement cycle, in `tests/integration/heal_propose.rs`

**Checkpoint**: US2 independently testable; reviewable PRs produced, no auto-merge.

## Phase 5: User Story 3 — Accept only a proven, non-regressing fix + reuse (P3)

**Goal**: admit only `grounded ∧ suite_green ∧ quality_green`; record defect→fix→test; reuse on recurrence.
**Independent test**: approval blocked unless gate passes; KnowledgeEntry stored and reused.

- [ ] T028 [US3] Implement `admissible(proposal) -> bool` (grounded ∧ suite_green ∧ quality_green) in `src/self_improvement/repair/mod.rs` (FR-008)
- [ ] T029 [US3] Wire operator approve/reject to the existing SI override path so merge requires Approve AND admissible; loop cannot self-approve in `src/self_improvement/system.rs`
- [ ] T030 [US3] On acceptance, write a `KnowledgeEntry` (failure_signature → fix+test) in `src/self_improvement/storage/operations.rs`
- [ ] T031 [US3] On a new DefectRecord matching a KnowledgeEntry signature, reference it and skip re-diagnosis in `src/self_improvement/analyzer.rs` (FR-011, SC-006)
- [ ] T032 [P] [US3] Integration test: merge blocked when reproducing test fails or any suite test breaks in `tests/integration/heal_accept.rs`
- [ ] T033 [P] [US3] Integration test: recurrence of an accepted class reuses the KnowledgeEntry (no re-diagnosis) in `tests/integration/heal_accept.rs`
- [ ] T043 [P] [US3] Integration test (FR-009): no improvement/resolution is recorded unless the admissibility gate passed — assert a non-admissible proposal records no win, in `tests/integration/heal_accept.rs`

**Checkpoint**: full loop closed safely; cumulative.

## Phase 6: Polish & Cross-Cutting

- [ ] T034 [P] Enforce per-cycle proposal cap end-to-end + test the flood guard (FR-013) in `src/self_improvement/plan.rs`
- [ ] T035 [P] Drift response: alert + record on drift classification (no patch) in `src/self_improvement/analyzer.rs` (FR-012)
- [ ] T036 [P] Ensure no `.unwrap()/.expect()` in new production paths; files ≤500 lines; `cargo fmt --check` + `cargo clippy -- -D warnings` clean (Constitution V)
- [ ] T037 Coverage: `cargo llvm-cov --fail-under-lines 95` passes for the new modules
- [ ] T038 [P] Update `docs/` + the feature `quickstart.md` if interfaces shifted during implementation

## Dependencies & Order

- Setup (T001–T002) → Foundational (T003–T009) → US1 (T010–T016) → US2 (T017–T027) → US3 (T028–T033) → Polish (T034–T038).
- US1 is the MVP and is independently shippable. US2 depends on US1 (needs DefectRecords + recurrence). US3 depends on US2 (needs FixProposals).
- `[P]` tasks within a phase touch different files and may run together (e.g., T003/T004; T015/T016; T025/T026/T027; T032/T033).

## Parallel example (Foundational)

```
T003 (DefectRecord) ┐
T004 (FixProposal)  ├─ parallel (different type defs)
T007 (guard test)   ┘
```

## Implementation strategy

- **MVP = Phase 1+2+3 (US1)**: detection + recording only — delivers visible defect rates with zero
  self-modification risk. Ship and validate before building the propose/accept loop.
- Then US2 (propose-PR, gated, no auto-merge), then US3 (accept + knowledge reuse).
- Every self-modifying task (US2/US3) is governed by Constitution I/III/IV: programmatic acceptance,
  reproducing-test gate, PR-for-review, integrity guard.
