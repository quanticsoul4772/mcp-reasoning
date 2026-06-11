//! Real-time end-to-end activity dashboard.
//!
//! An optional observability sidecar that animates the [end-to-end
//! flow](../../docs/reference/FLOW_OVERVIEW.md) on live activity: an operator
//! runs the server, opens the dashboard, and watches each request flow through
//! the e2e path plus the background loops firing.
//!
//! # Shape
//!
//! - [`event`] / [`bus`] — the [`ActivityEvent`] schema and the in-memory
//!   [`ActivityBus`] producers emit onto. **Always compiled**; emission is a
//!   no-op with zero subscribers, so the cost when the dashboard is off is one
//!   broadcast send per tool call.
//! - [`config`] — [`DashboardConfig`], off by default, loopback-bound, mirroring
//!   the `SELF_HEAL_*` env discipline.
//! - `server` — the axum SSE sidecar serving the embedded SPA. **Feature-gated**
//!   behind `--features dashboard` so the default binary carries no extra deps.
//!
//! Nothing here touches stdout: events ride an in-memory broadcast channel, so
//! the stdio MCP JSON-RPC channel is never disturbed. The sidecar is read-only
//! in v1.

pub mod bus;
pub mod config;
pub mod event;

#[cfg(feature = "dashboard")]
pub mod server;

pub use bus::{now_ms, ActivityBus, ACTIVITY_BUFFER};
pub use config::{DashboardConfig, DEFAULT_DASHBOARD_ADDR};
pub use event::{ActivityEvent, EdgeId, Node, Phase};
