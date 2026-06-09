# Quickstart: Self-Healing of Parse/Schema Failures

## What it does

When a reasoning tool/mode emits unparseable or schema-violating output, the server counts it,
records a defect, and — once a defect recurs — opens a **pull request** with a fix and a reproducing
test for the operator to review. It never merges into the running server on its own.

## Operator flow

1. Run the server / SI loop as usual (operator-run; not 24/7).
2. The Monitor reports parse-failure and schema-violation counts per tool/mode.
3. When a defect recurs (≥ N times), the loop diagnoses, localizes, and — if it is a code defect,
   not model drift — opens a PR (`gh`) containing the fix + reproducing test.
4. Review the PR. Merge only if you approve; CI enforces the gate (reproducing test fails on base,
   passes on head; full suite green; fmt/clippy/rustc green).
5. On merge, the defect→fix→test is recorded; a future recurrence of the same class is recognized,
   not re-diagnosed.

## Verifying it end-to-end (integration test)

1. Induce a tool/mode that returns non-conforming output.
2. Assert: a parse/schema counter increments and a DefectRecord is created (US1).
3. Repeat to cross the recurrence threshold.
4. Assert: a FixProposal is produced with `grounded = true`, a PR is opened, and **nothing was
   merged automatically** (US2).
5. Assert: the reproducing test fails on the base commit and passes on the proposed fix (D4).
6. Approve; assert merge is blocked unless the reproducing test passes AND the full suite is green;
   assert the KnowledgeEntry is stored (US3).

## Guardrails (must hold)

- No fix is applied to the live server without operator approval.
- No improvement is recorded without the passing test gate (no fabricated metrics).
- The repair action cannot modify tests, `metrics/`, `eval/`, `sensor.rs`, or `circuit_breaker.rs`.
- Model/provider drift is routed to alert/pin/rollback, never to a code patch.
