//! In-memory broadcast bus for [`ActivityEvent`]s.
//!
//! [`ActivityBus`] wraps a `tokio::broadcast` sender. Producers call
//! [`ActivityBus::emit`] at flow seams; the dashboard sidecar (when enabled)
//! subscribes a receiver per browser connection. Emission is **best-effort and
//! non-blocking**: when no one is subscribed (the dashboard is off, the default)
//! the send is a cheap no-op with zero overhead, and a slow subscriber that lags
//! drops oldest events rather than blocking the server.
//!
//! Nothing here touches stdout, so activity emission can never disturb the stdio
//! MCP channel.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;

use super::event::ActivityEvent;

/// Broadcast buffer size. Lossy by design: a lagging subscriber drops the oldest
/// events instead of applying backpressure to the server.
pub const ACTIVITY_BUFFER: usize = 256;

/// A cloneable handle to the activity broadcast channel.
///
/// Clones share the same channel and the same monotonic id counter, so every
/// emitted event gets a process-unique, ordered `id` regardless of which clone
/// emitted it.
#[derive(Debug, Clone)]
pub struct ActivityBus {
    tx: broadcast::Sender<ActivityEvent>,
    seq: Arc<AtomicU64>,
}

impl ActivityBus {
    /// Create a new bus with no subscribers.
    #[must_use]
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(ACTIVITY_BUFFER);
        Self {
            tx,
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Subscribe a new receiver for the event stream.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<ActivityEvent> {
        self.tx.subscribe()
    }

    /// Number of currently-subscribed receivers (0 when the dashboard is off).
    #[must_use]
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Stamp `event` with a monotonic id and the current timestamp, then publish.
    ///
    /// Best-effort: a send with no subscribers returns an error that is
    /// intentionally ignored — emission must never fail a caller or block.
    pub fn emit(&self, mut event: ActivityEvent) {
        event.id = self.seq.fetch_add(1, Ordering::Relaxed);
        event.ts_ms = now_ms();
        let _ = self.tx.send(event);
    }
}

impl Default for ActivityBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Current time in epoch milliseconds (0 if the system clock predates the epoch).
#[must_use]
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::dashboard::event::{Node, Phase};

    #[test]
    fn new_bus_has_no_subscribers() {
        let bus = ActivityBus::new();
        assert_eq!(bus.receiver_count(), 0);
    }

    #[test]
    fn default_matches_new() {
        let bus = ActivityBus::default();
        assert_eq!(bus.receiver_count(), 0);
    }

    #[test]
    fn emit_with_no_subscribers_is_a_noop() {
        // The whole point of "off by default": emitting must not panic or block
        // when nobody is listening.
        let bus = ActivityBus::new();
        bus.emit(ActivityEvent::new(Node::Mode, Phase::Completed));
        bus.emit(ActivityEvent::new(Node::Anthropic, Phase::Started));
        assert_eq!(bus.receiver_count(), 0);
    }

    #[tokio::test]
    async fn emit_stamps_monotonic_ids_and_timestamp() {
        let bus = ActivityBus::new();
        let mut rx = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        bus.emit(ActivityEvent::new(Node::Mode, Phase::Completed));
        bus.emit(ActivityEvent::new(Node::Mode, Phase::Completed));

        let first = rx.recv().await.expect("first event");
        let second = rx.recv().await.expect("second event");
        assert_eq!(first.id, 0);
        assert_eq!(second.id, 1);
        assert!(first.ts_ms > 0);
        assert!(second.ts_ms >= first.ts_ms);
    }

    #[tokio::test]
    async fn clones_share_the_id_counter() {
        let bus = ActivityBus::new();
        let clone = bus.clone();
        let mut rx = bus.subscribe();

        bus.emit(ActivityEvent::new(Node::Sqlite, Phase::Completed));
        clone.emit(ActivityEvent::new(Node::Sqlite, Phase::Completed));

        let a = rx.recv().await.expect("a");
        let b = rx.recv().await.expect("b");
        // Distinct, ordered ids even though two different handles emitted.
        assert_eq!(a.id, 0);
        assert_eq!(b.id, 1);
    }

    #[tokio::test]
    async fn delivers_to_multiple_subscribers() {
        let bus = ActivityBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);

        bus.emit(ActivityEvent::new(Node::Worker, Phase::Started));

        let e1 = rx1.recv().await.expect("rx1");
        let e2 = rx2.recv().await.expect("rx2");
        assert_eq!(e1.node, Node::Worker);
        assert_eq!(e2.node, Node::Worker);
    }

    #[test]
    fn now_ms_is_positive() {
        assert!(now_ms() > 0);
    }
}
