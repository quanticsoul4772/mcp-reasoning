# Phase 1 Contracts: Internal Interfaces

This feature is internal to the Rust server; the "contracts" are the module interfaces and the PR
artifact, not network endpoints.

## Detection counter (metrics)

```
record_parse_failure(component: &str)         // increment per (tool, mode)
record_schema_violation(component: &str)
```

Emitted at the parse/validate seam (D1). Surfaced in `MonitorResult` alongside existing
success/latency.

## Failure classification (analyzer)

```
classify(defect_signal) -> FailureClass        // Parse | Schema | Drift   (D3)
localize(defect_signal)  -> Component           // tool/mode + source span hint
```

## Plan step (new)

```
plan(recurring: Vec<DefectRecord>) -> Vec<DefectRecord>   // ranked by freq × severity × confidence,
                                                          // capped at K per cycle (D7)
```

## Repair action (executor → repair/)

```
enum ActionType { ConfigAdjust, PromptTune, ThresholdAdjust, LogObservation, ProposePR }  // + ProposePR

propose_pr(defect: DefectRecord) -> FixProposal
  // 1. synth reproducing test; run on unpatched tree; REQUIRE fail  (grounded=true) else abort
  // 2. generate fix on a branch
  // 3. run reproducing test (must pass) + full suite + fmt/clippy/rustc (must pass)
  // 4. gh pr create; set pr_url; review_status = Proposed
  // never merges; never edits tests/metrics/eval/sensor/circuit_breaker  (D6)
```

## Acceptance gate (operator + CI)

```
admissible(p: FixProposal) -> bool   // grounded ∧ suite_green ∧ quality_green
// merge happens only on operator Approve AND admissible(p); loop cannot self-approve
```

## PR artifact contract

A proposal PR MUST contain: the fix diff (touching only the diagnosed component), a reproducing test
that fails on the base commit and passes on the PR head, and a description linking the DefectRecord
and failure signature.
