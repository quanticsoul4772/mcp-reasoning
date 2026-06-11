//! Activity event schema for the real-time dashboard.
//!
//! An [`ActivityEvent`] is a typed, redaction-safe record of one thing happening
//! at one seam of the end-to-end flow (a request hitting a node, an edge firing,
//! a background loop advancing). Events are broadcast in-memory via
//! [`ActivityBus`](crate::dashboard::ActivityBus) and never touch stdout, so they
//! cannot disturb the stdio MCP channel.
//!
//! The schema mirrors the boxes/edges of `docs/reference/FLOW_OVERVIEW.md`: a
//! [`Node`] is a box that lights up, an [`EdgeId`] is an edge that pulses, and a
//! [`Phase`] is the lifecycle stage. The frontend addresses nodes/edges by these
//! stable identifiers to animate the live diagram.
//!
//! **Redaction discipline:** `note` is a short label only. Never put prompts,
//! completions, or secrets in any field — operation names and static status
//! labels only.

use serde::{Deserialize, Serialize};

/// A box in the end-to-end flow diagram that an event lights up.
///
/// Variants map 1:1 to the nodes of `FLOW_OVERVIEW`. Serialized as snake_case so
/// the frontend can address DOM elements by the same identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Node {
    /// MCP client (Claude Code / Desktop).
    Client,
    /// Tool registry → handler dispatch.
    Registry,
    /// Reasoning mode (`ModeCore`).
    Mode,
    /// Anthropic Claude API.
    Anthropic,
    /// `SQLite` datastore.
    Sqlite,
    /// Voyage AI (embeddings + rerank).
    Voyage,
    /// Background embedding worker.
    Worker,
    /// Self-improvement cycle.
    Si,
    /// Self-heal loop (off by default).
    Heal,
    /// GitHub PR (heal proposals).
    Github,
}

/// Lifecycle phase of an activity, used to choose the UI effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Work started at a node.
    Started,
    /// Intermediate progress (e.g. a streaming milestone).
    Progress,
    /// Work completed successfully.
    Completed,
    /// Work failed.
    Failed,
    /// A heal candidate was held back (not proposed) by a safety guard.
    HeldBack,
}

/// A directed edge in the flow diagram that an event pulses.
///
/// Named (rather than a `(src, dst)` tuple) so the frontend can map each variant
/// to a specific drawn edge. Variants mirror the spine + background edges of
/// `FLOW_OVERVIEW`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeId {
    /// ① client → registry (tool call).
    ClientToRegistry,
    /// registry → mode dispatch.
    RegistryToMode,
    /// ①④ mode ↔ `SQLite` (load context / persist).
    ModeToSqlite,
    /// ②③ mode ↔ Anthropic (prompt / completion).
    ModeToAnthropic,
    /// ⑤ mode → client (response).
    ModeToClient,
    /// worker → Voyage (embed + rerank).
    WorkerToVoyage,
    /// self-improvement cycle (db ↔ si).
    SiCycle,
    /// heal → GitHub (PR proposal).
    HealToGithub,
}

/// A single observable activity at one seam of the end-to-end flow.
///
/// `id` and `ts_ms` are stamped by [`ActivityBus::emit`](crate::dashboard::ActivityBus::emit)
/// on publish; construct with [`ActivityEvent::new`] and the `with_*` builders,
/// leaving those two at their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// Monotonic sequence number (stamped on emit).
    pub id: u64,
    /// Epoch milliseconds when emitted (stamped on emit).
    pub ts_ms: i64,
    /// Originating session, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The node that lit up.
    pub node: Node,
    /// The edge to pulse, when this event corresponds to one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<EdgeId>,
    /// Lifecycle phase.
    pub phase: Phase,
    /// Tool/mode name (e.g. `reasoning_mcts`), when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Pinned model identifier, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Duration of the work in milliseconds, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Bytes moved, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// Short, redacted label — never raw prompts/completions/secrets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl ActivityEvent {
    /// Create an event for `node` at `phase`. `id`/`ts_ms` are stamped on emit.
    #[must_use]
    pub fn new(node: Node, phase: Phase) -> Self {
        Self {
            id: 0,
            ts_ms: 0,
            session_id: None,
            node,
            edge: None,
            phase,
            tool: None,
            model: None,
            duration_ms: None,
            bytes: None,
            note: None,
        }
    }

    /// Set the edge to pulse.
    #[must_use]
    pub fn with_edge(mut self, edge: EdgeId) -> Self {
        self.edge = Some(edge);
        self
    }

    /// Set the originating session id.
    #[must_use]
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the tool/mode name.
    #[must_use]
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    /// Set the pinned model identifier.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the duration in milliseconds.
    #[must_use]
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Set the bytes moved.
    #[must_use]
    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes = Some(bytes);
        self
    }

    /// Set a short, redaction-safe label. Callers must pass label-only text.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_id_and_ts_to_zero() {
        let ev = ActivityEvent::new(Node::Mode, Phase::Completed);
        assert_eq!(ev.id, 0);
        assert_eq!(ev.ts_ms, 0);
        assert_eq!(ev.node, Node::Mode);
        assert_eq!(ev.phase, Phase::Completed);
        assert!(ev.edge.is_none());
        assert!(ev.tool.is_none());
        assert!(ev.note.is_none());
    }

    #[test]
    fn builders_set_all_fields() {
        let ev = ActivityEvent::new(Node::Anthropic, Phase::Progress)
            .with_edge(EdgeId::ModeToAnthropic)
            .with_session("sess-1")
            .with_tool("reasoning_mcts")
            .with_model("claude-opus-4-8")
            .with_duration_ms(1234)
            .with_bytes(2048)
            .with_note("Starting API call");
        assert_eq!(ev.edge, Some(EdgeId::ModeToAnthropic));
        assert_eq!(ev.session_id.as_deref(), Some("sess-1"));
        assert_eq!(ev.tool.as_deref(), Some("reasoning_mcts"));
        assert_eq!(ev.model.as_deref(), Some("claude-opus-4-8"));
        assert_eq!(ev.duration_ms, Some(1234));
        assert_eq!(ev.bytes, Some(2048));
        assert_eq!(ev.note.as_deref(), Some("Starting API call"));
    }

    #[test]
    fn serializes_node_and_phase_as_snake_case() {
        let ev = ActivityEvent::new(Node::Sqlite, Phase::HeldBack).with_edge(EdgeId::ModeToSqlite);
        let json = serde_json::to_string(&ev).expect("serialize");
        assert!(json.contains("\"node\":\"sqlite\""));
        assert!(json.contains("\"phase\":\"held_back\""));
        assert!(json.contains("\"edge\":\"mode_to_sqlite\""));
    }

    #[test]
    fn omits_none_fields_from_json() {
        let ev = ActivityEvent::new(Node::Client, Phase::Started);
        let json = serde_json::to_string(&ev).expect("serialize");
        assert!(!json.contains("session_id"));
        assert!(!json.contains("tool"));
        assert!(!json.contains("note"));
        assert!(!json.contains("edge"));
    }

    #[test]
    fn round_trips_through_json() {
        let ev = ActivityEvent::new(Node::Heal, Phase::Failed)
            .with_edge(EdgeId::HealToGithub)
            .with_tool("heal")
            .with_duration_ms(7);
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: ActivityEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev, back);
    }

    #[test]
    fn all_nodes_serialize_distinctly() {
        let nodes = [
            Node::Client,
            Node::Registry,
            Node::Mode,
            Node::Anthropic,
            Node::Sqlite,
            Node::Voyage,
            Node::Worker,
            Node::Si,
            Node::Heal,
            Node::Github,
        ];
        let mut seen = std::collections::HashSet::new();
        for n in nodes {
            let s = serde_json::to_string(&n).expect("serialize node");
            assert!(seen.insert(s), "duplicate node serialization");
        }
        assert_eq!(seen.len(), nodes.len());
    }

    #[test]
    fn all_edges_serialize_distinctly() {
        let edges = [
            EdgeId::ClientToRegistry,
            EdgeId::RegistryToMode,
            EdgeId::ModeToSqlite,
            EdgeId::ModeToAnthropic,
            EdgeId::ModeToClient,
            EdgeId::WorkerToVoyage,
            EdgeId::SiCycle,
            EdgeId::HealToGithub,
        ];
        let mut seen = std::collections::HashSet::new();
        for e in edges {
            let s = serde_json::to_string(&e).expect("serialize edge");
            assert!(seen.insert(s), "duplicate edge serialization");
        }
        assert_eq!(seen.len(), edges.len());
    }
}
