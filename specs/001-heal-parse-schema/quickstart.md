# Quickstart: Self-Healing of Parse/Schema Failures

## What it does

When a reasoning tool/mode emits unparseable or schema-violating output, the server counts it,
records a defect, and — once a defect recurs — opens a **pull request** with a fix and a reproducing
test for the operator to review. It never merges into the running server on its own.

## Enabling the propose loop (default OFF)

Detection (counting + recording defects) is always on. The **propose-PR loop is OFF by default** —
running the server never opens PRs on its own. Turn it on explicitly with two env vars:

```bash
# Required to enable. Default: false.
export SELF_HEAL_PROPOSE_ENABLED=true
# Required: the repo working dir the pipeline runs cargo/git/gh in.
# If unset/empty the loop does NOT start even when enabled.
export SELF_HEAL_WORKSPACE=/abs/path/to/this/repo
# Optional: max proposals opened per cycle (flood guard). Default 1, clamped to 5.
export SELF_HEAL_MAX_PROPOSALS=1
```

`gh` must be installed and authenticated for PRs to open. The loop ticks on the SI
`SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS` interval and shares the SI shutdown signal.

## Operator flow

1. Run the server. With the propose loop enabled (above), a background task ticks each cycle.
2. The Monitor reports parse-failure and schema-violation counts per tool/mode; recurring defects
   accumulate in the in-process `DefectLog`.
3. Each tick: drift defects (a class broad across ≥3 components, or model drift) are **alerted and
   recorded only** — never patched. The remaining localized code defects are ranked, capped, and for
   each one the loop checks for a prior accepted fix (reuse → skip re-diagnosis), else localizes,
   synthesizes a grounded reproducing test, generates a fix on a branch, validates it, and opens a
   PR (`gh`) — **never merging**. The resulting `FixProposal` is persisted.
4. Review the PR. Merge only if you approve. The admissibility gate (reproducing test fails on base /
   passes on head; full `cargo test` green; fmt/clippy green) must hold; the loop cannot self-approve.
5. On acceptance (`heal_review::accept_proposal`, admissible-only), the defect→fix→test is recorded as
   a `KnowledgeEntry`; a future recurrence of the same class is recognized and not re-diagnosed.

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
- The repair action cannot modify tests, `metrics/`, `eval/`, `sensor.rs`, `circuit_breaker.rs`, or
  `allowlist.rs` (the integrity guard rejects the fix *before* writing anything).
- Model/provider drift is routed to alert/record, never to a code patch.
- The propose loop is OFF unless `SELF_HEAL_PROPOSE_ENABLED=true` AND `SELF_HEAL_WORKSPACE` is set.
