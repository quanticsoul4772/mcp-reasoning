# End-to-End Flow

A detailed, source-grounded map of how a request travels through the MCP
Reasoning Server, and how the two background loops (self-improvement and
self-heal) operate alongside it. Diagrams are [Mermaid](https://mermaid.js.org/)
and render on GitHub.

Contents:

1. [System overview](#1-system-overview)
2. [Tool-call request lifecycle](#2-tool-call-request-lifecycle)
3. [Anthropic client: retry, thinking budgets, streaming](#3-anthropic-client-retry-thinking-budgets-streaming)
4. [Streaming milestone progress over MCP](#4-streaming-milestone-progress-over-mcp)
5. [Semantic memory (Voyage)](#5-semantic-memory-voyage)
6. [Self-improvement loop](#6-self-improvement-loop)
7. [Self-heal propose-PR pipeline](#7-self-heal-propose-pr-pipeline)
8. [Storage / data model](#8-storage--data-model)

---

## 1. System overview

The server speaks MCP over stdio (default) or HTTP, calls the Anthropic Claude
API for reasoning, optionally calls Voyage AI for semantic memory, and persists
everything to SQLite. Two background tasks run on intervals: the embedding
worker (warms memory) and — when explicitly enabled — the self-heal propose loop.

```mermaid
flowchart TB
    Client["MCP Client<br/>(Claude Code / Desktop)"]

    subgraph Server["MCP Reasoning Server (Rust)"]
        direction TB
        Transport["Transport<br/>stdio | HTTP"]
        RPC["JSON-RPC / rmcp<br/>tools/list, tools/call"]
        Router["Tool Registry<br/>35 tools → handlers"]
        Modes["Reasoning Modes<br/>linear, tree, graph, mcts, ..."]
        State["AppState<br/>progress broadcast bus"]
        subgraph BG["Background tasks"]
            EmbedWorker["Embedding worker<br/>(drains embedding_queue)"]
            HealLoop["Self-heal propose loop<br/>(OFF by default)"]
            SILoop["Self-improvement cycle"]
        end
    end

    Anthropic["Anthropic Claude API<br/>reasoning + thinking"]
    Voyage["Voyage AI<br/>embeddings + rerank"]
    SQLite[("SQLite<br/>sessions, thoughts, graph,<br/>metrics, embeddings, SI/heal")]

    Client <-->|"stdin/stdout"| Transport
    Transport --> RPC --> Router --> Modes
    Modes -->|"prompt + thinking budget"| Anthropic
    Modes --> SQLite
    Modes -.->|"milestones"| State
    State -.->|"notifications/progress"| Client
    EmbedWorker --> Voyage
    EmbedWorker --> SQLite
    SILoop --> SQLite
    HealLoop -->|"cargo/git/gh"| GH["GitHub PR<br/>(operator review, never merged)"]
```

---

## 2. Tool-call request lifecycle

Every `tools/call` follows the same spine: decode → route → run the mode (which
composes storage + the Anthropic client via `ModeCore`) → extract structured
JSON from the model output → enrich with metadata/next-tool suggestions →
respond. Request-size limits are enforced before any model call.

```mermaid
sequenceDiagram
    participant C as MCP Client
    participant T as Transport (stdio/http)
    participant R as rmcp router
    participant H as Handler (handlers_*)
    participant M as Mode (ModeCore)
    participant S as SqliteStorage
    participant A as AnthropicClient

    C->>T: tools/call {name, args, _meta}
    T->>R: JSON-RPC request
    R->>R: validate size (≤100KB, ≤50 msgs)
    R->>H: dispatch by tool name
    H->>M: build request, load session context
    M->>S: read prior thoughts / mode state
    M->>M: select prompt + thinking budget
    M->>A: complete(messages, config)
    A-->>M: completion (text, usage)
    M->>M: extract_json (raw JSON → fenced block → error)
    M->>S: persist thought / branch / graph / checkpoint
    M-->>H: structured Response
    H->>H: attach metadata + next-tool suggestions
    H-->>R: Response
    R-->>T: JSON-RPC result
    T-->>C: result
```

Key guards on this path:

- **Size limits**: `MAX_REQUEST_BYTES` 100KB, `MAX_MESSAGES` 50,
  `MAX_CONTENT_LENGTH` 50KB/message — rejected before the model call.
- **No panics**: production paths never `unwrap()`/`expect()`; failures return a
  typed `ModeError`/`AppError`.
- **JSON extraction** is tolerant: fast path raw JSON → fenced `json` block →
  clear error with a truncated preview.

---

## 3. Anthropic client: retry, thinking budgets, streaming

`AnthropicClient` wraps the HTTP call with bounded retries + backoff, selects an
extended-thinking budget per mode, and can stream Server-Sent Events,
accumulating them into a final response while emitting milestones.

```mermaid
flowchart TD
    Start["Mode calls complete()/complete_streaming()"] --> Budget{"Thinking budget<br/>for this mode?"}
    Budget -->|"linear/tree/auto/checkpoint"| None["None (fast)"]
    Budget -->|"graph"| Std["Standard 4096"]
    Budget -->|"divergent/reflection/decision/evidence"| Deep["Deep 8192"]
    Budget -->|"counterfactual/mcts"| Max["Maximum 16384"]

    None --> Build
    Std --> Build
    Deep --> Build
    Max --> Build["Build request"]

    Build --> Stream{"Streaming?"}
    Stream -->|"no"| Post["POST /messages"]
    Stream -->|"yes"| SSE["POST (SSE)<br/>parse events → StreamAccumulator"]
    SSE -.->|"report_milestone"| Bus["progress_tx broadcast"]

    Post --> Resp{"2xx?"}
    SSE --> Resp
    Resp -->|"yes"| Done["CompletionResponse"]
    Resp -->|"429 / 5xx / timeout"| Retry{"retries left?<br/>(MAX_RETRIES)"}
    Retry -->|"yes"| Backoff["exponential backoff"] --> Build
    Retry -->|"no"| Err["AnthropicError → ModeError"]
```

---

## 4. Streaming milestone progress over MCP

Modes emit milestones into a broadcast bus without depending on rmcp. At the tool
boundary, the `progress_bridge` forwards a call's milestones to the client as
`notifications/progress` — but only when the client opted in with a progress
token in the request `_meta`. Each call is correlated by a unique token so
concurrent calls never leak each other's progress.

```mermaid
flowchart LR
    subgraph Mode["Streaming mode (divergent/reflection/mcts/counterfactual)"]
        RM["ProgressReporter.report_milestone()<br/>5% → 15% → 90% → 100%"]
    end
    RM -->|"ProgressEvent{token, percent, msg}"| TX["progress_tx (broadcast)"]

    subgraph Boundary["Tool method"]
        Meta{"client sent<br/>progress token?"}
        WP["with_progress: subscribe + select loop"]
    end

    TX --> WP
    Meta -->|"no"| Passthrough["run handler, send nothing"]
    Meta -->|"yes"| WP
    WP -->|"ev.token == this call"| Notify["peer.notify_progress()"]
    WP -->|"other call's token"| Ignore["ignore"]
    Notify --> Client["MCP client receives<br/>notifications/progress"]
    WP -->|"on completion"| Drain["drain final 100% tick"] --> Client
```

---

## 5. Semantic memory (Voyage)

`reasoning_search` and `reasoning_relate` (and divergent's novelty scoring)
require `VOYAGE_API_KEY` — there is no keyword fallback. Embeddings are cached in
`session_embeddings`, keyed on a content hash **and** the model. A background
worker warms the cache so the first search/relate after a write is ready.

```mermaid
flowchart TB
    subgraph Write["Write path (warming)"]
        TW["Thought write"] --> EQ["enqueue session → embedding_queue"]
        EW["embed_worker (interval)"] --> DQ["dequeue"]
        DQ --> EMB["Voyage /embeddings (voyage-4)"]
        EMB --> Cache[("session_embeddings<br/>{model, dtype, dim, vector}")]
    end

    subgraph Read["Read path"]
        Q["reasoning_search(query)"] --> Key{"VOYAGE_API_KEY?"}
        Key -->|"unset"| CfgErr["clear config error"]
        Key -->|"set"| QE["embed query"]
        QE --> Cos["cosine recall over cached vectors"]
        Cos --> Rank["Voyage /rerank (rerank-2.5)"]
        Rank --> Top["top sessions"]
        Cache -.->|"cache hit (hash+model)"| Cos

        RL["reasoning_relate(session)"] --> Edges["cosine + shared-mode + temporal edges"]
        Edges --> BFS["depth-bounded BFS,<br/>capped at MAX_GRAPH_EDGES"]
        BFS --> Graph["relatedness graph"]
    end
```

---

## 6. Self-improvement loop

A 4-phase cycle measures the server's own performance, asks the model to
diagnose regressions, applies bounded parameter changes (or rolls them back), and
rewards measured improvement. Safety mechanisms gate every action.

```mermaid
flowchart LR
    subgraph Cycle["run_cycle"]
        Mon["1. Monitor<br/>collect success/latency/baseline<br/>+ low-success tool transitions"]
        Ana["2. Analyze<br/>LLM diagnosis of regressions"]
        Exe["3. Execute<br/>apply ThresholdAdjust / override"]
        Lrn["4. Learn<br/>reward = measured Δ gated on MDE"]
        Mon --> Ana --> Exe --> Lrn --> Mon
    end

    subgraph Safety["Safety gates"]
        CB["Circuit breaker<br/>halt on consecutive failures"]
        AL["Allowlist<br/>validate action bounds"]
        BL["Baseline tracking"]
        RB["Rollback on regression"]
    end

    Ana -.-> AL
    Exe -.-> CB
    Exe -.-> RB
    Lrn -.-> BL
    Cycle --> Sup["apply self-correcting suppression<br/>of sustained anti-pattern transitions"]
    Sup --> SuggEng["SuggestionEngine hard-blocks them"]
    Cycle --> Store[("SI storage<br/>diagnoses, actions, overrides, stats")]
```

Operator surface: `reasoning_si_status`, `si_diagnoses`, `si_overrides`,
`si_approve`, `si_reject`, `si_trigger`, `si_rollback`.

---

## 7. Self-heal propose-PR pipeline

The server detects its **own** recurring parse/schema defects and — when
explicitly enabled — opens operator-reviewed PRs that fix them. It **never
merges**. Two spec-002 guards (attribution + the validation-invariant guard) keep
it from acting on noise or weakening a correct check.

```mermaid
flowchart TD
    Fail["Tool parse/schema failure"] --> Detect["DefectLog.observe()<br/>per-signature + per-input counts"]
    Detect --> Recur{"recurring?<br/>(≥ threshold)"}
    Recur -->|"no"| Wait["wait"]
    Recur -->|"yes"| Drift{"partition_drift:<br/>broad across components?"}
    Drift -->|"yes"| Alert["route to drift<br/>(alert + record, no patch)"]
    Drift -->|"no"| Elig["classify_eligibility (spec 002)"]

    Elig -->|"model-version spike"| Alert
    Elig -->|"varied inputs"| Hold["HeldBack<br/>(recorded, operator-visible)"]
    Elig -->|"stable path"| Rank["rank_and_cap (≤ max_proposals)"]

    Rank --> Reuse{"known class?<br/>find_reusable_fix"}
    Reuse -->|"yes"| Skip["reuse prior accepted fix"]
    Reuse -->|"no"| Loc["localize (LLM → source_hint)"]

    Loc --> Synth["synthesize_reproducing_test<br/>GATE: must FAIL on base"]
    Synth -->|"not grounded"| Abort["abort, no PR"]
    Synth -->|"grounded"| Fix["generate_and_validate_fix"]

    Fix --> Guard{"invariant guard:<br/>weakens a check?"}
    Guard -->|"yes"| Block["blocked, not admissible, no PR"]
    Guard -->|"no"| Gates{"suite green ∧<br/>quality green?"}
    Gates -->|"no"| NotAdm["not admissible, no PR"]
    Gates -->|"yes"| Adm["is_admissible = grounded ∧ suite ∧ quality ∧ ¬weakens"]
    Adm --> PR["open_pr (gh)<br/>NEVER merges"]
    PR --> Op["operator accept → KnowledgeEntry → reuse"]
```

Gated by env: `SELF_HEAL_PROPOSE_ENABLED=true` **and** `SELF_HEAL_WORKSPACE` set;
`SELF_HEAL_MAX_PROPOSALS` caps PRs per cycle.

---

## 8. Storage / data model

SQLite is the single source of truth. Caches (`session_embeddings`) are derived
data that self-heal on a miss; the embedding queue decouples writes from Voyage.

```mermaid
flowchart LR
    Sessions["sessions"] --> Thoughts["thoughts"]
    Sessions --> Branches["branches"]
    Sessions --> Checkpoints["checkpoints"]
    Sessions --> GraphN["graph nodes/edges"]
    Thoughts --> EQ["embedding_queue"]
    Sessions --> SE["session_embeddings<br/>(content-hash + model)"]
    Metrics["metrics + tool transitions"]
    SIA["SI: diagnoses, actions, overrides, action stats"]
    Heal["heal: fix_proposals, knowledge_entries"]
    AgentM["agent metrics"]
```

---

## Legend

- **Solid arrow** — direct call / data flow.
- **Dotted arrow** — asynchronous / best-effort (milestones, cache hits, safety
  signals).
- **Cylinder** — persistent store. **Diamond** — decision point.

For the component breakdown behind these flows, see
[Architecture](ARCHITECTURE.md); for tool schemas, see
[Tool Reference](TOOL_REFERENCE.md) and [API Specification](API_SPECIFICATION.md).
