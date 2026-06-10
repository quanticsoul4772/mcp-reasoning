# Quickstart: Verifying Repair Safety

This feature has no operator-facing surface of its own — it changes what the self-heal loop *won't* do.
Verification is by test, using feature 001's scripted `CommandRunner` fake and in-memory storage (no
real cargo/git/gh, no repo mutation).

## US1 — validation-invariant guard

1. Build a `FixProposal` whose proposed change to a mode file **widens a range check** (e.g. the
   current content has `(0.0..=1.0).contains(&confidence)` and the fix changes it to `(0.0..=100.0)`).
2. Assert `scan_for_weakened_invariants` returns `weakens = true` with a reason naming the range.
3. Assert the proposal is **not admissible** even with `grounded = suite_green = quality_green = true`.
4. Assert no PR is opened (the `gh` runner is never reached).
5. Negative: a fix that edits the same file but touches **no** validation line returns
   `weakens = false` and proceeds normally (no false positive — SC-005).

## US2 — attribution before propose

1. Seed a `DefectLog` with the same schema-violation signature from **three different inputs**
   (distinct `input_hash`, each count = 1). Assert `is_propose_eligible(3)` is **false** and the cycle
   records it as `HeldBack` — no localize/propose.
2. Seed the same signature from **one stable input repeated three times** (same `input_hash`, count = 3).
   Assert `is_propose_eligible(3)` is **true** and the cycle proceeds to propose.
3. Seed a recurring defect whose window overlaps a recorded model-version change. Assert
   `classify_eligibility` returns `Drift` and it routes to the drift response, not propose.

## Guardrails (must hold)

- No fix that weakens a validation/range/contract check can become an admissible proposal (SC-001).
- A failure spread across varied inputs is recorded but never auto-proposed (SC-002).
- A failure correlated with a model-version change routes to drift (SC-003).
- Every held-back or blocked defect keeps a stated reason and stays operator-visible (SC-004).
- Genuine-defect fixes touching no validation invariant are not blocked (SC-005, zero false positives).
- All guards fail safe — on any uncertainty they block/hold-back, never silently admit.

## Accepted trade-off (intentional false-negative)

Attribution favors precision over recall: a **real** code defect that happens to be triggered by
varied inputs (distinct `input_hash` each time, none repeating to threshold) is **recorded** but is
**not** auto-proposed — it is held back with a stated reason and stays operator-visible. This is the
deliberate cost of refusing to act on input-induced noise. Detection is unchanged; only the propose
path is narrowed. An operator who recognizes a held-back defect as genuine can still act on it
manually; the loop simply will not open a PR on that evidence on its own.
