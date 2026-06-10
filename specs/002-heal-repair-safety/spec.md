# Feature Specification: Self-Heal Repair Safety — Attribution & Validation-Invariant Guard

**Feature Branch**: `002-heal-repair-safety`

**Created**: 2026-06-09

**Status**: Draft

**Input**: User description: "self-heal repair safety: defect attribution + validation-invariant guard. Two gaps surfaced while testing spec 001 against the running server — (1) recurring defects are eligible to propose even when the cause is input-induced/one-off, and (2) the integrity guard protects the measurement surface but not a mode's own validation invariants, so a fix could loosen a correct check to pass a grounded test."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A proposed fix can never weaken a correct validation check (Priority: P1)

As the operator of the self-healing server, when the repair loop proposes a fix for a recurring
parse/schema defect, that proposal must never weaken or remove a validation, range, or contract check —
the kind of check that *correctly* rejects malformed output. If a candidate fix would loosen such an
invariant, the loop must refuse to turn it into an admissible proposal (and surface why), rather than
open a PR that makes a currently-failing case pass by lowering the bar.

**Why this priority**: This is the last line of defense. The reproducing-test gate proves a behavior
*change*, not that the current behavior is *wrong* — "currently failing = bug" is false when the server
correctly rejected bad output. Without this guard, the loop can produce a green, admissible PR that
regresses a correct check, and a busy operator could merge it. It is independently valuable even if
attribution (US2) is never built.

**Independent Test**: Drive a recurring schema defect whose only "fix" is to loosen a correct range
check; assert the loop refuses to produce an admissible proposal that edits a validation invariant, and
that the block reason names the invariant.

**Acceptance Scenarios**:

1. **Given** a recurring confidence-out-of-range schema defect, **When** the loop generates a candidate fix that widens the accepted range, **Then** the proposal is blocked/flagged and no PR is opened that loosens the check.
2. **Given** a candidate fix that touches a protected validation invariant, **When** admissibility is evaluated, **Then** the proposal is not admissible regardless of whether its reproducing test passes.
3. **Given** a candidate fix that corrects a genuine defect WITHOUT weakening any validation, **When** evaluated, **Then** it proceeds normally (no false-positive block).

---

### User Story 2 - Only genuine code defects become eligible to propose (Priority: P2)

As the operator, a recurring parse/schema failure should become eligible for an automated fix proposal
only when the evidence points to a server *code* defect — not to an adversarial or one-off input that
merely made the model misbehave. Failures driven by varied/anomalous inputs, or coinciding with a
model-version change, must be recorded and visible but held back from the propose path.

**Why this priority**: Prevents the loop from spending effort and risk proposing fixes for non-defects,
and reduces how often the US1 guard is even exercised. Recurrence today is keyed only on
`(component, failure_class)`, so three different adversarial inputs co-promote to `Recurring`.

**Independent Test**: Induce the same failure class via three DIFFERENT inputs and assert it does NOT
become propose-eligible; induce it via the same stable path repeatedly and assert it DOES; induce a
failure coinciding with a model-version change and assert it routes to drift, not propose.

**Acceptance Scenarios**:

1. **Given** three schema failures of the same class from three different inputs, **When** eligibility is evaluated, **Then** the defect is recorded but NOT eligible to propose.
2. **Given** repeated schema failures from the same stable input/code path, **When** the threshold is met, **Then** the defect becomes eligible to propose.
3. **Given** a spike of failures coinciding with a recorded model-version change, **When** classified, **Then** it routes to the drift response, not the repair path.

---

### Edge Cases

- A candidate fix that both corrects a real defect AND touches a validation line — must be flagged for human judgement, not silently admitted nor silently blocked.
- An ambiguous attribution signal (some same-input, some varied) — default to NOT eligible (fail safe).
- A validation invariant expressed in a form the guard does not recognize — err toward flagging, never silent approval.
- A defect held back from propose must not disappear — it remains queryable by the operator.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST prevent any proposed fix from weakening or removing a validation/range/contract check that rejects malformed or non-conforming output; such a candidate MUST NOT become an admissible proposal.
- **FR-002**: The admissibility decision MUST treat "weakens a protected validation invariant" as disqualifying, independent of whether the reproducing test passes or the suite is green.
- **FR-003**: The system MUST distinguish a genuine code defect from an input-induced/one-off pathology before a recurring defect becomes eligible to propose a fix.
- **FR-004**: Propose-eligibility MUST require a stronger signal than `(component, failure_class)` alone — at minimum, evidence of a stable triggering path (e.g. the same redacted input recurring) rather than varied inputs all failing the same way.
- **FR-005**: A failure spike that correlates with a recorded model-version change MUST route to the drift response (alert/record), not the propose path.
- **FR-006**: When attribution is ambiguous, the system MUST default to NOT eligible (fail safe — no proposal).
- **FR-007**: A defect that is recorded but held back from propose MUST remain visible to the operator (no silent drop).
- **FR-008**: A fix that modifies no line adjacent to a validation/range/contract check MUST proceed normally — the guard MUST NOT block it. (A fix that edits near a validation check may be conservatively flagged for human review per D1/D6; that is expected, not a false block.)
- **FR-009**: A blocked or held-back proposal MUST carry a human-readable reason (which invariant it would weaken, or why it was deemed input-induced) so the operator can review.

### Key Entities

- **Defect eligibility signal**: the evidence that promotes a recorded defect to "eligible to propose" — separates stable-path recurrence from varied-input recurrence and from model drift.
- **Protected validation invariant**: a class of code (range checks, schema/contract validation, input rejection) a fix may not weaken — conceptually an extension of the existing integrity guard's protected set, but covering correctness invariants rather than only the measurement surface.
- **Block reason**: the human-readable explanation attached to a held-back or rejected proposal.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of candidate fixes that would weaken a validation/range/contract check are blocked from becoming admissible proposals — zero such PRs opened.
- **SC-002**: A schema/parse failure induced via 3 distinct inputs does NOT become propose-eligible; the same failure via a stable repeated path DOES.
- **SC-003**: 100% of failure spikes coinciding with a recorded model-version change route to drift, not repair.
- **SC-004**: Every blocked or held-back defect remains visible to the operator with a stated reason — no silent drops.
- **SC-005**: A genuine-defect fix that modifies no line adjacent to a validation/range/contract check is never blocked (no spurious blocks on the regression suite). A fix that *does* edit near a validation check MAY be flagged for human review by design (D1/D6); such conservative flags are expected, not counted as false blocks.

## Assumptions

- This refines the existing self-heal loop (feature `001-heal-parse-schema`); it does not introduce a new loop or change the operator-review / never-merge model.
- A "validation invariant" is recognizable by a static signal (a check that returns an error on out-of-range / missing / contract-violating values). A perfect semantic classifier is out of scope — the guard errs toward flagging.
- The model-version-change signal already recorded by feature 001 is available to the attribution step.
- The propose loop remains OFF by default; this feature changes what the loop does WHEN enabled, not whether it runs.
- Operators review every PR; this feature reduces the chance a harmful PR is generated and makes a generated one unmistakable, but the human gate remains the final backstop.
