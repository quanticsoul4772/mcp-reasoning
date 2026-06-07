# Eval Dataset — Plan

**Status:** Planning. Grounded in a verified multi-source research pass
(2026-06-07): 26 sources fetched, 25 claims adversarially verified (24 confirmed,
1 refuted). Sources cited inline.

## 1. Why

The harness (PR1–PR5) is built and correct, but it has nothing to measure. The
6-item arithmetic seed set scores **100%** under linear, so every per-item delta
is 0, the standard error is 0, and the Minimum Detectable Effect (MDE) is a
degenerate 0. A saturated benchmark has zero variance and **cannot tell whether a
config change helped or hurt** — which is precisely what the self-improvement
sensor needs to do.

This plan produces a dataset that is **non-saturated, contamination-resistant,
adequately powered, programmatically scoreable, and mode-taggable** — the five
constraints — so the sensor (`self_improvement::sensor::measure_delta`) finally
has signal to measure.

## 2. The sizing reality (settles the n question)

- Detecting a **3-percentage-point** accuracy gap at 80% power / 5% significance
  needs **~969 independent items**; Miller recommends **≥1,000**. MDE scales as
  `1/√n` — so 6 items has no resolving power and even a few hundred only catches
  large effects. *(Miller, "Adding Error Bars to Evals," arXiv:2411.00640.)*
- **Paired per-item inference (McNemar) is ~2.15× more sample-efficient** than
  unpaired — confirmed across sources. Our `eval::stats::paired_difference`
  already does this, so a baseline-vs-changed comparison on the *same* items
  needs roughly half the n of two independent runs. *(arXiv:2605.30315.)*
- **Run the power analysis first.** Use `eval::stats::required_n(sd, effect, …)`
  to pick the target n *before* the run, and `minimum_detectable_effect` *after*
  to report what the run could actually resolve. If the realized MDE exceeds the
  effect we care about, the run says "this set can't test this" — not a number.

**Target:** ~1,000 items for a production sensor; a **200-item pilot** first to
calibrate difficulty and confirm the pipeline (a 200-item paired set resolves
~7-point effects — enough to validate plumbing, not to gate small tweaks).

## 3. The three options and their tradeoffs

| Option | Saturation | Contamination | Scoreable (no judge) | Difficulty knob | Effort | Verdict |
|---|---|---|---|---|---|---|
| **Reuse an existing slice** (GSM8K/MMLU as-is) | saturated for strong models | **pervasively contaminated** (arXiv:2404.00699) | yes | none | low | **Rejected** — contaminated + saturated |
| **Generate perturbed variants** (GSM-Symbolic / GSM-Plus style) | **breaks saturation** | **resistant** (novel instances) | **yes** (numeric exact-match) | **yes** (clause add/remove) | medium | **Primary** |
| **Build fresh items** | controllable | most resistant | yes if designed so | manual | high | Later / niche |
| **Live benchmark slice** (LiveBench / LiveCodeBench) | non-saturated | **resistant** (dated release) | **yes** | indirect | low (download) | **Cross-check slice** |

**Decision:** **perturbation-generated set as the primary** dataset, with a
**LiveBench/LiveCodeBench slice as an independent cross-check**. Rationale: the
research confirms perturbation simultaneously satisfies all five constraints,
where reuse fails contamination and saturation, and build-fresh is high-effort.

## 4. How perturbation works (the mechanism we'll implement)

Programmatically transform seed word-problems so the *structure* is preserved but
the *surface* and *numbers* change, producing novel instances no model has seen:

- **Numeric/name variation** (GSM-Symbolic templates): swap quantities and
  entities; the solution method is unchanged but the literal string is new. This
  alone reverses GSM8K saturation (accuracy drops materially) and keeps answers
  exact-matchable. *(arXiv:2410.05229.)*
- **Clause add/remove** (GSM-Plus): add reasoning steps to raise difficulty,
  remove to lower it — **this is the difficulty knob** that moves a model into the
  40–70% band. *(arXiv:2402.19255.)*
- Evidence the knob works: across perturbations accuracy moved GPT-4 93→86 and
  Gemma2 84→42 — i.e. the same template can be tuned off the ceiling.

**Calibrate empirically, do not guess:** generate a small batch at several clause
counts, run our target model via the `eval` binary, and **keep the perturbation
level whose measured accuracy lands in 40–70%.** (This is the §2 "run it first"
discipline applied to difficulty.)

> **Refuted, so we will NOT rely on it:** the claim that a single irrelevant
> clause causes "up to 65% drops across all SOTA models" was killed 0-3 in
> verification. Use perturbation as a *measured, tunable* knob — not as a magic
> one-clause cliff.

## 5. Contamination resistance

- Perturbed instances are novel, which is the primary defense. Add a **temporal**
  guard where possible (favor post-training-cutoff source material).
- **Detect, don't assume:** screen candidate items with **n-gram overlap** and a
  **PaCoST-style** check against known benchmarks; treat post-cutoff performance
  decay as a weak signal only, not proof. *(arXiv:2406.18326, 2509.00072.)*

## 6. Fit to the harness we already built (no new schema)

The dataset drops straight into the existing pipeline — nothing new to design:

- **Format:** JSONL of `EvalTask { id, cluster_id, prompt, target, expected_mode,
  answer_kind, metadata }` under `eval/data/` (PR2).
- **Scoring:** `answer_kind: "numeric"`, `ExactMatch` two-filter extraction with
  the `#### <answer>` terminal format (PR2) — no LLM judge, so no self-preference
  bias enters the SI loop.
- **Clustering:** group perturbed variants of the same seed under one
  `cluster_id` → `clustered_stderr` (variants of one seed are correlated; this is
  the case the clustered-SE path was built for).
- **Mode tagging:** set `expected_mode` per item as a **weak prior** (Locked-10),
  used to validate `auto`/`meta`, never as ground truth.
- **Measurement:** the sensor (`measure_delta`) runs baseline vs changed config
  over the held-out slice and returns the paired delta + `clears_mde` verdict.

## 7. Staged steps

1. **Pilot generator + 200-item set.** A perturbation generator (numeric/name +
   clause add/remove) over a small seed of word-problems → `eval/data/`. Numeric,
   `####`-terminated, cluster_id per seed.
2. **Difficulty calibration run.** `eval` binary over the 200-item set; tune the
   perturbation level until measured accuracy is 40–70%. Publish the realized MDE.
3. **Contamination screen.** n-gram + PaCoST-style check; drop flagged items.
4. **Scale to ~1,000** at the calibrated difficulty. Re-publish n, mean, SE,
   clustered SE, extraction-failure rate, MDE.
5. **Add a LiveBench/LiveCodeBench cross-check slice** for an independent,
   contamination-resistant read.
6. **Wire the sensor into the live loop** (the remaining PR5 seam) once a
   non-saturated set exists for it to measure against.

## 8. Open questions (flagged by the research, to resolve empirically)

- What perturbation/clause count puts *our* target model in 40–70% — measurable
  only by running step 2, not by assuming.
- How to assign `expected_mode` per item without it becoming circular (it's a
  weak prior; the empirically-best mode remains the authority).
- Whether paired McNemar keeps its ~2× efficiency for *marginal* config changes
  (the regime SI actually operates in).

## 9. Sources (verified)

Statistical power: arXiv:2411.00640 (Miller), arXiv:2605.30315. Perturbation:
arXiv:2410.05229 (GSM-Symbolic), arXiv:2402.19255 (GSM-Plus). Contamination:
arXiv:2404.00699 (survey), arXiv:2406.18326 (PaCoST), arXiv:2509.00072. Live
benchmarks: livebench (arXiv:2403.07974), livecodebench. Full verified claim set
and source list archived in `claudedocs/research_eval-dataset_2026-06-07.md`.
