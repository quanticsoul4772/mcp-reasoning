//! Self-improvement system configuration.
//!
//! This module provides configuration for the self-improvement system.
//!
//! **NOTE**: Self-improvement is ALWAYS enabled. It is a core feature, not optional.
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::config::SelfImprovementConfig;
//!
//! // Use defaults
//! let config = SelfImprovementConfig::default();
//! assert!(!config.require_approval); // Auto-approve by default
//!
//! // Load from environment
//! let config = SelfImprovementConfig::from_env();
//! println!("Cycle interval: {}s", config.cycle_interval_secs);
//! ```

use std::env;

/// Default: auto-approve actions (no human approval required).
pub const DEFAULT_REQUIRE_APPROVAL: bool = false;

/// Default: minimum invocations before analysis runs.
pub const DEFAULT_MIN_INVOCATIONS: u64 = 10;

/// Default: interval between automatic cycles in seconds (5 minutes).
pub const DEFAULT_CYCLE_INTERVAL_SECS: u64 = 300;

/// Default: maximum actions per cycle.
pub const DEFAULT_MAX_ACTIONS_PER_CYCLE: u32 = 3;

/// Default: circuit breaker failure threshold.
pub const DEFAULT_CIRCUIT_BREAKER_THRESHOLD: u32 = 3;

/// Maximum allowed cycle interval (1 hour).
pub const MAX_CYCLE_INTERVAL_SECS: u64 = 3600;

/// Minimum allowed cycle interval (30 seconds).
pub const MIN_CYCLE_INTERVAL_SECS: u64 = 30;

/// Maximum allowed actions per cycle.
pub const MAX_ACTIONS_PER_CYCLE: u32 = 10;

/// Maximum circuit breaker threshold.
pub const MAX_CIRCUIT_BREAKER_THRESHOLD: u32 = 10;

/// Self-improvement system configuration.
///
/// **NOTE**: Self-improvement is ALWAYS enabled. It is a core feature, not optional.
/// There is no `enabled` flag - the system runs automatically whenever the server runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfImprovementConfig {
    /// Require human approval before executing actions.
    ///
    /// When `true` (default), diagnoses are queued for approval via MCP tools.
    /// When `false`, actions are auto-executed after analysis.
    pub require_approval: bool,

    /// Minimum invocations before analysis runs.
    ///
    /// The system won't analyze until at least this many tool invocations
    /// have been recorded. Default: 50.
    pub min_invocations_for_analysis: u64,

    /// Interval between automatic improvement cycles in seconds.
    ///
    /// The background task runs improvement cycles at this interval.
    /// Default: 300 (5 minutes).
    pub cycle_interval_secs: u64,

    /// Maximum actions per cycle.
    ///
    /// Limits how many improvement actions can be executed in a single cycle.
    /// Default: 3.
    pub max_actions_per_cycle: u32,

    /// Circuit breaker failure threshold.
    ///
    /// After this many consecutive failures, the system halts until reset.
    /// Default: 3.
    pub circuit_breaker_threshold: u32,
}

impl Default for SelfImprovementConfig {
    fn default() -> Self {
        Self {
            // No "enabled" flag - always on!
            require_approval: DEFAULT_REQUIRE_APPROVAL,
            min_invocations_for_analysis: DEFAULT_MIN_INVOCATIONS,
            cycle_interval_secs: DEFAULT_CYCLE_INTERVAL_SECS,
            max_actions_per_cycle: DEFAULT_MAX_ACTIONS_PER_CYCLE,
            circuit_breaker_threshold: DEFAULT_CIRCUIT_BREAKER_THRESHOLD,
        }
    }
}

impl SelfImprovementConfig {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `SELF_IMPROVEMENT_REQUIRE_APPROVAL`: `true` or `false` (default: `true`)
    /// - `SELF_IMPROVEMENT_MIN_INVOCATIONS`: minimum invocations (default: 50)
    /// - `SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS`: cycle interval (default: 300)
    /// - `SELF_IMPROVEMENT_MAX_ACTIONS`: max actions per cycle (default: 3)
    /// - `SELF_IMPROVEMENT_CIRCUIT_BREAKER_THRESHOLD`: failure threshold (default: 3)
    #[must_use]
    pub fn from_env() -> Self {
        let require_approval = env::var("SELF_IMPROVEMENT_REQUIRE_APPROVAL")
            .map(|v| v.to_lowercase() != "false")
            .unwrap_or(DEFAULT_REQUIRE_APPROVAL);

        let min_invocations_for_analysis = env::var("SELF_IMPROVEMENT_MIN_INVOCATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MIN_INVOCATIONS);

        let cycle_interval_secs = env::var("SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map_or(DEFAULT_CYCLE_INTERVAL_SECS, |v: u64| {
                v.clamp(MIN_CYCLE_INTERVAL_SECS, MAX_CYCLE_INTERVAL_SECS)
            });

        let max_actions_per_cycle = env::var("SELF_IMPROVEMENT_MAX_ACTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map_or(DEFAULT_MAX_ACTIONS_PER_CYCLE, |v: u32| {
                v.min(MAX_ACTIONS_PER_CYCLE)
            });

        let circuit_breaker_threshold = env::var("SELF_IMPROVEMENT_CIRCUIT_BREAKER_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .map_or(DEFAULT_CIRCUIT_BREAKER_THRESHOLD, |v: u32| {
                v.min(MAX_CIRCUIT_BREAKER_THRESHOLD)
            });

        Self {
            require_approval,
            min_invocations_for_analysis,
            cycle_interval_secs,
            max_actions_per_cycle,
            circuit_breaker_threshold,
        }
    }

    /// Validate the configuration values.
    ///
    /// Returns `true` if all values are within valid ranges.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.cycle_interval_secs >= MIN_CYCLE_INTERVAL_SECS
            && self.cycle_interval_secs <= MAX_CYCLE_INTERVAL_SECS
            && self.max_actions_per_cycle <= MAX_ACTIONS_PER_CYCLE
            && self.max_actions_per_cycle > 0
            && self.circuit_breaker_threshold <= MAX_CIRCUIT_BREAKER_THRESHOLD
            && self.circuit_breaker_threshold > 0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Helper to clear all SI-related env vars.
    fn clear_si_env_vars() {
        env::remove_var("SELF_IMPROVEMENT_REQUIRE_APPROVAL");
        env::remove_var("SELF_IMPROVEMENT_MIN_INVOCATIONS");
        env::remove_var("SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS");
        env::remove_var("SELF_IMPROVEMENT_MAX_ACTIONS");
        env::remove_var("SELF_IMPROVEMENT_CIRCUIT_BREAKER_THRESHOLD");
    }

    #[test]
    fn test_default_values() {
        let config = SelfImprovementConfig::default();

        // Default: auto-approve actions (no human approval required)
        assert!(!config.require_approval);
        assert_eq!(config.min_invocations_for_analysis, DEFAULT_MIN_INVOCATIONS);
        assert_eq!(config.cycle_interval_secs, DEFAULT_CYCLE_INTERVAL_SECS);
        assert_eq!(config.max_actions_per_cycle, DEFAULT_MAX_ACTIONS_PER_CYCLE);
        assert_eq!(
            config.circuit_breaker_threshold,
            DEFAULT_CIRCUIT_BREAKER_THRESHOLD
        );
    }

    #[test]
    fn test_default_is_valid() {
        let config = SelfImprovementConfig::default();
        assert!(config.is_valid());
    }

    #[test]
    #[serial]
    fn test_from_env_defaults() {
        clear_si_env_vars();

        let config = SelfImprovementConfig::from_env();

        // Default: auto-approve actions (no human approval required)
        assert!(!config.require_approval);
        assert_eq!(config.min_invocations_for_analysis, DEFAULT_MIN_INVOCATIONS);
        assert_eq!(config.cycle_interval_secs, DEFAULT_CYCLE_INTERVAL_SECS);
        assert_eq!(config.max_actions_per_cycle, DEFAULT_MAX_ACTIONS_PER_CYCLE);
        assert_eq!(
            config.circuit_breaker_threshold,
            DEFAULT_CIRCUIT_BREAKER_THRESHOLD
        );
    }

    #[test]
    #[serial]
    fn test_from_env_custom_values() {
        clear_si_env_vars();

        env::set_var("SELF_IMPROVEMENT_REQUIRE_APPROVAL", "false");
        env::set_var("SELF_IMPROVEMENT_MIN_INVOCATIONS", "100");
        env::set_var("SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS", "600");
        env::set_var("SELF_IMPROVEMENT_MAX_ACTIONS", "5");
        env::set_var("SELF_IMPROVEMENT_CIRCUIT_BREAKER_THRESHOLD", "5");

        let config = SelfImprovementConfig::from_env();

        assert!(!config.require_approval);
        assert_eq!(config.min_invocations_for_analysis, 100);
        assert_eq!(config.cycle_interval_secs, 600);
        assert_eq!(config.max_actions_per_cycle, 5);
        assert_eq!(config.circuit_breaker_threshold, 5);

        clear_si_env_vars();
    }

    #[test]
    #[serial]
    fn test_from_env_require_approval_true() {
        clear_si_env_vars();
        env::set_var("SELF_IMPROVEMENT_REQUIRE_APPROVAL", "true");

        let config = SelfImprovementConfig::from_env();
        assert!(config.require_approval);

        clear_si_env_vars();
    }

    #[test]
    #[serial]
    fn test_from_env_require_approval_false() {
        clear_si_env_vars();
        env::set_var("SELF_IMPROVEMENT_REQUIRE_APPROVAL", "FALSE");

        let config = SelfImprovementConfig::from_env();
        assert!(!config.require_approval);

        clear_si_env_vars();
    }

    #[test]
    #[serial]
    fn test_cycle_interval_clamped_low() {
        clear_si_env_vars();
        env::set_var("SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS", "10"); // Below minimum

        let config = SelfImprovementConfig::from_env();
        assert_eq!(config.cycle_interval_secs, MIN_CYCLE_INTERVAL_SECS);

        clear_si_env_vars();
    }

    #[test]
    #[serial]
    fn test_cycle_interval_clamped_high() {
        clear_si_env_vars();
        env::set_var("SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS", "10000"); // Above maximum

        let config = SelfImprovementConfig::from_env();
        assert_eq!(config.cycle_interval_secs, MAX_CYCLE_INTERVAL_SECS);

        clear_si_env_vars();
    }

    #[test]
    #[serial]
    fn test_max_actions_clamped() {
        clear_si_env_vars();
        env::set_var("SELF_IMPROVEMENT_MAX_ACTIONS", "100"); // Above maximum

        let config = SelfImprovementConfig::from_env();
        assert_eq!(config.max_actions_per_cycle, MAX_ACTIONS_PER_CYCLE);

        clear_si_env_vars();
    }

    #[test]
    #[serial]
    fn test_circuit_breaker_clamped() {
        clear_si_env_vars();
        env::set_var("SELF_IMPROVEMENT_CIRCUIT_BREAKER_THRESHOLD", "50"); // Above maximum

        let config = SelfImprovementConfig::from_env();
        assert_eq!(config.circuit_breaker_threshold, MAX_CIRCUIT_BREAKER_THRESHOLD);

        clear_si_env_vars();
    }

    #[test]
    #[serial]
    fn test_invalid_values_use_defaults() {
        clear_si_env_vars();
        env::set_var("SELF_IMPROVEMENT_MIN_INVOCATIONS", "not-a-number");
        env::set_var("SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS", "invalid");

        let config = SelfImprovementConfig::from_env();
        assert_eq!(config.min_invocations_for_analysis, DEFAULT_MIN_INVOCATIONS);
        assert_eq!(config.cycle_interval_secs, DEFAULT_CYCLE_INTERVAL_SECS);

        clear_si_env_vars();
    }

    #[test]
    fn test_is_valid_with_invalid_cycle_interval_low() {
        let config = SelfImprovementConfig {
            cycle_interval_secs: 10, // Below minimum
            ..Default::default()
        };
        assert!(!config.is_valid());
    }

    #[test]
    fn test_is_valid_with_invalid_cycle_interval_high() {
        let config = SelfImprovementConfig {
            cycle_interval_secs: 10000, // Above maximum
            ..Default::default()
        };
        assert!(!config.is_valid());
    }

    #[test]
    fn test_is_valid_with_zero_max_actions() {
        let config = SelfImprovementConfig {
            max_actions_per_cycle: 0,
            ..Default::default()
        };
        assert!(!config.is_valid());
    }

    #[test]
    fn test_is_valid_with_zero_circuit_breaker() {
        let config = SelfImprovementConfig {
            circuit_breaker_threshold: 0,
            ..Default::default()
        };
        assert!(!config.is_valid());
    }

    #[test]
    fn test_clone_and_eq() {
        let config1 = SelfImprovementConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_debug_output() {
        let config = SelfImprovementConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("SelfImprovementConfig"));
        assert!(debug.contains("require_approval"));
    }
}
