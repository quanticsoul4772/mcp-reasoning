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
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
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
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
        let resolved_content = self.resolve_content(content, node_id).await?;

        let prompt = graph_generate_prompt();
        let user_message = format!("{prompt}\n\nParent node:\n{resolved_content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
        let resolved_content = self.resolve_content(content, node_id).await?;

        let prompt = graph_score_prompt();
        let user_message = format!("{prompt}\n\nNode to score:\n{resolved_content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
            .with_max_tokens(4096)
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

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

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
    ) -> Result<String, ModeError> {
        // Check for direct content first
        if let Some(c) = content {
            if !c.trim().is_empty() {
                return Ok(c.to_string());
            }
        }

        // Try to look up by node_id
        if let Some(nid) = node_id {
            let node = self
                .storage
                .get_graph_node(nid)
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

        // Build a JSON representation of the graph
        let nodes_json: Vec<serde_json::Value> = nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "content": n.content,
                    "score": n.score,
                    "is_terminal": n.is_terminal
                })
            })
            .collect();

        let edges_json: Vec<serde_json::Value> = edges
            .iter()
            .map(|e| {
                serde_json::json!({
                    "from": e.from_node_id,
                    "to": e.to_node_id,
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::error::StorageError;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

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
}
