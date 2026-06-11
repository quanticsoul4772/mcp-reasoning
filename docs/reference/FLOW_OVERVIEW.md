# End-to-End Flow — At a Glance

The whole system in one diagram. A request enters over stdio, runs through a
reasoning mode (which composes storage + the Anthropic client) and returns, while
three background loops keep semantic memory warm, tune the server, and —
optionally — propose fixes for the server's own recurring defects.

For the per-subsystem detail behind each box (the request lifecycle, retry/
thinking budgets, streaming, the 4-phase self-improvement cycle, and the self-heal
decision tree), see **[End-to-End Flow](END_TO_END_FLOW.md)**.

```mermaid
flowchart TD
    Client["MCP Client<br/>(Claude Code / Desktop)"]:::client
    Client -->|"tool call · stdio / JSON-RPC"| Server["Tool Registry → handler<br/>(35 tools)"]:::proc
    Server --> Mode["Reasoning Mode · ModeCore<br/>(storage + Anthropic client)"]:::proc

    Mode -->|"① load session context"| DB[("SQLite<br/>sessions · thoughts · graph · metrics")]:::store
    Mode -->|"② prompt + thinking budget<br/>(bounded retry / backoff)"| Anthropic["Anthropic Claude API<br/>reasoning + thinking"]:::ext
    Anthropic -->|"③ completion (may stream)"| Mode
    Mode -->|"④ parse JSON + persist"| DB
    Mode ==>|"⑤ response"| Client
    Mode -.->|"progress notifications<br/>(if client opted in)"| Client

    DB -.->|"on interval"| Worker["Embedding worker"]:::proc
    Worker -->|"embed + rerank"| Voyage["Voyage AI<br/>semantic memory"]:::ext
    Worker --> DB
    DB -.->|"metrics"| SI["Self-improvement cycle<br/>tune thresholds"]:::proc
    SI --> DB
    DB -.->|"recurring defects"| Heal["Self-heal loop<br/>(OFF by default)"]:::guard
    Heal -->|"cargo / git / gh"| GH["GitHub PR<br/>never merged"]:::ext

    classDef proc fill:#e8eef9,stroke:#4a6fa5,color:#13243b
    classDef store fill:#fbf0d4,stroke:#b8902a,color:#4a3410
    classDef ext fill:#e2f1e8,stroke:#3a8a5a,color:#15401f
    classDef client fill:#efe9f7,stroke:#7a5aa5,color:#2c1a4a
    classDef guard fill:#f7e6e6,stroke:#b15a5a,color:#5a1818
```

**Reading it:** steps ①–⑤ are the synchronous request spine; dotted edges are
asynchronous (progress notifications, and the interval-driven background loops
that read from SQLite). Colors: 🟦 server process · 🟨 datastore ·
🟩 external service · 🟪 client · 🟥 safety loop (off by default).
