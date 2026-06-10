# Tasks: Self-Heal Repair Safety — Attribution & Validation-Invariant Guard

**Feature**: `specs/002-heal-repair-safety` | **Branch**: `002-heal-repair-safety`
**Inputs**: spec.md, plan.md, research.md (D1–D6), data-model.md, contracts/internal-interfaces.md, quickstart.md
**Testing**: tests included (Constitution III — Test-Gated Self-Modification; repo TDD standard).

Task format: `- [ ] [ID] [P?] [Story?] Description with file path`. `[P]` = parallelizable
(different files, no incomplete dependency). Refines feature 001 (`src/self_improvement/`).

## Phase 1: Setup

- [X] T001 [P] Created `src/self_improvement/heal/invariant_guard.rs` with `ChangedFile` + `InvariantVerdict`; declared `pub mod invariant_guard;` and re-exported `scan_for_weakened_invariants`, `ChangedFile`, `InvariantVerdict` from `heal/mod.rs`.

## Phase 2: Foundational

No cross-story blockers: US1 (validation-invariant guard) and US2 (attribution) touch disjoint types
(`FixProposal`/`fix_gen` vs `DefectLog`/`heal_cycle`) and may proceed in parallel after Setup. The
admissibility predicate (`FixProposal::is_admissible`) is the only shared surface and is owned by US1.

## Phase 3: User Story 1 — Validation-invariant guard (P1) — MVP

**Goal**: a candidate fix that weakens a validation/range/contract check can never become an admissible
proposal and never opens a PR — regardless of a passing reproducing test (FR-001/002/009).
**Independent test**: a fix widening a range check → `weakens=true` → not admissible → no `gh` call;
a fix touching no validation line → `weakens=false` → proceeds (no false positive).

- [X] T002 [P] [US1] Unit tests in `invariant_guard.rs` (8): widened range → weakens; removed rejection branch → weakens; relaxed `<`→`<=` → weakens; dropped `!` negation → weakens; unreadable current → weakens (fail-safe); non-validation edit → not weakens (SC-005); identical content → not weakens; range parser. Verdict `reason` asserts the file + pattern.
- [X] T003 [done] Test added (`weakens_invariant_blocks_admissibility_despite_green_gates`): fully-green proposal with `weakens_invariant=true` is not admissible.
- [X] T004 [US1] Implemented `scan_for_weakened_invariants(changed, read_current)` in `invariant_guard.rs`: pure over the reader closure (no real fs); three conservative signals — (1) validation/rejection line count dropped, (2) a numeric `..=` range widened in place, (3) a guard relaxed (canon-equal-but-raw-different on `<=`/`>=`/`||`/`!`); unreadable current → flag (D6); `reason` names file + pattern (FR-009). 297 lines.
- [X] T005 [done] `FixProposal` gained `weakens_invariant` + `block_reason`; `is_admissible()` now `grounded ∧ suite_green ∧ quality_green ∧ ¬weakens_invariant`; admissibility test constructor updated.
- [X] T006 [done] `GeneratedFix` gained `weakens_invariant` + `block_reason`; `fix_gen` reads current on-disk content per changed file and runs `scan_for_weakened_invariants` BEFORE creating a branch/writing — a flagged fix returns early (no branch, no side effects) with the verdict.
- [X] T007 [done] `orchestrate::propose_pr` threads `weakens_invariant`/`block_reason` from `GeneratedFix` into `FixProposal`; `open_pr` is gated on `is_admissible()`, so a flagged fix never reaches `gh`.
- [X] T008 [done] Migration 011 (`ALTER TABLE heal_fix_proposals ADD COLUMN weakens_invariant/block_reason`, idempotent — tolerates duplicate-column on re-run) wired in `core.rs`; `upsert_fix_proposal`/`get_fix_proposal` bind/read them; round-trip test extended (persisted flag ⇒ not admissible).
- [X] T009 [done] Orchestrator test `fix_that_weakens_a_range_check_is_blocked_and_opens_no_pr`: seeded target file with a range check, widening fix → `weakens_invariant`, not admissible, no PR, only the synth grounding command ran (SC-001).
- [X] T010 [done] Orchestrator test `fix_near_validation_but_not_weakening_proceeds`: a fix editing a line adjacent to a validation check (check intact) is not blocked and opens a PR (SC-005, no false positive).

**Checkpoint**: US1 independently shippable — no fix that weakens a check can be proposed or accepted.

## Phase 4: User Story 2 — Attribution before propose (P2)

**Goal**: a recurring defect is propose-eligible only on a stable triggering path (same input recurs);
varied-input recurrence is held back; model-version-correlated spikes route to drift; ambiguous →
held back (FR-003–007).
**Independent test**: same class via 3 different inputs → not eligible (HeldBack); same input ×3 →
eligible; failure overlapping a model-version change → Drift.

- [X] T011 [done] `DefectLog` tests (`varied_inputs_recur_but_are_not_propose_eligible` / `stable_input_recurrence_is_propose_eligible`): 3 distinct inputs → max_input_occurrences=1, eligible(3)=false though Recurring; same input ×3 → max=3, eligible(3)=true.
- [X] T012 [done] `heal/eligibility.rs` tests (4): stable-path → Eligible; varied-input → HeldBack; below-threshold → HeldBack (FR-006); model-change → Drift even on stable path. Plus `heal_cycle` integration tests `varied_input_defect_is_held_back_not_proposed` and `model_change_routes_recurring_defect_to_drift`.
- [X] T013 [done] `DefectLog` now holds a `Tracked { record, per_input: HashMap<input_hash,u32> }` per signature; `observe` bumps the per-input count (input_hash via `redact`) and `sync_input_stats` snapshots `max_input_occurrences`/`distinct_inputs` into the record. `DefectRecord` gained those two fields (set to 1 in `observe`).
- [X] T014 [done] `DefectRecord::is_propose_eligible(threshold)` = `max_input_occurrences >= threshold && failure_class != Drift` (FR-004).
- [X] T015 [done] `EligibilityOutcome { Eligible, HeldBack(reason), Drift(reason) }` + `classify_eligibility(defect, model_changed_in_window, threshold)` in NEW `src/self_improvement/heal/eligibility.rs` (split out to keep files ≤500). Default branch → `HeldBack` (fail-safe FR-006); `Eligible` only on a positive stable-path signal.
- [X] T016 [done] `run_propose_cycle` wires eligibility in order `partition_drift → classify_eligibility → keep Eligible → rank_and_cap → propose`. New `held_back` counter; model-version `Drift` feeds the existing `drift` counter. Eligibility runs before `rank_and_cap` (Constitution IV).
- [X] T017 [done] `HealManager` (moved to `src/self_improvement/heal_manager.rs`) gained an `Arc<MetricsCollector>`; `tick` computes `latest_model_change` from `model_version_changes()` and passes it to `run_propose_cycle`. Wired in `mcp.rs` via `Arc::clone(&state.metrics)`.
- [X] T018 [done] `ProposeCycleSummary` gained `held_back: usize` + `held_back_reasons: Vec<(signature, reason)>`; every held-back/drift defect is logged (`tracing`) with its reason and carried on the summary; `mcp.rs` cycle log includes `held_back` (FR-007/FR-009/SC-004, no silent drops).

**Checkpoint**: US2 independently testable — only stable-path code defects auto-propose, and every declined defect is operator-visible with a reason.

## Phase 5: Polish & Cross-Cutting

- [X] T019 [done] Constitution V: new production paths use no `.unwrap()/.expect()`; every file this feature authored is ≤500 lines (largest `fix_gen.rs` 447; `heal_cycle.rs` 429 after the split; `storage/operations.rs` at 846 is pre-existing/grandfathered, +8 net this feature). `cargo fmt --check` clean; `cargo clippy --lib --bins -- -D warnings` clean (exit 0).
- [X] T020 [done] `cargo llvm-cov --fail-under-lines 95` passes (exit 0; TOTAL line coverage 95.54%). Feature-002 files all ≥95% line cov: eligibility 100%, types 100%, heal_manager 100%, invariant_guard 99.51%, orchestrate 99.44%, fix_gen 99.32%, detect 97.01%, heal_cycle 96.80%.
- [X] T021 [done] Updated `quickstart.md` (added an explicit "Accepted trade-off" note: varied-input real defects are recorded, not auto-proposed), corrected `contracts/internal-interfaces.md` for the interface shifts (eligibility split to `heal/eligibility.rs`; tuple variants `HeldBack(String)`/`Drift(String)`; `max_input_occurrences`/`distinct_inputs` as fields), and added a spec-002 safety note to `CLAUDE.md`.

## Dependencies & Order

- Setup (T001) → US1 (T002–T010) and US2 (T011–T018) may proceed in parallel (disjoint files) → Polish (T019–T021).
- Within US1: T002/T003 (tests) → T004/T005 (guard + admissibility) → T006/T007 (fix_gen + orchestrate wiring) → T008 (persist) → T009/T010 (orchestrator tests).
- Within US2: T011/T012 (tests) → T013/T014 (per-input tracking + eligibility) → T015/T016/T017 (cycle wiring) → T018 (operator surfacing).
- `[P]` tasks touch different files: T002/T003 together; T009/T010 together; T011/T012 together.

## Implementation strategy

- **MVP = US1 (P1)**: the validation-invariant guard — the last line of defense against a merged
  regression. Independently shippable without US2.
- Then US2 (attribution) narrows what the loop acts on, reducing how often the guard is exercised.
- Both fail safe (uncertainty ⇒ block/hold-back) and preserve feature 001's never-merge / operator-
  review model. No new crates; one new file (`invariant_guard.rs`); one migration (011).
