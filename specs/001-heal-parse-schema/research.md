# Phase 0 Research: Self-Healing of Parse/Schema Failures

No `NEEDS CLARIFICATION` remained in Technical Context (stack and modules are known). This document
records the load-bearing design decisions.

## D1 — Detection point

- **Decision**: Instrument the existing parse/validation boundary — the mode `extract_json`/parser
  path in `modes/` and the JsonSchema validation for `server/params.rs`/`requests.rs`. Increment a
  per-(tool, mode, class) counter and emit a Defect Record on failure.
- **Rationale**: this is exactly where malformed-output and schema-violation failures already
  surface (the JSON-envelope failures that silently dropped eval items); instrumenting it is a
  counter increment with negligible overhead and no behavior change.
- **Alternatives**: log scraping (fragile, post-hoc) — rejected; wrapping every handler (broader
  blast radius) — rejected in favor of the single parse/validate seam.

## D2 — Recurrence vs transient

- **Decision**: A defect is "recurring" when the same (component, failure class) signature occurs ≥ N
  times within a **recurrence window** (default N=3, configurable, bounded by the allowlist). The
  window scope defaults to the **current operator-run session** (the server runs when started, not
  24/7), and MAY be configured as a rolling time bound; occurrences older than the window are not
  counted. One-offs are recorded but not proposed.
- **Rationale**: mirrors the project's existing anti-pattern detection (sustained low-success
  transitions) and the criterion-proposer pattern (root cause recurs 3+ times) in the SI design.
- **Alternatives**: act on first occurrence (noisy, floods PRs) — rejected.

## D3 — Code-defect vs model/provider drift

- **Decision**: Classify as **drift** (not a code patch) when parse failures rise across many
  unrelated tools/modes simultaneously or coincide with a model-version-change event; classify as a
  **code defect** when failures localize to one component's parser/contract. Drift routes to the
  drift response (alert + pin/rollback), not the repair path.
- **Signal source (FR-017)**: the classifier consumes two recorded signals — the pinned model
  identifier per call and an emitted model-version-change event when it changes. Without these,
  drift and code-defect parse failures are indistinguishable, so the signal is a prerequisite of
  correct classification.
- **Rationale**: Constitution II — the server can't fix the model; patching code for a model
  regression would be wrong and could mask the real cause.
- **Alternatives**: treat all parse failures as code defects — rejected (would generate spurious
  patches during a provider model update).

## D4 — Execution-grounded reproducing test

- **Decision**: Generate the reproducing test, then run it against the **unpatched** tree; require it
  to **fail**. Only then generate/apply the fix on a branch and require the test to pass plus the
  full suite green. A generated test that does not fail first is rejected (no proposal opened).
- **Rationale**: APR patch-overfitting is the central risk; naive LLM test generation has very low
  oracle precision, so the test must be execution-grounded to mean anything (Constitution III).
- **Alternatives**: accept on the existing suite alone — rejected (lets a masking "fix" through).

## D5 — Proposal delivery (no auto-merge)

- **Decision**: Create a branch, commit fix+test, open a PR via the `gh` CLI for operator review.
  Never merge or hot-patch the running server. Reuse the repo's existing approve/reject SI override
  path for the operator decision.
- **Rationale**: Constitution IV; `require_approval` already defaults true. PR-for-review is the
  production-safe pattern (WarpFix/Gitar).
- **Alternatives**: auto-apply to a live process — rejected (highest-risk form).

## D6 — Integrity of the success signal

- **Decision**: The allowlist forbids the repair action from editing test files, `metrics/`, the
  `eval/` scorer, or `self_improvement/sensor.rs`/`circuit_breaker.rs` — the acceptance/measurement
  surface. The patcher may only touch the diagnosed production component.
- **Rationale**: Constitution I/IV — a fix must not be able to alter its own success criterion
  (reward hacking is the default failure).
- **Alternatives**: trust the loop not to edit its oracle — rejected.

## D7 — The missing Plan step

- **Decision**: Add `self_improvement/plan.rs` between Analyze and Execute that ranks candidate
  defects by frequency × severity × fix-confidence and emits at most K proposals per cycle.
- **Rationale**: the existing loop is Monitor→Analyze→Execute→Learn with no Plan/prioritization;
  MAPE-K requires it, and it bounds API cost.
- **Alternatives**: propose for every recurring defect immediately — rejected (cost + flooding).
- **Severity / fix-confidence (FR-014/FR-015)**: both are bounded [0,1] scores computed from
  *observable* quantities only — severity from failure-class weight × recurrence × blast radius;
  fix-confidence from grounding + first-attempt validation pass + Knowledge-Entry match. Neither is a
  model self-rating (Constitution I/III). This makes the Plan ranking deterministic and testable.

## D8 — Redaction of captured input (FR-016)

- **Decision**: Defect records store a content **hash** (for recurrence matching) plus a bounded,
  **redacted** excerpt produced by a deterministic scrubber that strips credential-shaped tokens and
  caps length. Raw payloads and secrets are never persisted in cleartext.
- **Rationale**: defect capture must not become a secrets-leak surface; the project already forbids
  logging `ANTHROPIC_API_KEY`/`VOYAGE_API_KEY` or request payloads. Recurrence matching only needs a
  stable signature, not the raw input.
- **Alternatives**: store full input for richer diagnosis — rejected (leak risk outweighs the
  marginal diagnostic value; the redacted excerpt + hash suffice).
