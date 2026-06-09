# Eval Harness — Plan

**Status:** Planning (not started). Living document. The genuinely-unresolved decisions are
the empirical items (§7) **and the feasibility gate (§8)** — the latter decides whether the
self-improvement loop is even worth rewiring (§1). Everything in §5–§6 is settled by source
verification or by the research note's methodology.

**Revision note:** issues raised in review folded in — the probable "MDE rejects everything"
finding and the open-ended-mode sensor ceiling (§1), the per-cycle cost model (§9), the
run-variance model and clustering/`expected_mode` caveats (§5), and the feasibility gate
that makes PR4/PR5 conditional (§8).

**Source:** Research note "Concrete Improvements for mcp-reasoning…" (repo root). Stats
per Evan Miller, "Adding Error Bars to Evals," arXiv:2411.00640. Judge-bias evidence per
Panickssery, Bowman & Feng, "LLM Evaluators Recognize and Favor Their Own Generations,"
NeurIPS 2024, arXiv:2404.13076.

---

## 1. Why (goal & scope)

Every tuning value in the project is currently an unvalidated guess: thinking-budget
tiers (4096/8192/16384), `auto`/`meta` mode selection, and the self-improvement loop's
rewards. The harness exists to turn "did this change help?" into a measured quantity with
a correct error bar.

**The sharper framing (see §2):** for the self-improvement loop the harness is not
validation infrastructure that the loop happens to want — it is **the measurement sensor
the loop has never had**, with a hardcoded multiplier standing in for it.

**v1 goal:** a Rust-native harness that runs a reasoning mode to a final answer, scores
it programmatically, and reports a quality estimate with **SE (CLT), clustered SE where
applicable, and the Minimum Detectable Effect (MDE)** for that sample — so a change is
accepted only when a *paired* measured delta clears the MDE.

**Probable finding, stated up front (not buried in §7).** An *affordable* live-API dataset
(tens to low-hundreds of items) has an MDE far larger than the effects SI changes plausibly
produce — a config nudge, a threshold tweak, a prompt tune are small effects. §7 itself
implies ~1,000 items to detect a 3-pt effect at 80% power. So the most likely result
is **"no SI-tunable change clears the MDE at acceptable cost,"** which de-scopes — possibly
kills — the self-improvement *loop*: the correct move would then be to **freeze the loop and
keep the harness as a measurement/reporting tool**, not rewire the sensor into it.
Determining that is the harness's first job; assuming the loop is fixable is the error this
plan must not make. **PR4/PR5 (rewiring) are therefore conditional on a feasibility gate
(§8), not foregone.**

**A second, structural ceiling (caps the whole SI effort, not just v1).** Programmatic
scoring is the *only* signal allowed into the loop (judge barred, §5/§6), and it scores only
*extractable* answers (math / multiple-choice). The open-ended modes — `divergent`,
`reflection`, `counterfactual`, `decision`, `tree` — produce structured reasoning with **no
extractable number**, so they have **no in-loop sensor at all**. SI tuning is permanently
confined to what the exact-matchable slice can measure (global knobs like timeouts/
thresholds, observed only on math items) and **cannot tune the open-ended modes on their own
outputs** — which is where `auto`/`meta` selection arguably matters most.

**Non-goals (v1):** leaderboards, a UI, RAG/retrieval eval, prompt optimization (DSPy).

## 2. Verified facts — the SI loop has no measurement channel

Checked against source (not assumed):

- The executor fabricates the "measurement" with a per-action-type multiplier of the
  *estimate*: `execute_config_adjust` → `measured_improvement = expected_improvement * 0.8`
  (executor.rs:225, comment `// Estimate`); `execute_prompt_tune` → `* 0.7` (:269);
  `execute_threshold_adjust` → `* 0.75` (:322).
- `learner.calculate_reward` (learner.rs:124): on `!success` returns `-0.5`; otherwise
  `ratio = actual / expected` with `actual = measured_improvement`, then rewards
  `(ratio - 0.5) * improvement_weight` (below expectation) or
  `(ratio.min(2.0) - 1.0) * improvement_weight + 0.5` (at/above).
- On the live path `actual = expected * k`, so **`expected` cancels**: `ratio = k` (k < 1),
  and the reward for a *successful* action is the constant `(k - 0.5) * improvement_weight`.
  With the default `improvement_weight = 0.7`: **config 0.21, prompt 0.14, threshold
  0.175**; failure is **−0.5**. (Holds for `expected > 0`; the `expected ≤ 0` branch returns
  raw `actual`, which is also `0` on the live path — so the reward is a per-type constant
  either way.)
- `executor.rs:623` asserts the fabricated value is "≈80%", so that test **pins the bug**
  rather than catching it.

**Consequence:** the loop senses exactly one bit — executed vs errored. It is structurally
blind to *helped vs hurt*. This is past the Pan et al. reward-hacking framing (a proxy that
diverges under optimization); it is the **absence of a sensor**, with a multiply in its
place.

**Two consequences for this plan:**

1. It makes the Rust-native decision *more* justified than "the loop happens to want it"
   (§3).
2. It adds scope the first draft hid: **`calculate_reward` must be rewritten, not merely
   fed a real input.** As written it rewards `actual/expected` matching — i.e. *prediction
   calibration*, not improvement. Feed a real delta into it unchanged and a true +0.05
   improvement against an expectation of 0.5 scores `ratio 0.1` → deeply negative, while
   the same +0.05 against an expectation of 0.05 scores `1.0` → positive. So the SI fix is
   **three parts in series**: the sensor (harness), the stats (PR1), and a reward function
   that rewards *absolute measured improvement clearing the MDE* (its own PR, §6).

## 3. Architecture decision — Rust-native

**Decided: build the harness in Rust.** Rationale specific to this codebase:

- **Modes are in-process calls** returning structured responses
  (`LinearMode::process` → `LinearResponse`; multi-step modes expose a conclusion field —
  tree `synthesis`, graph `conclusions`, mcts `recommendation`). A Solver wraps the **real
  mode path**; no MCP round-trip.
- **The SI fix needs real measured deltas re-measured synchronously after a change.** Given
  in-process `async fn` modes, an in-process Rust harness is the **decisively cheaper
  correct path** and avoids a language boundary in the SI hot loop. (Precise claim: what is
  *required* is that the loop read real measured deltas and re-measure after applying a
  change; in-process is strongly preferred for these concrete reasons, not the only
  conceivable option — a sidecar feeding deltas through SQLite is possible but worse.)
- **One language, one CI, one quality bar** (`forbid(unsafe)`, `deny(unwrap/expect)`); the
  stats are ~200 lines we must own in Rust regardless.

**Python door left open:** export results in Inspect's eval-log JSON shape, so the Inspect
viewer / prebuilt evals / DSPy can be layered on later without coupling the core loop to
Python. (Inspect's eval-log JSON is an unversioned, moving target — treat the export as a
best-effort snapshot pinned to an Inspect version, not a stable contract.)

## 4. Components (Tasks → Solver → Scorer → Stats)

| Component | Design | Maps to |
|---|---|---|
| **Task / Dataset** | `EvalTask { id, cluster_id, prompt, target, expected_mode, answer_kind, metadata }` from JSONL under `eval/data/`. `cluster_id` → clustered SE (computed only once a clustered set exists — Locked-9); `expected_mode` → a **weak prior** for `auto`/`meta` validation, not ground truth (Locked-10). | new `eval` module |
| **Solver** | `trait Solver { async fn solve(&self, task) -> SolverOutput }`. **Wraps the real mode path** (does not reimplement a "canonical sequence", or we'd evaluate a parallel construction). Client injected via the existing trait DI (real or wiremock). Records each mode's temperature and **resamples K runs per item** so SE reflects run noise (Locked-8). `MockSolver` for deterministic tests. | `src/modes/*` (unchanged) |
| **Scorer** | `trait Scorer { fn score(&self, task, output) -> Score }`. `ExactMatch` with lm-eval-style two-filter extraction (strict terminal format + flexible last-number), normalize before compare. **Tracks extraction-failure rate as a first-class metric.** `LlmJudge` (offline reporting only — see §5/Open-5). | new; reuses `AnthropicClient` |
| **Stats / Report** | Per-item, **optionally-grouped, optionally-paired** sample API (locked, §5): `mean_and_stderr` (CLT), `clustered_stderr(group_ids)`, `paired_difference(a,b)` (item-aligned), `minimum_detectable_effect`, `required_n`. Works identically over binary `{0,1}` or continuous scores. | new `eval::stats` |
| **Runner** | a `bin` (or `eval` subcommand) loading a dataset, running a solver, scoring, printing a report + JSON. Live-client, **opt-in, never in normal CI**. | `main.rs` arg-dispatch; `Config::from_env` |

**Testing posture:** datasets, scorers, stats, and solver orchestration are unit-tested
offline; solver↔API paths use **wiremock** (`ClientConfig::with_base_url(mock.uri())`).
Real eval runs hit the live API and are opt-in only.

## 5. Locked decisions

1. **Rust-native harness** (§3), with an Inspect-compatible JSON export door.
2. **New `eval` module** (`src/eval/` — confirmed not to exist yet).
3. **Stats are the first deliverable and the foundation — but PR1 does NOT fix the SI
   loop.** The loop fix additionally requires the reward rewrite and the real sensor (§2).
4. **Stats API shape:** per-item, optionally-grouped, optionally-paired from day one.
   Clustered SE needs the group id and paired comparison needs item-level alignment;
   retrofitting either into a population-summary API is the rewrite to avoid.
5. **Programmatic scoring is the anchor and the *only* signal allowed into the SI loop.**
   An LLM judge anywhere in a closed Claude-grades-Claude loop reintroduces a biased
   fabricated sensor (Open-5). LLM-judge is confined to offline, human-anchored,
   order-swapped, open-ended *reporting* that never feeds SI.
6. **CI gating via wiremock** for plumbing; real runs opt-in/offline-by-default/cached.
7. **Stats follow Miller's recipe**, validated against closed-form values and his worked
   numbers; arXiv:2411.00640 cited in code.
8. **Variance model.** The live API is nondeterministic, so item-sampling SE is *not* the
   whole variance. Control it by (a) **declaring** the temperature the harness runs modes at
   — and *not* lowering it just to shrink error bars (Miller #3) — and (b) **resampling K
   runs per item** (mean / majority), so reported SE reflects *item + run* variance. A
   "delta" counts only if it survives run noise. (Modes set their own temperature today,
   e.g. 0.7 in tree-summarize; the harness must record it, and override only via an explicit
   declared setting.)
9. **Clustering: keep the grouped API, defer the computation.** `group_ids` stay in the API
   from day one (cheap, future-proof), but **clustered SE is only computed once a genuinely
   clustered dataset exists**. On an unclustered seed set, clustered SE equals CLT SE;
   computing it earlier is theater.
10. **`expected_mode` is a weak label, not ground truth.** It encodes the designer's prior
    about which mode wins — the very question the harness measures. `auto`/`meta` validation
    compares the router's pick against the **empirically best-scoring mode per item**
    (measured), using `expected_mode` only as a sanity prior, never as the grader.

## 6. Resolved by method / constrained (calibration deferred, not the decision)

These are settled in *kind*; only a parameter or a post-run number remains.

- **Answer elicitation & extraction (was Open-1) — methodology fixed.** lm-eval pattern:
  a `strict-match` filter (constrain the model to a terminal format, e.g. `#### <n>` or
  "The answer is X", regex it) plus a `flexible-extract` fallback (last number). Normalize
  (`$`, commas, trailing period) before compare. **Non-obvious, locked:** count the
  extraction-failure / `[invalid]` rate as a first-class metric — a rising invalid rate
  depresses scores and corrupts deltas while masquerading as a quality regression, exactly
  the artifact that would poison the SI sensor. **Decided:** the starting default terminal
  format is **`#### <answer>`** (GSM8K's native delimiter) — a unique sentinel with the
  lowest prose-collision / `[invalid]` rate, and existing lm-eval `#### (-?[0-9.,]+)`
  regexes + normalization transfer directly. This is the spike's **null hypothesis, not a
  lock**: PR2 runs both `####` and "The answer is X" on 5–10 real items, reports the
  per-format `[invalid]` rate, and the measurement keeps or overturns it. Deferral blocks
  neither PR1 nor the parser scaffolding, so the cost of treating `####` as a confirmable
  default is zero. *(Spike is calibration, not discovery.)*

- **Solver sequence (was Open-2) — collapses.** The mode *is* the sequence; the Solver
  drives the real `process` path. The only real question is the boundary — where the client
  is injected and the final answer text is captured — which the existing trait DI already
  supports. *Frame as "wrap the existing path" (PR3).*

- **SI gate metric (was Open-4) — constrained by §2.** The metric is determined: the
  **paired per-item delta** (changed config minus baseline on the same held-out items),
  CLT/clustered SE, accept **iff the lower confidence bound clears a pre-registered MDE**.
  What remains is the threshold and the reward rewrite (§2), not *what* to compute.
  **Scope ceiling:** this gate exists only for programmatically-scoreable items; the
  open-ended modes have no in-loop sensor (§1), so SI can never gate on *their* outputs.

- **Judge model (was Open-5) — required, not "likely".** Panickssery et al. (NeurIPS 2024)
  show self-preference is **causal** (label-reversal flips it), correlates with
  self-recognition, is family-level, and leaks through shared lineage. The Claude-only SI
  loop is precisely that setting. *Resolution is mandatory:* SI gating uses programmatic
  scoring only; the judge never enters the loop.

## 7. Genuinely open (empirical — the only items that stay open)

- **Open-A — Seed task-set content.** Methodology settled (contamination-resistant sets
  e.g. GPQA-Diamond; GSM8K/MMLU only as a saturated floor or via perturbed GSM-Plus/
  VarBench; tag each item by expected-winning mode to validate `auto`/`meta`). The *content
  selection* is external work, correctly deferred; start with a small programmatically
  scoreable slice so PR2/PR3 don't depend on the judge.
- **Open-B — MDE reality / dataset adequacy.** Only computable post-run. Discipline =
  **pre-registration**: state `n`, the metric, and the hypothesized effect *before* the
  run; compute the MDE *after*; if the MDE exceeds the effect you care about, the correct
  output is "this set cannot test this," not a number. (Miller's example implies ~1,000
  items for a 3-pt effect at 80% power; a 50-item set catches only large effects — say so.)
  **And if the realized MDE exceeds every plausible SI effect (the likely case, §1), the
  verdict is to freeze the SI loop and ship the harness as a measurement tool — not to weaken
  the gate until something passes.**

## 8. Staged sequence (PRs)

Each PR ships green under `fmt` / `clippy -D warnings` / tests.

- **PR1 — `eval::stats`** (unblocked by everything). Per-item/grouped/paired API: SE/CLT,
  clustered SE, paired difference, MDE, required-n, `clears_mde`. Pure; tested against
  closed-form values + Miller's worked numbers. *Foundation; also what the reward rewrite
  consumes.*
- **PR2 — data model + scorers.** `EvalTask`/`Dataset` (JSONL), `Scorer` trait,
  `ExactMatch` with two-filter extraction + extraction-failure-rate metric, `EvalReport`,
  a tiny seed dataset. Calibration spike for the terminal format. Pure/wiremock-tested.
- **PR3 — real Solver + runner.** Per-mode adapters wrapping the real `process` path; the
  `eval` bin; debiased `LlmJudge` (reporting only). Wiremock plumbing tests; first real run
  documented (opt-in) and **publishes its MDE** (begins resolving Open-B).
- **Feasibility gate (a decision, not a PR — the pivot point).** After PR3's first real run,
  *before any rewiring*: (a) compute the realized **MDE** and the per-cycle **cost** (§9);
  (b) decide whether *any* SI-tunable change produces an effect the dataset can detect at an
  acceptable cost. **If no → STOP: freeze the SI loop, keep the harness as measurement/
  reporting; PR4/PR5 do not proceed.** Only a *pass* unlocks PR4/PR5.
- **PR4 — reward-function redesign.** Rewrite `calculate_reward` to reward *absolute
  measured improvement clearing the MDE*, not `actual/expected` calibration. Depends only on
  PR1; testable now with synthetic measured deltas. Delete/replace the fabrication-pinning
  test (executor.rs:623). *Explicit, separate PR by design — a reviewer reading the current
  `calculate_reward` would assume a real input suffices, and it does not.*
- **PR5 — real sensor + tripwire (conditional on the feasibility gate).** Replace the
  `expected * k` multipliers with a measured paired delta from a held-out harness slice; add
  a proxy-vs-measured divergence tripwire to the circuit breaker. Depends on PR3 + PR4 **and
  a passing feasibility gate**. If the gate fails, PR5 is *not* shipped and the SI loop is
  frozen instead.
- **PR6+ (optional)** — Inspect-log export; per-difficulty budget validation; DSPy.

**Critical path:** PR1 → PR2 → PR3 → **feasibility gate** → (PR4 ∥ after PR1) → PR5. PR1 can
start now; PR4 needs only PR1; PR5 is gated. The gate may end the line at PR3.

## 9. Risks & guardrails

- **Reinventing stats badly** → closed-form unit tests; cite Miller; one small module.
- **API cost / nondeterminism in CI** → wiremock for plumbing; real runs opt-in/cached.
- **Self-preference / judge bias** → programmatic anchor; judge for reporting only; never
  in the SI loop (Locked-5).
- **Reward hacking persists if the gate is weak** → gate on *paired measured* delta
  clearing the MDE; divergence tripwire (PR5); reward rewards absolute improvement (PR4).
- **Extraction artifacts masquerading as regressions** → extraction-failure rate is a
  first-class, gated metric (§6).
- **Dataset overfitting / contamination** → contamination-resistant items; pre-register;
  publish the distribution and the MDE; re-baseline on change.
- **Per-cycle live-API cost (must be a number before PR5)** → a paired re-measurement runs
  `n_heldout × modes × K` live calls **per candidate action, per cycle**. At n=50, K=3 that
  is ≥150 calls/action/cycle; on a 5-min SI cadence it is a real, recurring spend. **Budget
  gate:** if cost/cycle exceeds an agreed ceiling, the loop runs on a coarser cadence or not
  at all — a direct input to the feasibility gate (§8).
- **Run-to-run API variance** → item-sampling SE is not the whole variance; declare
  temperature and resample K per item (Locked-8). Never lower temperature to shrink bars.

## 10. v1 success criteria

- A reproducible run reports, for a mode on a tagged dataset: `n`, mean score, **SE (CLT)**,
  clustered SE where applicable, the **extraction-failure rate**, and the **MDE** for that
  `n`.
- Two configs can be compared with a **paired** question-level difference + SE, with an
  explicit "clears MDE?" verdict.
- `calculate_reward` is rewritten to reward absolute measured improvement, and a unit test
  proves a change whose measured delta does **not** clear the MDE is **not** rewarded
  (replacing the test that currently pins the fabrication). *(Only if the feasibility gate
  passes; otherwise this is explicitly not shipped.)*
- The harness produces a **feasibility verdict**: whether any SI-tunable change clears the
  MDE at acceptable per-cycle cost. If not, the documented, accepted outcome is **freeze the
  SI loop** and keep the harness as a measurement tool — the plan's own logic, followed.
