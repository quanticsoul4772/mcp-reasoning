# Deep-dive — The Memory / Experience Layer

**Status:** Deep-dive / proposal. **Parent:**
[`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) §F (memory/experience) and §G
(state/coherence). **One line:** *the model's context is ephemeral; the server
remembering — and accumulating verified experience — is a capability the model
structurally lacks, and the literature says it may matter more than the model
itself.*

## Why this may be the highest-leverage layer

The repeated finding across the memory research: *"the gap between has-memory and
no-memory is often larger than the gap between model backbones"*
([memory survey](https://arxiv.org/html/2603.07670v1)); Voyager was **15.3×
slower** without its skill library ([Voyager], [SoK: Agentic Skills](https://arxiv.org/pdf/2602.20867v1)).
The failures it offloads are the expensive, recurring ones: re-deriving solved
problems, repeating known mistakes, losing decisions over a long task, and never
learning across sessions. We've spent most effort on reasoning correctives; the
evidence says memory deserves at least equal weight.

## The four memory types (and what each offloads)

The CoALA framework formalizes four types from cognitive science
([CoALA](https://arxiv.org/pdf/2309.02427)):

| Type | What it is | What it offloads | Failure it fixes |
|---|---|---|---|
| **Working** | the in-context window — "RAM" | nothing; it *is* the bottleneck everything competes for | (the constraint, not a fix) |
| **Episodic** | time-stamped situational events ("what happened") | recall of specific past events/decisions | drift, lost decisions, re-deriving |
| **Semantic** | consolidated facts/policies ("user prefers DD/MM/YYYY") | stable knowledge that shouldn't be re-derived | re-asking, inconsistency |
| **Procedural** | "knowing how" — workflows, skills, tool-use | reusable *solutions*, not just facts | re-solving solved problems |

The four form a stack: *procedure says how, semantic says what the policy is,
episodic says what happened, working holds the live reasoning.* The catch from the
research: **most systems implement only two layers well; the transitions between
them are handled by crude heuristics** — and the transitions (episodic → semantic
via reflection, episodic → procedural via skill extraction) are exactly where both
the value and the hard problems live.

## The two flagship "experience" capabilities

1. **Skill library (procedural).** Voyager's pattern: a successful solution is
   stored, indexed by a natural-language description, and surfaced for similar future
   tasks; the library grows *without catastrophic forgetting* because new skills are
   added, not overwritten. For a reasoning server the "skills" are **reusable
   reasoning artifacts** — a decision framework that worked, a verified plan, a
   debugging approach, a research result — retrievable by description.
2. **Lesson buffer (episodic → semantic via reflection).** Reflexion's pattern: a
   natural-language self-reflection on *what failed and why* is appended to an
   experience buffer that conditions future behavior — "last time, approach X failed
   because Y." Reflection periodically synthesizes raw episodes into higher-level
   inferences and writes them back ([Generative Agents](https://arxiv.org/pdf/2304.03442)).

## The read path: retrieval (where it lives or dies)

The canonical retrieval score (Generative Agents), which everything since refines:

```
score(memory) = α_recency · recency + α_importance · importance + α_relevance · relevance
```

- **recency** — exponential decay since last access.
- **importance** — LLM-scored 1–10 (mundane vs. poignant), set at write time.
- **relevance** — cosine similarity of the memory's embedding to the current query.

Extend it for this server with two more terms the agent-memory literature now
insists on:

- **trust / provenance** — how the memory was sourced and whether it was *verified*
  before storage (see poisoning, below). Untrusted-origin memories are down-weighted
  or quarantined.
- **task-state relevance** — relevance to the *current task's* entities/goal, not just
  lexical similarity; retrieval must be **state-aware** (recency + approval +
  relevance), not pure cosine.

**Retrieval precision is the product.** Surface the wrong memories and you *poison
the current reasoning* with irrelevant or stale context — the layer fails not by
forgetting but by recalling badly.

## The write path: capture → reflect → consolidate

Storing everything is as bad as storing nothing. Four consolidation levers
([Letta](https://www.letta.com/blog/agent-memory), [mem0](https://mem0.ai/blog/memory-eviction-and-forgetting-in-ai-agents)):

- **Importance** — which observations become memories at all (a gate, not a firehose).
- **Merge** — unify related facts into a single canonical record.
- **Decay** — confidence degrades over time.
- **Eviction** — when a memory leaves entirely (*a compliance/privacy tool, not a
  performance one*).

Two drift traps to design against:

- **Summarization drift** — repeatedly compressing history to fit the window throws
  away entity-level detail each pass; "summarize-then-drop" is lossy *compaction*,
  not consolidation, and retrieval depends on the detail it discards.
- **Memory blindness** — archive too aggressively and the agent doesn't know a
  critical fact exists in cold storage.

## The contract / API — effortless, not manual

The current server's `relate` is **dead (0 uses)** precisely because recall is
*manual* — the model has to remember to ask. The design fix is to make it the
*default*, two-way:

- **Push (proactive):** the server auto-surfaces relevant prior skills/lessons/state
  for the current task — "you decided X before; last time approach Y failed here" —
  injected as context or offered, without being asked. This is what turns memory from
  unused to load-bearing.
- **Pull (explicit):** a `recall` / `save` tool as a fallback for when the model
  *does* know to ask.

Auto-capture outcomes (success → candidate skill; failure → candidate lesson) so the
library grows from ordinary use, not a separate curation chore.

## The hard problems (honest)

1. **Memory poisoning — the one that reframes everything.** A skill/lesson library
   is a high-value attack surface: **sleeper poisoning** plants a fabricated
   "successful experience" via an ordinary document, webpage, or repo, and it is then
   *treated as a trusted prior example* that steers all future behavior
   ([AgentPoison], [sleeper poisoning](https://arxiv.org/abs/2605.15338),
   [MemoryGraft](https://arxiv.org/pdf/2512.16962)). This forces the central design
   move: **verify before you store.** The verification layer (Verify / Research) and
   the memory layer are *complementary* — accumulated experience must be **curated,
   not credulous**: provenance on every write, trust-weighted retrieval, never
   auto-trust externally-sourced experience, and an independent check before a new
   "skill" is admitted. That synthesis is how you get the *memory > backbone* upside
   without the poisoning downside — and it ties the two highest-leverage layers
   together ([SSGM governance](https://arxiv.org/html/2603.11768v1)).
2. **Conflicting memories / false supersession.** "I'm in Berlin this week" *looks*
   like it contradicts "user lives in Lisbon" but doesn't — distinguish an *updated*
   fact from a *context-specific* one before overwriting.
3. **Semantic drift & catastrophic forgetting.** Unconstrained autonomy is the
   catalyst; constrain what the agent may silently rewrite, keep entity detail, govern
   evolution (SSGM).
4. **Privacy / redaction.** What's stored about the user; eviction as the compliance
   lever; privacy-aware memory ([Forgetful but Faithful](https://arxiv.org/pdf/2512.12856)).
5. **Eval.** Hardest open problem: how do you measure whether memory is *helping vs
   hurting*? A poisoned or stale library degrades silently.

## How it ties to the rest of the landscape

- It **is** the real form of the **Recall** corrective and the §G **world-state**
  piece (episodic + state for a single long task).
- **Research** results and **Verify** verdicts feed it — and **gate its write path**:
  only verified experience is admitted, making the memory curated.
- The **watchdog** reads state to detect drift-from-prior-decisions.
- **Cost routing** loves it: recalling a stored skill is far cheaper than
  re-deriving.

## Open questions

- **Reflection/consolidation triggers** — periodic? on task completion? on
  contradiction?
- **Push vs pull balance** — how much to auto-surface before it becomes noise that
  pollutes the working context (the very thing §lost-in-the-middle warns about)?
- **Trust model for externally-sourced experience** — the poisoning defense; what
  may ever be admitted without an independent verify?
- **Per-user vs shared experience** — a shared skill library multiplies value across
  users *and* multiplies poisoning blast-radius. Isolation boundary?
- **Forgetting policy** — decay/eviction tuned for compliance and freshness without
  memory blindness.
