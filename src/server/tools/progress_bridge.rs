//! Bridge from internal milestone events to MCP `notifications/progress`.
//!
//! Modes emit milestones through a [`ProgressReporter`](crate::server::progress::ProgressReporter)
//! onto the shared `progress_tx` broadcast bus — that keeps the mode layer free of
//! any rmcp dependency. This module supplies the missing last mile: at the tool
//! boundary, when the **client opted in** by sending a progress token in the
//! request `_meta`, this call's milestones are forwarded to the client's
//! [`Peer`] as `notifications/progress`. Without a client token, nothing is sent
//! (per the MCP spec) and the handler runs with zero added overhead.

use std::future::Future;

use rmcp::model::{ProgressNotificationParam, ProgressToken};
use rmcp::service::RoleServer;
use rmcp::Peer;
use tokio::sync::broadcast::error::RecvError;

use crate::server::progress::ProgressEvent;

/// Map an internal [`ProgressEvent`] (percent 0–100 + status label) to an MCP
/// [`ProgressNotificationParam`] addressed to the client's progress token.
#[must_use]
pub fn to_progress_param(ev: &ProgressEvent, token: &ProgressToken) -> ProgressNotificationParam {
    let mut param = ProgressNotificationParam::new(token.clone(), f64::from(ev.progress));
    param.total = ev.total.map(f64::from);
    param.message = ev.message.clone();
    param
}

/// Ensure `slot` holds a progress token, generating a unique prefixed one when
/// absent, and return it. The handler tags its `ProgressReporter` with the same
/// token, so the forwarder can correlate this call's milestones on the shared bus.
#[must_use]
pub fn ensure_progress_token(slot: &mut Option<String>, prefix: &str) -> String {
    if let Some(existing) = slot {
        return existing.clone();
    }
    let token = format!("{prefix}{}", uuid::Uuid::new_v4());
    *slot = Some(token.clone());
    token
}

impl super::ReasoningServer {
    /// Run `fut`, forwarding this call's progress milestones to the client as MCP
    /// `notifications/progress` — but only when `client_token` is `Some` (the
    /// client opted in via the request `_meta`). Otherwise `fut` runs unchanged.
    ///
    /// Milestones are correlated by `internal_token`: the handler tags its
    /// `ProgressReporter` with the same token, so only this call's events are
    /// forwarded even though the broadcast bus is shared across concurrent calls.
    pub(super) async fn with_progress<Fut, R>(
        &self,
        peer: Peer<RoleServer>,
        client_token: Option<ProgressToken>,
        internal_token: String,
        fut: Fut,
    ) -> R
    where
        Fut: Future<Output = R>,
    {
        let Some(client_token) = client_token else {
            return fut.await;
        };

        // Subscribe before the handler runs so early milestones aren't missed.
        let mut rx = self.state.progress_tx.subscribe();
        tokio::pin!(fut);

        loop {
            tokio::select! {
                result = &mut fut => return result,
                event = rx.recv() => match event {
                    Ok(ev) if ev.token == internal_token => {
                        // Best-effort: a failed notify must not fail the tool call.
                        let _ = peer
                            .notify_progress(to_progress_param(&ev, &client_token))
                            .await;
                    }
                    // Another concurrent call's milestone — ignore.
                    Ok(_) => {}
                    // Forwarder fell behind the bus; drop the gap and continue.
                    Err(RecvError::Lagged(_)) => {}
                    // Bus closed (only at shutdown): stop forwarding, finish the call.
                    Err(RecvError::Closed) => return (&mut fut).await,
                },
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use rmcp::model::NumberOrString;

    use super::*;

    fn token(s: &str) -> ProgressToken {
        ProgressToken(NumberOrString::String(s.to_string().into()))
    }

    #[test]
    fn test_to_progress_param_maps_fields() {
        let ev = ProgressEvent::new("mcts-1", 15)
            .with_total(100)
            .with_message("Starting API call");
        let param = to_progress_param(&ev, &token("client-tok"));

        assert_eq!(param.progress, 15.0);
        assert_eq!(param.total, Some(100.0));
        assert_eq!(param.message, Some("Starting API call".to_string()));
        assert_eq!(param.progress_token, token("client-tok"));
    }

    #[test]
    fn test_to_progress_param_no_total_no_message() {
        let ev = ProgressEvent::new("t", 50);
        let param = to_progress_param(&ev, &token("c"));
        assert_eq!(param.progress, 50.0);
        assert!(param.total.is_none());
        assert!(param.message.is_none());
    }

    #[test]
    fn test_ensure_progress_token_generates_when_absent() {
        let mut slot = None;
        let tok = ensure_progress_token(&mut slot, "divergent-");
        assert!(tok.starts_with("divergent-"));
        assert_eq!(slot, Some(tok));
    }

    #[test]
    fn test_ensure_progress_token_preserves_existing() {
        let mut slot = Some("caller-supplied".to_string());
        let tok = ensure_progress_token(&mut slot, "divergent-");
        assert_eq!(tok, "caller-supplied");
        assert_eq!(slot, Some("caller-supplied".to_string()));
    }
}
