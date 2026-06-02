//! Graph-of-Thoughts mode.
//!
//! This mode provides 8 graph operations:
//! - `init`: Create a graph with root node
//! - `generate`: Generate child nodes
//! - `score`: Evaluate nodes
//! - `aggregate`: Merge nodes into synthesis
//! - `refine`: Improve nodes through self-critique
//! - `prune`: Identify nodes to remove
//! - `finalize`: Extract conclusions
//! - `state`: Get current graph state
//!
//! # Module Structure
//!
//! - `types`: Response types for all operations
//! - `parsing`: JSON parsing utilities

#![allow(clippy::missing_const_for_fn)]

mod parsing;
pub mod types;

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{
    graph_aggregate_prompt, graph_finalize_prompt, graph_generate_prompt, graph_init_prompt,
    graph_prune_prompt, graph_refine_prompt, graph_score_prompt, graph_state_prompt,
};
use crate::storage::{GraphEdgeType, GraphNodeType};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, StoredGraphEdge,
    StoredGraphNode, Thought,
};

pub use types::{
    AggregateResponse, ChildNode, ComplexityLevel, ExpansionDirection, FinalizeResponse,
    FrontierNodeInfo, GenerateResponse, GraphConclusion, GraphMetadata, GraphMetrics, GraphPath,
    GraphStructure, InitResponse, IntegrationNotes, NodeAssessment, NodeCritique,
    NodeRecommendation, NodeRelationship, NodeScores, NodeType, PruneCandidate, PruneImpact,
    PruneReason, PruneResponse, RefineResponse, RefinedNode, RootNode, ScoreResponse,
    SessionQuality, StateResponse, SuggestedAction, SynthesisNode,
};

// ============================================================================
// GraphMode
// ============================================================================

/// Graph-of-Thoughts mode.
///
/// Provides a rich set of operations for building and manipulating
/// a graph of reasoning nodes.
pub struct GraphMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> GraphMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new graph mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Initialize a graph with a root node.
    ///
    /// # Arguments
    ///
    /// * `content` - Topic to create graph for
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn init(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<InitResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = graph_init_prompt();
        let user_message = format!("{prompt}\n\nTopic:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.4)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let root = parsing::parse_root(&json)?;
        let expansion_directions = parsing::parse_expansion_directions(&json)?;
        let graph_metadata = parsing::parse_graph_metadata(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Graph init: {} expansion directions",
                expansion_directions.len()
            ),
            "graph_init",
            root.score,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        self.persist_node(
            &session.id,
            &root.id,
            &root.content,
            root.score,
            GraphNodeType::Thought,
        )
        .await;

        Ok(InitResponse::new(
            thought_id,
            session.id,
            root,
            expansion_directions,
            graph_metadata,
        ))
    }

    /// Generate child nodes from a parent node.
    ///
    /// # Arguments
    ///
    /// * `content` - Parent node content to expand (optional if `node_id` provided)
    /// * `node_id` - Node ID to look up content from storage (optional if `content` provided)
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if both content and node_id are missing, API fails, or parsing fails.
    pub async fn generate(
        &self,
        content: Option<&str>,
        node_id: Option<&str>,
        session_id: Option<String>,
    ) -> Result<GenerateResponse, ModeError> {
        let session = self.get_or_create_session(session_id).await?;
        let resolved_content = self.resolve_content(content, node_id, &session.id).await?;

        let prompt = graph_generate_prompt();
        let user_message = format!("{prompt}\n\nParent node:\n{resolved_content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.5)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let parent_id = parsing::get_str(&json, "parent_id")?;
        let children = parsing::parse_children(&json)?;
        let generation_notes = parsing::get_str(&json, "generation_notes")?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Graph generate: {} children", children.len()),
            "graph_generate",
            0.7,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        for child in &children {
            self.persist_node(
                &session.id,
                &child.id,
                &child.content,
                child.score,
                GraphNodeType::Thought,
            )
            .await;
            self.persist_edge(&session.id, &parent_id, &child.id, GraphEdgeType::Continues)
                .await;
        }

        Ok(GenerateResponse::new(
            thought_id,
            session.id,
            parent_id,
            children,
            generation_notes,
        ))
    }

    /// Score and evaluate a node.
    ///
    /// # Arguments
    ///
    /// * `content` - Node content to score (optional if `node_id` provided)
    /// * `node_id` - Node ID to look up content from storage (optional if `content` provided)
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if both content and node_id are missing, API fails, or parsing fails.
    pub async fn score(
        &self,
        content: Option<&str>,
        node_id: Option<&str>,
        session_id: Option<String>,
    ) -> Result<ScoreResponse, ModeError> {
        let session = self.get_or_create_session(session_id).await?;
        let resolved_content = self.resolve_content(content, node_id, &session.id).await?;

        let prompt = graph_score_prompt();
        let user_message = format!("{prompt}\n\nNode to score:\n{resolved_content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.3)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let response_node_id = parsing::get_str(&json, "node_id")?;
        let scores = parsing::parse_node_scores(&json)?;
        let assessment = parsing::parse_node_assessment(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Graph score: overall {:.2}", scores.overall),
            "graph_score",
            scores.overall,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        self.persist_score(&session.id, &response_node_id, scores.overall)
            .await;

        Ok(ScoreResponse::new(
            thought_id,
            session.id,
            response_node_id,
            scores,
            assessment,
        ))
    }

    /// Aggregate multiple nodes into a synthesis.
    ///
    /// # Arguments
    ///
    /// * `content` - Nodes to aggregate
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn aggregate(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<AggregateResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = graph_aggregate_prompt();
        let user_message = format!("{prompt}\n\nNodes to aggregate:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.4)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let input_node_ids = parsing::get_string_array(&json, "input_node_ids")?;
        let synthesis = parsing::parse_synthesis(&json)?;
        let integration_notes = parsing::parse_integration_notes(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Graph aggregate: {} inputs", input_node_ids.len()),
            "graph_aggregate",
            synthesis.score,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        self.persist_node(
            &session.id,
            &synthesis.id,
            &synthesis.content,
            synthesis.score,
            GraphNodeType::Aggregation,
        )
        .await;
        for input_id in &input_node_ids {
            self.persist_edge(
                &session.id,
                input_id,
                &synthesis.id,
                GraphEdgeType::Aggregates,
            )
            .await;
        }

        Ok(AggregateResponse::new(
            thought_id,
            session.id,
            input_node_ids,
            synthesis,
            integration_notes,
        ))
    }

    /// Refine a node through self-critique.
    ///
    /// # Arguments
    ///
    /// * `content` - Node to refine
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn refine(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<RefineResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = graph_refine_prompt();
        let user_message = format!("{prompt}\n\nNode to refine:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.4)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let original_node_id = parsing::get_str(&json, "original_node_id")?;
        let critique = parsing::parse_critique(&json)?;
        let refined_node = parsing::parse_refined_node(&json)?;
        let improvement_delta = parsing::get_f64(&json, "improvement_delta")?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Graph refine: +{improvement_delta:.2}"),
            "graph_refine",
            refined_node.score,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        self.persist_node(
            &session.id,
            &refined_node.id,
            &refined_node.content,
            refined_node.score,
            GraphNodeType::Refinement,
        )
        .await;
        self.persist_edge(
            &session.id,
            &original_node_id,
            &refined_node.id,
            GraphEdgeType::Refines,
        )
        .await;

        Ok(RefineResponse::new(
            thought_id,
            session.id,
            original_node_id,
            critique,
            refined_node,
            improvement_delta,
        ))
    }

    /// Identify nodes to prune from the graph.
    ///
    /// # Arguments
    ///
    /// * `content` - Graph state to analyze
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn prune(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<PruneResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = graph_prune_prompt();
        let user_message = format!("{prompt}\n\nGraph state:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.3)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let prune_candidates = parsing::parse_prune_candidates(&json)?;
        let preserve_nodes = parsing::get_string_array(&json, "preserve_nodes")?;
        let pruning_strategy = parsing::get_str(&json, "pruning_strategy")?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Graph prune: {} candidates", prune_candidates.len()),
            "graph_prune",
            0.75,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        for candidate in &prune_candidates {
            let storage_id = Self::namespaced_id(&session.id, &candidate.node_id);
            if let Err(e) = self.storage.delete_graph_node(&storage_id).await {
                tracing::warn!(error = %e, node_id = %candidate.node_id, "Graph node deletion failed");
            }
        }

        Ok(PruneResponse::new(
            thought_id,
            session.id,
            prune_candidates,
            preserve_nodes,
            pruning_strategy,
        ))
    }

    /// Finalize the graph and extract conclusions.
    ///
    /// # Arguments
    ///
    /// * `content` - Full graph to finalize
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn finalize(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<FinalizeResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = graph_finalize_prompt();
        let user_message = format!("{prompt}\n\nGraph to finalize:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.3)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let best_paths = parsing::parse_best_paths(&json)?;
        let conclusions = parsing::parse_conclusions(&json)?;
        let final_synthesis = parsing::get_str(&json, "final_synthesis")?;
        let session_quality = parsing::parse_session_quality(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Graph finalize: {} conclusions, quality {:.2}",
                conclusions.len(),
                session_quality.overall
            ),
            "graph_finalize",
            session_quality.overall,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        Ok(FinalizeResponse::new(
            thought_id,
            session.id,
            best_paths,
            conclusions,
            final_synthesis,
            session_quality,
        ))
    }

    /// Get the current state of the graph.
    ///
    /// # Arguments
    ///
    /// * `content` - Graph to describe (optional if `session_id` provided to retrieve from storage)
    /// * `session_id` - Session ID (required to retrieve graph state from storage)
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if API fails or parsing fails.
    pub async fn state(
        &self,
        content: Option<&str>,
        session_id: &str,
    ) -> Result<StateResponse, ModeError> {
        let session = self
            .get_or_create_session(Some(session_id.to_string()))
            .await?;

        // Resolve content: use provided content or build from storage
        let resolved_content = match content {
            Some(c) if !c.trim().is_empty() => c.to_string(),
            _ => self.build_graph_state_from_storage(session_id).await?,
        };

        let prompt = graph_state_prompt();
        let user_message = format!("{prompt}\n\nGraph:\n{resolved_content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(8192)
            .with_temperature(0.2)
            .with_standard_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let structure = parsing::parse_structure(&json)?;
        let frontiers = parsing::parse_frontiers(&json)?;
        let metrics = parsing::parse_metrics(&json)?;
        let next_steps = parsing::get_string_array(&json, "next_steps")?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Graph state: {} nodes, {} frontiers",
                structure.total_nodes,
                frontiers.len()
            ),
            "graph_state",
            metrics.average_score,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        Ok(StateResponse::new(
            thought_id, session.id, structure, frontiers, metrics, next_steps,
        ))
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    async fn get_or_create_session(
        &self,
        session_id: Option<String>,
    ) -> Result<Session, ModeError> {
        self.storage
            .get_or_create_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get or create session: {e}"),
            })
    }

    /// Resolve content from either direct content or node_id lookup.
    ///
    /// If content is provided and non-empty, uses it directly.
    /// If node_id is provided, looks up the content from storage.
    /// Returns an error if neither is provided.
    async fn resolve_content(
        &self,
        content: Option<&str>,
        node_id: Option<&str>,
        session_id: &str,
    ) -> Result<String, ModeError> {
        // Check for direct content first
        if let Some(c) = content {
            if !c.trim().is_empty() {
                return Ok(c.to_string());
            }
        }

        // Try to look up by node_id (stored under its session-namespaced key)
        if let Some(nid) = node_id {
            let storage_id = Self::namespaced_id(session_id, nid);
            let node = self
                .storage
                .get_graph_node(&storage_id)
                .await
                .map_err(|e| ModeError::ApiUnavailable {
                    message: format!("Failed to get graph node: {e}"),
                })?
                .ok_or_else(|| ModeError::InvalidValue {
                    field: "node_id".to_string(),
                    reason: format!("Node '{nid}' not found"),
                })?;
            return Ok(node.content);
        }

        // Neither provided
        Err(ModeError::MissingField {
            field: "content or node_id".to_string(),
        })
    }

    /// Build a JSON representation of the graph from storage for the state operation.
    async fn build_graph_state_from_storage(&self, session_id: &str) -> Result<String, ModeError> {
        let nodes = self
            .storage
            .get_graph_nodes(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get graph nodes: {e}"),
            })?;

        let edges = self
            .storage
            .get_graph_edges(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get graph edges: {e}"),
            })?;

        // Build a JSON representation of the graph, stripping the session
        // namespace prefix so the model sees the original short node IDs.
        let prefix = format!("{session_id}::");
        let strip = |id: &str| -> String { id.strip_prefix(&prefix).unwrap_or(id).to_string() };

        let nodes_json: Vec<serde_json::Value> = nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": strip(&n.id),
                    "content": n.content,
                    "score": n.score,
                    "node_type": n.node_type.as_str(),
                    "is_terminal": n.is_terminal
                })
            })
            .collect();

        let edges_json: Vec<serde_json::Value> = edges
            .iter()
            .map(|e| {
                serde_json::json!({
                    "from": strip(&e.from_node_id),
                    "to": strip(&e.to_node_id),
                    "type": e.edge_type.as_str()
                })
            })
            .collect();

        let graph = serde_json::json!({
            "nodes": nodes_json,
            "edges": edges_json
        });

        Ok(graph.to_string())
    }

    /// Namespace a model-supplied node ID with its session.
    ///
    /// The `graph_nodes` primary key is global, but models reuse short IDs
    /// (`root`, `c1`, ...) across sessions. Prefixing with the session ID keeps
    /// nodes from different sessions from colliding in storage.
    fn namespaced_id(session_id: &str, node_id: &str) -> String {
        format!("{session_id}::{node_id}")
    }

    /// Persist a graph node. Storage failures are logged, not propagated, so a
    /// write error never discards a reasoning result already returned to the caller.
    async fn persist_node(
        &self,
        session_id: &str,
        node_id: &str,
        content: &str,
        score: f64,
        node_type: GraphNodeType,
    ) {
        let node = StoredGraphNode::new(
            Self::namespaced_id(session_id, node_id),
            session_id,
            content,
        )
        .with_score(score)
        .with_node_type(node_type);

        if let Err(e) = self.storage.save_graph_node(&node).await {
            tracing::warn!(error = %e, node_id, "Graph node persistence failed");
        }
    }

    /// Persist a directed graph edge between two (namespaced) nodes. Failures are
    /// logged, not propagated.
    async fn persist_edge(
        &self,
        session_id: &str,
        from_id: &str,
        to_id: &str,
        edge_type: GraphEdgeType,
    ) {
        let edge = StoredGraphEdge::new(
            Self::namespaced_id(session_id, &format!("{from_id}->{to_id}")),
            session_id,
            Self::namespaced_id(session_id, from_id),
            Self::namespaced_id(session_id, to_id),
        )
        .with_edge_type(edge_type);

        if let Err(e) = self.storage.save_graph_edge(&edge).await {
            tracing::warn!(error = %e, from = from_id, to = to_id, "Graph edge persistence failed");
        }
    }

    /// Update a node's score in storage. Failures are logged, not propagated.
    async fn persist_score(&self, session_id: &str, node_id: &str, score: f64) {
        let storage_id = Self::namespaced_id(session_id, node_id);
        if let Err(e) = self
            .storage
            .update_graph_node_score(&storage_id, score)
            .await
        {
            tracing::warn!(error = %e, node_id, "Graph node score update failed");
        }
    }
}

impl<S, C> std::fmt::Debug for GraphMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .finish()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;
    use crate::error::StorageError;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    /// Register permissive (`times(..)`) expectations for the best-effort graph
    /// persistence writes so happy-path tests, which focus on API parsing, don't
    /// panic when an operation also persists nodes/edges/scores.
    fn expect_graph_writes(mock: &mut MockStorageTrait) {
        mock.expect_save_graph_node()
            .times(..)
            .returning(|_| Ok(()));
        mock.expect_save_graph_edge()
            .times(..)
            .returning(|_| Ok(()));
        mock.expect_update_graph_node_score()
            .times(..)
            .returning(|_, _| Ok(()));
        mock.expect_delete_graph_node()
            .times(..)
            .returning(|_| Ok(()));
    }

    fn mock_init_response() -> String {
        r#"{
            "root": {"id": "root", "content": "Main topic", "score": 0.5, "type": "root"},
            "expansion_directions": [
                {"direction": "Direction A", "potential": 0.7}
            ],
            "graph_metadata": {"complexity": "medium", "estimated_depth": 3}
        }"#
        .to_string()
    }

    fn mock_generate_response() -> String {
        r#"{
            "parent_id": "root",
            "children": [
                {"id": "c1", "content": "Child 1", "score": 0.6, "type": "reasoning", "relationship": "elaborates"}
            ],
            "generation_notes": "Generated reasoning children"
        }"#
        .to_string()
    }

    fn mock_score_response() -> String {
        r#"{
            "node_id": "c1",
            "scores": {"relevance": 0.8, "coherence": 0.7, "depth": 0.6, "novelty": 0.5, "overall": 0.65},
            "assessment": {"strengths": ["Clear"], "weaknesses": ["Shallow"], "recommendation": "expand"}
        }"#
        .to_string()
    }

    fn mock_aggregate_response() -> String {
        r#"{
            "input_node_ids": ["c1", "c2"],
            "synthesis": {"id": "s1", "content": "Synthesis", "score": 0.75, "type": "synthesis"},
            "integration_notes": {"common_themes": ["Theme"], "complementary_aspects": ["Aspect"], "resolved_contradictions": []}
        }"#
        .to_string()
    }

    fn mock_refine_response() -> String {
        r#"{
            "original_node_id": "c1",
            "critique": {"issues": ["Issue"], "missing_elements": ["Missing"], "unclear_aspects": []},
            "refined_node": {"id": "r1", "content": "Refined", "score": 0.8, "type": "refined"},
            "improvement_delta": 0.15
        }"#
        .to_string()
    }

    fn mock_prune_response() -> String {
        r#"{
            "prune_candidates": [
                {"node_id": "bad_node", "reason": "low_score", "confidence": 0.8, "impact": "minor"}
            ],
            "preserve_nodes": ["root", "c1"],
            "pruning_strategy": "Remove low-quality nodes"
        }"#
        .to_string()
    }

    fn mock_finalize_response() -> String {
        r#"{
            "best_paths": [
                {"path": ["root", "c1"], "path_quality": 0.85, "key_insight": "Main insight"}
            ],
            "conclusions": [
                {"conclusion": "Key conclusion", "confidence": 0.8, "supporting_nodes": ["c1"]}
            ],
            "final_synthesis": "Final summary",
            "session_quality": {"depth_achieved": 0.75, "breadth_achieved": 0.8, "coherence": 0.85, "overall": 0.8}
        }"#
        .to_string()
    }

    fn mock_state_response() -> String {
        r#"{
            "structure": {"total_nodes": 10, "depth": 3, "branches": 4, "pruned_count": 2},
            "frontiers": [
                {"node_id": "f1", "potential": 0.7, "suggested_action": "expand"}
            ],
            "metrics": {"average_score": 0.65, "max_score": 0.9, "coverage": 0.6},
            "next_steps": ["Expand f1"]
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_init_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_init_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.init("Topic", Some("test".to_string())).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.root.id, "root");
    }

    #[tokio::test]
    async fn test_generate_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_generate_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.generate(Some("Parent"), None, None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.children.len(), 1);
    }

    #[tokio::test]
    async fn test_score_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_score_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.score(Some("Node"), None, None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!((response.scores.overall - 0.65).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_aggregate_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_aggregate_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.aggregate("Nodes", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_refine_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_refine_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.refine("Node", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_prune_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_prune_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.prune("Graph", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.prune_candidates.len(), 1);
    }

    #[tokio::test]
    async fn test_finalize_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_finalize_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.finalize("Graph", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_state_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        let resp = mock_state_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.state(Some("Graph"), "test").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.structure.total_nodes, 10);
    }

    #[tokio::test]
    async fn test_init_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.init("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|_| {
            Err(StorageError::ConnectionFailed {
                message: "DB error".to_string(),
            })
        });

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.init("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[test]
    fn test_graph_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = GraphMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("GraphMode"));
    }

    #[tokio::test]
    async fn test_generate_with_node_id_lookup() {
        use crate::storage::StoredGraphNode;

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        // Mock the node lookup
        mock_storage.expect_get_graph_node().returning(|_id| {
            Ok(Some(StoredGraphNode::new(
                "node-1",
                "test",
                "Stored node content",
            )))
        });

        let resp = mock_generate_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        // Use node_id instead of content
        let result = mode.generate(None, Some("node-1"), None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.children.len(), 1);
    }

    #[tokio::test]
    async fn test_generate_missing_content_and_node_id() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        // Need to mock session creation as it happens before content resolution
        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.generate(None, None, None).await;

        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "content or node_id")
        );
    }

    #[tokio::test]
    async fn test_generate_node_id_not_found() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        // Need to mock session creation as it happens before content resolution
        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));

        mock_storage
            .expect_get_graph_node()
            .returning(|_id| Ok(None));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.generate(None, Some("nonexistent"), None).await;

        assert!(matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "node_id"));
    }

    #[tokio::test]
    async fn test_state_from_storage() {
        use crate::storage::{StoredGraphEdge, StoredGraphNode};

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        // Mock the graph retrieval from storage
        mock_storage
            .expect_get_graph_nodes()
            .returning(|_session_id| {
                Ok(vec![
                    StoredGraphNode::new("n1", "test", "Node 1"),
                    StoredGraphNode::new("n2", "test", "Node 2"),
                ])
            });
        mock_storage
            .expect_get_graph_edges()
            .returning(|_session_id| Ok(vec![StoredGraphEdge::new("e1", "test", "n1", "n2")]));

        let resp = mock_state_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        // Call state without content - should retrieve from storage
        let result = mode.state(None, "test").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.structure.total_nodes, 10);
    }

    // ========================================================================
    // Additional Coverage Tests for Error Paths
    // ========================================================================

    #[tokio::test]
    async fn test_init_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_init_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.init("Topic", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_init_api_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));

        mock_client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiUnavailable {
                message: "API error".to_string(),
            })
        });

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.init("Topic", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_generate_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_generate_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.generate(Some("Parent"), None, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_score_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_score_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.score(Some("Node"), None, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_aggregate_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.aggregate("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_aggregate_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_aggregate_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.aggregate("Nodes", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_refine_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.refine("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_refine_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_refine_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.refine("Node", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_prune_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.prune("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_prune_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_prune_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.prune("Graph", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_finalize_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.finalize("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_finalize_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_finalize_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.finalize("Graph", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_state_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT".to_string(),
                message: "Save failed".to_string(),
            })
        });
        expect_graph_writes(&mut mock_storage);

        let resp = mock_state_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.state(Some("Graph"), "test").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_node_lookup_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));

        mock_storage.expect_get_graph_node().returning(|_id| {
            Err(StorageError::QueryFailed {
                query: "SELECT".to_string(),
                message: "Lookup failed".to_string(),
            })
        });

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.generate(None, Some("node-1"), None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_state_get_nodes_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));

        mock_storage.expect_get_graph_nodes().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "SELECT".to_string(),
                message: "Get nodes failed".to_string(),
            })
        });

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.state(None, "test").await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_state_get_edges_storage_error() {
        use crate::storage::StoredGraphNode;

        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));

        mock_storage
            .expect_get_graph_nodes()
            .returning(|_| Ok(vec![StoredGraphNode::new("n1", "test", "Node 1")]));

        mock_storage.expect_get_graph_edges().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "SELECT".to_string(),
                message: "Get edges failed".to_string(),
            })
        });

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.state(None, "test").await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_score_with_node_id_lookup() {
        use crate::storage::StoredGraphNode;

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        mock_storage.expect_get_graph_node().returning(|_id| {
            Ok(Some(StoredGraphNode::new(
                "node-1",
                "test",
                "Stored node content",
            )))
        });

        let resp = mock_score_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.score(None, Some("node-1"), None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_score_missing_content_and_node_id() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.score(None, None, None).await;

        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "content or node_id")
        );
    }

    #[tokio::test]
    async fn test_generate_empty_content_with_node_id() {
        use crate::storage::StoredGraphNode;

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        // Providing empty content should fall back to node_id lookup
        mock_storage.expect_get_graph_node().returning(|_id| {
            Ok(Some(StoredGraphNode::new(
                "node-1",
                "test",
                "Stored node content",
            )))
        });

        let resp = mock_generate_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.generate(Some("  "), Some("node-1"), None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_state_with_empty_content() {
        use crate::storage::{StoredGraphEdge, StoredGraphNode};

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        expect_graph_writes(&mut mock_storage);

        // Providing empty content should retrieve from storage
        mock_storage
            .expect_get_graph_nodes()
            .returning(|_| Ok(vec![StoredGraphNode::new("n1", "test", "Node 1")]));
        mock_storage
            .expect_get_graph_edges()
            .returning(|_| Ok(vec![StoredGraphEdge::new("e1", "test", "n1", "n2")]));

        let resp = mock_state_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = GraphMode::new(mock_storage, mock_client);
        let result = mode.state(Some("  "), "test").await;

        assert!(result.is_ok());
    }

    // ========================================================================
    // End-to-end persistence tests (real in-memory storage)
    //
    // These verify the graph is actually written to and read back from storage,
    // with per-session node-ID namespacing, which the mocked tests above cannot.
    // ========================================================================

    use std::sync::Arc;

    use crate::storage::SqliteStorage;

    /// Build a mock client that returns `resp` for every completion call.
    fn fixed_client(resp: String) -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));
        client
    }

    async fn in_memory_storage() -> Arc<SqliteStorage> {
        Arc::new(
            SqliteStorage::new_in_memory()
                .await
                .expect("create in-memory storage"),
        )
    }

    /// Create the session row (foreign keys are enforced, so nodes/edges need it).
    async fn seed_session(storage: &Arc<SqliteStorage>, session_id: &str) {
        storage
            .get_or_create_session(Some(session_id.to_string()))
            .await
            .expect("create session");
    }

    /// Persist a node under its session-namespaced key so edges can reference it.
    async fn seed_node(storage: &Arc<SqliteStorage>, session_id: &str, node_id: &str) {
        storage
            .save_graph_node(&StoredGraphNode::new(
                format!("{session_id}::{node_id}"),
                session_id,
                "seed content",
            ))
            .await
            .expect("seed node");
    }

    #[tokio::test]
    async fn test_init_persists_root_node() {
        let storage = in_memory_storage().await;
        let mode = GraphMode::new(Arc::clone(&storage), fixed_client(mock_init_response()));

        let resp = mode
            .init("Topic", Some("sess-init".to_string()))
            .await
            .expect("init succeeds");
        assert_eq!(resp.root.id, "root");

        let nodes = storage
            .get_graph_nodes("sess-init")
            .await
            .expect("read nodes");
        assert_eq!(nodes.len(), 1);
        // Stored under its session-namespaced key, not the bare model ID.
        assert_eq!(nodes[0].id, "sess-init::root");
        assert_eq!(nodes[0].content, "Main topic");
        assert!((nodes[0].score.unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_generate_persists_children_and_edges() {
        let storage = in_memory_storage().await;
        seed_session(&storage, "sess-gen").await;
        // Parent must exist before the child edge can reference it (FK enforced).
        seed_node(&storage, "sess-gen", "root").await;
        let mode = GraphMode::new(Arc::clone(&storage), fixed_client(mock_generate_response()));

        mode.generate(Some("Parent"), None, Some("sess-gen".to_string()))
            .await
            .expect("generate succeeds");

        let nodes = storage.get_graph_nodes("sess-gen").await.expect("nodes");
        assert!(nodes.iter().any(|n| n.id == "sess-gen::c1"));

        let edges = storage.get_graph_edges("sess-gen").await.expect("edges");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from_node_id, "sess-gen::root");
        assert_eq!(edges[0].to_node_id, "sess-gen::c1");
    }

    #[tokio::test]
    async fn test_score_updates_persisted_node() {
        let storage = in_memory_storage().await;
        seed_session(&storage, "sess-score").await;
        // Seed the node the score response refers to ("c1") under the session.
        storage
            .save_graph_node(
                &StoredGraphNode::new("sess-score::c1", "sess-score", "Node").with_score(0.1),
            )
            .await
            .expect("seed node");

        let mode = GraphMode::new(Arc::clone(&storage), fixed_client(mock_score_response()));
        mode.score(Some("Node"), None, Some("sess-score".to_string()))
            .await
            .expect("score succeeds");

        let node = storage
            .get_graph_node("sess-score::c1")
            .await
            .expect("read node")
            .expect("node exists");
        assert!((node.score.unwrap() - 0.65).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_aggregate_persists_synthesis_and_edges() {
        let storage = in_memory_storage().await;
        seed_session(&storage, "sess-agg").await;
        // Input nodes must exist before aggregation edges can reference them.
        seed_node(&storage, "sess-agg", "c1").await;
        seed_node(&storage, "sess-agg", "c2").await;
        let mode = GraphMode::new(
            Arc::clone(&storage),
            fixed_client(mock_aggregate_response()),
        );

        mode.aggregate("Nodes", Some("sess-agg".to_string()))
            .await
            .expect("aggregate succeeds");

        let synthesis = storage
            .get_graph_node("sess-agg::s1")
            .await
            .expect("read synthesis")
            .expect("synthesis exists");
        assert_eq!(synthesis.node_type, GraphNodeType::Aggregation);

        // One edge from each input node ("c1", "c2") into the synthesis.
        let edges = storage.get_graph_edges("sess-agg").await.expect("edges");
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.to_node_id == "sess-agg::s1"));
    }

    #[tokio::test]
    async fn test_refine_persists_refined_node() {
        let storage = in_memory_storage().await;
        seed_session(&storage, "sess-refine").await;
        // Original node must exist before the refinement edge can reference it.
        seed_node(&storage, "sess-refine", "c1").await;
        let mode = GraphMode::new(Arc::clone(&storage), fixed_client(mock_refine_response()));

        mode.refine("Node", Some("sess-refine".to_string()))
            .await
            .expect("refine succeeds");

        let refined = storage
            .get_graph_node("sess-refine::r1")
            .await
            .expect("read refined")
            .expect("refined exists");
        assert_eq!(refined.node_type, GraphNodeType::Refinement);

        let edges = storage.get_graph_edges("sess-refine").await.expect("edges");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from_node_id, "sess-refine::c1");
        assert_eq!(edges[0].to_node_id, "sess-refine::r1");
    }

    #[tokio::test]
    async fn test_prune_deletes_persisted_node() {
        let storage = in_memory_storage().await;
        seed_session(&storage, "sess-prune").await;
        storage
            .save_graph_node(&StoredGraphNode::new(
                "sess-prune::bad_node",
                "sess-prune",
                "Weak node",
            ))
            .await
            .expect("seed node");

        let mode = GraphMode::new(Arc::clone(&storage), fixed_client(mock_prune_response()));
        mode.prune("Graph", Some("sess-prune".to_string()))
            .await
            .expect("prune succeeds");

        let gone = storage
            .get_graph_node("sess-prune::bad_node")
            .await
            .expect("read node");
        assert!(gone.is_none(), "pruned node should be deleted from storage");
    }

    #[tokio::test]
    async fn test_node_id_lookup_uses_namespaced_key() {
        let storage = in_memory_storage().await;
        seed_session(&storage, "sess-lookup").await;
        // Persist a node, then drive generate by node_id (not content).
        storage
            .save_graph_node(&StoredGraphNode::new(
                "sess-lookup::n1",
                "sess-lookup",
                "Stored parent content",
            ))
            .await
            .expect("seed node");

        let mode = GraphMode::new(Arc::clone(&storage), fixed_client(mock_generate_response()));
        // Bare "n1" must resolve via the session-namespaced key "sess-lookup::n1".
        let resp = mode
            .generate(None, Some("n1"), Some("sess-lookup".to_string()))
            .await
            .expect("generate by node_id succeeds");
        assert_eq!(resp.children.len(), 1);
    }

    #[tokio::test]
    async fn test_state_reads_persisted_graph_with_stripped_ids() {
        let storage = in_memory_storage().await;
        // init then generate to build a small persisted graph.
        GraphMode::new(Arc::clone(&storage), fixed_client(mock_init_response()))
            .init("Topic", Some("sess-state".to_string()))
            .await
            .expect("init");
        GraphMode::new(Arc::clone(&storage), fixed_client(mock_generate_response()))
            .generate(Some("Parent"), None, Some("sess-state".to_string()))
            .await
            .expect("generate");

        // build_graph_state_from_storage strips the namespace prefix for the model.
        let json = GraphMode::new(Arc::clone(&storage), fixed_client(mock_state_response()))
            .build_graph_state_from_storage("sess-state")
            .await
            .expect("build state");
        assert!(json.contains("\"id\":\"root\""), "state JSON: {json}");
        assert!(json.contains("\"id\":\"c1\""), "state JSON: {json}");
        assert!(
            !json.contains("sess-state::"),
            "namespace prefix should be stripped: {json}"
        );
    }
}
