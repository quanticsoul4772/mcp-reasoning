//! Type definitions for memory tools.

use serde::{Deserialize, Serialize};

/// Summary of a reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Unique session identifier.
    pub session_id: String,
    /// When the session was created.
    pub created_at: String,
    /// When the session was last updated.
    pub updated_at: String,
    /// Number of thoughts in the session.
    pub thought_count: u32,
    /// Preview of the first thought.
    pub preview: String,
    /// Primary reasoning mode used.
    pub primary_mode: Option<String>,
}

/// Full context from a reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Unique session identifier.
    pub session_id: String,
    /// When the session was created.
    pub created_at: String,
    /// Compressed summary of the session.
    pub summary: String,
    /// Chain of thoughts.
    pub thought_chain: Vec<ThoughtSummary>,
    /// Key conclusions reached.
    pub key_conclusions: Vec<String>,
    /// Last mode used.
    pub last_mode: Option<String>,
    /// Latest checkpoint if available.
    pub checkpoint: Option<CheckpointInfo>,
    /// Suggestions for continuing.
    pub continuation_suggestions: Vec<String>,
}

/// Summary of a single thought.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSummary {
    /// Thought ID.
    pub id: String,
    /// Reasoning mode used.
    pub mode: String,
    /// Thought content (may be truncated).
    pub content: String,
    /// Confidence score.
    pub confidence: f64,
}

/// Checkpoint information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    /// Checkpoint ID.
    pub id: String,
    /// Checkpoint name.
    pub name: String,
    /// Checkpoint description.
    pub description: Option<String>,
}

/// Search result from semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Session ID.
    pub session_id: String,
    /// Similarity score (0.0-1.0).
    pub similarity_score: f64,
    /// Preview of the session.
    pub preview: String,
    /// When the session was created.
    pub created_at: String,
    /// Primary mode used.
    pub primary_mode: Option<String>,
}

/// Relationship graph between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipGraph {
    /// Nodes in the graph (sessions).
    pub nodes: Vec<SessionNode>,
    /// Edges in the graph (relationships).
    pub edges: Vec<RelationshipEdge>,
    /// Detected clusters.
    pub clusters: Vec<SessionCluster>,
}

/// Node in the relationship graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNode {
    /// Session ID.
    pub session_id: String,
    /// Preview of the session.
    pub preview: String,
    /// When the session was created.
    pub created_at: String,
}

/// Edge in the relationship graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEdge {
    /// Source session.
    pub from_session: String,
    /// Target session.
    pub to_session: String,
    /// Type of relationship.
    pub relationship_type: RelationshipType,
    /// Strength of relationship (0.0-1.0).
    pub strength: f64,
}

/// Type of relationship between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Session B continues from session A.
    ContinuesFrom,
    /// Sessions have similar topics.
    SimilarTopic,
    /// Sessions use the same reasoning mode.
    SharedMode,
    /// Sessions created close in time.
    TemporallyAdjacent,
    /// Sessions reach similar conclusions.
    CommonConclusion,
}

/// Cluster of related sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCluster {
    /// Session IDs in the cluster.
    pub sessions: Vec<String>,
    /// Common theme or topic.
    pub common_theme: String,
}
