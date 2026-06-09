# Feature Specification: Self-Healing of Parse/Schema Failures (Operator-Reviewed)

**Feature Branch**: `001-heal-parse-schema`

**Created**: 2026-06-09

**Status**: Draft

**Input**: User description: "heal parse/schema failures, propose-PR, reproducing-test gate"

## User Scenarios & Testing *(mandatory)*

The actor is the **operator/maintainer** of the server. The feature lets the server notice when its
own tools produce malformed or schema-violating output, and turn each recurring defect into a
reviewable fix that cannot be accepted unless a test proves it.

### User Story 1 - Detect and record malformed/schema-violating output (Priority: P1)

When a reasoning tool or mode returns output that cannot be parsed, or that violates its declared
output contract, the server detects it, counts it per tool/mode, and records a defect entry instead
of silently dropping or mis-attributing it.

**Why this priority**: malformed/schema output is the single largest failure class for this kind of
server; without reliable detection there is no signal to act on, and failures get mis-read as
"the model was wrong." This is the foundation the rest depends on.

**Independent Test**: induce a tool/mode that returns non-conforming output; confirm the failure is
counted against that tool/mode and a defect record is created with the offending input and the
failure class. Delivers value on its own (operators can see real defect rates).

**Acceptance Scenarios**:

1. **Given** a tool that returns unparseable output, **When** the tool runs, **Then** a parse-failure
   is counted for that tool and a defect record (input + failure class) is stored.
2. **Given** a tool that returns output violating its declared schema, **When** the tool runs,
   **Then** a schema-violation is counted and recorded, distinct from a generic error.
3. **Given** a transient one-off failure, **When** it occurs once, **Then** it is recorded but not
   yet treated as a recurring defect.

### User Story 2 - Propose a reviewed fix with a reproducing test (Priority: P2)

When a parse/schema defect recurs, the server diagnoses the responsible component, generates a
candidate fix **and** a test that reproduces the defect, and opens a pull request for operator
review. It never merges a change into the running server on its own.

**Why this priority**: this is the "self-improvement" the operator wants — but delivered as a
reviewable proposal, not an unsupervised edit to a live service.

**Independent Test**: given a recurring defect, confirm a PR is produced that contains (a) a test
which fails on the current code and passes with the proposed fix, and (b) the fix; and that nothing
was merged automatically.

**Acceptance Scenarios**:

1. **Given** a recurring parse/schema defect, **When** the loop runs, **Then** a PR is opened
   containing a candidate fix and a reproducing test.
2. **Given** a proposed fix, **When** the reproducing test is run against the unpatched code,
   **Then** it fails; **When** run against the patched code, **Then** it passes.
3. **Given** any proposed fix, **When** it is generated, **Then** it is not merged or applied to the
   live server without operator approval.

### User Story 3 - Accept only a proven, non-regressing fix (Priority: P3)

The operator reviews a proposed PR; an accepted fix is admitted only if its reproducing test passes,
the full existing test suite stays green, and quality gates pass. A defect, its accepted fix, and
its test are recorded so the same failure class is recognized next time.

**Why this priority**: closes the loop safely and makes it cumulative — the server stops
re-diagnosing a defect it has already fixed.

**Independent Test**: approve a PR and confirm the gate blocks merge unless the reproducing test
passes AND the full suite is green; confirm the defect→fix→test record is stored.

**Acceptance Scenarios**:

1. **Given** an approved fix, **When** its reproducing test fails or any existing test breaks,
   **Then** the fix is rejected and not merged.
2. **Given** an accepted fix, **When** the same failure class recurs later, **Then** the recorded
   knowledge is used and the defect is not re-diagnosed from scratch.

### Edge Cases

- A parse failure caused by **model/provider drift** (not a code defect) is classified as drift and
  surfaced for the drift response, not turned into a code patch.
- A fix that passes the existing suite but does not actually address the defect (overfitting) is
  blocked because the reproducing test (which must fail first) would not pass.
- A **non-reproducible/transient** failure does not produce a patch proposal.
- A generated reproducing test that does not fail on the current code is rejected (not
  execution-grounded) and the proposal is not opened.
- Detection volume spikes (many failures at once) are rate-limited so the loop does not open a flood
  of PRs.

## Requirements *(mandatory)*

### Functional Requirements

*Terminology (canonical)*: **parse failure** denotes any malformed/unparseable tool output (the
phrases "malformed output" and "unparseable output" are synonyms for it); **schema violation**
denotes output that violates its declared contract. Together these are the "parse/schema" failure
classes referenced throughout this spec.

- **FR-001**: The system MUST detect output that cannot be parsed (a parse failure), and output that
  violates its declared contract (a schema violation), and count each per originating tool/mode.
- **FR-002**: The system MUST record each detected failure as a defect entry capturing the
  triggering input and the failure class, without silently dropping it.
- **FR-003**: The system MUST distinguish a recurring defect from a one-off/transient failure before
  proposing a fix. A defect is "recurring" when the same `(component, failure_class)` signature
  occurs at least N times (default N=3, bounded/configurable) within a **recurrence window**. The
  window defaults to the current operator-run session and MAY be configured as a rolling time bound;
  occurrences older than the window do not count toward recurrence.
- **FR-004**: The system MUST localize the component responsible for a recurring defect.
- **FR-005**: The system MUST generate, for a recurring defect, a candidate fix and a test that
  reproduces the defect.
- **FR-006**: The reproducing test MUST be execution-grounded — it MUST fail on the current
  (unpatched) code and pass on the patched code; a test that does not fail first MUST be rejected.
- **FR-007**: The system MUST present fixes as operator-reviewable proposals (pull requests) and MUST
  NOT merge or apply them to the running server without operator approval.
- **FR-008**: A fix MUST be admissible only if its reproducing test passes AND the full existing test
  suite passes AND quality gates pass. (Boundary: FR-007 governs *how* a fix is delivered — as a
  review PR, never auto-merged; FR-008 governs *whether* a fix is even eligible to be admitted. Both
  must hold for a change to merge.)
- **FR-009**: The system MUST NOT report an improvement as achieved unless it is backed by the test
  gate; no estimated or fabricated success value is permitted (Constitution I).
- **FR-010**: The detection/acceptance signal MUST be independent of the component being changed, so
  a fix cannot alter its own success criterion.
- **FR-011**: The system MUST record each defect, its accepted fix, and its reproducing test, and
  reuse that record when the same failure class recurs.
- **FR-012**: The system MUST classify a failure attributable to model/provider drift separately from
  a code defect and not propose a code patch for it.
- **FR-013**: The system MUST bound how many proposals it opens per cycle to avoid flooding.
- **FR-014** (defines "severity"): The system MUST compute a defect's **severity** as a bounded
  score in [0,1] from three observable inputs: the failure-class weight (schema-violation ≥
  parse-failure ≥ drift-routed), the recurrence count, and the blast radius (number of distinct
  tools/modes exhibiting the same failure signature). Higher recurrence and wider blast radius
  raise severity.
- **FR-015** (defines "fix-confidence"): The system MUST compute a proposal's **fix-confidence** as
  a bounded score in [0,1] from observable validation outcomes only: whether the reproducing test is
  execution-grounded (failed on base), whether the patched code passed the reproducing test + full
  suite + quality gates on the first attempt, and whether the failure signature matches an existing
  Knowledge Entry. Confidence MUST NOT be a model self-rating.
- **FR-016** (redaction): The system MUST persist only a bounded, redacted excerpt of the triggering
  input plus a content hash for recurrence matching. Credentials, API keys, and full request
  payloads MUST NOT be stored in cleartext in any defect record, log, or proposal artifact.
- **FR-017** (drift signal): The system MUST record the pinned model identifier in effect for each
  call and emit a model-version-change event when that identifier changes. Drift classification
  (FR-012) MUST use these signals: failures that rise broadly across components coinciding with a
  model-version change are classified as drift, not a code defect.

### Key Entities *(include if feature involves data)*

- **Defect Record**: a detected failure — originating tool/mode, failure class (parse vs schema vs
  drift), triggering input, first-seen and recurrence count.
- **Fix Proposal**: a candidate change for a recurring defect — the proposed change, the reproducing
  test, validation status, review status (proposed/approved/rejected).
- **Knowledge Entry**: an accepted defect→fix→test mapping, keyed by failure class, reused on
  recurrence.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At least 99% of malformed/schema-violating tool outputs are detected and recorded (no
  silent drops), measured against a seeded set of induced failures.
- **SC-002**: 100% of opened fix proposals include a reproducing test that fails on the unpatched
  code and passes on the patched code.
- **SC-003**: 0% of fixes reach the running server without operator approval.
- **SC-004**: 0 improvement claims are recorded without a passing test gate backing them.
- **SC-005**: For a recurring defect, a ready-for-review proposal is produced within one improvement
  cycle of the recurrence threshold being met.
- **SC-006**: After a fix is accepted, a later recurrence of the same failure class reuses the stored
  record (0 re-diagnoses of an already-fixed class).

## Assumptions

- Scope for v1 is the **parse/malformed-output and schema-violation** failure classes only; other
  classes (timeouts, auth/quota, upstream-API) are out of scope for this feature.
- The operator runs the loop deliberately (the server is not assumed to run 24/7); API cost is a
  real constraint, so detection and proposal generation are bounded.
- Code fixes are delivered as **proposals for operator review**, not auto-merged to a live service;
  only the operator admits a change.
- The existing automated test suite and quality gates are the acceptance oracle; no human-in-the-loop
  judgment substitutes for the test gate.
- The server already has a monitor/analyze/execute/learn loop with action allowlisting and a circuit
  breaker; this feature extends those rather than replacing them.
- A "Plan/prioritization" step (rank by frequency × severity × confidence) is introduced between
  diagnosis and proposal, since the current loop lacks one.
