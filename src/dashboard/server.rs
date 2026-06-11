//! Axum SSE sidecar that serves the live dashboard (feature `dashboard`).
//!
//! Three routes on a loopback port:
//! - `GET /` — the embedded single-file SPA.
//! - `GET /events` — a Server-Sent Events stream of [`ActivityEvent`]s, merging
//!   the activity bus with the existing progress bus (progress milestones become
//!   `Anthropic` activity, so the ②③ spine animates on streaming tools).
//! - `GET /health` — liveness.
//!
//! Read-only in v1: no write endpoints. The stream is lossy (a slow browser drops
//! events rather than slowing the server) and never touches stdout.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use futures_util::{Stream, StreamExt};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::server::ProgressEvent;

use super::bus::{now_ms, ActivityBus};
use super::config::DashboardConfig;
use super::event::{ActivityEvent, EdgeId, Node, Phase};

/// Shared sidecar state: the activity bus and a handle to the progress bus.
#[derive(Clone)]
struct DashboardState {
    activity: ActivityBus,
    progress_tx: broadcast::Sender<ProgressEvent>,
}

/// The embedded SPA, compiled into the binary.
const INDEX_HTML: &str = include_str!("ui/index.html");

/// Run the dashboard sidecar until the process exits.
///
/// Binds `config.addr` and serves the SPA + SSE stream. A bind failure is logged
/// and the sidecar simply does not start — it never aborts the main server.
pub async fn serve(
    config: DashboardConfig,
    activity: ActivityBus,
    progress_tx: broadcast::Sender<ProgressEvent>,
) {
    let state = DashboardState {
        activity,
        progress_tx,
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/events", get(events))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(&config.addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(
                addr = %config.addr,
                error = %e,
                "Dashboard failed to bind; sidecar not started"
            );
            return;
        }
    };
    tracing::info!(addr = %config.addr, "Dashboard sidecar listening");
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "Dashboard sidecar server error");
    }
}

/// Serve the embedded SPA.
async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// Liveness probe.
async fn health() -> &'static str {
    "ok"
}

/// Stream merged activity + progress events as SSE.
async fn events(
    State(state): State<DashboardState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let activity =
        BroadcastStream::new(state.activity.subscribe()).filter_map(|r| async move { r.ok() });
    let progress = BroadcastStream::new(state.progress_tx.subscribe())
        .filter_map(|r| async move { r.ok().map(progress_to_activity) });

    let merged = futures_util::stream::select(activity, progress).map(|ev| {
        Ok(Event::default()
            .json_data(&ev)
            .unwrap_or_else(|_| Event::default().comment("serialize error")))
    });

    Sse::new(merged).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// Map an internal progress milestone onto an `Anthropic` activity event so the
/// ②③ spine animates for streaming tools. Progress messages are static labels
/// (e.g. "Starting API call"), so copying one into `note` is redaction-safe.
fn progress_to_activity(ev: ProgressEvent) -> ActivityEvent {
    let phase = match ev.progress {
        0 => Phase::Started,
        100 => Phase::Completed,
        _ => Phase::Progress,
    };
    let mut out = ActivityEvent::new(Node::Anthropic, phase).with_edge(EdgeId::ModeToAnthropic);
    out.ts_ms = now_ms();
    if let Some(msg) = ev.message {
        out = out.with_note(msg);
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn progress_zero_maps_to_started() {
        let ev = progress_to_activity(ProgressEvent::new("t", 0).with_message("Preparing request"));
        assert_eq!(ev.node, Node::Anthropic);
        assert_eq!(ev.phase, Phase::Started);
        assert_eq!(ev.edge, Some(EdgeId::ModeToAnthropic));
        assert_eq!(ev.note.as_deref(), Some("Preparing request"));
        assert!(ev.ts_ms > 0);
    }

    #[test]
    fn progress_hundred_maps_to_completed() {
        let ev = progress_to_activity(ProgressEvent::new("t", 100));
        assert_eq!(ev.phase, Phase::Completed);
        assert!(ev.note.is_none());
    }

    #[test]
    fn progress_mid_maps_to_progress() {
        let ev =
            progress_to_activity(ProgressEvent::new("t", 15).with_message("Starting API call"));
        assert_eq!(ev.phase, Phase::Progress);
        assert_eq!(ev.note.as_deref(), Some("Starting API call"));
    }

    #[test]
    fn index_html_is_embedded_and_nonempty() {
        assert!(INDEX_HTML.contains("<!DOCTYPE html>"));
        assert!(INDEX_HTML.len() > 500);
    }
}
