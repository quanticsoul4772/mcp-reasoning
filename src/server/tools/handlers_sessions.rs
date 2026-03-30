use crate::metrics::{MetricEvent, Timer};
use crate::server::requests::{
    ListSessionsRequest, RelateSessionsRequest, ResumeSessionRequest, SearchSessionsRequest,
};
use crate::server::responses::{
    CheckpointInfo, ListSessionsResponse, RelateSessionsResponse, RelationshipEdge,
    ResumeSessionResponse, SearchResult, SearchSessionsResponse, SessionNode, SessionSummary,
    ThoughtSummary,
};

impl super::ReasoningServer {
    pub(super) async fn handle_list_sessions(
        &self,
        req: ListSessionsRequest,
    ) -> ListSessionsResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_list_sessions",
            limit = ?req.limit,
            offset = ?req.offset,
            mode_filter = ?req.mode_filter,
            "Listing reasoning sessions"
        );

        // Call memory::list function
        let result = crate::modes::memory::list_sessions(
            &self.state.storage,
            req.limit,
            req.offset,
            req.mode_filter,
        )
        .await;

        let elapsed_ms = timer.elapsed_ms();
        let success = result.is_ok();

        self.state
            .metrics
            .record(MetricEvent::new("list_sessions", elapsed_ms, success));

        match result {
            Ok((sessions, total, has_more)) => ListSessionsResponse {
                sessions: sessions
                    .into_iter()
                    .map(|s| SessionSummary {
                        session_id: s.session_id,
                        created_at: s.created_at,
                        updated_at: s.updated_at,
                        thought_count: s.thought_count,
                        preview: s.preview,
                        primary_mode: s.primary_mode,
                    })
                    .collect(),
                total,
                has_more,
                metadata: None,
            },
            Err(e) => {
                tracing::error!(
                    tool = "reasoning_list_sessions",
                    error = %e,
                    "Failed to list sessions"
                );
                ListSessionsResponse {
                    sessions: vec![],
                    total: 0,
                    has_more: false,
                    metadata: None,
                }
            }
        }
    }

    pub(super) async fn handle_resume(&self, req: ResumeSessionRequest) -> ResumeSessionResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_resume",
            session_id = %req.session_id,
            include_checkpoints = ?req.include_checkpoints,
            compress = ?req.compress,
            "Resuming reasoning session"
        );

        // Call memory::resume function
        let result = crate::modes::memory::resume_session(
            &self.state.storage,
            &self.state.client,
            &req.session_id,
            req.include_checkpoints.unwrap_or(false),
            req.compress.unwrap_or(false),
        )
        .await;

        let elapsed_ms = timer.elapsed_ms();
        let success = result.is_ok();

        self.state
            .metrics
            .record(MetricEvent::new("resume_session", elapsed_ms, success));

        match result {
            Ok(context) => ResumeSessionResponse {
                session_id: context.session_id,
                created_at: context.created_at,
                summary: context.summary,
                thought_chain: context
                    .thought_chain
                    .into_iter()
                    .map(|t| ThoughtSummary {
                        id: t.id,
                        mode: t.mode,
                        content: t.content,
                        confidence: t.confidence,
                    })
                    .collect(),
                key_conclusions: context.key_conclusions,
                last_mode: context.last_mode,
                checkpoint: context.checkpoint.map(|c| CheckpointInfo {
                    id: c.id,
                    name: c.name,
                    description: c.description,
                }),
                continuation_suggestions: context.continuation_suggestions,
                metadata: None,
            },
            Err(e) => {
                tracing::error!(
                    tool = "reasoning_resume",
                    error = %e,
                    "Failed to resume session"
                );
                ResumeSessionResponse {
                    session_id: req.session_id,
                    created_at: String::new(),
                    summary: format!(
                        "resume failed: {e}. \
                         Verify the session_id is from a previous reasoning session. \
                         Use reasoning_list_sessions to find valid session IDs."
                    ),
                    thought_chain: vec![],
                    key_conclusions: vec![],
                    last_mode: None,
                    checkpoint: None,
                    continuation_suggestions: vec![],
                    metadata: None,
                }
            }
        }
    }

    pub(super) async fn handle_search(&self, req: SearchSessionsRequest) -> SearchSessionsResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_search",
            query = %req.query,
            limit = ?req.limit,
            min_similarity = ?req.min_similarity,
            "Searching reasoning sessions"
        );

        // Call memory::search function
        let result = crate::modes::memory::search_sessions(
            &self.state.storage,
            &self.state.client,
            &req.query,
            req.limit.unwrap_or(10),
            req.min_similarity.unwrap_or(0.5),
            req.mode_filter,
        )
        .await;

        let elapsed_ms = timer.elapsed_ms();
        let success = result.is_ok();

        self.state
            .metrics
            .record(MetricEvent::new("search_sessions", elapsed_ms, success));

        match result {
            Ok(results) => SearchSessionsResponse {
                count: results.len() as u32,
                results: results
                    .into_iter()
                    .map(|r| SearchResult {
                        session_id: r.session_id,
                        similarity_score: r.similarity_score,
                        preview: r.preview,
                        created_at: r.created_at,
                        primary_mode: r.primary_mode,
                    })
                    .collect(),
                metadata: None,
            },
            Err(e) => {
                tracing::error!(
                    tool = "reasoning_search",
                    error = %e,
                    "Failed to search sessions"
                );
                SearchSessionsResponse {
                    results: vec![],
                    count: 0,
                    metadata: None,
                }
            }
        }
    }

    pub(super) async fn handle_relate(&self, req: RelateSessionsRequest) -> RelateSessionsResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_relate",
            session_id = ?req.session_id,
            depth = ?req.depth,
            min_strength = ?req.min_strength,
            "Analyzing session relationships"
        );

        // Call memory::relate function
        let result = crate::modes::memory::relate_sessions(
            &self.state.storage,
            &self.state.client,
            req.session_id,
            req.depth.unwrap_or(2),
            req.min_strength.unwrap_or(0.5),
        )
        .await;

        let elapsed_ms = timer.elapsed_ms();
        let success = result.is_ok();

        self.state
            .metrics
            .record(MetricEvent::new("relate_sessions", elapsed_ms, success));

        match result {
            Ok(graph) => RelateSessionsResponse {
                nodes: graph
                    .nodes
                    .into_iter()
                    .map(|n| SessionNode {
                        session_id: n.session_id,
                        preview: n.preview,
                        created_at: n.created_at,
                    })
                    .collect(),
                edges: graph
                    .edges
                    .into_iter()
                    .map(|e| RelationshipEdge {
                        from_session: e.from_session,
                        to_session: e.to_session,
                        relationship_type: format!("{:?}", e.relationship_type),
                        strength: e.strength,
                    })
                    .collect(),
                metadata: None,
            },
            Err(e) => {
                tracing::error!(
                    tool = "reasoning_relate",
                    error = %e,
                    "Failed to analyze relationships"
                );
                RelateSessionsResponse {
                    nodes: vec![],
                    edges: vec![],
                    metadata: None,
                }
            }
        }
    }
}
