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
    // Mock Anthropic: a streaming (SSE) body, since counterfactual uses
    // complete_streaming. Content need not parse — the early milestones
    // (RequestPrepared/ApiCallStarted/...) fire regardless.
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(anthropic_sse_response("{\"levels\": []}")),
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
    let first = first.expect("channel not closed");

    // It carries a percent and a status label — milestone progress, not content.
    assert!(first.progress >= 0.0);
    assert!(
        first.message.is_some(),
        "milestone should carry a status label"
    );
}
