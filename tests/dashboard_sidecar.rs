//! End-to-end test for the dashboard sidecar (feature `dashboard`).
//!
//! Proves the live pipe: bind the axum sidecar on a free loopback port, then
//! confirm `/health` answers, the embedded SPA is served at `/`, and an event
//! emitted on the [`ActivityBus`] arrives over the `/events` SSE stream as JSON.
//! This is the wire-level counterpart to the in-crate unit tests.

#![cfg(feature = "dashboard")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures_util::StreamExt;
use mcp_reasoning::dashboard::server::serve;
use mcp_reasoning::dashboard::{ActivityBus, ActivityEvent, DashboardConfig, EdgeId, Node, Phase};
use mcp_reasoning::server::create_progress_channel;

/// Find a free loopback port by binding to :0 and releasing it.
fn free_loopback_addr() -> String {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("probe addr");
    drop(probe);
    addr.to_string()
}

/// Poll `GET {base}/health` until it returns 200 or the timeout elapses.
async fn wait_until_up(client: &reqwest::Client, base: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(resp) = client.get(format!("{base}/health")).send().await {
            if resp.status().is_success() {
                return;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "dashboard sidecar did not come up in time"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn sidecar_serves_spa_health_and_streams_events() {
    let addr = free_loopback_addr();
    let base = format!("http://{addr}");
    let config = DashboardConfig {
        enabled: true,
        addr: addr.clone(),
    };

    let activity = ActivityBus::new();
    let (progress_tx, _progress_rx) = create_progress_channel();

    // Start the sidecar.
    let serve_activity = activity.clone();
    let serve_progress = progress_tx.clone();
    tokio::spawn(async move {
        serve(config, serve_activity, serve_progress).await;
    });

    let client = reqwest::Client::new();
    wait_until_up(&client, &base).await;

    // /health
    let health = client.get(format!("{base}/health")).send().await.unwrap();
    assert!(health.status().is_success());
    assert_eq!(health.text().await.unwrap(), "ok");

    // / serves the embedded SPA
    let index = client.get(format!("{base}/")).send().await.unwrap();
    assert!(index.status().is_success());
    let body = index.text().await.unwrap();
    assert!(body.to_lowercase().contains("<!doctype html>"));
    assert!(body.contains("live activity"));
    assert!(body.contains("<div id=\"root\">"));

    // /events streams an emitted ActivityEvent as JSON.
    let resp = client.get(format!("{base}/events")).send().await.unwrap();
    assert!(resp.status().is_success());
    let mut stream = resp.bytes_stream();

    // Subscriber is connected; emit a distinctive event.
    let bus = activity.clone();
    tokio::spawn(async move {
        // Small delay so the SSE handler has subscribed before we emit.
        tokio::time::sleep(Duration::from_millis(50)).await;
        bus.emit(
            ActivityEvent::new(Node::Mode, Phase::Completed)
                .with_edge(EdgeId::ModeToClient)
                .with_tool("reasoning_linear")
                .with_duration_ms(123),
        );
    });

    // Read SSE chunks until we see our event's data line (or time out).
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.expect("sse chunk");
            buf.push_str(&String::from_utf8_lossy(&bytes));
            if buf.contains("reasoning_linear") {
                return true;
            }
        }
        false
    })
    .await
    .expect("timed out waiting for SSE event");

    assert!(found, "did not receive the emitted activity event over SSE");
}
