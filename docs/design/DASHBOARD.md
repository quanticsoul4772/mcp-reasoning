# Design — Real-time E2E Activity Dashboard

**Status:** Proposal. **Author:** design pass (sc:design). **Backs:**
`claudedocs/research_e2e_dashboard_2026-06-11.md`.

Turn the [Flow Overview](../reference/FLOW_OVERVIEW.md) into a **live** view: an
operator runs the server, opens the dashboard, and watches each request flow
through the e2e path in real time, plus the three background loops firing —
seeing *what* happens and *when*.

## Goals / non-goals

**Goals.** Live animation of the e2e flow on real activity; an event timeline
("what just happened, when"); core metrics (latency, success, tokens, tool
chains, SI/heal status); **single self-contained binary**; **off by default**,
zero cost when disabled; **never disturb the stdio MCP channel**.

**Non-goals (v1).** Not a control plane (read-only); not a production HTTP MCP
transport; not a full APM (deep distributed tracing → optional OTLP export later);
no auth/multi-tenant (loopback dev tool).

## 1. Architecture

```
                 ┌──────────────────────── mcp-reasoning binary ───────────────────────┐
   stdin/stdout  │  StdioTransport ──► McpServer ──► handlers ──► modes (ModeCore)      │
   (MCP JSON-RPC)│        │                                  │           │              │
                 │        │   emit(ActivityEvent) at seams ──┼───────────┘              │
                 │        ▼                                  ▼                          │
                 │   activity_tx: broadcast::Sender<ActivityEvent>  ◄── tracing Layer   │
                 │        │                                                             │
                 │   (only when MCP_DASHBOARD set)                                      │
                 │        ▼                                                             │
                 │   dashboard sidecar: axum @ 127.0.0.1:PORT                           │
                 │     GET /            → embedded React Flow SPA (rust-embed)          │
                 │     GET /events      → SSE stream of ActivityEvent                   │
                 │     GET /metrics     → JSON snapshot (MetricsCollector)              │
                 └─────────────────────────────────────────────────────────────────────┘
                            ▲ browser: EventSource('/events') → animate nodes/edges
```

### Rust modules (new `src/dashboard/`)

- `dashboard/event.rs` — `ActivityEvent` + `Node`/`EdgeId`/`Phase` enums; `emit()`
  helper that does a best-effort `activity_tx.send` (drop on no-subscribers, never
  blocks, never errors up).
- `dashboard/bus.rs` — `ActivityBus` wrapping `broadcast::Sender<ActivityEvent>`
  (buffer ~256). Lives in `AppState` alongside the existing `progress_tx`.
- `dashboard/layer.rs` — a `tracing_subscriber::Layer` that turns annotated spans
  (`#[instrument(fields(dash.node = ...))]`) into `ActivityEvent`s — auto-capture
  for stages that already have spans, so we don't hand-place every `emit()`.
- `dashboard/server.rs` — the axum app: `/events` (SSE via
  `BroadcastStream::new(activity_tx.subscribe())`), `/metrics`, embedded SPA.
- `dashboard/assets.rs` — `#[derive(rust_embed::Embed)]` over `dashboard/ui/dist/`.
- Spawn site: `server/mcp.rs`, mirroring the self-heal block —
  `if dashboard_config.enabled { tokio::spawn(serve(addr, state.activity_tx.clone(), state.metrics.clone())) }`.

### Frontend (`src/dashboard/ui/`, React + Vite)

- `FlowCanvas` — React Flow graph; nodes/edges mirror `FLOW_OVERVIEW` (same role
  colors). Subscribes to the event stream; on each event, pulses the matching edge
  (`animated` toggled briefly) and flashes the node, with a live per-node counter.
- `Timeline` — reverse-chronological scrolling list of events (ts · node · phase ·
  tool · duration), filterable by session/node/phase.
- `Metrics` — small panels (recharts): p50/p95 latency, success rate, tokens/min,
  top tool-chain transitions, SI phase + circuit-breaker state, heal: defects /
  held-back / proposed.
- `useEventStream()` — `EventSource('/events')` hook with auto-reconnect + a bounded
  ring buffer.

## 2. `ActivityEvent` schema + API

```rust
// dashboard/event.rs
pub struct ActivityEvent {
    pub id: u64,                 // monotonic seq (AtomicU64)
    pub ts_ms: i64,              // epoch millis (TimeProvider, not Instant)
    pub session_id: Option<String>,
    pub node: Node,              // which box lit up
    pub edge: Option<EdgeId>,    // src→dst, for edge animation
    pub phase: Phase,            // Started | Progress | Completed | Failed | HeldBack
    pub tool: Option<String>,    // e.g. "reasoning_mcts"
    pub model: Option<String>,
    pub duration_ms: Option<u64>,
    pub bytes: Option<u64>,
    pub note: Option<String>,    // short, redacted — never raw prompts/keys
}
pub enum Node { Client, Registry, Mode, Anthropic, Sqlite, Voyage, Worker, Si, Heal, Github }
```

Serialized as JSON. **Redaction:** `note` is a short label only — reuse the heal
module's `redact()` discipline; never emit prompts, completions, or secrets.

### HTTP API (loopback only)

| Method · Path | Returns |
|---|---|
| `GET /` | embedded SPA (`index.html`, SPA fallback) |
| `GET /events` | `text/event-stream` — one `data: <ActivityEvent json>` per event; `:` heartbeat every 15s; `Last-Event-ID` resume from the ring buffer |
| `GET /metrics` | JSON snapshot from `MetricsCollector` + SI/heal status + a recent-events tail |
| `GET /health` | `200 ok` (sidecar liveness) |

No write endpoints in v1.

## 3. Event production per e2e stage (no stdio impact)

All `emit()`s go to `activity_tx` (in-memory broadcast) — **nothing touches
stdout**; this composes with the existing stderr `tracing` layer. Seams:

- **Request spine** (matches FLOW_OVERVIEW ①–⑤): at the tool boundary
  (`server/tools/` handlers / `progress_bridge::with_progress`): `Registry Started`
  on dispatch; `Mode Started` when the handler enters the mode; `Sqlite`/`Anthropic`
  on `ModeCore` reads/calls (wrap `complete()` and storage reads); `Mode Completed`/
  `Failed` on return with `duration_ms`. The **existing `ProgressReporter`
  milestones** (5/15/90/100%) map straight to `Anthropic Progress` events — reuse
  the progress bus, don't duplicate.
- **Embedding worker → Voyage:** `embed_worker` already drains the queue on an
  interval — `emit(Worker Started)` on dequeue, `Voyage Completed` after embed/rerank.
- **Self-improvement cycle:** `run_cycle` emits one event per phase
  (`Si Started/Progress` Monitor→Analyze→Execute→Learn) + circuit-breaker trips.
- **Self-heal loop:** `HealManager::tick` emits per recurring defect —
  `Heal Started`, then `HeldBack` / `Github Completed` (PR opened) / `Failed`,
  carrying the `held_back_reason`. This makes the OFF-by-default loop visible the
  moment it's enabled.

Auto-capture: annotate the existing spans with `#[instrument(fields(dash.node = "mode"))]`
so `dashboard/layer.rs` derives events without manual `emit()` at every call.

## 4. Node/edge → event mapping (drives the live diagram)

| Event (`node`, `edge`, `phase`) | UI effect |
|---|---|
| `Registry`, `client→registry`, Started | pulse the `tool call` edge; flash Registry |
| `Mode`, `registry→mode`, Started | pulse registry→mode |
| `Anthropic`, `mode↔anthropic`, Progress | pulse the ②③ edge per milestone (5→15→90→100%) |
| `Sqlite`, `mode↔sqlite`, Completed | pulse the ①④ edge; bump SQLite write counter |
| `Mode`, `mode→client`, Completed | pulse the ⑤ response edge (bold); record latency |
| `Worker`/`Voyage`, dotted | pulse the warming loop |
| `Si`, dotted | advance a 4-phase ring on the SI node |
| `Heal`/`Github` | flash heal (red) → github; show reason on hover |

The frontend keeps the FLOW_OVERVIEW node ids stable so events address nodes
directly.

## 5. Gating / safety

- **Off by default.** `DashboardConfig::from_env()` (mirrors `SelfImprovementConfig`):
  `MCP_DASHBOARD` (bool, default false) · `MCP_DASHBOARD_ADDR` (default
  `127.0.0.1:3777`). When unset: **no listener, no axum, no overhead** — the
  `activity_tx` send is a cheap no-op with zero subscribers.
- **Loopback only** by default; binding a non-loopback addr requires an explicit
  opt-in env and logs a warning (it's an unauthenticated dev tool).
- **Read-only** in v1 (no control endpoints).
- **Redaction** on every `note`; no prompts/completions/keys ever leave the process.
- **Conventions:** no `unwrap`/`expect` in the sidecar; `emit` is infallible
  (ignores send errors); the tracing layer never panics.

## 6. Phased plan

- **Phase 0 — plumbing.** `dashboard/event.rs` + `bus.rs`; add `activity_tx` to
  `AppState`; `DashboardConfig::from_env`; `emit()` no-op-safe. Tests: schema
  round-trips; emit with no subscribers is a no-op.
- **Phase 1 — MVP (spine + timeline).** axum sidecar with `/events` + embedded
  minimal SPA (FlowCanvas + Timeline). Emit the request-spine events (reuse progress
  milestones). Animate ①–⑤; scrolling timeline. Gated, loopback. *Deliverable: watch
  a request flow live.*
- **Phase 2 — loops + metrics.** Emit worker / SI / heal events; add `/metrics` +
  the metric panels (latency, success, tokens, chains, SI/heal). *Deliverable:
  background loops visible; the OFF-by-default heal loop lights up when enabled.*
- **Phase 3 — polish + (optional) controls.** Filters, session focus, resume via
  `Last-Event-ID`; *optional* WebSocket + a couple of write actions (trigger SI
  cycle, pause a loop) behind a second explicit flag. *Optional* OTLP export for
  deep tracing.

## 7. Risks / tradeoffs

- **New deps** (`axum`, `tower-http`, `rust-embed`, a frontend toolchain). Mitigate:
  feature-gate the sidecar (`--features dashboard`) so the default build/binary is
  unchanged for users who don't want it; the UI build is a separate step.
- **Broadcast lag/backpressure.** A slow browser shouldn't slow the server —
  `broadcast` drops oldest for lagging receivers; `emit` never blocks. Acceptable for
  a dashboard (lossy is fine; the timeline shows "N dropped").
- **Event volume.** High request rates → many events. Mitigate: per-node coalescing
  on the client; sampling for `Progress` events; the ring buffer is bounded.
- **Redaction discipline** is the main correctness risk — enforce that `note` is
  label-only in code review and a test that scans emitted events for key/prompt
  patterns.
- **Frontend build in a Rust repo** adds CI surface. Mitigate: commit the built
  `dist/` (like the SVG) or build in CI under the `dashboard` feature only.

## Dependencies & repo fit

- Reuses: `AppState.progress_tx`, `MetricsCollector`, `DefectLog`, the env-gated
  `tokio::spawn` pattern in `mcp.rs`, the heal `redact()`, `TimeProvider`.
- Adds (behind `dashboard` feature): `axum`, `tower-http` (fs/compression),
  `rust-embed`; React + React Flow + Vite under `src/dashboard/ui/`.
- Single binary preserved; stdio MCP path untouched; off by default like
  `SELF_HEAL_*`.
