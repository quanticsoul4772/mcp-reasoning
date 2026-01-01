//! Progress notification infrastructure for streaming responses.
//!
//! This module provides:
//! - [`ProgressEvent`]: Events sent during long-running operations
//! - [`ProgressReporter`]: Helper for reporting progress milestones
//! - Broadcast channel integration for progress distribution

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Progress event emitted during long-running operations.
///
/// These events are sent through a broadcast channel to notify
/// clients of operation progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Unique token identifying this operation.
    pub token: String,
    /// Current progress (0-100 percentage).
    pub progress: u32,
    /// Optional total steps (for discrete progress).
    pub total: Option<u32>,
    /// Optional human-readable status message.
    pub message: Option<String>,
}

impl ProgressEvent {
    /// Create a new progress event.
    #[must_use]
    pub fn new(token: impl Into<String>, progress: u32) -> Self {
        Self {
            token: token.into(),
            progress: progress.min(100),
            total: None,
            message: None,
        }
    }

    /// Set the total steps.
    #[must_use]
    pub fn with_total(mut self, total: u32) -> Self {
        self.total = Some(total);
        self
    }

    /// Set a status message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Helper for reporting progress during long-running operations.
///
/// This struct simplifies sending progress events at key milestones.
/// It maintains the operation token and broadcast sender internally.
#[derive(Clone)]
pub struct ProgressReporter {
    token: String,
    total: Option<u32>,
    tx: broadcast::Sender<ProgressEvent>,
}

impl ProgressReporter {
    /// Create a new progress reporter.
    ///
    /// # Arguments
    ///
    /// * `token` - Unique identifier for this operation
    /// * `tx` - Broadcast sender for progress events
    #[must_use]
    pub fn new(token: impl Into<String>, tx: broadcast::Sender<ProgressEvent>) -> Self {
        Self {
            token: token.into(),
            total: None,
            tx,
        }
    }

    /// Set the total number of steps.
    #[must_use]
    pub fn with_total(mut self, total: u32) -> Self {
        self.total = Some(total);
        self
    }

    /// Report progress as a percentage (0-100).
    ///
    /// # Arguments
    ///
    /// * `percent` - Progress percentage (clamped to 0-100)
    /// * `message` - Optional status message
    pub fn report_percent(&self, percent: u32, message: Option<&str>) {
        let mut event = ProgressEvent::new(&self.token, percent);

        if let Some(total) = self.total {
            event = event.with_total(total);
        }

        if let Some(msg) = message {
            event = event.with_message(msg);
        }

        // Best-effort send - don't block if no receivers
        let _ = self.tx.send(event);
    }

    /// Report starting (0%).
    pub fn report_started(&self, message: Option<&str>) {
        self.report_percent(0, message);
    }

    /// Report completion (100%).
    pub fn report_completed(&self, message: Option<&str>) {
        self.report_percent(100, message);
    }

    /// Report a key milestone.
    ///
    /// Common milestones:
    /// - 5%: Request prepared
    /// - 15%: API call started
    /// - 90%: Processing response
    /// - 100%: Complete
    pub fn report_milestone(&self, milestone: ProgressMilestone) {
        let (percent, message) = match milestone {
            ProgressMilestone::RequestPrepared => (5, "Preparing request"),
            ProgressMilestone::ApiCallStarted => (15, "Starting API call"),
            ProgressMilestone::StreamingStarted => (20, "Receiving response"),
            ProgressMilestone::ProcessingResponse => (90, "Processing response"),
            ProgressMilestone::Complete => (100, "Complete"),
        };
        self.report_percent(percent, Some(message));
    }

    /// Get the operation token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl std::fmt::Debug for ProgressReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressReporter")
            .field("token", &self.token)
            .field("total", &self.total)
            .finish_non_exhaustive()
    }
}

/// Standard progress milestones for reasoning operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMilestone {
    /// Request has been prepared (5%).
    RequestPrepared,
    /// API call has started (15%).
    ApiCallStarted,
    /// Streaming response started (20%).
    StreamingStarted,
    /// Processing the response (90%).
    ProcessingResponse,
    /// Operation complete (100%).
    Complete,
}

/// Create a new broadcast channel for progress events.
///
/// Returns a sender that can be cloned into `AppState` and
/// multiple receivers for clients.
#[must_use]
pub fn create_progress_channel() -> (broadcast::Sender<ProgressEvent>, broadcast::Receiver<ProgressEvent>) {
    broadcast::channel(100) // Buffer 100 events
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::unused_async
)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_event_new() {
        let event = ProgressEvent::new("token-1", 50);
        assert_eq!(event.token, "token-1");
        assert_eq!(event.progress, 50);
        assert!(event.total.is_none());
        assert!(event.message.is_none());
    }

    #[test]
    fn test_progress_event_clamps_progress() {
        let event = ProgressEvent::new("token", 150);
        assert_eq!(event.progress, 100);
    }

    #[test]
    fn test_progress_event_with_total() {
        let event = ProgressEvent::new("token", 25).with_total(4);
        assert_eq!(event.total, Some(4));
    }

    #[test]
    fn test_progress_event_with_message() {
        let event = ProgressEvent::new("token", 50).with_message("Processing...");
        assert_eq!(event.message, Some("Processing...".to_string()));
    }

    #[test]
    fn test_progress_reporter_new() {
        let (tx, _rx) = broadcast::channel(10);
        let reporter = ProgressReporter::new("test-token", tx);
        assert_eq!(reporter.token(), "test-token");
    }

    #[test]
    fn test_progress_reporter_with_total() {
        let (tx, _rx) = broadcast::channel(10);
        let reporter = ProgressReporter::new("test-token", tx).with_total(10);
        assert_eq!(reporter.total, Some(10));
    }

    #[tokio::test]
    async fn test_progress_reporter_report_percent() {
        let (tx, mut rx) = broadcast::channel(10);
        let reporter = ProgressReporter::new("test-token", tx);

        reporter.report_percent(50, Some("Halfway"));

        let event = rx.recv().await.unwrap();
        assert_eq!(event.token, "test-token");
        assert_eq!(event.progress, 50);
        assert_eq!(event.message, Some("Halfway".to_string()));
    }

    #[tokio::test]
    async fn test_progress_reporter_report_started() {
        let (tx, mut rx) = broadcast::channel(10);
        let reporter = ProgressReporter::new("test-token", tx);

        reporter.report_started(Some("Starting..."));

        let event = rx.recv().await.unwrap();
        assert_eq!(event.progress, 0);
        assert_eq!(event.message, Some("Starting...".to_string()));
    }

    #[tokio::test]
    async fn test_progress_reporter_report_completed() {
        let (tx, mut rx) = broadcast::channel(10);
        let reporter = ProgressReporter::new("test-token", tx);

        reporter.report_completed(Some("Done!"));

        let event = rx.recv().await.unwrap();
        assert_eq!(event.progress, 100);
        assert_eq!(event.message, Some("Done!".to_string()));
    }

    #[tokio::test]
    async fn test_progress_reporter_report_milestone() {
        let (tx, mut rx) = broadcast::channel(10);
        let reporter = ProgressReporter::new("test-token", tx);

        reporter.report_milestone(ProgressMilestone::ApiCallStarted);

        let event = rx.recv().await.unwrap();
        assert_eq!(event.progress, 15);
        assert_eq!(event.message, Some("Starting API call".to_string()));
    }

    #[test]
    fn test_progress_reporter_debug() {
        let (tx, _rx) = broadcast::channel(10);
        let reporter = ProgressReporter::new("test-token", tx).with_total(5);
        let debug = format!("{:?}", reporter);
        assert!(debug.contains("ProgressReporter"));
        assert!(debug.contains("test-token"));
    }

    #[test]
    fn test_create_progress_channel() {
        let (tx, _rx) = create_progress_channel();
        // Verify we can subscribe
        let _rx2 = tx.subscribe();
    }

    #[test]
    fn test_progress_event_serialize() {
        let event = ProgressEvent::new("token-1", 50)
            .with_total(4)
            .with_message("Processing...");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("token-1"));
        assert!(json.contains("50"));
        assert!(json.contains("Processing"));
    }

    #[test]
    fn test_progress_milestone_values() {
        assert_eq!(ProgressMilestone::RequestPrepared, ProgressMilestone::RequestPrepared);
        assert_ne!(ProgressMilestone::ApiCallStarted, ProgressMilestone::Complete);
    }
}
