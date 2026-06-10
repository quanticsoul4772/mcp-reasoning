# Phase 1 Data Model: Repair Safety

All changes extend feature-001 types in `src/self_improvement/`. No new DB tables (the per-input map is
in-process; `FixProposal` persistence already stores arbitrary string/bool columns).

## Changed: `DefectRecord` / `DefectLog` (attribution, US2)

- **`DefectLog`** keeps, per defect signature, a `HashMap<input_hash, u32>` of per-input occurrence
  counts (alongside the existing aggregate). On each `observe`, increment the entry for that
  observation's `input_hash`.
- **`DefectRecord`** exposes `max_input_occurrences: u32` — the largest count for any single
  `input_hash` of this signature — and `distinct_inputs: u32`.
- **Eligibility rule**: `is_propose_eligible(threshold)` ⇔ `max_input_occurrences >= threshold`
  (a stable, repeatable path) AND `failure_class != Drift`. Varied-input recurrence
  (`distinct_inputs` high, `max_input_occurrences` low) is **not** eligible.

## New: `InvariantVerdict` (validation-invariant guard, US1)

- `weakens: bool` — true if any changed hunk loosens a validation/range/contract check.
- `reason: Option<String>` — human-readable: which file/pattern would be weakened (FR-009).
- Conservative: produced `weakens = true` on unrecognized validation forms or on read/scan error (D6).

## Changed: `FixProposal` (admissibility, US1)

- New field `weakens_invariant: bool` (default false), set from the `InvariantVerdict`.
- **Admissibility** becomes `grounded ∧ suite_green ∧ quality_green ∧ ¬weakens_invariant`.
- Optional `block_reason: Option<String>` carried for operator visibility (FR-007/FR-009). Persisted
  alongside the existing proposal columns (string).

## New: `EligibilityOutcome` (cycle decision, US2)

Returned by the propose-eligibility gate in `heal_cycle`, per recurring defect:

- `Eligible` — stable path, not drift → continue to localize/propose.
- `HeldBack { reason }` — varied-input / input-induced → record + alert, no propose.
- `Drift { reason }` — overlaps a model-version change → route to the drift response.

`HeldBack` and `Drift` defects remain queryable (FR-007); they are counted in the existing
`ProposeCycleSummary` (extend with `held_back` alongside `drift`) AND carry their reasons
(`held_back_reasons: Vec<(signature, reason)>`) so the SI status/metrics path can surface every
declined defect with a stated reason (FR-007/FR-009/SC-004 — no silent drops).

## State transitions (unchanged shape, new gate)

```
Observed ──(same input ≥ threshold)──▶ Recurring&Eligible ──▶ localize ──▶ fix
                                                                              │
                                            (varied inputs / drift)           ▼
Observed ──────────────────────────▶ Recurring&HeldBack          InvariantVerdict.weakens?
                                       (recorded, no propose)        ├─ yes ▶ blocked (not admissible)
                                                                      └─ no  ▶ admissibility gate (001)
```
