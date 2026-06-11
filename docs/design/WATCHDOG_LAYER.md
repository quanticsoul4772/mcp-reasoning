# Deep-dive — The Watchdog Layer

**Status:** Deep-dive / proposal. **Parent:**
[`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) §C (runtime monitoring) and §K
(safety). **One line:** *the model won't call a corrective when it's the one
failing — because it can't tell — so the watchdog runs beside it, watches for the
failures it's blind to, and fires the help unprompted.*

## Why this is the answer to the deepest problem in the design

Every callable corrective ([`NEXT_REASONING_SERVER.md`](NEXT_REASONING_SERVER.md))
assumes the model **recognizes it needs help and calls the tool.** But the worst
failures — anchoring, sycophancy, drift, unfaithful reasoning — are *failures of the
frame*, invisible from inside (the model that's convinced it's right won't ask to be
challenged; the one rationalizing post-hoc believes its own explanation). A callable
tool cannot fix a failure the model can't perceive.

The watchdog removes the **self-diagnosis dependency.** It is automatic metacognition:
an independent process that watches the model's trajectory and surfaces problems the
model would never have flagged. This is exactly what runtime verification (RV) is
for — on-line assurance when the model can't self-assess, since LLMs are stochastic
and opaque, making pre-deployment guarantees impractical
([RvLLM](https://arxiv.org/pdf/2505.18585),
[Watchdogs & Oracles](https://arxiv.org/pdf/2511.14435)). It is the single
highest-leverage layer *precisely for the failures that matter most.*

## Architecture: an independent supervisor beside the loop

The grounded pattern is the **Large Supervisor Model** — a lightweight model running
*concurrently* with the main model, monitoring its output stream in real time and
issuing structured intervention signals (**abstain / feedback / interrupt**) **without
rewriting outputs**; it interrupts and notifies via a structured payload
([LSM](https://www.researchgate.net/publication/401283765)). Adapted here:

- **Independent context + budget.** The watchdog is *not anchored where the model is*
  — it judges the bare trajectory, blind to the model's self-justification. (The
  judge-bias contract from the value model applies: blind it, diversify it.)
- **It runs on the activity/event stream we already built.** The dashboard's
  in-memory activity bus ([`DASHBOARD.md`](DASHBOARD.md)) — Client→Registry→Mode→…
  events with tool/model/duration — *is* the substrate a watchdog consumes. Build the
  observability once; the dashboard *shows* the stream, the watchdog *acts* on it.
- **Async, non-blocking**, intervening in milliseconds–seconds; it never sits in the
  stdio critical path.

## What it watches for

| Signal | How detected | Failure it catches |
|---|---|---|
| **Self-contradiction** | NLI / consistency vs earlier turns & stored decisions; inter-sample checks ([SelfCheckGPT]) | drift, flip-flopping |
| **Sycophantic flip** | answer changed after pushback with *no new evidence* | sycophancy |
| **Drift from goal/constraints** | current step diverges from the stated goal / constraints / world-state | drift, lost-the-thread |
| **Hallucination / ungrounded claim** | claim not supported by provided context; **entity-level** diffs (dates/names) are most prone | unverified assertion |
| **Overclaiming** | high assertiveness + low calibrated confidence | miscalibration |
| **Unsafe / irreversible action** | risk score on a pending tool call | consequential mistakes |
| **Injection in input / tool stream** | pattern + provenance checks (§K) | adversarial compromise |

## How it detects (mechanics)

- **Checkpoint vs streaming.** Checkpoint (between steps/tool-calls) is cheaper and
  enough for most signals; reserve token-stream supervision for *interrupt-before-bad-output*.
- **Cheap signals gate the expensive judge.** Heuristics first — entity diff,
  contradiction-with-stored-decision, action risk score — and escalate to an LLM (or
  deterministic, §B) check only when a cheap signal fires. This is cost-aware routing
  (§I) applied internally; a critic on *every* token is unaffordable.
- **It must read state.** A watchdog with nothing to check against is useless — it
  needs the goal, the constraints, and prior decisions, which live in the memory /
  world-state layer ([`MEMORY_LAYER.md`](MEMORY_LAYER.md)). **Watchdog + memory are a
  pair:** memory holds what *should* be true; the watchdog checks the live trajectory
  against it.

## The intervention model

Borrow the LSM's three signals and add a gate, tiered by stakes:

- **Feedback (passive)** — annotate/flag: "you contradicted decision X." The model
  *can* correct once an external pass points the error out, even though it couldn't
  find it itself (the faithfulness/self-correction research) — so flag-and-let-it-fix
  is the right default.
- **Interrupt (active)** — stop the stream and notify with a structured signal before
  a bad output ships.
- **Gate / approve** — block a consequential or irreversible action pending
  verify-before-commit or human approval; tiered autonomy (low-risk autonomous,
  high-risk requires verification/human) ([guardrails + HITL](https://developers.openai.com/api/docs/guides/agents/guardrails-approvals)).

The watchdog **never rewrites** the model's output — it surfaces or gates; the model
fixes. That keeps it honest about authority and keeps the model in the loop.

## The watchdog is the meta-layer that fires the correctives

This is the connective insight. The correctives are the *what*; the watchdog is the
*when*. It **invokes Verify / abstention / reframe on the model's behalf** when it
detects the matching signal — closing the loop the model can't close itself ("I'm
convinced I'm right" → watchdog sees the contradiction → fires Challenge). Three ties
make it the spine of the whole server:

1. runs on the **observability** stream (dashboard infra);
2. reads the **memory/world-state** to know what to check against;
3. triggers the **correctives** + gates **actions** (§K).

## The hard problems (honest)

1. **Alarm fatigue — the make-or-break.** Too many flags and every flag is ignored.
   Mitigate: fire only on real signals, tier by risk, threshold on confidence, and
   emit a **trace event per trigger** so you can measure *catch rate vs noise* and
   whether the flag actually led to a correction. If precision is low, the layer is
   worse than nothing.
2. **Cost / latency.** Cheap heuristics gate the expensive judge; checkpoint over
   token-stream where possible; prefer deterministic checks (§B).
3. **The watchdog is still an LLM** — own blind spots and biases. Keep monitors
   *narrow and specialized per signal* (a contradiction-checker, an action-risk
   scorer) rather than one omniscient critic; use deterministic checks where the
   signal allows; *who watches the watchdog* is a real question.
4. **Authority calibration.** Flag vs interrupt vs require-human is policy; mis-set =
   runaway or constant interruption. Tier by risk domain.
5. **Limited observability.** It sees the trajectory, not the model's internals; some
   failures leave no external trace and slip through.

## Open questions

- Checkpoint vs streaming as the default?
- One general supervisor vs a panel of narrow specialized monitors?
- The precision/fatigue tuning — the same calibration problem the Verify spike hit,
  now in real time and higher-stakes.
- Trust/authority model: what may it block autonomously vs escalate?
- Keeping the watchdog itself honest (judge-bias, deterministic where possible).
