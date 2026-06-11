# Design Note — A New Reasoning Server (greenfield)

**Status:** Proposal / north-star. **Scope:** a *new* MCP reasoning server built
from scratch, informed by operating `mcp-reasoning`. This is **not** a v2 or a
refactor plan — it records the load-bearing decisions to make differently the
next time, and the few things worth carrying over unchanged.

## Why build new instead of evolving

`mcp-reasoning` works (35 tools, ~64k lines of production Rust, 2,895+ tests).
But a large share of those lines is **scaffolding to cope with two early
choices**:

1. Parsing free-form LLM text into strict per-mode types.
2. Letting the server modify itself at runtime.

Almost everything that feels heavy — the JSON-extraction fallbacks, the
parse/schema defect detection, the 4-phase self-improvement loop, the self-heal
propose-PR pipeline, the circuit breakers and allowlists guarding them — exists
to survive those two choices. A clean slate that avoids both removes most of the
accidental complexity. The user-visible capability (structured reasoning tools
over an LLM, with memory and observability) is worth keeping; the machinery
around it mostly is not.

## North star

A **thin, fast, stateless-by-default prompt executor**. The model does the
reasoning under a *constrained* output contract; the server does orchestration,
persistence, observability, and safety. Anything that can run offline (analysis,
tuning, "improvement") runs offline. The hot path stays simple enough that it
rarely needs guards wrapped around it.

## Why Claude calls these tools (the value model)

This is the foundation the rest of the design rests on, and it is easy to get
wrong. When Claude calls a reasoning tool, **Claude is calling Claude** — the
server's "reasoning" is just another LLM call with a methodology-specific prompt.
So the only durable value is whatever a *separate, templated* call provides that
in-context thinking does not. Ranked, most real first:

1. **Context separation / independence (strongest).** A separate call has a fresh
   context. That is what lets N divergent perspectives avoid contaminating each
   other, an adversarial critic stay unanchored by the reasoning it judges, and M
   search candidates be evaluated independently. This is genuinely hard to do in
   one context, and the client cannot replicate it by "thinking harder."
2. **Methodology as a callable (strong).** Pearl's ladder, TOPSIS, MCTS/UCB1,
   bias taxonomies — applied rigorously. The model *can* do these but won't
   reliably without the scaffold; the server installs disciplined methods.
3. **Offloaded budget + clean client context (real, smaller).** Heavy reasoning
   spends a separate token budget and returns something compact. Overlaps with
   the client's own extended thinking.
4. **Memory / resumption (situational).** Valuable when used; clients rarely
   thread session ids reliably.

**Not** real value, and to be shed: raw reasoning horsepower (the client has it),
and the `next_tools` / timing / preset machinery that second-guesses a client
already good at planning.

> **Corrected against real usage (see "What the usage data says" below).** The
> ranking above was partly wrong. A single clean structured step (`linear`) is the
> *most-used* organic tool, and the methodology bucket dominates — so "offloaded
> budget + a clean structured step" (which this note had ranked third and
> "smaller") and "methodology" are what's actually reached for, while pure
> independence (`divergent`) is modest. So: **do not shed `linear`** — promote it
> to a first-class *Step* primitive — and hold "independence is the strongest
> value" loosely. The genuine dead weight is elsewhere (agents/team, `relate`,
> graph's heavy half), found only by looking at the data.

Selection mechanics matter here too: Claude picks a tool from its **description**
("use instead of X when Y"). The descriptions are the real product surface — they
install methodology into the client's tool-selection — so they are a first-class,
versioned, *tested* asset (eval that the right tool is selected for representative
problems), not boilerplate.

### The value-driven taxonomy

Organize the tool surface by *why* you would offload at all, not by reasoning
style. Each primitive is justified by one specific value that in-context thinking
cannot supply:

| Primitive | What it does | Why offload | Organic use |
|---|---|---|---|
| **Step** | one externalized, structured reasoning step | a clean step without spending the client's own context | **highest** (`linear` #1) |
| **Decide** | methodology frameworks (weighted / TOPSIS / causal / evidence) | rigor — a disciplined method applied reliably | **high** (`decision`+`evidence`+`counterfactual`+`timeline` ≈ 116) |
| **Verify** | adversarial, unanchored critique | separation — a critic must not be anchored | medium (`detect`+`reflection` ≈ 52) |
| **Search** | parallel independent evaluation of candidates | parallelism — many independent judgments | medium (`mcts`/`tree`/`graph`) |
| **Diverge** | N independent perspectives | separation — views must not contaminate | modest (`divergent` 19) |
| **Recall** | session memory / resumption | continuity — state across turns | low when manual; make it effortless |

Six primitives, each earning its existence — versus 13 overlapping modes and four
routers. The "Organic use" column is the proxy usage data below: it **reorders**
the value model (Step and Decide lead; independence is modest) but **keeps every
primitive** — none is dead. And **parallelism becomes a server primitive**: one
call fans out N *independent* sub-calls and synthesizes, instead of the client
driving a loop (today's `mcts` / `graph`). That fan-out-and-synthesize is
precisely the work the client cannot do cheaply itself.

### What the usage data says

Pulled from the live `mcp-reasoning` SQLite (`thoughts`/`branches`), ~271 organic
sessions over 5.5 months, with this session's ~500 synthetic `linear` calls
filtered out. **Caveat:** the `metrics`/`invocations` tables were never persisted,
so this is a *thoughts-produced proxy*, not a call log, over modest dev/eval-heavy
data — trust the **zeros and relative magnitudes**, not absolute counts.

- **`linear` is the #1 organic tool (67).** The cheap structured step is the
  workhorse, not low-value. (This is what corrected the ranking above.)
- **The methodology bucket dominates** (`decision` 66, `evidence` 20,
  `counterfactual` 19, `timeline` 11). "Methodology as a callable" is the most
  validated value.
- **`verify` and `search` have healthy use** (`detect` 31, `reflection` 21,
  `mcts` 32, `tree` 51 branches) — both buckets earn their place.
- **Provably dead weight:** the **agent/team tools — 0 invocations in 5.5 months**
  (7 of 35 tools); **`relate` — 0**; **`graph` — 16 nodes, 0 edges** (its
  aggregate/refine/prune machinery, i.e. the reason graph is heavyweight, never
  fired). These — not `linear` — are the real cuts.
- **The chain/transition machinery is unmeasurable:** the suggestion engine
  tracks tool transitions but **never persists them**, so its value can't be
  evaluated from data — itself an argument to cut it.

## Lessons that drive the design (the evidence)

- The self-heal PR pipeline + the SI loop exist **largely to fix the server's
  own recurring parse/schema failures** → the free-text→strict-JSON contract is
  brittle, and we built a self-modifying repair loop to compensate.
- `extract_json` with raw / ```json / balanced-brace fallbacks, plus
  parse-failure and schema-violation metrics and a defect log → same root cause.
- Circuit breakers, allowlists, and baselines exist **because** the system
  mutates itself at runtime (we watched the breaker trip open under load).
- Four meta-routers (`auto`, `meta`, `confidence_route`, `preset`) plus ~13
  overlapping reasoning modes → routing and capability sprawl.
- ~40k lines of tests (103k with tests vs 64k prod), much of it mock-response
  wiring → a coverage bar that chases lines on glue.
- Observability was bolted on after: token usage was never recorded in the
  metric event, and the dashboard is a separate concern fighting the server for
  a port → it should be designed in from day one.

## Architecture decisions

### 1. Constrained output is the core contract (biggest lever)

Every mode declares an output JSON Schema, and the model is **forced** to it via
the provider's structured-output / tool-use feature. No free-text parsing, no
extraction fallbacks, no self-heal-for-parsing. Keep a thin schema validator for
defense-in-depth, but the happy path is schema-guaranteed by the API.

> Removes: `extract_json` and its fallbacks, parse/schema defect detection, and
> the entire "heal my own parsers" rationale for the self-improvement/self-heal
> subsystems.

### 2. Improvement is offline

The server emits structured metrics and traces. A **separate offline job** reads
them, proposes threshold/prompt changes, and a human applies them. No runtime
self-mutation.

> Removes from the hot path: the SI manager/executor/learner, `heal_manager`,
> `heal_cycle`, `repair`, the circuit breaker, and the allowlist — none are
> needed when the server never changes itself while running.

### 3. Modes are data, not three files of code each

A mode is `{ id, prompt template, thinking budget, output schema, routing
hints }`. One generic executor runs any mode. Adding a mode is a registry entry,
not new files across `prompts/`, `modes/`, and `handlers_*`. The few modes with
real algorithmic bodies (e.g. MCTS scoring, graph aggregation) register as
**plugins** — code behind the same interface — rather than forcing everything
into either pure data or pure code.

### 4. A few orthogonal primitives + one router

Expose the five value-driven primitives — **diverge, verify, search, decide,
recall** (see the value model above) — not 13 modes with heavy overlap. Each is
justified by a *specific* value in-context thinking cannot supply, so there is no
redundancy to route around. Selection is the tool descriptions doing their job;
if an explicit router is needed at all, it is **one** thin selection/escalation
layer replacing `auto` + `meta` + `confidence_route` + `preset` — not a
second-guessing suggestion engine. Validate the cut against `mcp-reasoning`'s
existing tool-usage and chain data before committing to it.

### 5. Explicit orchestration boundary

Decide up front and hold the line: the server owns multi-step workflows as
first-class operations (with the streaming progress it already does well), and
the client just starts one and watches. No mixed model where some tools are
client-driven loops (today's `mcts`/`graph`, where the client re-calls and feeds
results back) and others are server-owned (presets).

### 6. Observability designed in, not bolted on

- The metric/event record carries **tokens, model, cost, latency, and session**
  from the first commit.
- Activity events publish to a **shared sink** (a localhost UDP port, a named
  pipe, or an append-only SQLite table) that a **separate dashboard binary**
  aggregates. Multiple server instances → one dashboard, no port collisions
  between servers, and the dashboard survives server restarts. (Full rationale,
  learned the hard way: [`DASHBOARD.md`](DASHBOARD.md) and the "build it better"
  notes that followed it.)
- A first-class `--demo`/replay mode emits synthetic-but-realistic activity
  (including heal/SI events) so the dashboard is demonstrable without a live
  client, real API spend, or inducing real defects.

### 7. Right-sized testing

Gate coverage **hard on the invariants** — schema validators, safety guards,
storage, redaction — and stop mandating 95–100% on glue. Property-test the
validators, integration-test the seams, and skip the exhaustive mock-response
unit tests that make up much of today's test mass.

## What we carry over (earned — keep it)

- `ModeCore`-style **composition over trait inheritance**.
- **Trait-based mockability** (storage / client / time) so the whole thing tests
  without network access.
- `#![forbid(unsafe_code)]` and **no `unwrap`/`expect` in production**.
- **Structured `tracing` to stderr only**; the stdio JSON-RPC channel is never
  disturbed.
- **stdio transport** and **SQLite** persistence.
- The **semantic-memory** idea (embeddings + rerank for session recall) — but as
  an optional capability, not part of the core.

## What stops existing (the payoff)

Subsystems this design removes or shrinks toward zero: `extract_json` fallbacks;
parse/schema defect detection; the self-heal propose pipeline (`heal_manager`,
`heal_cycle`, `repair`, `eligibility`, `heal_review`); the runtime SI loop's
executor / circuit-breaker / allowlist / baseline; the four meta-routers; most of
the per-mode code triplication; and a large fraction of the mock-response test
wiring.

**Cut with hard usage evidence (not taste):** the **agent/team tools** (7 of 35,
**0 invocations in 5.5 months**); **`relate`** (0); **`graph`'s heavy half** —
aggregate/refine/prune (0 edges ever written); and the **chain/transition
suggestion engine**, whose value is unmeasurable because it never persisted what
it tracked. The hypothesis — to be validated, not promised — is a server
delivering the same user-visible capability in a small fraction of the lines.

## Beyond reasoning: other things worth offloading

The value model generalizes. The server is, underneath, a **parallel /
independent-compute + external-tool orchestrator that returns compact, verified
results** — reasoning modes are one instance. The general rule: *offload anything
that needs many independent or parallel LLM/tool calls and hands back something
small.* That opens capabilities the current server doesn't have, ranked by
leverage:

1. **Research & synthesis (biggest new win).** Fan out web searches → fetch →
   extract claims → adversarially verify → synthesize a compact, **cited** report.
   The client genuinely cannot run 15 parallel fetch-and-verify cycles in its own
   context without blowing it up; the server can, on a separate budget, and return
   a page. Exploits parallelism + budget + verification at once. (Requires the
   server to host search/fetch tools.)
2. **Adversarial verification / red-team.** "Is this claim true / what's the
   strongest counter / find the flaw in this plan, proof, or diff" — run by N
   independent critics each prompted to *refute*, with majority voting. In-context
   self-critique is anchored; independent unanchored critics are not. This sharpens
   today's `detect`/`reflection` into a parallel **Verify** primitive and is where
   the strongest value (independence) actually pays off.
3. **Self-consistency / ensemble.** For high-stakes or noisy questions, sample the
   same problem N *independent* times and return the consensus **plus a
   disagreement measure** (a real calibration signal). The client can't cheaply
   sample itself N independent times; the answer comes back with confidence
   grounded in agreement, not vibes.
4. **Durable cross-session memory — the one underused thing worth *investing* in,
   not cutting.** The semantic-memory infrastructure already exists, but `relate`
   is dead (0 uses) because recall is *manual*. Make it **effortless** — the server
   surfaces relevant prior decisions/lessons automatically — and "Recall" becomes
   high-leverage instead of unused. The client's context is ephemeral; the server
   remembering across sessions is a capability it structurally lacks.
5. **Critique-and-revise (actor–critic) loops.** Generate → independent critique →
   revise, K rounds, critic in a separate context. Good for drafts, plans, diffs.
   The old "crew" path gestured at this; make it a clean primitive.
6. **Estimation & calibration / structured extraction at scale** — Fermi/base-rate
   forecasts with a disciplined method + ensemble interval (methodology + ensemble);
   and bulk field-extraction/triage over large inputs (pure budget + parallelism
   offload that keeps the client's context clean). Lower priority, same model.

**What not to chase:** general agent/team orchestration — the data says it's dead
(0 uses). Don't rebuild it; if multi-agent shows up, it's the *fan-out-and-verify*
primitive above, not a standing agent framework.

## Open questions / risks

- **Constrained output portability:** not every model/provider supports forced
  schemas equally. Need a fallback that **degrades to validated free-text, never
  back to self-heal**.
- **Losing in-production self-tuning:** offline improvement gives up runtime
  adaptation. Likely fine — it was low-ROI and high-risk — but call it out.
- **How much per-mode logic is truly data vs code:** some modes have real
  algorithmic bodies; those stay code-as-plugins. Don't force them into config.
- **Usage data is a proxy, not ground truth.** The primitive set was checked
  against `mcp-reasoning`'s `thoughts`/`branches` history, but `metrics`/
  `invocations`/transitions were never persisted, and the data is dev/eval-heavy.
  Before committing, instrument a real call log + chain data in the new server and
  re-validate the cuts (especially "promote Step", "agents are dead") against
  production traffic.

## Out of scope for this note

Naming, repo layout, and language (Rust stays the default given the keepers
above). This is about the load-bearing architecture, not the bikeshed.
