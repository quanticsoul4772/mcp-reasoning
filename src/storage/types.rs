//! Storage types for database operations.
//!
//! This module defines the types stored in the database:
//! - [`StoredSession`]: Session data
//! - [`StoredThought`]: Individual thought/reasoning step
//! - [`StoredBranch`]: Tree mode branches
//! - [`StoredCheckpoint`]: State checkpoints
//! - [`StoredGraphNode`]: Graph-of-Thoughts nodes
//! - [`StoredGraphEdge`]: Graph edges
//! - [`StoredMetric`]: Usage metrics
//! - [`StoredSelfImprovementAction`]: Self-improvement actions

#![allow(clippy::should_implement_trait)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Session stored in database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSession {
    /// Unique session identifier.
    pub id: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Optional JSON metadata.
    pub metadata: Option<String>,
}

impl StoredSession {
    /// Create a new stored session.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }

    /// Create a session with specific timestamps.
    #[must_use]
    pub fn with_timestamps(
        id: impl Into<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            created_at,
            updated_at,
            metadata: None,
        }
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }
}

/// Thought stored in database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredThought {
    /// Unique thought identifier.
    pub id: String,
    /// Parent session identifier.
    pub session_id: String,
    /// Parent thought identifier (for hierarchical thoughts).
    pub parent_id: Option<String>,
    /// Reasoning mode used.
    pub mode: String,
    /// Thought content.
    pub content: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Optional JSON metadata.
    pub metadata: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl StoredThought {
    /// Create a new stored thought.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        mode: impl Into<String>,
        content: impl Into<String>,
        confidence: f64,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            parent_id: None,
            mode: mode.into(),
            content: content.into(),
            confidence,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Set parent thought ID.
    #[must_use]
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Set creation timestamp.
    #[must_use]
    pub const fn with_timestamp(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }
}

/// Branch status for tree mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BranchStatus {
    /// Branch is active and being explored.
    #[default]
    Active,
    /// Branch exploration completed.
    Completed,
    /// Branch was abandoned.
    Abandoned,
}

impl BranchStatus {
    /// Convert to string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }

    /// Parse from string.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            "abandoned" => Some(Self::Abandoned),
            _ => None,
        }
    }
}

/// Branch stored in database (for tree mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredBranch {
    /// Unique branch identifier.
    pub id: String,
    /// Parent session identifier.
    pub session_id: String,
    /// Parent branch identifier.
    pub parent_branch_id: Option<String>,
    /// Branch content.
    pub content: String,
    /// Score for this branch.
    pub score: f64,
    /// Branch status.
    pub status: BranchStatus,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl StoredBranch {
    /// Create a new stored branch.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            parent_branch_id: None,
            content: content.into(),
            score: 0.0,
            status: BranchStatus::Active,
            created_at: Utc::now(),
        }
    }

    /// Set parent branch ID.
    #[must_use]
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_branch_id = Some(parent_id.into());
        self
    }

    /// Set score.
    #[must_use]
    pub const fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }

    /// Set status.
    #[must_use]
    pub const fn with_status(mut self, status: BranchStatus) -> Self {
        self.status = status;
        self
    }
}

/// Checkpoint stored in database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCheckpoint {
    /// Unique checkpoint identifier.
    pub id: String,
    /// Parent session identifier.
    pub session_id: String,
    /// Checkpoint name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Serialized state (JSON).
    pub state: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl StoredCheckpoint {
    /// Create a new stored checkpoint.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        name: impl Into<String>,
        state: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            name: name.into(),
            description: None,
            state: state.into(),
            created_at: Utc::now(),
        }
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Graph node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GraphNodeType {
    /// Regular thought node.
    #[default]
    Thought,
    /// Aggregation of multiple nodes.
    Aggregation,
    /// Refinement of a node.
    Refinement,
}

impl GraphNodeType {
    /// Convert to string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Thought => "thought",
            Self::Aggregation => "aggregation",
            Self::Refinement => "refinement",
        }
    }

    /// Parse from string.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "thought" => Some(Self::Thought),
            "aggregation" => Some(Self::Aggregation),
            "refinement" => Some(Self::Refinement),
            _ => None,
        }
    }
}

/// Graph node stored in database (for `GoT` mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredGraphNode {
    /// Unique node identifier.
    pub id: String,
    /// Parent session identifier.
    pub session_id: String,
    /// Node content.
    pub content: String,
    /// Node type.
    pub node_type: GraphNodeType,
    /// Optional score.
    pub score: Option<f64>,
    /// Whether this is a terminal node.
    pub is_terminal: bool,
    /// Optional JSON metadata.
    pub metadata: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl StoredGraphNode {
    /// Create a new stored graph node.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            content: content.into(),
            node_type: GraphNodeType::Thought,
            score: None,
            is_terminal: false,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Set node type.
    #[must_use]
    pub const fn with_node_type(mut self, node_type: GraphNodeType) -> Self {
        self.node_type = node_type;
        self
    }

    /// Set score.
    #[must_use]
    pub const fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    /// Set as terminal node.
    #[must_use]
    pub const fn as_terminal(mut self) -> Self {
        self.is_terminal = true;
        self
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }
}

/// Graph edge type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GraphEdgeType {
    /// Edge continues a thought.
    #[default]
    Continues,
    /// Edge aggregates multiple nodes.
    Aggregates,
    /// Edge refines a node.
    Refines,
}

impl GraphEdgeType {
    /// Convert to string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Continues => "continues",
            Self::Aggregates => "aggregates",
            Self::Refines => "refines",
        }
    }

    /// Parse from string.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "continues" => Some(Self::Continues),
            "aggregates" => Some(Self::Aggregates),
            "refines" => Some(Self::Refines),
            _ => None,
        }
    }
}

/// Graph edge stored in database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredGraphEdge {
    /// Unique edge identifier.
    pub id: String,
    /// Parent session identifier.
    pub session_id: String,
    /// Source node identifier.
    pub from_node_id: String,
    /// Target node identifier.
    pub to_node_id: String,
    /// Edge type.
    pub edge_type: GraphEdgeType,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl StoredGraphEdge {
    /// Create a new stored graph edge.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        from_node_id: impl Into<String>,
        to_node_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            from_node_id: from_node_id.into(),
            to_node_id: to_node_id.into(),
            edge_type: GraphEdgeType::Continues,
            created_at: Utc::now(),
        }
    }

    /// Set edge type.
    #[must_use]
    pub const fn with_edge_type(mut self, edge_type: GraphEdgeType) -> Self {
        self.edge_type = edge_type;
        self
    }
}

/// Metric stored in database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMetric {
    /// Auto-incremented ID (None for new records).
    pub id: Option<i64>,
    /// Reasoning mode.
    pub mode: String,
    /// Tool name.
    pub tool_name: String,
    /// Latency in milliseconds.
    pub latency_ms: i64,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl StoredMetric {
    /// Create a new stored metric for success.
    #[must_use]
    pub fn success(mode: impl Into<String>, tool_name: impl Into<String>, latency_ms: i64) -> Self {
        Self {
            id: None,
            mode: mode.into(),
            tool_name: tool_name.into(),
            latency_ms,
            success: true,
            error_message: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new stored metric for failure.
    #[must_use]
    pub fn failure(
        mode: impl Into<String>,
        tool_name: impl Into<String>,
        latency_ms: i64,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            mode: mode.into(),
            tool_name: tool_name.into(),
            latency_ms,
            success: false,
            error_message: Some(error_message.into()),
            created_at: Utc::now(),
        }
    }
}

/// Self-improvement action status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Action is pending execution.
    #[default]
    Pending,
    /// Action is currently executing.
    Executing,
    /// Action completed successfully.
    Completed,
    /// Action failed.
    Failed,
    /// Action was rolled back.
    RolledBack,
}

impl ActionStatus {
    /// Convert to string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
        }
    }

    /// Parse from string.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "executing" => Some(Self::Executing),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "rolled_back" => Some(Self::RolledBack),
            _ => None,
        }
    }
}

/// Self-improvement action stored in database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSelfImprovementAction {
    /// Unique action identifier.
    pub id: String,
    /// Action type.
    pub action_type: String,
    /// Serialized parameters (JSON).
    pub parameters: String,
    /// Action status.
    pub status: ActionStatus,
    /// Serialized result (JSON).
    pub result: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Completion timestamp.
    pub completed_at: Option<DateTime<Utc>>,
}

impl StoredSelfImprovementAction {
    /// Create a new stored self-improvement action.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        action_type: impl Into<String>,
        parameters: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            action_type: action_type.into(),
            parameters: parameters.into(),
            status: ActionStatus::Pending,
            result: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Set status.
    #[must_use]
    pub const fn with_status(mut self, status: ActionStatus) -> Self {
        self.status = status;
        self
    }

    /// Set result.
    #[must_use]
    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    /// Mark as completed with timestamp.
    #[must_use]
    pub const fn mark_completed(mut self, completed_at: DateTime<Utc>) -> Self {
        self.status = ActionStatus::Completed;
        self.completed_at = Some(completed_at);
        self
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // StoredSession Tests
    #[test]
    fn test_stored_session_new() {
        let session = StoredSession::new("sess-123");
        assert_eq!(session.id, "sess-123");
        assert!(session.metadata.is_none());
        let diff = Utc::now() - session.created_at;
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn test_stored_session_with_timestamps() {
        let created = Utc::now() - chrono::Duration::hours(1);
        let updated = Utc::now();
        let session = StoredSession::with_timestamps("sess-123", created, updated);
        assert_eq!(session.created_at, created);
        assert_eq!(session.updated_at, updated);
    }

    #[test]
    fn test_stored_session_with_metadata() {
        let session = StoredSession::new("sess-123").with_metadata(r#"{"key": "value"}"#);
        assert_eq!(session.metadata, Some(r#"{"key": "value"}"#.to_string()));
    }

    #[test]
    fn test_stored_session_clone() {
        let session = StoredSession::new("sess-123");
        let cloned = session.clone();
        assert_eq!(session, cloned);
    }

    #[test]
    fn test_stored_session_debug() {
        let session = StoredSession::new("sess-123");
        let debug = format!("{session:?}");
        assert!(debug.contains("sess-123"));
    }

    #[test]
    fn test_stored_session_serde() {
        let session = StoredSession::new("sess-123").with_metadata("{}");
        let json = serde_json::to_string(&session).expect("serialize");
        let deserialized: StoredSession = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(session.id, deserialized.id);
        assert_eq!(session.metadata, deserialized.metadata);
    }

    // StoredThought Tests
    #[test]
    fn test_stored_thought_new() {
        let thought = StoredThought::new("t-1", "sess-1", "linear", "Content here", 0.85);
        assert_eq!(thought.id, "t-1");
        assert_eq!(thought.session_id, "sess-1");
        assert_eq!(thought.mode, "linear");
        assert_eq!(thought.content, "Content here");
        assert!((thought.confidence - 0.85).abs() < f64::EPSILON);
        assert!(thought.parent_id.is_none());
        assert!(thought.metadata.is_none());
    }

    #[test]
    fn test_stored_thought_with_parent() {
        let thought = StoredThought::new("t-2", "sess-1", "tree", "Child", 0.9).with_parent("t-1");
        assert_eq!(thought.parent_id, Some("t-1".to_string()));
    }

    #[test]
    fn test_stored_thought_with_metadata() {
        let thought =
            StoredThought::new("t-1", "sess-1", "linear", "Content", 0.8).with_metadata("{}");
        assert_eq!(thought.metadata, Some("{}".to_string()));
    }

    #[test]
    fn test_stored_thought_with_timestamp() {
        let ts = Utc::now() - chrono::Duration::hours(1);
        let thought =
            StoredThought::new("t-1", "sess-1", "linear", "Content", 0.8).with_timestamp(ts);
        assert_eq!(thought.created_at, ts);
    }

    #[test]
    fn test_stored_thought_clone() {
        let thought = StoredThought::new("t-1", "sess-1", "linear", "Content", 0.8);
        let cloned = thought.clone();
        assert_eq!(thought, cloned);
    }

    #[test]
    fn test_stored_thought_serde() {
        let thought = StoredThought::new("t-1", "sess-1", "linear", "Content", 0.8);
        let json = serde_json::to_string(&thought).expect("serialize");
        let deserialized: StoredThought = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(thought.id, deserialized.id);
        assert_eq!(thought.content, deserialized.content);
    }

    // BranchStatus Tests
    #[test]
    fn test_branch_status_default() {
        let status = BranchStatus::default();
        assert_eq!(status, BranchStatus::Active);
    }

    #[test]
    fn test_branch_status_as_str() {
        assert_eq!(BranchStatus::Active.as_str(), "active");
        assert_eq!(BranchStatus::Completed.as_str(), "completed");
        assert_eq!(BranchStatus::Abandoned.as_str(), "abandoned");
    }

    #[test]
    fn test_branch_status_from_str() {
        assert_eq!(BranchStatus::from_str("active"), Some(BranchStatus::Active));
        assert_eq!(
            BranchStatus::from_str("completed"),
            Some(BranchStatus::Completed)
        );
        assert_eq!(
            BranchStatus::from_str("abandoned"),
            Some(BranchStatus::Abandoned)
        );
        assert_eq!(BranchStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_branch_status_serde() {
        let status = BranchStatus::Completed;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, r#""completed""#);
        let deserialized: BranchStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, deserialized);
    }

    // StoredBranch Tests
    #[test]
    fn test_stored_branch_new() {
        let branch = StoredBranch::new("b-1", "sess-1", "Branch content");
        assert_eq!(branch.id, "b-1");
        assert_eq!(branch.session_id, "sess-1");
        assert_eq!(branch.content, "Branch content");
        assert!((branch.score - 0.0).abs() < f64::EPSILON);
        assert_eq!(branch.status, BranchStatus::Active);
        assert!(branch.parent_branch_id.is_none());
    }

    #[test]
    fn test_stored_branch_with_parent() {
        let branch = StoredBranch::new("b-2", "sess-1", "Child").with_parent("b-1");
        assert_eq!(branch.parent_branch_id, Some("b-1".to_string()));
    }

    #[test]
    fn test_stored_branch_with_score() {
        let branch = StoredBranch::new("b-1", "sess-1", "Content").with_score(0.75);
        assert!((branch.score - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_stored_branch_with_status() {
        let branch =
            StoredBranch::new("b-1", "sess-1", "Content").with_status(BranchStatus::Completed);
        assert_eq!(branch.status, BranchStatus::Completed);
    }

    #[test]
    fn test_stored_branch_clone() {
        let branch = StoredBranch::new("b-1", "sess-1", "Content");
        let cloned = branch.clone();
        assert_eq!(branch, cloned);
    }

    // StoredCheckpoint Tests
    #[test]
    fn test_stored_checkpoint_new() {
        let checkpoint = StoredCheckpoint::new("cp-1", "sess-1", "Checkpoint 1", "{}");
        assert_eq!(checkpoint.id, "cp-1");
        assert_eq!(checkpoint.session_id, "sess-1");
        assert_eq!(checkpoint.name, "Checkpoint 1");
        assert_eq!(checkpoint.state, "{}");
        assert!(checkpoint.description.is_none());
    }

    #[test]
    fn test_stored_checkpoint_with_description() {
        let checkpoint = StoredCheckpoint::new("cp-1", "sess-1", "Checkpoint 1", "{}")
            .with_description("A description");
        assert_eq!(checkpoint.description, Some("A description".to_string()));
    }

    #[test]
    fn test_stored_checkpoint_clone() {
        let checkpoint = StoredCheckpoint::new("cp-1", "sess-1", "Checkpoint 1", "{}");
        let cloned = checkpoint.clone();
        assert_eq!(checkpoint, cloned);
    }

    // GraphNodeType Tests
    #[test]
    fn test_graph_node_type_default() {
        let node_type = GraphNodeType::default();
        assert_eq!(node_type, GraphNodeType::Thought);
    }

    #[test]
    fn test_graph_node_type_as_str() {
        assert_eq!(GraphNodeType::Thought.as_str(), "thought");
        assert_eq!(GraphNodeType::Aggregation.as_str(), "aggregation");
        assert_eq!(GraphNodeType::Refinement.as_str(), "refinement");
    }

    #[test]
    fn test_graph_node_type_from_str() {
        assert_eq!(
            GraphNodeType::from_str("thought"),
            Some(GraphNodeType::Thought)
        );
        assert_eq!(
            GraphNodeType::from_str("aggregation"),
            Some(GraphNodeType::Aggregation)
        );
        assert_eq!(
            GraphNodeType::from_str("refinement"),
            Some(GraphNodeType::Refinement)
        );
        assert_eq!(GraphNodeType::from_str("invalid"), None);
    }

    #[test]
    fn test_graph_node_type_serde() {
        let node_type = GraphNodeType::Aggregation;
        let json = serde_json::to_string(&node_type).expect("serialize");
        assert_eq!(json, r#""aggregation""#);
    }

    // StoredGraphNode Tests
    #[test]
    fn test_stored_graph_node_new() {
        let node = StoredGraphNode::new("n-1", "sess-1", "Node content");
        assert_eq!(node.id, "n-1");
        assert_eq!(node.session_id, "sess-1");
        assert_eq!(node.content, "Node content");
        assert_eq!(node.node_type, GraphNodeType::Thought);
        assert!(node.score.is_none());
        assert!(!node.is_terminal);
        assert!(node.metadata.is_none());
    }

    #[test]
    fn test_stored_graph_node_with_node_type() {
        let node = StoredGraphNode::new("n-1", "sess-1", "Content")
            .with_node_type(GraphNodeType::Aggregation);
        assert_eq!(node.node_type, GraphNodeType::Aggregation);
    }

    #[test]
    fn test_stored_graph_node_with_score() {
        let node = StoredGraphNode::new("n-1", "sess-1", "Content").with_score(0.9);
        assert_eq!(node.score, Some(0.9));
    }

    #[test]
    fn test_stored_graph_node_as_terminal() {
        let node = StoredGraphNode::new("n-1", "sess-1", "Content").as_terminal();
        assert!(node.is_terminal);
    }

    #[test]
    fn test_stored_graph_node_with_metadata() {
        let node = StoredGraphNode::new("n-1", "sess-1", "Content").with_metadata("{}");
        assert_eq!(node.metadata, Some("{}".to_string()));
    }

    #[test]
    fn test_stored_graph_node_clone() {
        let node = StoredGraphNode::new("n-1", "sess-1", "Content");
        let cloned = node.clone();
        assert_eq!(node, cloned);
    }

    // GraphEdgeType Tests
    #[test]
    fn test_graph_edge_type_default() {
        let edge_type = GraphEdgeType::default();
        assert_eq!(edge_type, GraphEdgeType::Continues);
    }

    #[test]
    fn test_graph_edge_type_as_str() {
        assert_eq!(GraphEdgeType::Continues.as_str(), "continues");
        assert_eq!(GraphEdgeType::Aggregates.as_str(), "aggregates");
        assert_eq!(GraphEdgeType::Refines.as_str(), "refines");
    }

    #[test]
    fn test_graph_edge_type_from_str() {
        assert_eq!(
            GraphEdgeType::from_str("continues"),
            Some(GraphEdgeType::Continues)
        );
        assert_eq!(
            GraphEdgeType::from_str("aggregates"),
            Some(GraphEdgeType::Aggregates)
        );
        assert_eq!(
            GraphEdgeType::from_str("refines"),
            Some(GraphEdgeType::Refines)
        );
        assert_eq!(GraphEdgeType::from_str("invalid"), None);
    }

    // StoredGraphEdge Tests
    #[test]
    fn test_stored_graph_edge_new() {
        let edge = StoredGraphEdge::new("e-1", "sess-1", "n-1", "n-2");
        assert_eq!(edge.id, "e-1");
        assert_eq!(edge.session_id, "sess-1");
        assert_eq!(edge.from_node_id, "n-1");
        assert_eq!(edge.to_node_id, "n-2");
        assert_eq!(edge.edge_type, GraphEdgeType::Continues);
    }

    #[test]
    fn test_stored_graph_edge_with_edge_type() {
        let edge = StoredGraphEdge::new("e-1", "sess-1", "n-1", "n-2")
            .with_edge_type(GraphEdgeType::Aggregates);
        assert_eq!(edge.edge_type, GraphEdgeType::Aggregates);
    }

    #[test]
    fn test_stored_graph_edge_clone() {
        let edge = StoredGraphEdge::new("e-1", "sess-1", "n-1", "n-2");
        let cloned = edge.clone();
        assert_eq!(edge, cloned);
    }

    // StoredMetric Tests
    #[test]
    fn test_stored_metric_success() {
        let metric = StoredMetric::success("linear", "reasoning_linear", 150);
        assert!(metric.id.is_none());
        assert_eq!(metric.mode, "linear");
        assert_eq!(metric.tool_name, "reasoning_linear");
        assert_eq!(metric.latency_ms, 150);
        assert!(metric.success);
        assert!(metric.error_message.is_none());
    }

    #[test]
    fn test_stored_metric_failure() {
        let metric = StoredMetric::failure("linear", "reasoning_linear", 100, "API error");
        assert!(!metric.success);
        assert_eq!(metric.error_message, Some("API error".to_string()));
    }

    #[test]
    fn test_stored_metric_clone() {
        let metric = StoredMetric::success("linear", "reasoning_linear", 150);
        let cloned = metric.clone();
        assert_eq!(metric, cloned);
    }

    // ActionStatus Tests
    #[test]
    fn test_action_status_default() {
        let status = ActionStatus::default();
        assert_eq!(status, ActionStatus::Pending);
    }

    #[test]
    fn test_action_status_as_str() {
        assert_eq!(ActionStatus::Pending.as_str(), "pending");
        assert_eq!(ActionStatus::Executing.as_str(), "executing");
        assert_eq!(ActionStatus::Completed.as_str(), "completed");
        assert_eq!(ActionStatus::Failed.as_str(), "failed");
        assert_eq!(ActionStatus::RolledBack.as_str(), "rolled_back");
    }

    #[test]
    fn test_action_status_from_str() {
        assert_eq!(
            ActionStatus::from_str("pending"),
            Some(ActionStatus::Pending)
        );
        assert_eq!(
            ActionStatus::from_str("executing"),
            Some(ActionStatus::Executing)
        );
        assert_eq!(
            ActionStatus::from_str("completed"),
            Some(ActionStatus::Completed)
        );
        assert_eq!(ActionStatus::from_str("failed"), Some(ActionStatus::Failed));
        assert_eq!(
            ActionStatus::from_str("rolled_back"),
            Some(ActionStatus::RolledBack)
        );
        assert_eq!(ActionStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_action_status_serde() {
        let status = ActionStatus::Completed;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, r#""completed""#);
    }

    // StoredSelfImprovementAction Tests
    #[test]
    fn test_stored_self_improvement_action_new() {
        let action = StoredSelfImprovementAction::new("a-1", "adjust_temperature", "{}");
        assert_eq!(action.id, "a-1");
        assert_eq!(action.action_type, "adjust_temperature");
        assert_eq!(action.parameters, "{}");
        assert_eq!(action.status, ActionStatus::Pending);
        assert!(action.result.is_none());
        assert!(action.completed_at.is_none());
    }

    #[test]
    fn test_stored_self_improvement_action_with_status() {
        let action = StoredSelfImprovementAction::new("a-1", "adjust_temperature", "{}")
            .with_status(ActionStatus::Executing);
        assert_eq!(action.status, ActionStatus::Executing);
    }

    #[test]
    fn test_stored_self_improvement_action_with_result() {
        let action = StoredSelfImprovementAction::new("a-1", "adjust_temperature", "{}")
            .with_result(r#"{"success": true}"#);
        assert_eq!(action.result, Some(r#"{"success": true}"#.to_string()));
    }

    #[test]
    fn test_stored_self_improvement_action_mark_completed() {
        let completed_at = Utc::now();
        let action = StoredSelfImprovementAction::new("a-1", "adjust_temperature", "{}")
            .mark_completed(completed_at);
        assert_eq!(action.status, ActionStatus::Completed);
        assert_eq!(action.completed_at, Some(completed_at));
    }

    #[test]
    fn test_stored_self_improvement_action_clone() {
        let action = StoredSelfImprovementAction::new("a-1", "adjust_temperature", "{}");
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }
}
