//! Dashboard sidecar configuration.
//!
//! Mirrors the `SELF_HEAL_*` discipline: **off by default**, opt-in via a single
//! env flag, loopback-bound. When [`DashboardConfig::enabled`] is false the
//! server never opens a listener and the activity bus stays subscriber-less, so
//! the cost of the whole feature is one no-op broadcast send per tool call.

use std::env;
use std::net::SocketAddr;

/// Default: the dashboard sidecar is OFF.
pub const DEFAULT_DASHBOARD_ENABLED: bool = false;

/// Default bind address — loopback only (an unauthenticated read-only dev tool).
pub const DEFAULT_DASHBOARD_ADDR: &str = "127.0.0.1:3777";

/// Configuration for the real-time activity dashboard sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardConfig {
    /// Whether to start the dashboard sidecar. Default `false`.
    pub enabled: bool,
    /// Address to bind the sidecar HTTP listener to. Default `127.0.0.1:3777`.
    pub addr: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_DASHBOARD_ENABLED,
            addr: DEFAULT_DASHBOARD_ADDR.to_string(),
        }
    }
}

impl DashboardConfig {
    /// Load configuration from the environment.
    ///
    /// Environment variables:
    /// - `MCP_DASHBOARD`: `true` to enable the sidecar (default: `false`).
    /// - `MCP_DASHBOARD_ADDR`: bind address (default: `127.0.0.1:3777`).
    #[must_use]
    pub fn from_env() -> Self {
        let enabled = env::var("MCP_DASHBOARD")
            .map_or(DEFAULT_DASHBOARD_ENABLED, |v| v.to_lowercase() == "true");
        let addr = env::var("MCP_DASHBOARD_ADDR")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_DASHBOARD_ADDR.to_string());
        Self { enabled, addr }
    }

    /// Whether `addr` parses to a loopback socket address.
    ///
    /// An unparseable or non-loopback address returns `false` so the caller can
    /// warn before binding an unauthenticated dev tool beyond localhost.
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        self.addr
            .parse::<SocketAddr>()
            .is_ok_and(|sa| sa.ip().is_loopback())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_env() {
        env::remove_var("MCP_DASHBOARD");
        env::remove_var("MCP_DASHBOARD_ADDR");
    }

    #[test]
    fn default_is_off_and_loopback() {
        let c = DashboardConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.addr, DEFAULT_DASHBOARD_ADDR);
        assert!(c.is_loopback());
    }

    #[test]
    #[serial]
    fn from_env_defaults_when_unset() {
        clear_env();
        let c = DashboardConfig::from_env();
        assert!(!c.enabled);
        assert_eq!(c.addr, DEFAULT_DASHBOARD_ADDR);
        clear_env();
    }

    #[test]
    #[serial]
    fn from_env_enables_and_reads_addr() {
        clear_env();
        env::set_var("MCP_DASHBOARD", "true");
        env::set_var("MCP_DASHBOARD_ADDR", "127.0.0.1:9999");
        let c = DashboardConfig::from_env();
        assert!(c.enabled);
        assert_eq!(c.addr, "127.0.0.1:9999");
        assert!(c.is_loopback());
        clear_env();
    }

    #[test]
    #[serial]
    fn from_env_enabled_is_case_insensitive_and_strict() {
        clear_env();
        env::set_var("MCP_DASHBOARD", "TRUE");
        assert!(DashboardConfig::from_env().enabled);
        env::set_var("MCP_DASHBOARD", "1");
        assert!(!DashboardConfig::from_env().enabled); // only "true" enables
        env::set_var("MCP_DASHBOARD", "yes");
        assert!(!DashboardConfig::from_env().enabled);
        clear_env();
    }

    #[test]
    #[serial]
    fn from_env_blank_addr_falls_back_to_default() {
        clear_env();
        env::set_var("MCP_DASHBOARD_ADDR", "   ");
        let c = DashboardConfig::from_env();
        assert_eq!(c.addr, DEFAULT_DASHBOARD_ADDR);
        clear_env();
    }

    #[test]
    fn is_loopback_detects_non_loopback() {
        let c = DashboardConfig {
            enabled: true,
            addr: "0.0.0.0:3777".to_string(),
        };
        assert!(!c.is_loopback());
    }

    #[test]
    fn is_loopback_false_for_unparseable_addr() {
        let c = DashboardConfig {
            enabled: true,
            addr: "not-an-address".to_string(),
        };
        assert!(!c.is_loopback());
    }

    #[test]
    fn is_loopback_true_for_ipv6_loopback() {
        let c = DashboardConfig {
            enabled: true,
            addr: "[::1]:3777".to_string(),
        };
        assert!(c.is_loopback());
    }
}
