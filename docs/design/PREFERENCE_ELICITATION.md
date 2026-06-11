# Deep-dive — Value / Preference Elicitation

**Status:** Deep-dive / proposal. **Parent:**
[`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) §A / §H. **Scope note:** like ToM, this
is not a load-bearing layer — it's the **user-preference half of two layers we
already designed** ([`MEMORY_LAYER.md`](MEMORY_LAYER.md) holds preferences;
[`WATCHDOG_LAYER.md`](WATCHDOG_LAYER.md) enforces them) plus the ask-channel of
requirement-clarification. **One line:** *the model optimizes for an assumed
objective; this captures the user's real one — and, more important, makes the server
enforce it.*

## The failure it corrects

Two failures, and the second is the one that actually hurts:

1. **Wrong objective.** The model solves the *assumed* problem correctly — egocentric
   anchoring on its own read of intent (ties [`THEORY_OF_MIND.md`](THEORY_OF_MIND.md)).
2. **The enforcement gap.** A preference is *stated* and then *not applied* — "heard
   you, did it anyway." Capturing a preference does nothing if the server doesn't make
   the model obey it. This is the failure value-elicitation exists to close, and it's
   the harder half.

## How it's gathered — by inference, not interrogation

Asking "what do you value?" is the weakest channel: **stated preferences diverge from
revealed ones** — documented in humans, and in LLMs themselves, which articulate
principles then act on different ones in context
([stated vs revealed](https://arxiv.org/html/2506.00751v1)). Three channels, in
priority order:

1. **Passive capture of *revealed* preferences (primary).** Watch **edits,
   rejections, complaints, dissatisfaction** — and note that *dissatisfaction is far
   more abundant than satisfaction*, so it's the richest signal
   ([DRIFT](https://arxiv.org/pdf/2510.02341),
   [learning from user edits / PRELUDE](https://proceedings.neurips.cc/paper_files/paper/2024/file/f75744612447126da06767daecce1a84-Paper-Conference.pdf)).
   A rejected output or a complaint outweighs any survey answer. Decompose each into a
   reusable **attribute** (Drift-style), and write it to Memory.
2. **Surface the implicit tradeoff for cheap confirmation.** Rather than open
   elicitation, name the value the current plan is silently optimizing — speed vs
   thoroughness, cost vs quality, reversibility vs progress, scope vs effort — and let
   the user correct it. Cheap, and it catches the egocentric assumption directly.
3. **Explicit ask — only at a genuine fork** the model can't resolve from what it
   already knows, and ask the *maximally informative* question, not an open-ended one
   ([optimal preference elicitation](https://arxiv.org/pdf/2404.13895)).

## When — lazily, continuously, on contradiction

- **Not upfront interrogation** — stated preferences are unreliable until the options
  are concrete, and a pre-task survey annoys.
- **At the fork** — elicit the moment a value-tradeoff actually bears on a choice; the
  **Converge** corrective surfaces it there.
- **Continuously / passive** — the Watchdog updates the preference model from every
  turn's revealed signals; no asking required.
- **On contradiction** — when an action conflicts with a stored preference, the
  Watchdog flags it *before* it ships. **This is the enforcement half**, and it's what
  makes elicitation worth anything.

## The loop: capture → store → recall → **enforce**

```
revealed signal (edit/reject/complaint) ─► decompose to attribute
        │                                          │
   explicit ask (at a fork) ──────────────────────►├─► Memory (semantic, per-user)
        │                                          │
   surfaced tradeoff (confirm) ───────────────────►┘
                                                   ▼
   every future task ─► recall relevant prefs ─► Watchdog enforces on each output
                                                   │
                                  flags any output that violates a stored preference
```

The recall-and-enforce end is the point. A preference store the model can *ignore* is
the status quo; the Watchdog turning "stored preference" into "blocked violation" is
the capability.

### Preference record (sketch)

```jsonc
{
  "attribute": "verbosity | scope | format | tone | risk | ...",
  "value": "string",                 // "full feature, never MVP scope"
  "scope": "global | project | task",
  "source": "stated | revealed",     // revealed weighted higher
  "strength": 0.0,                    // accrues from repeated signals
  "provenance": "the edit/complaint/answer it came from",
  "status": "active | superseded",
  "updated_at": "rfc3339"
}
```

## Hard problems

1. **Stated ≠ revealed** — trust the revealed signal over the stated one when they
   conflict; the model's articulated principles are the *less* reliable source.
2. **Sparse and noisy revealed signal** — a single rejection isn't a standing
   preference. Accrue `strength` over repeated signals before treating it as durable;
   most users give little explicit feedback (sparsity).
3. **Over-personalization / catastrophic forgetting** — over-fitting to inferred
   preferences degrades the base model's general alignment; keep preferences as a
   thin overlay, not a rewrite.
4. **Preference drift** — preferences change; apply recency/decay (ties Memory's
   consolidation levers) and watch for false supersession ("this task is different"
   ≠ "the standing preference changed").
5. **The enforcement gap** — the central one. Capturing without enforcing produces
   exactly "I heard you and did it anyway"; the Watchdog check on every output is the
   non-negotiable other half.
6. **Privacy** — preference data is a per-user profile; eviction/redaction apply
   (Memory §privacy).
7. **Interrogation fatigue** — asking too often is itself a disliked behavior; the
   ask channel must stay rare and fork-gated.

## Where it lives

Not a new layer. The **user-model half of Memory's semantic store** (write on
capture, recall on every task), with the **Watchdog** doing passive capture and
enforcement. It overlaps **requirement-clarification** (the ask-at-a-fork channel),
**ToM** (modeling the user's real objective), and **Converge** (surfacing the
tradeoff at a fork).

The canonical illustration is the enforcement gap in this very session: a stated
"don't use word X" preference that the model captured and then violated repeatedly.
That failure is precisely what capture-without-enforcement produces — and exactly
what the Memory-stores-it / Watchdog-enforces-it loop is built to prevent.

## Open questions

- The right `strength` threshold before a revealed signal becomes a durable
  preference, and the decay curve for drift.
- Attribute taxonomy — fixed predefined set (Drift) vs open-vocabulary.
- Enforcement authority — does the Watchdog *block* a preference-violating output, or
  *flag and let the model revise*? (Likely revise, except for hard bans.)
- Per-user vs shared preference isolation (same blast-radius question as Memory).
