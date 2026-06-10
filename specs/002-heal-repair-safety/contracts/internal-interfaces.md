# Phase 1 Contracts: Internal Interfaces

Internal Rust module interfaces (no network endpoints). Extends feature 001.

## Validation-invariant guard (US1) — `heal/invariant_guard.rs`

```
struct ChangedFile { path: String, new_contents: String }
struct InvariantVerdict { weakens: bool, reason: Option<String> }

// Pure: compares each proposed file against its current on-disk content (supplied
// via the reader closure so it is unit-testable with no real fs) and scans the
// delta for a weakened validation/range/contract check. Conservative: weakens=true
// on any unrecognized validation form or read/scan error (D1/D6).
fn scan_for_weakened_invariants(
    changed: &[ChangedFile],
    read_current: impl Fn(&str) -> Option<String>,
) -> InvariantVerdict
```

Detected weakenings (heuristic, line/diff level): removed `return Err`/`bail!` on a guard line; a
range literal (`..=`, `..`) or comparison constant widened outward; a guard comparison relaxed
(`<`→`<=`, added `||`, removed `!`); a `.contains(&x)` / membership check deleted.

## Fix generation (US1) — `repair/fix_gen.rs`

```
// Runs scan_for_weakened_invariants BEFORE writing any file (like the integrity
// path guard). Sets GeneratedFix.weakens_invariant + reason.
GeneratedFix { …existing…, weakens_invariant: bool, block_reason: Option<String> }
```

## Admissibility (US1) — `heal/types.rs`

```
impl FixProposal {
    // grounded ∧ suite_green ∧ quality_green ∧ ¬weakens_invariant
    fn is_admissible(&self) -> bool
}
```

`heal_review::accept_proposal` already gates on `is_admissible()`, so the new conjunct closes the
operator-accept path with no further change there (a flagged fix returns `NotAdmissible`).

## Attribution / eligibility (US2) — `heal/detect.rs`, `heal/types.rs`, `heal/eligibility.rs`, `heal_cycle.rs`

```
struct DefectRecord {
    …existing…,
    max_input_occurrences: u32,  // largest count for any single input_hash
    distinct_inputs: u32,        // number of distinct input_hashes seen
}
impl DefectRecord {
    fn is_propose_eligible(&self, threshold: u32) -> bool  // stable path ∧ not drift
}

// heal/eligibility.rs (split out to keep files ≤500):
enum EligibilityOutcome { Eligible, HeldBack(String), Drift(String) }

// Called by run_propose_cycle BEFORE localize/propose, per recurring defect:
fn classify_eligibility(
    defect: &DefectRecord,
    model_changed_in_window: bool,
    threshold: u32,
) -> EligibilityOutcome
```

`DefectLog` (in `heal/detect.rs`) tracks per-input occurrence counts internally and snapshots
`max_input_occurrences`/`distinct_inputs` onto each `DefectRecord` on `observe`. `HealManager`
(`heal_manager.rs`) supplies `model_changed_in_window` from `MetricsCollector::model_version_changes`.

`run_propose_cycle` only localizes/proposes `Eligible` defects; `HeldBack`/`Drift` are recorded and
counted in `ProposeCycleSummary { …, held_back, drift }` and remain operator-visible (FR-007).

## Guarantees (must hold)

- A fix that weakens any validation/range/contract check is **never admissible** and **never opens a
  PR** — independent of grounding/suite/quality (FR-001/FR-002).
- A defect recurring across varied inputs, or correlated with a model-version change, is **never
  auto-proposed**; it is recorded with a reason (FR-003/FR-005/FR-007).
- Every block/hold-back carries a human-readable reason (FR-009).
- All guards fail safe: uncertainty ⇒ block/hold-back (FR-006/D6).
