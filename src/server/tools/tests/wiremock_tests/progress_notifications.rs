//! End-to-end proof that streaming modes deliver milestone progress to the client
//! as MCP `notifications/progress`.
//!
//! This wires a real rmcp client to the `ReasoningServer` over an in-process duplex
//! transport (Anthropic mocked via wiremock). The rmcp client auto-attaches a
//! progress token to every request, so a streaming tool call exercises the
//! `progress_bridge` forwarder; a custom `ClientHandler` captures the resulting
//! notifications.

use std::future::Future;

use rmcp::model::{CallToolRequestParams, ProgressNotificationParam};
use rmcp::service::{NotificationContext, RoleClient};
use rmcp::{ClientHandler, ServiceExt};
use tokio::sync::mpsc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_sse_response, create_mocked_server};

/// Client handler that records every progress notification it receives.
#[derive(Clone)]
struct ProgressCapture {
    tx: mpsc::UnboundedSender<ProgressNotificationParam>,
}

impl ClientHandler for ProgressCapture {
    fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        let _ = self.tx.send(params);
        std::future::ready(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn counterfactual_delivers_progress_notifications_to_client() {
    // Mock Anthropic with a streaming (SSE) body carrying a valid counterfactual
    // payload, so the handler runs to completion and emits its final 100%
    // Complete milestone (which the bridge must flush, not drop).
    let cf_json = serde_json::json!({
        "causal_question": {
            "statement": "Does X cause Y?",
            "ladder_rung": "counterfactual",
            "variables": {"cause": "X", "effect": "Y", "intervention": "remove X"}
        },
        "causal_model": {
            "nodes": ["X", "Y", "Z"],
            "edges": [
                {"from": "X", "to": "Y", "type": "direct"},
                {"from": "Z", "to": "X", "type": "confounded"},
                {"from": "Z", "to": "Y", "type": "confounded"}
            ],
            "confounders": ["Z"]
        },
        "analysis": {
            "association_level": {"observed_correlation": 0.7, "interpretation": "Confounded by Z"},
            "intervention_level": {"causal_effect": 0.4, "mechanism": "X raises Y"},
            "counterfactual_level": {"scenario": "If X removed", "outcome": "Y lower", "confidence": 0.6}
        },
        "conclusions": {
            "causal_claim": "X contributes ~0.4 to Y",
            "strength": "moderate",
            "caveats": ["Z confounds the correlation"],
            "actionable_insight": "Run an A/B test isolating X"
        }
    })
    .to_string();
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(anthropic_sse_response(&cf_json)),
        )
        .mount(&mock)
        .await;

    let server = create_mocked_server(&mock).await;

    // In-process transport: two ends of a duplex pipe.
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);

    // Drive the server handshake concurrently with the client handshake.
    let server_task = tokio::spawn(async move { server.serve(server_io).await });

    let (tx, mut rx) = mpsc::unbounded_channel();
    let client = ProgressCapture { tx }
        .serve(client_io)
        .await
        .expect("client init");
    let _server = server_task
        .await
        .expect("join server")
        .expect("server init");

    // The rmcp client auto-attaches a progress token to this request.
    let mut params = CallToolRequestParams::new("reasoning_counterfactual");
    params.arguments = serde_json::json!({
        "scenario": "The deploy succeeded",
        "intervention": "What if the migration had been skipped?",
        "analysis_depth": "counterfactual"
    })
    .as_object()
    .cloned();

    let result = client.call_tool(params).await.expect("call_tool");
    // The MCP call itself succeeds (handler returns a response object).
    assert!(!result.is_error.unwrap_or(false));

    // At least one progress notification must have reached the client.
    let first = tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv())
        .await
        .expect("a progress notification within timeout");
    let mut received = vec![first.expect("channel not closed")];

    // Collect the remaining buffered notifications (all are delivered before the
    // tool response, so they're already queued).
    while let Ok(Some(p)) =
        tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
    {
        received.push(p);
    }

    // Each milestone carries a percent and a status label — progress, not content.
    assert!(received.iter().all(|p| p.progress >= 0.0));
    assert!(received.iter().all(|p| p.message.is_some()));

    // The final 100% Complete tick must be delivered (the bug this fix closes:
    // it is emitted just before the handler returns and was previously dropped).
    let progresses: Vec<f64> = received.iter().map(|p| p.progress).collect();
    assert!(
        received
            .iter()
            .any(|p| (p.progress - 100.0).abs() < f64::EPSILON),
        "expected a 100% Complete notification, got {progresses:?}"
    );
}
