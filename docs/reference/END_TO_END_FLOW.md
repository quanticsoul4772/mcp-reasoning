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

---

## 1. System overview

The server speaks MCP over stdio, calls the Anthropic Claude API for reasoning,
optionally calls Voyage AI for semantic memory, and persists everything to
SQLite. Background tasks run on intervals: the self-improvement cycle, the
embedding worker (when Voyage is configured), and — only when explicitly
enabled — the self-heal propose loop.

```mermaid
flowchart TB
    Client["MCP Client<br/>(Claude Code / Desktop)"]:::client
    Client -->|"requests (stdio · JSON-RPC)"| Server
    State -.->|"notifications / progress"| Client

    subgraph Server["MCP Reasoning Server (Rust)"]
        direction TB
        Pipeline["Request pipeline<br/>Transport (stdio) → JSON-RPC (rmcp)<br/>→ Tool Registry (35 tools) → Reasoning Modes"]:::proc
        State["AppState<br/>(progress broadcast bus)"]:::proc
        BG["Background tasks<br/>self-improvement cycle · embed worker ·<br/>self-heal propose loop (OFF by default)"]:::proc
    end

    Pipeline -->|"prompt + thinking budget"| Anthropic["Anthropic Claude API<br/>(reasoning + thinking)"]:::ext
    Pipeline --> SQLite[("SQLite<br/>sessions · thoughts · graph ·<br/>metrics · embeddings · SI/heal")]:::store
    BG -->|"embed + rerank"| Voyage["Voyage AI<br/>(semantic memory)"]:::ext
    BG --> SQLite
    BG -->|"cargo / git / gh"| GH["GitHub PR<br/>(operator review · never merged)"]:::ext

    style Server fill:transparent,stroke:#9aa4b2,stroke-width:1px
    classDef proc fill:#e8eef9,stroke:#4a6fa5,color:#13243b
    classDef store fill:#fbf0d4,stroke:#b8902a,color:#4a3410
    classDef ext fill:#e2f1e8,stroke:#3a8a5a,color:#15401f
    classDef client fill:#efe9f7,stroke:#7a5aa5,color:#2c1a4a
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
    participant T as Transport (stdio)
    participant R as rmcp router
    participant H as Handler (handlers_*)
    participant M as Mode (ModeCore)
    participant S as SqliteStorage
    participant A as AnthropicClient

    C->>T: tools/call {name, args, _meta}
    T->>R: JSON-RPC request
    R->>H: dispatch by tool name
    H->>M: build request, load session context
    M->>S: read prior thoughts / mode state
    M->>M: select prompt + thinking budget
    M->>A: complete(messages, config)
    Note over A: enforce ≤50 msgs, ≤50KB/msg<br/>before POST
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

- **Size limits**, enforced in the Anthropic client just before the POST:
  `MAX_MESSAGES` 50 and `MAX_CONTENT_LENGTH` 50KB/message — rejected before the
  model call.
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
    Start["Mode calls complete()/complete_streaming()"]:::proc --> Budget{"Thinking budget<br/>for this mode?"}:::decision
    Budget -->|"linear/tree/auto/checkpoint"| None["None (fast)"]:::proc
    Budget -->|"graph"| Std["Standard 4096"]:::proc
    Budget -->|"divergent/reflection/decision/<br/>evidence/detect/timeline"| Deep["Deep 8192"]:::proc
    Budget -->|"counterfactual/mcts"| Max["Maximum 16384"]:::proc

    None --> Build
    Std --> Build
    Deep --> Build
    Max --> Build["Build request"]:::proc

    Build --> Stream{"Streaming?"}:::decision
    Stream -->|"no"| Post["POST /messages"]:::proc
    Stream -->|"yes"| SSE["POST (SSE)<br/>parse events → StreamAccumulator"]:::proc
    SSE -.->|"report_milestone"| Bus["progress_tx broadcast"]:::proc

    Post --> Resp{"2xx?"}:::decision
    SSE --> Resp
    Resp -->|"yes"| Done["CompletionResponse"]:::term
    Resp -->|"429 / 5xx / timeout"| Retry{"retries left?<br/>(MAX_RETRIES)"}:::decision
    Retry -->|"yes"| Backoff["exponential backoff"]:::proc --> Build
    Retry -->|"no"| Err["AnthropicError → ModeError"]:::guard

    classDef proc fill:#e8eef9,stroke:#4a6fa5,color:#13243b
    classDef decision fill:#fde9d9,stroke:#c47a2a,color:#5a3410
    classDef term fill:#eef0f2,stroke:#8a93a0,color:#2a2f36
    classDef guard fill:#f7e6e6,stroke:#b15a5a,color:#5a1818
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
    RM["Streaming mode emits milestones<br/>report_milestone: 5% → 15% → 90% → 100%"]:::proc
    RM -->|"ProgressEvent{token, percent, msg}"| TX["progress_tx<br/>(broadcast bus)"]:::proc

    Meta{"client sent a<br/>progress token?"}:::decision
    Meta -->|"no"| Passthrough["run handler,<br/>send nothing"]:::term
    Meta -->|"yes"| WP["with_progress (tool boundary)<br/>subscribe + select loop"]:::proc
    TX --> WP

    WP -->|"ev.token == this call"| Notify["peer.notify_progress()"]:::proc
    WP -->|"another call's token"| Ignore["ignore"]:::term
    WP -->|"on completion"| Drain["drain final 100% tick"]:::proc
    Notify --> Client["MCP client receives<br/>notifications/progress"]:::client
    Drain --> Client

    classDef proc fill:#e8eef9,stroke:#4a6fa5,color:#13243b
    classDef decision fill:#fde9d9,stroke:#c47a2a,color:#5a3410
    classDef term fill:#eef0f2,stroke:#8a93a0,color:#2a2f36
    classDef client fill:#efe9f7,stroke:#7a5aa5,color:#2c1a4a
```

---

## 5. Semantic memory (Voyage)

`reasoning_search` and `reasoning_relate` (and divergent's novelty scoring)
require `VOYAGE_API_KEY` — there is no keyword fallback. Embeddings are cached in
`session_embeddings`, keyed on a content hash **and** the model. A background
worker warms the cache so the first search/relate after a write is ready.

```mermaid
flowchart TB
    %% Write / warming path fills the cache
    TW["Thought write"]:::proc --> EQ["enqueue session →<br/>embedding_queue"]:::proc
    EQ -.->|"warms"| EW["embed_worker<br/>(interval)"]:::proc
    EW --> DQ["dequeue"]:::proc
    DQ --> EMB["Voyage /embeddings<br/>(voyage-4)"]:::ext
    EMB --> Cache[("session_embeddings cache<br/>keyed on content hash + model")]:::store

    %% Read path — search reads the cache
    Q["reasoning_search(query)"]:::proc --> Key{"VOYAGE_API_KEY<br/>set?"}:::decision
    Key -->|"no"| CfgErr["clear config error"]:::guard
    Key -->|"yes"| QE["embed query"]:::proc
    QE --> Cos["cosine recall<br/>over cached vectors"]:::proc
    Cache -->|"cache hit"| Cos
    Cos --> Rank["Voyage /rerank<br/>(rerank-2.5)"]:::ext
    Rank --> Top["top sessions"]:::term

    %% Read path — relate
    RL["reasoning_relate(session)"]:::proc --> Edges["cosine + shared-mode<br/>+ temporal edges"]:::proc
    Edges --> BFS["depth-bounded BFS<br/>(capped at MAX_GRAPH_EDGES)"]:::proc
    BFS --> Graph["relatedness graph"]:::term

    classDef proc fill:#e8eef9,stroke:#4a6fa5,color:#13243b
    classDef store fill:#fbf0d4,stroke:#b8902a,color:#4a3410
    classDef ext fill:#e2f1e8,stroke:#3a8a5a,color:#15401f
    classDef decision fill:#fde9d9,stroke:#c47a2a,color:#5a3410
    classDef guard fill:#f7e6e6,stroke:#b15a5a,color:#5a1818
    classDef term fill:#eef0f2,stroke:#8a93a0,color:#2a2f36
```

---

## 6. Self-improvement loop

A 4-phase cycle measures the server's own performance, asks the model to
diagnose regressions, applies bounded parameter changes (or rolls them back), and
rewards measured improvement. Safety mechanisms gate every action.

```mermaid
flowchart LR
    subgraph Cycle["run_cycle (4 phases)"]
        direction LR
        Mon["1. Monitor<br/>success / latency / baseline<br/>+ low-success transitions"]:::proc
        Ana["2. Analyze<br/>LLM diagnosis"]:::proc
        Exe["3. Execute<br/>apply ThresholdAdjust / override"]:::proc
        Lrn["4. Learn<br/>reward = measured Δ, gated on MDE"]:::proc
        Mon --> Ana --> Exe --> Lrn
        Lrn -->|"next cycle"| Mon
    end

    %% Safety gates are free nodes; dotted edges exit the cycle (no cross-cluster tangle)
    AL["Allowlist<br/>validate action bounds"]:::guard
    CB["Circuit breaker<br/>halt on consecutive failures"]:::guard
    RB["Rollback on regression"]:::guard
    BL["Baseline tracking"]:::guard
    Ana -.-> AL
    Exe -.-> CB
    Exe -.-> RB
    Lrn -.-> BL

    Lrn --> Sup["self-correcting suppression<br/>of anti-pattern transitions"]:::proc
    Sup --> SuggEng["SuggestionEngine<br/>hard-blocks them"]:::proc
    Mon --> Store[("SI storage<br/>diagnoses, actions, overrides, stats")]:::store

    style Cycle fill:transparent,stroke:#9aa4b2,stroke-width:1px
    classDef proc fill:#e8eef9,stroke:#4a6fa5,color:#13243b
    classDef store fill:#fbf0d4,stroke:#b8902a,color:#4a3410
    classDef guard fill:#f7e6e6,stroke:#b15a5a,color:#5a1818
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
    Fail["Tool parse/schema failure"]:::proc --> Detect["DefectLog.observe()<br/>per-signature + per-input counts"]:::proc
    Detect --> Recur{"recurring?<br/>(≥ threshold)"}:::decision
    Recur -->|"no"| Wait["wait"]:::term
    Recur -->|"yes"| Drift{"partition_drift:<br/>broad across components?"}:::decision
    Drift -->|"yes"| Alert["route to drift<br/>(alert + record, no patch)"]:::guard
    Drift -->|"no"| Elig["classify_eligibility (spec 002)"]:::proc

    Elig -->|"model-version spike"| Alert
    Elig -->|"varied inputs"| Hold["HeldBack<br/>(recorded, operator-visible)"]:::guard
    Elig -->|"stable path"| Rank["rank_and_cap (≤ max_proposals)"]:::proc

    Rank --> Reuse{"known class?<br/>find_reusable_fix"}:::decision
    Reuse -->|"yes"| Skip["reuse prior accepted fix"]:::term
    Reuse -->|"no"| Loc["localize (LLM → source_hint)"]:::proc

    Loc --> Synth["synthesize_reproducing_test<br/>GATE: must FAIL on base"]:::proc
    Synth -->|"not grounded"| Abort["abort, no PR"]:::guard
    Synth -->|"grounded"| Fix["generate_and_validate_fix"]:::proc

    Fix --> Guard{"invariant guard:<br/>weakens a check?"}:::decision
    Guard -->|"yes"| Block["blocked, not admissible, no PR"]:::guard
    Guard -->|"no"| Gates{"suite green ∧<br/>quality green?"}:::decision
    Gates -->|"no"| NotAdm["not admissible, no PR"]:::guard
    Gates -->|"yes"| Adm["is_admissible = grounded ∧ suite ∧ quality ∧ ¬weakens"]:::proc
    Adm --> PR["open_pr (gh)<br/>NEVER merges"]:::proc
    PR --> Op["operator accept → KnowledgeEntry → reuse"]:::term

    classDef proc fill:#e8eef9,stroke:#4a6fa5,color:#13243b
    classDef decision fill:#fde9d9,stroke:#c47a2a,color:#5a3410
    classDef guard fill:#f7e6e6,stroke:#b15a5a,color:#5a1818
    classDef term fill:#eef0f2,stroke:#8a93a0,color:#2a2f36
```

Gated by env: `SELF_HEAL_PROPOSE_ENABLED=true` **and** `SELF_HEAL_WORKSPACE` set;
`SELF_HEAL_MAX_PROPOSALS` caps PRs per cycle.

---

## Legend

Node colors encode each box's role:

- 🟦 **Process** — server logic / pipeline step
- 🟨 **Datastore** — SQLite table or cache (drawn as a cylinder)
- 🟩 **External service** — Anthropic, Voyage, or GitHub
- 🟪 **Client** — the MCP client
- 🟧 **Decision** — a branch point (drawn as a diamond)
- 🟥 **Guard / blocked** — a safety gate or a rejected/aborted outcome
- ⬜ **Terminal** — a normal end state

Edges:

- **Solid arrow** — direct call / data flow.
- **Dotted arrow** — asynchronous / best-effort (milestones, cache hits, safety
  signals).

For the component breakdown behind these flows, see
[Architecture](ARCHITECTURE.md); for tool schemas, see
[Tool Reference](TOOL_REFERENCE.md) and [API Specification](API_SPECIFICATION.md).
