# Offload Landscape — what an LLM can hand to a server

**Status:** Living map (grows as we pull). **Companion to**
[`NEXT_REASONING_SERVER.md`](NEXT_REASONING_SERVER.md), which frames the value
model as *failure-mode correction*. This note widens the lens to **every kind of
offload**, grounded in the literature, so the design isn't limited to reasoning
correctives.

## The organizing question

We started from cognitive correctives — catch the ways the model goes wrong that
it can't catch from inside its own context. But that's *one kind* of offload. The
unifying rule stays the same: **offload what the model is structurally bad at, or
can't do in its own context.** The kinds differ in *what they provide* and, more
importantly, in *whether the model has to recognize it needs them.*

## The architecture taking shape (three layers around the correctives)

The cognitive correctives only fire **when the model knows it needs help.** But the
worst failures — anchoring, sycophancy, drift, overclaiming — are exactly the ones
the model **can't recognize from inside** (failures of the frame). So three layers
must operate *without* self-diagnosis:

- **Deterministic / symbolic layer** — anything checkable is settled by a solver or
  interpreter, not a probabilistic judge. No judge to fool, no calibration knob, no
  sycophancy.
- **Continuous watchdog** — runs beside generation and surfaces drift / contradiction
  / sycophancy / unsafe-action *unprompted*, before output ships.
- **Experience / state store** — accumulated skills, lessons, and world-state; the
  memory literature says this can outweigh the model itself.

Correctives need self-awareness to invoke; these three don't — which is precisely
the regime where the failures are worst. And the deterministic and accumulated
layers sidestep the LLM-judge calibration problem the Verify spike ran into.

## The catalog of offload kinds (grounded)

### A. Cognitive correctives

The failure-mode catalog in [`NEXT_REASONING_SERVER.md`](NEXT_REASONING_SERVER.md):
Converge / Unstick / Challenge / Hold-the-line, plus tunnel vision, bad causal
reasoning, lost sequence, too-large-to-evaluate, miscalibration, unverified
assertion, sycophancy, order/position bias, lost-in-the-middle, omission bias. The
**Verify** spike (`spikes/verify_spike.py`) validated this layer empirically.

### B. Deterministic / symbolic offload

- **Verify-by-execution** — turn a checkable claim into code or a formal spec and
  *run* it. Unforgeable, no precision/recall knob, immune to pushback. Complements
  the LLM-critic Verify: **route checkable claims (math, logic, code contracts,
  constraints) to a solver; use the adversarial critic only for judgment claims.**
  ([PAL / SymCode / code-as-proof](https://arxiv.org/html/2510.25975v1))
- **Symbolic planning** — LLM translates the goal to a formal problem (PDDL), a
  classical solver returns a *guaranteed-valid* plan, symbolic verifiers re-prompt on
  failure ([LLM+P](https://arxiv.org/pdf/2304.11477)). Fragile step = the
  formalization, so it needs the same validate-and-retry the Research spec uses.
- **Constraint / contract checking** — formal compliance and instruction-following
  verification.

### C. Runtime monitoring / watchdog

- A continuous critic that intercepts drift / hallucination / unsafe instruction
  *before* it reaches the user, including predictive flagging
  ([Watchdogs & Oracles](https://arxiv.org/pdf/2511.14435),
  [LlamaFirewall](https://arxiv.org/pdf/2505.03574)). Removes the "model must
  self-diagnose to call the tool" dependency — the highest-leverage fix for the
  failures the model can't see.

### D. Knowledge / grounding

- **Research** (parallel fetch → verify → cited synthesis; see
  [`RESEARCH_PRIMITIVE.md`](RESEARCH_PRIMITIVE.md)), RAG / retrieval+structuring,
  citation grounding, domain-expertise lookup
  ([RAG survey](https://arxiv.org/html/2509.10697v1)).

### E. Planning / process

- Task decomposition, multi-plan selection, symbolic planning (→ B), plan-vs-progress
  tracking, constraint satisfaction
  ([planning survey](https://arxiv.org/pdf/2402.02716)).

### F. Memory / experience — possibly the highest-leverage layer

- **Skill library** (Voyager: reusable solutions indexed by description — the agent
  was **15.3× slower without it**) + **lesson buffer** (Reflexion: what failed and
  why, in plain language) + durable recall. The standout finding: *the gap between
  has-memory and no-memory is often larger than the gap between model backbones.*
  ([memory survey](https://arxiv.org/html/2603.07670v1),
  [SoK: Agentic Skills](https://arxiv.org/pdf/2602.20867v1)) This turns the dead
  `relate` capability into the layer that may matter most.

### G. State / coherence

- **External world/task state** — entities, attributes, decisions, completed steps,
  open threads, constraints, approval status — queryable and updated, so the model
  doesn't rely on attention over a huge drifting context. Grounds "Hold the line"
  with numbers: long-running agents see ~**42% drop in task success** and **3.2×**
  more human intervention; retrieval must be *state-aware* (recency + approval +
  relevance), not just semantic ([Agent Drift](https://arxiv.org/abs/2601.04170),
  [LLM-State](https://arxiv.org/pdf/2311.17406)).

### H. Self-knowledge

- **Calibrated abstention** — defer / retrieve / say "I don't know" when outside the
  reliable zone. Caveat: token probabilities and *verbalized* confidence correlate
  weakly with correctness, so use ensemble agreement or internal-state signals
  ([Know Your Limits](https://arxiv.org/html/2407.18418v2)).
- **Consistency under perturbation** — run a claim under paraphrases / framings /
  reversal; instability is a flag. Caveat: **consistency ≠ correctness** (can be
  memorization) ([prompt-reverse inconsistency](https://arxiv.org/html/2504.01282v1)).
- **Requirement clarification** — detect an underspecified / ambiguous request and
  surface the clarifying question *before* charging ahead; models default to
  non-interactive and hallucinate the missing requirements
  ([underspecification](https://arxiv.org/html/2505.13360v2)).

### I. Economic / efficiency offload (a different kind)

- **Cost-aware routing & cascades** — estimate query difficulty, route easy → cheap
  model, hard → capable; cascade (try cheap, escalate on low confidence). FrugalGPT;
  BEST-Route reports ~60% cost cut at <1% quality loss
  ([routing survey](https://arxiv.org/pdf/2603.04445)). Doubles as a meta-optimization
  of the server's *own* internals — route the cheap verifier votes to a small model,
  reserve the big one for hard judgment.

### J. Raw capability

- Exact computation (→ B), real-time data (→ D), large-corpus map-reduce, persistent
  project/world state (→ G). Things the model can't do in-context at all.

## The bigger picture

This is no longer "a reasoning server." It's an **LLM-augmentation substrate**: a
place to put everything the model is structurally weak at, organized by *whether it
can ask for the help* (correctives) or *not* (deterministic checks, watchdog,
experience/state). Two design consequences worth holding onto:

1. **Prefer deterministic and accumulated offloads where they apply** — they dodge
   the calibration/sycophancy problems that plague LLM-judge approaches (the Verify
   spike's precision/recall tuning).
2. **The memory/experience layer may be the single highest-leverage piece** — the
   literature says it can beat a better model. We've spent most effort on the
   reasoning correctives; the data suggests memory deserves at least equal weight.

## Still pulling (open threads, not yet researched)

- Problem-reframing / "are we solving the right problem" (partly H-clarification)
- Multi-modal grounding
- Adversarial-input / jailbreak resistance (→ C guardrails)
- Tool/corrective selection routing (which corrective applies)
- Value / preference elicitation
- Faithfulness: does the model's stated reasoning match its actual decision?

*(This list grows. Pull a thread → research it → add it here with its grounding and
the failure it offloads.)*
