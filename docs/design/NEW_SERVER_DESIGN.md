# A New Reasoning Server — Master Design

**Status:** Consolidated design. **Scope:** the complete design corpus for a *new*
(greenfield) MCP reasoning server, informed by operating `mcp-reasoning`. This is
**not** a v2 or a refactor plan — it is the single document that gathers the value
model, the offload landscape, every layer deep-dive, the validated spike, and the
research grounding into one place.

This note is the synthesis. Each section links the standing deep-dive that carries
the full detail; the [deep-dive index](#13-deep-dive-index) lists them all.

---

## 1. Thesis — why build new, and why it exists

`mcp-reasoning` works: 35 tools, ~64k lines of production Rust, 2,895+ tests. But a
large share of those lines is **scaffolding to survive two early choices** — parsing
free-form LLM text into strict per-mode types, and letting the server modify itself
at runtime. The `extract_json` fallbacks, the parse/schema defect detection, the
4-phase self-improvement loop, the self-heal propose-PR pipeline, the circuit
breakers and allowlists guarding them — almost all of it exists to cope with those
two decisions. A clean slate that avoids both removes most of the accidental
complexity. The user-visible capability is worth keeping; the machinery around it
mostly is not.

**North star:** a thin, fast, stateless-by-default prompt executor. The model does
the reasoning under a *constrained* output contract; the server does orchestration,
persistence, observability, and safety. Anything that can run offline runs offline.
The hot path stays simple enough that it rarely needs guards wrapped around it.

### The value model: metacognition the model can't run on itself

When Claude calls a reasoning tool, **Claude is calling Claude** — so the server adds
nothing by reasoning *harder*. What it adds is **catching the ways the model reliably
goes wrong that it cannot catch from inside its own context.** These are failures of
the *frame*, not of effort: you cannot out-think a blind spot from inside it. Asking
an anchored model to "reconsider" in the same context just yields more confident
wrong reasoning. An external, structured, **independent** pass is the only thing that
breaks the frame — which is exactly why in-context self-reflection fails and a
separate tool works.

That is the irreducible value: **metacognition the model can't run on itself.** Not
horsepower (the client has it), not a methodology library (nice-to-have), not
parallelism for its own sake. The server is a **catalog of correctives for the
calling model's predictable failure modes** — what you reach for when the model is in
trouble and can't tell.

This is a documented finding, not just a framing:

- Intrinsic self-correction — fixing your own reasoning with no external feedback —
  *degrades* performance ([Huang et al. 2023](https://arxiv.org/abs/2310.01798)).
- Models **cannot detect their own reasoning errors but can fix them once an external
  pass surfaces them** ([Tyen et al. 2023](https://arxiv.org/html/2311.08516v2)).
- Separating production from a fresh review session measurably improves output
  ([Cross-Context Review](https://arxiv.org/pdf/2603.12123)).

The offload value is therefore concentrated in **detection and judgment** — the parts
the model provably cannot do on itself — not in generating more reasoning. Full
treatment: [`NEXT_REASONING_SERVER.md`](NEXT_REASONING_SERVER.md).

---

## 2. The failure-mode catalog (open, not fixed)

The daily-driver correctives — why it becomes a go-to when the model is stuck in its
own bullshit:

| Failure mode | What it looks like | Corrective |
|---|---|---|
| **Indecision** | dumps N options instead of committing | **Converge** — weigh, commit, recommend |
| **Stuck / looping** | no next move; plausible motion that goes nowhere | **Unstick** — externalize one structured step |
| **Anchoring / overconfidence** | convinced while wrong; defends the answer | **Challenge** — an independent pass not standing where the model stands |
| **Drift** | loses the goal, constraints, prior decisions on a long task | **Hold the line** — keep the thread |

**This list is open.** The catalog grows by the same move each time: name a reliable
failure the model can't see from inside, design the external corrective. The research
backs a longer, cited list:

| Failure mode | Evidence | Corrective |
|---|---|---|
| **Sycophancy / caves under pushback** | flips correct→incorrect ~15% under disagreement; judges correctly in parallel, caves sequentially ([SycEval](https://arxiv.org/html/2502.08177v4)) | evaluate **blind to the user's stance**, outside the pressured thread |
| **Can't detect own errors** | self-correction without external signal degrades ([Huang](https://arxiv.org/abs/2310.01798), [Tyen](https://arxiv.org/html/2311.08516v2)) | independent error-finder; model fixes once told |
| **Order / position bias** | option order shapes the answer; partly architectural ([CoBBLEr](https://arxiv.org/html/2412.00323v1)) | re-run under permuted orderings; position-invariant aggregation |
| **Lost in the middle** | U-shaped attention ignores middle content ([Liu et al.](https://arxiv.org/abs/2406.16008)) | chunked **independent** extraction |
| **Authority / bandwagon deference** | "according to this paper…" triggers the most regressive caving ([SycEval](https://arxiv.org/html/2502.08177v4)) | source-/authority-blind evaluation |
| **Omission / status-quo bias** | biased *against acting*, stronger than humans ([PNAS](https://www.pnas.org/doi/10.1073/pnas.2412015122)) | weigh action vs inaction symmetrically |
| **Tunnel vision** | first plausible answer, no alternatives | breadth before depth |
| **Bad causal reasoning** | correlation taken for cause | a causal method (Pearl's ladder) |
| **Lost sequence** | how earlier choices constrain later ones | temporal / timeline |
| **Too large to evaluate** | more candidates than context holds | offload the search (MCTS-style) |
| **Miscalibration** | documented confidence–competence gap ([study](https://arxiv.org/pdf/2309.16145)) | confidence from ensemble agreement |
| **Unverified assertion** | plausible but false | adversarial verification / grounded research |
| **No memory across turns** | re-deriving or losing prior work | durable recall |
| **Functional fixedness / Einstellung** | locks onto a familiar frame that doesn't fit ([Only Connect](https://arxiv.org/pdf/2306.11167)) | **reframing** — reinterpret the problem |
| **Unfaithful reasoning / post-hoc rationalization** | stated reasoning is theater; biasing input flips the answer without changing the explanation ([CoT not faithful](https://arxiv.org/abs/2503.08679)) | **test the decision, not the explanation** |
| **Wrong objective** | solves the *assumed* problem correctly | preference elicitation + enforcement |
| **Egocentric anchoring (curse of knowledge)** | assumes others share its knowledge ([Ullman]) | perception→belief perspective-taking |

Every entry is a place the model is predictably weak *and blind to from inside*. None
is the final list — the citations turn this from priors into something grounded.

---

## 3. The architecture: four layers around the correctives

The cognitive correctives only fire **when the model knows it needs help.** But the
worst failures — anchoring, sycophancy, drift, overclaiming — are exactly the ones
the model **can't recognize from inside.** So the architecture is four layers, split
by *whether the model has to recognize it needs them*:

1. **Cognitive correctives** (§A) — the *what*. Invoked when the model can
   self-diagnose. Delivered by the primitives below, each engineered for real
   independence.
2. **Watchdog** ([`WATCHDOG_LAYER.md`](WATCHDOG_LAYER.md)) — the *when*. Runs beside
   generation and fires correctives the model can't self-diagnose to call. Removes the
   self-diagnosis dependency — the highest-leverage fix for the failures that matter
   most.
3. **Memory / experience** ([`MEMORY_LAYER.md`](MEMORY_LAYER.md)) — accumulated
   skills, lessons, world-state. The literature says this can outweigh the model
   itself. Verified-before-stored.
4. **Deterministic / symbolic** ([`DETERMINISTIC_LAYER.md`](DETERMINISTIC_LAYER.md)) —
   anything checkable is settled by a solver, not a probabilistic judge. No judge to
   fool, no calibration knob, no sycophancy.

How they cohere: **the watchdog reads memory** (what *should* be true) to check the
**live trajectory**, and **fires the correctives** + gates actions; **deterministic
checks** are preferred wherever the signal is checkable, by both the watchdog and
Verify; **memory's write path is gated by verification** (Verify/Research/execution)
so the store stays curated, not credulous. Correctives need self-awareness to invoke;
the other three don't — which is precisely the regime where the failures are worst.

Full map of all 11 offload kinds (A–K): [`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md).

---

## 4. The primitives (the buildable units)

Design the surface around the *failure modes*; reach for whichever mechanism delivers
the correction. The primitives are the units that deliver them — each maps to one or
more failure modes, and the list is as open as the catalog.

| Primitive | Corrects | Mechanism |
|---|---|---|
| **Step** | stuck / looping | externalized structured step |
| **Decide** | indecision, miscalibration | methodology (weigh / causal / probabilistic) |
| **Verify** | overconfidence, unverified assertion | independent adversarial pass |
| **Diverge** | anchoring, tunnel vision, perspective-taking | independent perspectives |
| **Search** | too-large-to-evaluate | parallel independent evaluation |
| **Recall** | drift, lost prior work | durable memory |
| **Research** | unverified assertion, at scale | parallel fetch + verify |

The mechanisms (independence, parallelism, methodology, memory, offloaded budget) are
**how** the corrections get delivered, not **why** the tool exists.

### Designing real independence (the judge-bias contract)

Every corrective is delivered by an LLM critic/judge — and **an LLM judge is not
automatically independent.** Judges carry documented biases: verbosity, order,
bandwagon, egocentric/self-preference ([CoBBLEr](https://arxiv.org/html/2412.00323v1)).
Offloading naively just re-launders the biases the corrective was meant to remove. So
independence must be **engineered**, by construction, on every Verify/Decide/Diverge:

- **Blind the judge** to source, author, and the user's stance.
- **Permute order** of options/evidence and aggregate across permutations.
- **Length-normalize** judgments.
- **Use diverse lenses, not N identical critics** — different failure modes need
  different critics.
- **Judge in parallel, never sequentially under pushback** — the model judges
  correctly in isolation and caves in a thread; keep the critic out of the pressured
  conversation entirely.

This is a hard contract, not a nice-to-have. Multi-agent debate and cross-examination
beat a single-pass critic for factuality (arithmetic 67→82%, GSM8K 77→85%), with role
diversity critical ([multiagent debate](https://www.emergentmind.com/papers/2305.14325)).

---

## 5. What's validated — the Verify spike

The central architectural bet (constrained output + independent verification) is not
just argued; it was tested in `spikes/verify_spike.py`.

- **Constrained output works.** 15/15 schema-valid responses via forced `tool_use`,
  zero parse failures — the empirical basis for making constrained output the core
  contract (§6).
- **Independent verification catches the model's own confident errors**, including two
  the author made during the session. In-context self-critique missed them; an
  unanchored pass caught them.
- **Pushback resistance.** A naive sequential critic caved on 1/4 pushback cases; the
  independent `verify` (k=3, parallel) was immune — the SycEval finding reproduced in
  miniature.
- **Calibration matters and is tunable.** Switching the verifier profile from
  ADVERSARIAL to CALIBRATED — requiring each refutation to **name a specific concrete
  error**, plus a steelman lens — moved false positives from 1/6 → 0/6 while keeping
  catch at 6/6. Over-refutation is fixable without losing recall.

This is why the deterministic layer matters too: the spike's precision/recall tuning
is the LLM-critic's burden. For checkable claims you don't calibrate — you execute
(§3 layer 4).

---

## 6. Architecture decisions

### 1. Constrained output is the core contract (biggest lever)

Every mode declares an output JSON Schema; the model is **forced** to it via the
provider's structured-output / tool-use feature. No free-text parsing, no extraction
fallbacks, no self-heal-for-parsing. Keep a thin schema validator for
defense-in-depth, but the happy path is schema-guaranteed by the API. *Removes:*
`extract_json` and its fallbacks, parse/schema defect detection, and the entire "heal
my own parsers" rationale for the self-improvement/self-heal subsystems.

### 2. Improvement is offline

The server emits structured metrics and traces. A **separate offline job** reads them,
proposes threshold/prompt changes, and a human applies them. No runtime
self-mutation. *Removes from the hot path:* the SI manager/executor/learner,
`heal_manager`, `heal_cycle`, `repair`, the circuit breaker, the allowlist.

### 3. Modes are data, not three files of code each

A mode is `{ id, prompt template, thinking budget, output schema, routing hints }`.
One generic executor runs any mode. The few modes with real algorithmic bodies (MCTS
scoring, graph aggregation) register as **plugins** — code behind the same interface —
rather than forcing everything into either pure data or pure code.

### 4. A few orthogonal primitives + one router

Expose the value-driven primitives (§4), not 13 modes with heavy overlap. Each is
justified by a *specific* value in-context thinking cannot supply. Selection is the
tool descriptions doing their job; if an explicit router is needed at all, it is
**one** thin selection/escalation layer replacing `auto` + `meta` + `confidence_route`

- `preset`. (Routing is itself a deep-dive — see §7.)

### 5. Explicit orchestration boundary

Decide up front and hold the line: the server owns multi-step workflows as
first-class operations (with the streaming progress it already does well), and the
client just starts one and watches. No mixed model where some tools are client-driven
loops and others are server-owned.

### 6. Observability designed in, not bolted on

- The metric/event record carries **tokens, model, cost, latency, and session** from
  the first commit.
- Activity events publish to a **shared sink** (localhost UDP, named pipe, or
  append-only SQLite) that a **separate dashboard binary** aggregates. Multiple server
  instances → one dashboard, no port collisions, dashboard survives restarts.
  ([`DASHBOARD.md`](DASHBOARD.md) for the rationale, learned the hard way.)
- A first-class `--demo`/replay mode emits synthetic-but-realistic activity so the
  dashboard is demonstrable without live spend or induced defects.

### 7. Right-sized testing

Gate coverage **hard on the invariants** — schema validators, safety guards, storage,
redaction — and stop mandating 95–100% on glue. Property-test the validators,
integration-test the seams, skip the exhaustive mock-response unit tests.

---

## 7. Selection & routing — four problems, four owners

With ~11 kinds of help plus a watchdog, "which corrective, when" *looks* like a
routing problem. The data says the centralized meta-router is the wrong answer: the
old server shipped **four** of them (`auto`, `meta`, `confidence_route`, `preset`) and
`auto` saw 12 organic uses against `linear`'s 67. A second datum is sharper: **more
tools catastrophically degrade selection** — accuracy drops from 78% with 10 tools to
13.6% with 100+ ([Less is More](https://arxiv.org/pdf/2411.15399)). So the *first*
routing job isn't selecting — it's **reducing the menu**.

Routing dissolves into four smaller problems:

1. **Selection — *which* corrective. Mostly not the server's job.** Model-can-tell →
   the *client* routes via tool descriptions; the server's real job is **tool-set
   reduction** (retrieve the relevant few, keep the surface small). Model-can't-tell →
   the **watchdog** routes (signal → corrective). The dead middle — a meta-selector
   second-guessing a capable client — is what the old four routers were; don't rebuild
   it.
2. **Mechanism — *how* to deliver. The server's legit job.** Deterministic symbolic
   check vs adversarial LLM critic; which model tier; one-shot vs multi-round. The
   server knows what the client doesn't — whether a claim is checkable, what each path
   costs.
3. **Escalation — cascades on a reliable signal.** Try cheap, escalate on the cheap
   result's *own* uncertainty — but on **ensemble-agreement/consistency**, never
   self-reported confidence (poorly calibrated).
4. **Composition — pipelines.** An orchestration-boundary question (§6.5), not a
   per-call meta-router.

Prefer **deterministic routing signals** (checkable-ness, action-type) and ensemble
signals over the model's self-report. Full treatment:
[`CORRECTIVE_SELECTION.md`](CORRECTIVE_SELECTION.md).

---

## 8. What the usage data says

Pulled from the live `mcp-reasoning` SQLite (`thoughts`/`branches`), ~271 organic
sessions over 5.5 months, this session's ~500 synthetic `linear` calls filtered out.
**Caveat:** `metrics`/`invocations` were never persisted, so this is a
*thoughts-produced proxy* over modest dev/eval-heavy data — trust the **zeros and
relative magnitudes**, not absolute counts.

The data backs the failure-mode framing — the most-used tools *are* the daily-driver
correctives:

- **`linear` is the #1 organic tool (67)** — *Unstick*. The cheap structured step is
  the workhorse, not low-value.
- **The methodology bucket dominates** (`decision` 66, `evidence` 20, `counterfactual`
  19, `timeline` 11). "Methodology as a callable" is the most validated value.
- **`verify`/`search` earn their place** (`detect` 31, `reflection` 21, `mcts` 32,
  `tree` 51 branches).
- **Provably dead weight:** the **agent/team tools — 0 invocations in 5.5 months** (7
  of 35 tools); **`relate` — 0**; **`graph` — 16 nodes, 0 edges** (its heavy
  aggregate/refine/prune half never fired). These — not `linear` — are the real cuts.
- **The chain/transition machinery is unmeasurable** — tracked but never persisted.

---

## 9. The layer deep-dives (condensed)

### 9.1 Watchdog — the *when*

The model won't call a corrective when *it* is the one failing, because it can't tell.
The watchdog runs beside the loop, watches for the failures it's blind to, and fires
the help unprompted. It is automatic metacognition — runtime verification for when the
model can't self-assess ([RvLLM](https://arxiv.org/pdf/2505.18585)).

- **Architecture:** a **Large Supervisor Model** pattern — a lightweight model running
  concurrently, issuing **abstain / feedback / interrupt** signals **without rewriting
  outputs**. Independent context + budget (blind it, diversify it). Runs on the
  **activity/event stream the dashboard already built**. Async, never in the stdio
  critical path.
- **Watches for:** self-contradiction (NLI vs stored decisions), sycophantic flip
  (answer changed after pushback with no new evidence), drift from goal/constraints,
  ungrounded claims (entity-level diffs most prone), overclaiming, unsafe/irreversible
  actions, injection in input/tool stream.
- **Mechanics:** cheap heuristics gate the expensive judge; checkpoint over
  token-stream where possible; **it must read state** (goal, constraints, prior
  decisions) — so **watchdog + memory are a pair**.
- **Intervention:** feedback (flag, the right default — the model fixes once told),
  interrupt (stop before bad output), gate (block consequential/irreversible actions
  pending verify-before-commit). **Never rewrites** — surfaces or gates; the model
  fixes.
- **Make-or-break:** alarm fatigue. Fire only on real signals, tier by risk, emit a
  trace event per trigger to measure catch-rate vs noise. Low precision = worse than
  nothing.

Full: [`WATCHDOG_LAYER.md`](WATCHDOG_LAYER.md).

### 9.2 Memory / experience — possibly the highest-leverage layer

*The gap between has-memory and no-memory is often larger than the gap between model
backbones* ([memory survey](https://arxiv.org/html/2603.07670v1)); Voyager was
**15.3× slower** without its skill library.

- **Four types (CoALA):** working (the in-context bottleneck), episodic (what
  happened), semantic (consolidated facts/policies), procedural (skills/workflows).
  Most systems implement two well; the **transitions** (episodic→semantic via
  reflection, episodic→procedural via skill extraction) are where value and hard
  problems live.
- **Flagship capabilities:** a **skill library** (reusable reasoning artifacts indexed
  by description, grown without catastrophic forgetting) and a **lesson buffer**
  (Reflexion: what failed and why, in plain language).
- **Read path:** `score = α·recency + α·importance + α·relevance`, extended with
  **trust/provenance** and **task-state relevance**. Retrieval precision *is* the
  product — surface the wrong memories and you poison the current reasoning.
- **Write path:** capture → reflect → consolidate, via importance / merge / decay /
  eviction. Guard against summarization drift and memory blindness.
- **Contract: effortless, not manual.** `relate` is dead (0 uses) because recall is
  manual. Make it the default: **push** (server auto-surfaces relevant
  skills/lessons/state) + **pull** (explicit `recall`/`save` fallback).
- **The reframing hard problem: memory poisoning.** A skill library is a high-value
  attack surface — sleeper poisoning plants a fabricated "successful experience." This
  forces the central move: **verify before you store.** Provenance on every write,
  trust-weighted retrieval, never auto-trust externally-sourced experience, an
  independent check before a new skill is admitted. This ties the two
  highest-leverage layers (memory + verification) together.

Full: [`MEMORY_LAYER.md`](MEMORY_LAYER.md).

### 9.3 Deterministic / symbolic — the most reliable layer

A large class of claims is checkable by *execution*, not judgment — and for those a
solver beats a probabilistic critic, because there is no judge to fool, no calibration
knob, no sycophancy. PAL: translate to code, run it, accuracy jumps **+15 on GSM8K,
+40 on GSM-Hard, +11 on BIG-Bench-Hard** ([PAL](https://arxiv.org/abs/2211.10435)).
The output is deterministic, reproducible, and **unforgeable**.

- **Checkable (the routing-in signal):** arithmetic → CAS; logic/constraints → SMT/SAT;
  code → run tests/type-check; planning → classical planner; **format/schema → schema
  validation** (this is the constrained-output gate); units/dates → libraries;
  contract compliance → formal spec.
- **Not checkable:** judgment, values, open-ended questions, common-sense. Those stay
  with the LLM critic.
- **Architecture:** translate → execute → feed back. The engine's errors are *ground
  truth* — re-prompt on **actual** constraint violations, not a critic's opinion.
- **The failure moves to translation (autoformalization):** the model can formalize
  the *wrong problem*. Defenses: back-translation/round-trip, the solver's free
  signals (validity, infeasibility), ensemble formalizations, keep the formal target
  small and typed. A net win *where it applies*.
- **Safety is non-negotiable:** executing model-generated code is an RCE risk. Strong
  isolation (Docker/microVM/E2B), whitelisted libraries, filesystem/network disabled
  by default, timeouts + quotas. A verify-by-execution layer without a sandbox *is* an
  RCE hole.

Full: [`DETERMINISTIC_LAYER.md`](DETERMINISTIC_LAYER.md).

### 9.4 Research — the biggest new capability

Offload a question to a separate budget; get back a short, cited,
adversarially-verified answer — not 15 articles. It exercises every value at once:
parallelism, offloaded budget, adversarial verification, compact return. The client
*structurally cannot* run 15 parallel fetch-and-verify cycles in its own context.

Five-phase pipeline: **Scope** (1 call → N angles) → **Search** (N parallel, dedup
URLs) → **Fetch+Extract** (pipeline per source, no barrier → falsifiable claims) →
**Verify** (dedup claims → K independent adversarial votes each, diverse lenses at
depth) → **Synthesize** (compact cited answer + completeness critic). One knob —
`depth` (quick/standard/deep/exhaustive) — scales fan-out and rigor. Hard ceilings
(`budget_tokens`, `deadline_ms`) trigger graceful early synthesis; **no silent
truncation**. A **grounding gate** rejects any output citing an unfetched source —
no fabricated citations, ever. Results feed the memory store (the Recall tie-in).

Full: [`RESEARCH_PRIMITIVE.md`](RESEARCH_PRIMITIVE.md).

### 9.5 Two correctives that sharpen primitives (not new layers)

- **Theory-of-mind** ([`THEORY_OF_MIND.md`](THEORY_OF_MIND.md)) — corrects egocentric
  anchoring (the curse of knowledge). The method that works:
  **perception→belief decomposition** — establish what a party *perceived/has access
  to*, then infer belief *from that*, excluding what only the model knows. Delivered by
  **Diverge/perspectives** as a methodology; its user-modeling half feeds
  requirement-clarification. A clean split: ToM-of-others is this corrective;
  ToM-of-self (the model can't model its own mind) is what the **watchdog** exists for.
- **Value / preference elicitation**
  ([`PREFERENCE_ELICITATION.md`](PREFERENCE_ELICITATION.md)) — the user-preference half
  of Memory (stores) + Watchdog (enforces). Two failures: *wrong objective* (solving
  the assumed problem) and **the enforcement gap** (a preference stated then not
  applied — "heard you, did it anyway"), which is the harder half. Gathered **by
  inference, not interrogation** — stated preferences diverge from revealed ones, so
  watch edits/rejections/complaints (dissatisfaction is the richest signal), surface
  the implicit tradeoff for cheap confirmation, and ask explicitly only at a genuine
  fork. The loop: capture → store → recall → **enforce**. The recall-and-enforce end
  is the point; a preference store the model can ignore is the status quo. The
  canonical illustration is the enforcement gap in this very session: a stated "don't
  use word X" preference captured and then violated repeatedly — exactly what
  capture-without-enforcement produces.

---

## 10. Cross-cutting design principles

- **Don't trust the model's account of its own reasoning.** Faithfulness research
  shows stated reasoning is often post-hoc — so correctives must **test the decision**
  (intervene, re-derive, perturb), never grade the explanation. This is the strongest
  form of "metacognition it can't do on itself": it can't even reliably report *how*
  it decided.
- **Prefer deterministic and accumulated offloads where they apply** — they dodge the
  calibration/sycophancy problems that plague LLM-judge approaches.
- **Engineer independence; never assume it** — the judge-bias contract (§4) applies to
  every LLM critic in the system, including the watchdog's monitors.
- **Reduce the menu before selecting** — the 82% tool-count degradation makes a small,
  orthogonal primitive surface an empirical requirement, not taste.
- **Verify before you store** — the memory layer is curated, not credulous; the
  poisoning surface forces it.
- **Off by default, gated, read-only first** — every new capability (network egress,
  code execution, self-mutation if any) is opt-in like the existing `SELF_HEAL_*`
  flags.

---

## 11. What we carry over, and what stops existing

**Carry over (earned):** `ModeCore`-style composition over trait inheritance;
trait-based mockability (storage/client/time); `#![forbid(unsafe_code)]` and no
`unwrap`/`expect` in production; structured `tracing` to stderr only; stdio transport;
SQLite persistence; the semantic-memory idea — but as an optional capability, not core.

**Stops existing (the payoff):** `extract_json` fallbacks; parse/schema defect
detection; the self-heal propose pipeline (`heal_manager`, `heal_cycle`, `repair`,
`eligibility`, `heal_review`); the runtime SI loop's executor / circuit-breaker /
allowlist / baseline; the four meta-routers; most per-mode code triplication; a large
fraction of the mock-response test wiring.

**Cut with hard usage evidence (not taste):** the agent/team tools (7 of 35, 0
invocations in 5.5 months); `relate` (0); `graph`'s heavy half (0 edges ever); the
chain/transition suggestion engine (never persisted what it tracked).

**What not to chase:** general agent/team orchestration — the data says it's dead. If
multi-agent shows up, it's the fan-out-and-verify primitive, not a standing framework.

---

## 12. Consolidated open questions & risks

- **Constrained output portability:** not every provider supports forced schemas
  equally. Need a fallback that **degrades to validated free-text, never back to
  self-heal**.
- **Usage data is a proxy, not ground truth.** Instrument a real call log + chain data
  in the new server and re-validate the cuts against production traffic before
  committing.
- **Watchdog precision/fatigue** — the same calibration problem the Verify spike hit,
  now in real time and higher-stakes. One general supervisor vs a panel of narrow
  monitors? Checkpoint vs streaming default? What may it block autonomously?
- **Memory:** reflection/consolidation triggers; push-vs-pull balance before recall
  becomes noise; trust model for externally-sourced experience; per-user vs shared
  isolation (poisoning blast-radius); forgetting policy.
- **Deterministic layer:** which engines first (Python sandbox + schema validator
  covers most); round-trip check always or only on low confidence; the checkable-ness
  classifier (bias toward "not checkable" is the safer default); sandbox tier.
- **Routing:** the right K for tool-set reduction (and does retrieving tools inherit
  its own lost-in-the-middle?); escalation cost vs just calling the capable model;
  which problem-shapes deserve a fixed server pipeline.
- **Preference elicitation:** `strength` threshold before a revealed signal is durable;
  enforcement authority (block vs flag-and-revise, likely revise except hard bans).
- **Losing in-production self-tuning** — offline improvement gives up runtime
  adaptation. Likely fine (low-ROI, high-risk) but called out.
- **Eval is the hardest open problem across every layer** — how do you measure whether
  memory / the watchdog / a corrective is *helping vs hurting*? A poisoned or stale
  store, or a noisy watchdog, degrades silently.

---

## 13. Deep-dive index

| Document | Layer / topic |
|---|---|
| [`NEXT_REASONING_SERVER.md`](NEXT_REASONING_SERVER.md) | North-star design note: value model, architecture decisions, what stops existing |
| [`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) | The full map of 11 offload kinds (A–K), grounded |
| [`WATCHDOG_LAYER.md`](WATCHDOG_LAYER.md) | The watchdog — automatic metacognition, the *when* |
| [`MEMORY_LAYER.md`](MEMORY_LAYER.md) | Memory / experience — four types, skill library, poisoning |
| [`DETERMINISTIC_LAYER.md`](DETERMINISTIC_LAYER.md) | Deterministic / symbolic — verify-by-execution, sandboxing |
| [`CORRECTIVE_SELECTION.md`](CORRECTIVE_SELECTION.md) | Routing — four problems, four owners, no meta-router |
| [`RESEARCH_PRIMITIVE.md`](RESEARCH_PRIMITIVE.md) | The Research primitive — end-to-end spec |
| [`THEORY_OF_MIND.md`](THEORY_OF_MIND.md) | ToM as a corrective — perception→belief decomposition |
| [`PREFERENCE_ELICITATION.md`](PREFERENCE_ELICITATION.md) | Value/preference elicitation + the enforcement gap |
| [`DASHBOARD.md`](DASHBOARD.md) | Observability substrate — the activity stream the watchdog consumes |
| `spikes/verify_spike.py` | The validated spike — constrained output + independent verification |

---

## 14. Research grounding (consolidated citations)

**Value model / self-correction:** [Huang 2023 — self-correction degrades](https://arxiv.org/abs/2310.01798),
[Tyen 2023 — can't detect, can fix once told](https://arxiv.org/html/2311.08516v2),
[Cross-Context Review](https://arxiv.org/pdf/2603.12123).

**Failure modes:** [SycEval — sycophancy](https://arxiv.org/html/2502.08177v4),
[CoBBLEr — judge biases](https://arxiv.org/html/2412.00323v1),
[Lost in the middle](https://arxiv.org/abs/2406.16008),
[Omission bias (PNAS)](https://www.pnas.org/doi/10.1073/pnas.2412015122),
[Confidence–competence gap](https://arxiv.org/pdf/2309.16145),
[CoT not always faithful](https://arxiv.org/abs/2503.08679),
[Measuring faithfulness](https://arxiv.org/pdf/2307.13702),
[Functional fixedness](https://arxiv.org/pdf/2306.11167).

**Watchdog:** [RvLLM — runtime verification](https://arxiv.org/pdf/2505.18585),
[Watchdogs & Oracles](https://arxiv.org/pdf/2511.14435),
[LSM — Large Supervisor Model](https://www.researchgate.net/publication/401283765),
[SelfCheckGPT], [guardrails + HITL](https://developers.openai.com/api/docs/guides/agents/guardrails-approvals).

**Memory:** [Memory survey](https://arxiv.org/html/2603.07670v1),
[CoALA](https://arxiv.org/pdf/2309.02427),
[Generative Agents](https://arxiv.org/pdf/2304.03442),
[SoK: Agentic Skills](https://arxiv.org/pdf/2602.20867v1),
[Letta](https://www.letta.com/blog/agent-memory),
[mem0 eviction](https://mem0.ai/blog/memory-eviction-and-forgetting-in-ai-agents),
[sleeper poisoning](https://arxiv.org/abs/2605.15338),
[MemoryGraft](https://arxiv.org/pdf/2512.16962),
[SSGM governance](https://arxiv.org/html/2603.11768v1),
[Agent Drift](https://arxiv.org/abs/2601.04170),
[LLM-State](https://arxiv.org/pdf/2311.17406).

**Deterministic / symbolic:** [PAL](https://arxiv.org/abs/2211.10435),
[LLM+P](https://arxiv.org/pdf/2304.11477),
[autoformalization survey](https://arxiv.org/pdf/2505.23486),
[secure code execution](https://huggingface.co/docs/smolagents/tutorials/secure_code_execution),
[SandboxEval](https://arxiv.org/pdf/2504.00018).

**Routing:** [Less is More — tool-count degradation](https://arxiv.org/pdf/2411.15399),
[cascade survey](https://arxiv.org/html/2603.04445v2),
[confidence tokens — miscalibration](https://arxiv.org/pdf/2410.13284).

**Research / grounding:** [RAG survey](https://arxiv.org/html/2509.10697v1),
[multiagent debate](https://www.emergentmind.com/papers/2305.14325).

**Self-knowledge / ToM / preferences:** [Know Your Limits — abstention](https://arxiv.org/html/2407.18418v2),
[prompt-reverse inconsistency](https://arxiv.org/html/2504.01282v1),
[underspecification](https://arxiv.org/html/2505.13360v2),
[ToM in LLMs](https://www.emergentmind.com/topics/theory-of-mind-in-large-language-models),
[curse of knowledge / feedback](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC8107504/),
[UniToMBench / perception decomposition](https://arxiv.org/html/2506.09450),
[stated vs revealed preferences](https://arxiv.org/html/2506.00751v1),
[DRIFT](https://arxiv.org/pdf/2510.02341),
[learning from user edits / PRELUDE](https://proceedings.neurips.cc/paper_files/paper/2024/file/f75744612447126da06767daecce1a84-Paper-Conference.pdf),
[optimal preference elicitation](https://arxiv.org/pdf/2404.13895).

**Safety / adversarial:** [OWASP LLM Top 10 2025](https://genai.owasp.org/llmrisk/llm01-prompt-injection/),
[prompt-injection review](https://www.preprints.org/manuscript/202511.0088),
[VIGIL](https://arxiv.org/pdf/2601.05755),
[AgentSentry](https://arxiv.org/pdf/2602.22724),
[LlamaFirewall](https://arxiv.org/pdf/2505.03574).

**Economic routing:** [routing survey / FrugalGPT / BEST-Route](https://arxiv.org/pdf/2603.04445).
