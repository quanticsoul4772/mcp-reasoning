# Tasks: Self-Healing of Parse/Schema Failures (Operator-Reviewed)

**Feature**: `specs/001-heal-parse-schema` | **Branch**: `001-heal-parse-schema`
**Inputs**: spec.md, plan.md, research.md, data-model.md, contracts/internal-interfaces.md, quickstart.md
**Testing**: tests included (Constitution III — Test-Gated Self-Modification; repo TDD standard).

Task format: `- [ ] [ID] [P?] [Story?] Description with file path`. `[P]` = parallelizable
(different files, no incomplete dependency).

## Phase 1: Setup

- [X] T001 Module skeletons: `src/self_improvement/repair/{mod.rs,test_synth.rs,fix_gen.rs,pr.rs}` created and declared (`pub mod repair;`); `mod.rs` hosts the `CommandRunner` boundary + `SystemCommandRunner` + `RepairError` + shared helpers. The plan/rank logic lives under `heal/plan.rs` (not a root `self_improvement/plan.rs`); `repair/pr.rs` reuses the `heal::pr_create_args` builder from T002 rather than moving it (keeps T002's no-merge test intact).
- [X] T002 [P] `gh` availability check + PR-arg builder — implemented in `src/self_improvement/heal/pr.rs` (`gh_available`, `pr_create_args`) with a test asserting the args never request an auto-merge (FR-007). Actually shelling out to `gh` is part of T023.

## Phase 2: Foundational (blocks all user stories)

- [X] T003 [P] Add `DefectRecord` (with `FailureClass` enum Parse|Schema|Drift, `DefectStatus` enum) — implemented in `src/self_improvement/heal/types.rs`
- [X] T004 [P] Add `FixProposal` and `KnowledgeEntry` types — implemented in `src/self_improvement/heal/types.rs`
- [X] T005 `ProposePR` variant added to `ActionType` (in `types/legacy.rs` — the live enum, not `enums.rs`) with `Display`/`FromStr` (`"propose_pr"`) and registered in `allowlist.rs` as an allowed type (NOT auto-applied — safety comes from `require_approval` + PR review, not from disallowing it; it takes no tunable parameters). Exhaustive match arms added everywhere required: `executor.rs` `execute` returns a failed result (ProposePR is async-dispatched, never run by the sync executor — a routing error if reached, never a panic/silent success) and `rollback` rejects with "close the PR" guidance; `manager.rs` `persist_config_recommendations` records nothing (no Config field). `action_outcome`'s `_` arm correctly yields `Recommended` (a PR is advisory, not applied to the live server). Tests: Display/FromStr roundtrip, allowlist-allows, executor defensive-arm (fails honestly), rollback-rejected. No regressions (executor 27 / allowlist 16 / manager 57).
- [X] T006 Implement the success-signal integrity guard — implemented in `src/self_improvement/heal/guard.rs`: protected-path set (`tests/`, `src/metrics/`, `src/eval/`, `src/self_improvement/sensor.rs`, `circuit_breaker.rs`, `allowlist.rs`) the repair action may never modify (D6)
- [X] T007 [P] Unit test the integrity guard rejects edits to forbidden paths — in `src/self_improvement/heal/guard.rs` (`#[cfg(test)]`)
- [X] T008 Persist/load `KnowledgeEntry` (`upsert_knowledge_entry` / `get_knowledge_by_signature`, migration `009`) AND `FixProposal` (`upsert_fix_proposal` / `get_fix_proposal` / `update_proposal_review`, migration `010_heal_fix_proposals.sql`) in `src/self_improvement/storage/operations.rs`. FixProposal persistence is required by US3 so an operator can approve/reject a proposal from an earlier cycle and "merge requires admissible AND approved" survives a restart; bools stored as INTEGER, `ProposalReview` via new `as_str`/`from_db` (unknown → `Proposed`, never an accidental approval). `DefectRecord` recurrence stays session-scoped in the in-memory `DefectLog` (FR-003).
- [X] T009 [P] Unit test for the `KnowledgeEntry` storage round-trip + upsert-overwrite in `src/self_improvement/storage/tests.rs` (`test_knowledge_entry_roundtrip_and_upsert`)
- [X] T039 Implement the redaction scrubber + content hash — implemented in `src/self_improvement/heal/redact.rs`: strip credential-shaped tokens, cap excerpt length, return `RedactedInput { hash, excerpt }` (FR-016, D8). `DefectRecord::observe` uses it — never the raw input
- [X] T040 [P] Unit test the scrubber removes API-key/credential patterns and bounds the excerpt, and that the hash is stable — in `src/self_improvement/heal/redact.rs` (`#[cfg(test)]`) (FR-016)

> Note: T039/T040 were added during `/speckit-analyze` remediation (finding U2). They execute in
> this Foundational phase (before US1), despite the higher IDs.

**Checkpoint**: types, action, integrity guard, storage, and the redaction scrubber exist and are tested.

## Phase 3: User Story 1 — Detect & record malformed/schema output (P1) — MVP

**Goal**: failures are detected, counted per tool/mode, and recorded — never silently dropped.
**Independent test**: induce non-conforming output → counter increments + DefectRecord created.

- [X] T010 [US1] Add `record_parse_failure(component)` / `record_schema_violation(component)` counters + `parse_failure_count` / `schema_violation_count` getters in `src/metrics/mod.rs`
- [~] T011 [US1] Parse-failure recording at the JSON-extract seam — LIVE for `reasoning_linear`: `DefectSink` (`heal/sink.rs`) wired into `linear.rs` `process` at the `extract_json` Err path (redacted, never raw); `AppState` now holds a shared `DefectLog` (`server/types.rs`) and the linear handler (`handlers_basic.rs`) attaches the sink — so it records in the running server. Proven by `records_parse_failure_via_sink`. `reasoning_tree` also wired live (parse recording at all 3 `extract_json` sites + handler sink). REMAINING: the other ~11 modes (same `with_defect_sink` pattern per handler) — mechanical.
- [~] T012 [US1] Schema-violation recording — LIVE for `reasoning_linear`: missing-field / invalid-confidence paths in `linear.rs` `process` record a schema violation via the sink. REMAINING: other modes' handlers + the server param-validation site (same pattern).
- [X] T013 [US1] Surface parse/schema counts — exposed via `MetricsCollector` getters (`parse_failure_count`/`schema_violation_count` per component + `total_parse_failures`/`total_schema_violations`), which `Monitor::check` already holds. Adding fields to `MonitorResult` itself was deliberately skipped: that struct is constructed at 13 sites (monitor/analyzer/manager, mostly tests) and the ripple isn't worth it vs. the available getters.
- [X] T014 [US1] Implement recurrence tracking (Observed→Recurring at ≥N, FR-003) — implemented as `DefectLog` in `src/self_improvement/heal/detect.rs` (process-scoped recurrence; rolling time-bound window still configurable-pending)
- [X] T015 [P] [US1] Test: induced parse failure increments counter + creates DefectRecord — implemented as unit tests in `src/self_improvement/heal/detect.rs` (end-to-end integration test via the live `extract_json` seam pending T011)
- [X] T016 [P] [US1] Test: induced schema violation recorded distinctly; one-off stays Observed (not Recurring) — implemented as unit tests in `src/self_improvement/heal/detect.rs`

**Checkpoint**: US1 independently testable; operators can see real defect rates.

## Phase 4: User Story 2 — Propose a reviewed fix with a reproducing test (P2)

**Goal**: a recurring defect yields a PR with a fix + grounded reproducing test; nothing auto-merges.
**Independent test**: recurring defect → PR opened with a test that fails on base, passes on fix; no merge.

- [X] T041 [US2] Model-version drift signal (FR-017, D3) — `MetricsCollector` gained `record_model_version(model, now_millis)` (first observation pins the baseline; a later differing identifier emits + stores a bounded `ModelVersionChange`), plus `current_model()` and `model_version_changes()` getters for the classifier. `AnthropicClient` carries an opt-in metrics handle (`with_metrics`, mirroring `with_defect_sink`) and records `request.model` on every `complete`/`complete_streaming`; wired live in `server/mcp.rs`. Proven by metrics unit tests + a wiremock client test that detects a model swap across two calls. The classify-side correlation (matching a defect spike against a change timestamp) consumes `model_version_changes()` and lands with the drift-response wiring (T035).
- [X] T017 [US2] `classify` (Parse|Schema|Drift) — implemented in `src/self_improvement/heal/plan.rs` as `classify` + `is_drift_class` + `blast_radius`: structural drift detection (a class broad across ≥threshold distinct components → Drift; localized → code defect). The model-version correlation (FR-017/T041) refines it when that signal lands.
- [X] T018 [US2] `localize(defect) -> Localization { component, source_hint }` on `Analyzer<C>` in `src/self_improvement/analyzer.rs`: LLM diagnosis (temp 0.0) over the failure class + already-redacted excerpt (FR-016 — no raw input reaches the model), parsed via the existing `extract_json_block`. Empty/missing component falls back to the defect's recorded component; a missing `source_hint` is a parse error. The hint is advisory — it focuses where the repair generates a fix; the integrity guard (`heal::guard`) still governs what may be edited. Tested with a mocked client (happy path, component fallback, missing-hint error).
- [X] T019 [US2] Plan step — implemented in `src/self_improvement/heal/plan.rs`: `severity` (FR-014: class-weight × recurrence × blast-radius, bounded [0,1]) and `rank_and_cap` (rank by frequency × severity, cap ≤K, exclude drift per FR-012/FR-013) + tests. Fix-confidence (FR-015) is an attempt outcome, applied post-attempt, not in selection. The new module lives under `heal/` (not the originally-named `plan.rs` at the `self_improvement/` root).
- [X] T020 [P] [US2] Unit tests for classify + plan ranking/cap — in `src/self_improvement/heal/plan.rs` (severity ordering/monotonicity, rank-and-cap, blast-radius, classify/drift)
- [X] T021 [US2] Reproducing-test synthesis + execution grounding in `src/self_improvement/repair/test_synth.rs` (`synthesize_reproducing_test`): LLM (temp 0.0) emits `{test_name,test_path,test_code}`; the path is validated (workspace-relative `.rs`, no `..`/absolute escape); the test is written and run via the injected `CommandRunner`. Grounding verdict (D4/FR-006): exit 0 → `NotGrounded` (abort); failed with a `test result: FAILED` summary → grounded; failed without it → `TestDidNotRun` (compile error, not a real repro). Non-grounded outcomes remove the written file. All externals go through the new `CommandRunner` trait (real `SystemCommandRunner` via `tokio::process`; scripted fake in tests) — no real `cargo`/`git` runs and the repo is never mutated. 5 tests (grounded / not-grounded / compile-error / path-escape / missing-code). Architecture chosen via `reasoning_decision` (CommandRunner trait, 0.90).
- [X] T022 [US2] Fix generation on a branch + validation in `src/self_improvement/repair/fix_gen.rs` (`generate_and_validate_fix` → `GeneratedFix`): LLM (temp 0.0) emits `{change_summary, files:[{path,contents}]}`; **the integrity guard (`heal::is_protected`) runs FIRST and hard-errors (`RepairError::Protected`) before any file is written** if the fix touches tests/metrics/eval/sensor/circuit_breaker/allowlist (D6/FR-010); then `git checkout -b <branch>` (non-zero = hard error), write files, run reproducing test (must pass — else short-circuit, no further gates), full `cargo test`, `cargo fmt --check`, `cargo clippy -D warnings`. Returns `{reproducing_passes, suite_green, quality_green}` for the admissibility gate to judge — this function never decides to proceed. Shared JSON/path/fs helpers + the scripted test runner were lifted into `repair/mod.rs` (DRY with test_synth). 5 tests (all-green / protected-path-rejected-before-any-side-effect / fix-doesn't-fix / suite+quality-fail / branch-creation-fail). No real cargo/git runs.
- [X] T023 [US2] `open_pr` in `src/self_improvement/repair/pr.rs` (D5/FR-007): stages exactly the proposal's files (`git add -- <files>`, never `git add -A`), commits on the branch, and runs `gh` with the args from `heal::pr_create_args` (T002 builder reused — whose test asserts no merge flag); returns the PR URL from `gh` stdout (empty URL = hard error). Never merges, never edits. A shared `run_checked` helper (returns the captured output) was added to `repair/mod.rs` and `fix_gen`'s branch creation refactored onto it. 4 tests (opens-PR-without-merge / no-URL error / commit-failure aborts before gh / empty-file-set refused). No real git/gh runs.
- [~] T024 [US2] Propose-PR pipeline DONE: `repair::propose_pr` (`orchestrate.rs`) chains synth→fix→(if admissible)PR into a `FixProposal`; AND the dispatch brain `heal_cycle::run_propose_cycle` (`src/self_improvement/heal_cycle.rs`) — over ranked/capped recurring defects (FR-012/FR-013: drift excluded, ≤K), it checks `find_reusable_fix` first (skip re-diagnosis, FR-011), else `localize → propose_pr → upsert_fix_proposal`, returning a `ProposeCycleSummary {proposed, not_admissible, reused, errored}`. Per-defect LLM/repair errors are counted (cycle continues); only storage errors abort. `localize` was extracted to a free fn in `analyzer.rs` (method delegates) so the cycle threads ONE client through localize+synth+fix. 5 heal_cycle tests. AND `HealManager` (`heal_cycle.rs`) — owns `client + SystemCommandRunner + Arc<DefectLog> + Arc<storage> + workspace + max_proposals`; `tick()` reads the live `DefectLog::recurring()` each call (newly-recurring defects picked up without restart) and runs the cycle, no-op when nothing recurs. 2 manager tests (ticks-proposes-for-recurring / no-op-when-empty). T024c DONE: gated spawn live in `mcp.rs`. Config gained `heal_propose_enabled` (default false), `heal_workspace` (Option, default None), `heal_max_proposals` (default 1, clamped ≤5) + `SELF_HEAL_*` env reads. The propose loop is spawned ONLY when `heal_propose_enabled && heal_workspace` is set — constructs `HealManager` with a dedicated `AnthropicClient` + `SystemCommandRunner` + `Arc::clone(&si_storage)` + `Arc::clone(&state.defect_log)`, then a `tokio::interval` loop calling `tick()` (shares the SI shutdown watch). Default-off → running the server never opens PRs; turning it on is an explicit operator choice and logs a `warn!`. 2 config tests (off-by-default / opt-in+clamp).
- [~] T025 [P] [US2] Scenario COVERED at the orchestrator level (`orchestrate.rs::admissible_fix_opens_a_pr_and_never_merges`): grounded `FixProposal`, PR opened, no merge subcommand/flag in any runner call. REMAINING: a dedicated `tests/integration/heal_propose.rs` exercising it end-to-end through the live dispatch (after T024b).
- [~] T026 [P] [US2] Scenario COVERED (`orchestrate.rs::not_grounded_aborts_before_any_fix_or_pr` + `test_synth` grounding tests): the reproducing test must fail on base (grounded) or the proposal aborts before any fix/PR. REMAINING: same end-to-end integration file.
- [~] T027 [P] [US2] Scenario COVERED (`orchestrate.rs::drift_or_protected_fix_routes_away_with_no_patch` + `fix_gen` integrity-guard test): a fix touching a protected/drift surface is rejected before any branch/patch. REMAINING: same end-to-end integration file.
- [~] T042 [P] [US2] Scenario COVERED (`heal_cycle::manager_tick_proposes_for_recurring_defects_in_the_log`): a recurring defect in the live `DefectLog` yields a ready-for-review proposal within a single `HealManager::tick()` (= one improvement cycle), satisfying SC-005 timing. REMAINING: the named end-to-end `tests/integration/heal_propose.rs` form.

**Checkpoint**: US2 independently testable; reviewable PRs produced, no auto-merge.

## Phase 5: User Story 3 — Accept only a proven, non-regressing fix + reuse (P3)

**Goal**: admit only `grounded ∧ suite_green ∧ quality_green`; record defect→fix→test; reuse on recurrence.
**Independent test**: approval blocked unless gate passes; KnowledgeEntry stored and reused.

- [X] T028 [US3] Admissibility gate — implemented as `FixProposal::is_admissible()` (grounded ∧ suite_green ∧ quality_green) in `src/self_improvement/heal/types.rs` + test (FR-008)
- [X] T029 [US3] Operator accept/reject in `src/self_improvement/heal_review.rs` (`accept_proposal`/`reject_proposal`): accept is gated on `FixProposal::is_admissible()` — a non-admissible proposal is REFUSED (`ReviewError::NotAdmissible`) and NO approval/knowledge is recorded (FR-008/FR-009); accept marks `review_status=Approved` via the operator-only `update_proposal_review`. The loop cannot self-approve: these are the operator override path; the cycle only ever produces `Proposed`. (`FixProposal` gained a `failure_signature` field — migration 010 column + storage bind/read — so an accepted proposal can be keyed into knowledge.)
- [X] T030 [US3] On acceptance, `accept_proposal` writes a `KnowledgeEntry` via `KnowledgeEntry::from_accepted_proposal` (pure builder in `heal/types.rs`, keyed by `failure_signature` → fix_summary + test_ref) and `upsert_knowledge_entry`.
- [X] T031 [US3] `find_reusable_fix(storage, defect)` in `heal_review.rs` looks up `get_knowledge_by_signature(defect.signature())` — a new defect of a previously-accepted (component, class) finds the guarding fix so the loop can skip re-diagnosis (FR-011, SC-006). 5 tests (accept-admissible writes+approves / accept-non-admissible refused records nothing / accept-missing errors / reject marks rejected / reuse recognizes same class, different class misses).
- [~] T032 [P] [US3] Scenario COVERED (`heal_review::accept_non_admissible_is_refused_and_records_nothing`): a non-admissible (suite/quality not green) proposal cannot be accepted. REMAINING: a dedicated `tests/integration/heal_accept.rs` end-to-end form.
- [~] T033 [P] [US3] Scenario COVERED (`heal_review::reuse_recognizes_a_previously_accepted_class`): recurrence of an accepted class finds the `KnowledgeEntry` (skip re-diagnosis); a different class misses. REMAINING: same integration file.
- [~] T043 [P] [US3] Scenario COVERED (`heal_review::accept_non_admissible_is_refused_and_records_nothing` asserts no approval AND no `KnowledgeEntry` written): no win/resolution recorded unless the admissibility gate passed (FR-009). REMAINING: same integration file.

**Checkpoint**: full loop closed safely; cumulative.

## Phase 6: Polish & Cross-Cutting

- [X] T034 [P] Per-cycle proposal cap enforced end-to-end: `heal_cycle::run_propose_cycle` ranks+caps via `heal::plan::rank_and_cap(..., max_proposals)` before any propose, so at most K proposals per cycle (FR-013). Flood guard tested by `heal_cycle::tests::caps_proposals_per_cycle` (two recurring defects, cap 1 → exactly one attempted). The rank/cap unit is also covered by `heal/plan.rs::rank_caps_and_orders_by_score`.
- [X] T035 [P] Drift response (FR-012, D3): `heal::plan::partition_drift(recurring, DEFAULT_DRIFT_THRESHOLD=3)` splits recurring defects into `(code, drift)` — drift = already-`Drift` OR a class broad across ≥3 distinct components (a model swap, not a code bug). `run_propose_cycle` now partitions FIRST: each drift defect is alerted (`tracing::warn!` with signature/component) and counted in `ProposeCycleSummary.drift`, and NEVER enters the propose path; only the localized code defects are ranked/capped/proposed. This closes the gap where a broad Parse/Schema failure (not literally `Drift`) would have been proposed. Tests: `plan::partition_drift_routes_broad_and_literal_drift_away`, `heal_cycle::broad_parse_failure_is_routed_to_drift_not_proposed`, and the updated literal-drift test (asserts `drift=1`). `mcp.rs` cycle log includes the drift count.
- [X] T036 [P] Constitution V clean: no `.unwrap()/.expect()` in new production paths (only under `#[allow]` in test modules — enforced by `cargo clippy --lib --bins -- -D warnings` which gates `unwrap_used`/`expect_used` in production); all new files ≤500 lines (largest: `repair/mod.rs` 472, `heal_cycle.rs` 461); `cargo fmt --check` clean.
- [X] T037 Coverage: crate TOTAL line coverage 95.61% — `cargo llvm-cov --fail-under-lines 95` PASSES. Every new module ≥95% lines: heal/{detect 95.05, guard/plan/pr/redact/sink/types 100}, heal_cycle 96.99, heal_review 99.30, repair/{fix_gen 99.26, mod 98.70, orchestrate/pr 100, test_synth 97.97}, analyzer 98.38. Added targeted tests for the previously-thin surface (SystemCommandRunner real run + spawn-failure, RepairError/ReviewError Display+From, CommandOutput helpers, shared-helper error branches, DefectSink Debug).
- [X] T038 [P] Docs updated to the as-built interfaces: `specs/001-heal-parse-schema/quickstart.md` gained an "Enabling the propose loop (default OFF)" section with the `SELF_HEAL_PROPOSE_ENABLED` / `SELF_HEAL_WORKSPACE` / `SELF_HEAL_MAX_PROPOSALS` env vars, the as-built operator flow (drift-alert → rank/cap → reuse-or-propose → persist → operator accept), and tightened guardrails (integrity guard rejects before writing; default-off). `CLAUDE.md` Environment Variables section documents the three `SELF_HEAL_*` vars (default-off, never-merges, requires `gh`).

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
