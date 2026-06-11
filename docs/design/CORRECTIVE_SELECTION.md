# Deep-dive — Corrective Selection & Routing

**Status:** Deep-dive / proposal. **Parent:**
[`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) ("still pulling": corrective-selection
routing). **One line:** *with ~11 kinds of help plus a watchdog, "which corrective,
when" looks like a routing problem — but the data says the centralized meta-router
is the wrong answer, and routing is really four smaller problems with four different
owners.*

## Start from the data: the router you're tempted to build is the one that died

The old server shipped **four** meta-routers — `auto`, `meta`, `confidence_route`,
`preset`. The usage data ([`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) §"usage"):
`auto` saw **12** organic uses against `linear`'s **67**, and the others barely
register. A router that *re-decides what a capable client already decides well* is
dead weight. Any routing design has to explain why it isn't a fifth dead router.

And a second, sharper datum: **more tools catastrophically degrades selection.**
Accuracy drops from **78% with 10 tools to 13.6% with 100+** — an 82% collapse — via
choice paralysis and tool-list position bias (tools "lost in the middle"); *five
well-scoped tools beat fifty* ([Less is More](https://arxiv.org/pdf/2411.15399),
[semantic tool selection](https://vllm-semantic-router.com/blog/semantic-tool-selection/)).
So the **first** routing job isn't *selecting* — it's *reducing the menu*.

## Routing is four problems, by owner

### 1. Selection — *which* corrective. Mostly **not** the server's job

Split by whether the model can self-diagnose:

- **Model can tell → the client routes, via tool descriptions.** The value model
  already makes descriptions the product surface, and the data says the model is a
  good planner. The server's *real* job here is **tool-set reduction**: retrieve and
  expose the relevant *few* correctives (semantic tool retrieval / RAG-tool fusion),
  and keep the primitive surface genuinely small and orthogonal — now **empirically
  forced** by the 82% degradation, not just taste. The helpful component is a
  **retriever (narrow the menu)**, not a **selector (decide for the model)**.
- **Model can't tell → the watchdog routes, via signal → corrective.** This is the
  whole point of [`WATCHDOG_LAYER.md`](WATCHDOG_LAYER.md): the failures the model
  can't perceive (anchoring, sycophancy, drift) can't be client-selected, so a
  detector maps the signal to the corrective and fires it.
- **The dead middle** — a meta-selector that second-guesses a capable client
  (`auto`/`meta`). The data already returned its verdict. Don't rebuild it.

### 2. Mechanism — *how* to deliver a chosen corrective. The server's legit job

The client says "verify this"; the **server** decides *how*: a deterministic symbolic
check (§B) when the claim is executable-checkable, or the adversarial LLM critic when
it's a judgment claim; which **model tier** (cheap vs capable, §I); one-shot vs
multi-round debate. This routing earns its keep because the server knows things the
client doesn't — whether a claim is checkable, what each path costs. *This* is the
routing worth building.

### 3. Escalation — cascades. Keep it, but on a **reliable** signal

Try cheap first, escalate on the cheap result's own uncertainty. It's the most robust
pattern because **you don't predict difficulty upfront** — the cheap attempt's signal
drives it ([cascade survey](https://arxiv.org/html/2603.04445v2)). `confidence_route`
was the least-bad of the four old routers; cascades are its principled form. **Hard
caveat:** self-reported/verbal confidence is **poorly calibrated**
([confidence tokens](https://arxiv.org/pdf/2410.13284)) — so escalate on
**ensemble-agreement / consistency**, not the model saying "I'm unsure."

### 4. Composition — pipelines. An orchestration-boundary question

A hard problem may need *diverge → verify → decide* in sequence. Does the server own
that pipeline, or does the client chain the steps? This is the orchestration-boundary
decision ([`NEXT_REASONING_SERVER.md`](NEXT_REASONING_SERVER.md) §5). If the server
owns workflows, it routes known *problem-shapes* through fixed pipelines; otherwise
the client composes. Either way it's *not* a per-call meta-router.

## Signals and their reliability (the hard part)

Every routing decision needs a signal, and the signals are unreliable in known ways:

| Signal | Use | Reliability |
|---|---|---|
| **Checkable-ness** (can it become code?) | → symbolic vs LLM (mechanism) | **deterministic — prefer it** |
| **Action risk / type** | → watchdog gate (§K) | rule-based, reliable |
| **Ensemble agreement / consistency** | → escalation, confidence | decent (but consistency ≠ correctness) |
| **Query difficulty heuristics** (length, rarity, complexity) | → model-tier routing | rough |
| **Self-reported confidence** | — | **poorly calibrated — do not route on it** |

Principle: **prefer deterministic routing signals (checkable-ness, action-type) and
ensemble signals over the model's self-report.**

## The synthesis

Corrective-selection is itself a *decision-under-options* (the **Converge** failure
mode) and a *meta-reasoning* task — both things the model is unreliable at on itself.
That unreliability tempts you to build a server-side router. But the router is **also**
unreliable (self-routing is miscalibrated) **and** historically unused (the dead four).
The resolution is to stop trying to *route* and instead:

- **Reduce** the menu (retrieve the relevant few correctives; keep the surface small),
- **Detect** when the model can't ask (watchdog: signal → corrective),
- **Escalate** on a *reliable* signal (cascade on ensemble agreement, not verbal
  confidence),
- and pick **mechanism, not corrective** (symbolic vs LLM, model tier).

The "which corrective, when" problem **dissolves** into four smaller problems with
clear owners — and the one big centralized meta-router, the thing the old server built
four times, is not one of them.

## Open questions

- **Tool-set reduction**: retrieving the relevant K correctives — what's the right K,
  and does *retrieving tools* inherit its own lost-in-the-middle?
- **Escalation cost**: ensemble agreement needs N samples; when is that worth it
  versus just calling the capable model directly?
- **Composition**: which problem-shapes deserve a fixed server pipeline vs client
  chaining?
- **The watchdog's map**: should signal → corrective be hand-specified or learned —
  and if learned, on what (we have almost no chain data, per the usage section)?
