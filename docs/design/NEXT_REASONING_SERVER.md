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

Start from the usage data `mcp-reasoning` already has and identify the genuinely
distinct reasoning shapes (roughly: single-pass, branch-and-compare,
decompose-into-graph, search, critique/reflect, decide). Expose those — not 13
modes with heavy overlap — behind **one** selection/escalation layer that
replaces `auto` + `meta` + `confidence_route` + `preset`.

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
wiring. The hypothesis — to be validated, not promised — is a server delivering
the same user-visible capability in a small fraction of the lines.

## Open questions / risks

- **Constrained output portability:** not every model/provider supports forced
  schemas equally. Need a fallback that **degrades to validated free-text, never
  back to self-heal**.
- **Losing in-production self-tuning:** offline improvement gives up runtime
  adaptation. Likely fine — it was low-ROI and high-risk — but call it out.
- **How much per-mode logic is truly data vs code:** some modes have real
  algorithmic bodies; those stay code-as-plugins. Don't force them into config.
- **Which primitives are actually orthogonal** is empirical — derive it from
  `mcp-reasoning`'s existing tool-usage and chain-transition data, not taste.

## Out of scope for this note

Naming, repo layout, and language (Rust stays the default given the keepers
above). This is about the load-bearing architecture, not the bikeshed.
