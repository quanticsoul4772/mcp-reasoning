# Phase 0 Research: Repair Safety — Attribution & Validation-Invariant Guard

All decisions reuse feature 001's infrastructure. No new dependencies. No `NEEDS CLARIFICATION` remain.

## D1 — How to protect a "validation invariant" the fix may not weaken

- **Decision**: A **diff-scanning content guard** (`heal::invariant_guard`), NOT a path guard. The
  existing integrity guard (`heal::is_protected`) is path-based — it can't be used here because
  validation invariants live *inside* mode files (e.g. the `(0.0..=1.0).contains(&confidence)` check in
  `linear.rs`) and a legitimate fix may edit those same files. Instead, for each changed file, compare
  the proposed new content against the current on-disk content and scan the delta for **weakening**:
  - a removed/relaxed rejection branch (a deleted `return Err(...)`/`bail!`/validation `?` on a guard
    line, or an `if` condition that previously rejected now widened),
  - a widened numeric/range bound (a `..=`/`..` range literal or comparison constant moved outward),
  - a relaxed comparison operator on a guard (`<` → `<=`, `==` → `!=`-style loosening, `&&` → `||`).
- **Rationale**: Constitution III — the reproducing-test gate proves a behavior *change*; it cannot tell
  a fix from a weakened oracle. The only place to catch "the fix loosened a correct check" is the fix
  *diff* itself. Heuristic, deliberately conservative.
- **Fail-safe**: ambiguous or unrecognized validation forms ⇒ **flag** (treated as weakening), never
  silent pass (FR-006/FR-009). False positives are acceptable (a legitimate fix gets flagged for human
  review); false negatives are not.
- **Alternatives**: (a) protect whole mode files — rejected, blocks all legitimate mode fixes. (b) LLM
  judges whether the fix weakens a check — rejected, Constitution III forbids an LLM as the acceptance
  oracle. (c) AST/semantic analysis — rejected for v1 as over-engineered; a line/diff heuristic that
  errs toward flagging is sufficient and auditable.

## D2 — Where the guard verdict enters the pipeline

- **Decision**: Run the guard in `fix_gen` **before writing any file** (mirroring where the integrity
  path guard runs), comparing proposed contents to current on-disk contents. Carry a boolean
  `weakens_invariant` on `GeneratedFix`/`FixProposal`. **Admissibility** (`FixProposal::is_admissible`)
  gains a fourth conjunct: `grounded ∧ suite_green ∧ quality_green ∧ ¬weakens_invariant`. A flagged fix
  can still be *recorded* (so the operator sees the attempt + reason) but is **never admissible** and
  **never opens a PR** (FR-001/FR-002).
- **Rationale**: admissibility is already the single choke point the operator-accept path and the
  propose path both consult; adding the conjunct there closes both at once.
- **Alternatives**: block only at PR-open time — rejected; the verdict belongs in the one admissibility
  predicate so `heal_review::accept_proposal` enforces it too.

## D3 — Attribution: stable-path vs varied-input recurrence (US2)

- **Decision**: Track occurrences **per redacted input hash** within a defect signature, not just an
  aggregate count. A defect is **propose-eligible** only when *some single `input_hash` reached the
  recurrence threshold* (a deterministic, repeatable code path), NOT when N different inputs each failed
  once. `DefectRecord` already carries `input_hash`; `DefectLog` currently aggregates across inputs —
  extend it to keep a small `HashMap<input_hash, count>` per signature and expose
  `is_propose_eligible(threshold)`.
- **Rationale**: An adversarial/one-off pathology (e.g. "report confidence 0–100") manifests across
  *varied* content with the *same* override; a genuine deterministic code defect repeats on the *same*
  input. Requiring same-input recurrence trades recall for precision — the safe direction (FR-004).
- **Known limitation (documented, accepted for v1)**: a real defect triggered by genuinely varied
  inputs (e.g. a parser bug hit by many different strings) will be recorded but **not** auto-proposed.
  That is a deliberate false-negative; the operator still sees it (FR-007) and can act. A future
  refinement could add input-override pattern detection to safely re-admit varied-input defects.
- **Alternatives**: scan the redacted excerpt for schema-conflicting instructions ("use 0–100", "no
  JSON") — deferred (brittle, and the same-input gate already covers the motivating case). Keep the
  excerpt-scan as a documented future option.

## D4 — Model-drift correlation routes away from repair

- **Decision**: Reuse the existing `MetricsCollector::model_version_changes()` signal (feature 001,
  FR-017). In `heal_cycle`, before propose-eligibility, if a recurring defect's window overlaps a
  recorded model-version change, route it to the **drift response** (alert/record, no patch) — the same
  sink `partition_drift` already feeds. This makes FR-005 concrete.
- **Rationale**: a failure spike coinciding with a model swap is drift, not a code bug (Constitution II).
  The signal is already recorded; this wires it into eligibility.
- **Alternatives**: ignore drift in eligibility (rely only on the structural breadth check) — rejected;
  the model-version signal is the more direct attribution and is already available.

## D5 — Operator visibility of held-back / blocked defects (FR-007/FR-009)

- **Decision**: Every defect that is recorded-but-held-back (varied-input, drift, or guard-flagged)
  retains a human-readable **block reason** and stays queryable. For guard-flagged fixes, the reason
  names the invariant/diff hunk that would be weakened; for attribution, it states "input-induced
  (varied inputs)" or "model drift". Surface via the existing SI status / metrics path (no new MCP tool
  required for v1).
- **Rationale**: FR-007 forbids silent drops; a blocked safety decision the operator can't see is worse
  than no decision.

## D6 — Fail-safe defaults everywhere

- **Decision**: Both guards default to the *restrictive* outcome on any uncertainty: the invariant guard
  flags on unrecognized patterns; attribution holds back on ambiguous recurrence; a guard error (e.g.
  unreadable current file) blocks the proposal rather than letting it through.
- **Rationale**: this is a safety feature — a missed proposal costs nothing (operator can act manually),
  a wrongly-admitted one risks a merged regression.
