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

- **Functional fixedness / Einstellung** — the model locks onto a familiar
  procedure or frame that doesn't fit, "fixated by red herrings"
  ([Only Connect](https://arxiv.org/pdf/2306.11167),
  [chat-search fixedness](https://arxiv.org/pdf/2504.02074)). Corrective:
  **reframing** — deliberately reinterpret the problem or its resources to break
  the mental set (the "problem-reframing" thread, grounded).
- *On delivery:* **multi-agent debate** and **cross-examination** beat a single-pass
  critic for factuality (arithmetic 67→82%, GSM8K 77→85%), with **role diversity
  critical** — which validates the diverse-lens design and points to multi-round
  debate for the hardest claims ([multiagent debate](https://www.emergentmind.com/papers/2305.14325)).

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
- **Unfaithful reasoning / post-hoc rationalization** — the model's *stated*
  reasoning can be theater: biasing the input flips the answer **without changing
  the explanation**, and it will answer "yes" to both "is X > Y?" and "is Y > X?"
  with convincing, contradictory chains ([CoT not always faithful](https://arxiv.org/abs/2503.08679),
  [measuring faithfulness](https://arxiv.org/pdf/2307.13702)). Implication: **don't
  grade the explanation — test the decision** (intervene on inputs / re-derive
  independently / perturb). This is *why* a corrective must judge the bare claim,
  not the model's account of how it got there.

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

### K. Safety / adversarial inputs

When the model can **act** (tools, MCP), injected text becomes a *workflow*
compromise — credential theft, unauthorized actions, policy bypass. ~2.6% of agent
posts in one production network carried hidden injection payloads, and "Policy
Puppetry" jailbroke every major model ([OWASP LLM Top 10 2025](https://genai.owasp.org/llmrisk/llm01-prompt-injection/),
[prompt-injection review](https://www.preprints.org/manuscript/202511.0088)).
Offloads: **injection/jailbreak detection** on inputs and tool streams, an
**instruction hierarchy** the data can't override, and **verify-before-commit**
gating on consequential / irreversible actions
([VIGIL](https://arxiv.org/pdf/2601.05755),
[AgentSentry](https://arxiv.org/pdf/2602.22724)). This is a watchdog (→ C) pointed
at adversaries rather than the model's own slips — and it matters doubly here
because the server runs in an agentic context where the model acts.

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
3. **Don't trust the model's account of its own reasoning.** Faithfulness research
   shows stated reasoning is often *post-hoc* — so correctives must test the
   **decision** (intervene, re-derive, perturb), never grade the explanation. This
   is the strongest form of "metacognition it can't do on itself": it can't even
   reliably report *how* it decided.

## Still pulling (open threads)

Resolved this round: problem-reframing (→ A, functional fixedness), adversarial /
jailbreak resistance (→ K), reasoning faithfulness (→ H). Still open:

- Multi-modal grounding
- Tool/corrective selection routing (which corrective applies, when)
- Value / preference elicitation (eliciting the user's real objective)
- Reversibility / blast-radius assessment before consequential actions (partly K)
- Theory-of-mind / perspective-taking failures
- Temporal reasoning + knowledge-cutoff awareness
- Faithfulness: does the model's stated reasoning match its actual decision?

*(This list grows. Pull a thread → research it → add it here with its grounding and
the failure it offloads.)*
