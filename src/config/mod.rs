//! Configuration management.
//!
//! This module handles:
//! - Environment variable loading
//! - Configuration validation
//! - Default value handling
//! - Secure API key storage via [`SecretString`]
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::config::{Config, SecretString, DEFAULT_MODEL};
//!
//! // Create a config directly (use Config::from_env() in production)
//! let config = Config {
//!     api_key: SecretString::new("sk-ant-example-key"),
//!     database_path: "./data/reasoning.db".to_string(),
//!     log_level: "info".to_string(),
//!     request_timeout_ms: 30000,
//!     request_timeout_deep_ms: 60000,
//!     request_timeout_maximum_ms: 120000,
//!     max_retries: 3,
//!     model: DEFAULT_MODEL.to_string(),
//! };
//!
//! println!("Using model: {}", config.model);
//! // API key is protected from accidental logging
//! let debug = format!("{:?}", config);
//! assert!(debug.contains("<REDACTED>"));
//! assert!(!debug.contains("sk-ant-example-key"));
//! ```

mod secret;
mod validation;

mod self_improvement;

pub use secret::SecretString;
pub use self_improvement::SelfImprovementConfig;
pub use validation::{validate_config, MAX_RETRIES, MAX_TIMEOUT_MS, MIN_TIMEOUT_MS};

use crate::error::ConfigError;

/// Default database path.
pub const DEFAULT_DATABASE_PATH: &str = "./data/reasoning.db";

/// Default log level.
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// Default request timeout in milliseconds (fast/standard modes).
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 30_000;

/// Default request timeout for deep thinking modes (8K tokens).
pub const DEFAULT_REQUEST_TIMEOUT_DEEP_MS: u64 = 60_000;

/// Default request timeout for maximum thinking modes (16K tokens).
pub const DEFAULT_REQUEST_TIMEOUT_MAXIMUM_MS: u64 = 120_000;

/// Default maximum retry attempts.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default Anthropic model.
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Application configuration.
///
/// This struct holds all configuration values for the MCP Reasoning Server.
/// Use [`Config::from_env`] to load configuration from environment variables.
///
/// The `api_key` field uses [`SecretString`] to prevent accidental logging.
///
/// ## Tiered Timeouts
///
/// Different reasoning modes have different timeout values based on their complexity:
/// - Fast/Standard modes (no thinking or 4K tokens): `request_timeout_ms` (default: 30s)
/// - Deep modes (8K tokens): `request_timeout_deep_ms` (default: 60s)
/// - Maximum modes (16K tokens): `request_timeout_maximum_ms` (default: 120s)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Anthropic API key (protected from logging via [`SecretString`]).
    pub api_key: SecretString,
    /// Database path.
    pub database_path: String,
    /// Log level (error, warn, info, debug, trace).
    pub log_level: String,
    /// Request timeout in milliseconds (fast/standard modes).
    pub request_timeout_ms: u64,
    /// Request timeout for deep thinking modes (8K tokens).
    pub request_timeout_deep_ms: u64,
    /// Request timeout for maximum thinking modes (16K tokens).
    pub request_timeout_maximum_ms: u64,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Anthropic model to use.
    pub model: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required environment variables:
    /// - `ANTHROPIC_API_KEY`: Anthropic API key
    ///
    /// Optional environment variables (with defaults):
    /// - `DATABASE_PATH`: Path to `SQLite` database (default: `./data/reasoning.db`)
    /// - `LOG_LEVEL`: Logging level (default: `info`)
    /// - `REQUEST_TIMEOUT_MS`: Request timeout for fast/standard modes (default: `30000`)
    /// - `REQUEST_TIMEOUT_DEEP_MS`: Request timeout for deep modes (default: `60000`)
    /// - `REQUEST_TIMEOUT_MAXIMUM_MS`: Request timeout for maximum modes (default: `120000`)
    /// - `MAX_RETRIES`: Maximum retry attempts (default: `3`)
    /// - `ANTHROPIC_MODEL`: Model to use (default: `claude-sonnet-4-20250514`)
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if:
    /// - `ANTHROPIC_API_KEY` is missing
    /// - `REQUEST_TIMEOUT_MS` is not a valid positive integer
    /// - `MAX_RETRIES` is not a valid positive integer
    /// - Any value fails validation (see [`validate_config`])
    #[must_use = "configuration should be used"]
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if present (ignore errors)
        let _ = dotenvy::dotenv();

        let api_key =
            std::env::var("ANTHROPIC_API_KEY").map_err(|_| ConfigError::MissingRequired {
                var: "ANTHROPIC_API_KEY".into(),
            })?;

        let database_path =
            std::env::var("DATABASE_PATH").unwrap_or_else(|_| DEFAULT_DATABASE_PATH.into());

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| DEFAULT_LOG_LEVEL.into());

        let request_timeout_ms = parse_env_u64("REQUEST_TIMEOUT_MS", DEFAULT_REQUEST_TIMEOUT_MS)?;
        let request_timeout_deep_ms =
            parse_env_u64("REQUEST_TIMEOUT_DEEP_MS", DEFAULT_REQUEST_TIMEOUT_DEEP_MS)?;
        let request_timeout_maximum_ms = parse_env_u64(
            "REQUEST_TIMEOUT_MAXIMUM_MS",
            DEFAULT_REQUEST_TIMEOUT_MAXIMUM_MS,
        )?;

        let max_retries = parse_env_u32("MAX_RETRIES", DEFAULT_MAX_RETRIES)?;

        let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into());

        let config = Self {
            api_key: SecretString::new(api_key),
            database_path,
            log_level,
            request_timeout_ms,
            request_timeout_deep_ms,
            request_timeout_maximum_ms,
            max_retries,
            model,
        };

        validate_config(&config)?;
        Ok(config)
    }

    /// Get the appropriate timeout based on thinking budget (in tokens).
    ///
    /// Returns:
    /// - `request_timeout_ms` for None or Standard (≤ 4096 tokens)
    /// - `request_timeout_deep_ms` for Deep (4097-8192 tokens)
    /// - `request_timeout_maximum_ms` for Maximum (> 8192 tokens)
    ///
    /// # Example
    ///
    /// ```
    /// use mcp_reasoning::config::Config;
    /// # let config = Config {
    /// #     api_key: mcp_reasoning::config::SecretString::new("test"),
    /// #     database_path: "./test.db".into(),
    /// #     log_level: "info".into(),
    /// #     request_timeout_ms: 30_000,
    /// #     request_timeout_deep_ms: 60_000,
    /// #     request_timeout_maximum_ms: 120_000,
    /// #     max_retries: 3,
    /// #     model: "claude-sonnet-4-20250514".into(),
    /// # };
    ///
    /// assert_eq!(config.timeout_for_thinking_budget(None), 30_000);
    /// assert_eq!(config.timeout_for_thinking_budget(Some(4096)), 30_000);
    /// assert_eq!(config.timeout_for_thinking_budget(Some(8192)), 60_000);
    /// assert_eq!(config.timeout_for_thinking_budget(Some(16384)), 120_000);
    /// ```
    #[must_use]
    pub const fn timeout_for_thinking_budget(&self, thinking_budget: Option<u32>) -> u64 {
        match thinking_budget {
            None | Some(0..=4096) => self.request_timeout_ms,
            Some(4097..=8192) => self.request_timeout_deep_ms,
            Some(_) => self.request_timeout_maximum_ms,
        }
    }
}

/// Parse an environment variable as u64, using a default if not set.
fn parse_env_u64(name: &str, default: u64) -> Result<u64, ConfigError> {
    std::env::var(name).map_or(Ok(default), |val| {
        val.parse().map_err(|_| ConfigError::InvalidValue {
            var: name.into(),
            reason: "must be a positive integer".into(),
        })
    })
}

/// Parse an environment variable as u32, using a default if not set.
fn parse_env_u32(name: &str, default: u32) -> Result<u32, ConfigError> {
    std::env::var(name).map_or(Ok(default), |val| {
        val.parse().map_err(|_| ConfigError::InvalidValue {
            var: name.into(),
            reason: "must be a positive integer".into(),
        })
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Helper to set up a clean test environment.
    fn setup_test_env() {
        // Clear all relevant env vars
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("DATABASE_PATH");
        env::remove_var("LOG_LEVEL");
        env::remove_var("REQUEST_TIMEOUT_MS");
        env::remove_var("MAX_RETRIES");
        env::remove_var("ANTHROPIC_MODEL");
    }

    #[test]
    #[serial]
    fn test_config_from_env_with_all_vars() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key-123");
        env::set_var("DATABASE_PATH", "/custom/path.db");
        env::set_var("LOG_LEVEL", "debug");
        env::set_var("REQUEST_TIMEOUT_MS", "60000");
        env::set_var("MAX_RETRIES", "5");
        env::set_var("ANTHROPIC_MODEL", "claude-opus-4");

        let config = Config::from_env().expect("should load config");

        assert_eq!(config.api_key.expose(), "sk-ant-test-key-123");
        assert_eq!(config.database_path, "/custom/path.db");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.request_timeout_ms, 60000);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.model, "claude-opus-4");
    }

    #[test]
    #[serial]
    fn test_config_from_env_defaults() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key");

        let config = Config::from_env().expect("should load config");

        assert_eq!(config.api_key.expose(), "sk-ant-test-key");
        assert_eq!(config.database_path, DEFAULT_DATABASE_PATH);
        assert_eq!(config.log_level, DEFAULT_LOG_LEVEL);
        assert_eq!(config.request_timeout_ms, DEFAULT_REQUEST_TIMEOUT_MS);
        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(config.model, DEFAULT_MODEL);
    }

    #[test]
    #[serial]
    fn test_config_missing_api_key() {
        setup_test_env();

        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::MissingRequired { var } if var == "ANTHROPIC_API_KEY"
        ));
    }

    #[test]
    #[serial]
    fn test_config_invalid_timeout_format() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key");
        env::set_var("REQUEST_TIMEOUT_MS", "not-a-number");

        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidValue { var, .. } if var == "REQUEST_TIMEOUT_MS"
        ));
    }

    #[test]
    #[serial]
    fn test_config_invalid_retries_format() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key");
        env::set_var("MAX_RETRIES", "not-a-number");

        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidValue { var, .. } if var == "MAX_RETRIES"
        ));
    }

    #[test]
    #[serial]
    fn test_config_timeout_validation_failure() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key");
        env::set_var("REQUEST_TIMEOUT_MS", "100"); // Too low

        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidValue { var, .. } if var == "REQUEST_TIMEOUT_MS"
        ));
    }

    #[test]
    #[serial]
    fn test_config_retries_validation_failure() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key");
        env::set_var("MAX_RETRIES", "20"); // Too high

        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidValue { var, .. } if var == "MAX_RETRIES"
        ));
    }

    #[test]
    #[serial]
    fn test_config_empty_api_key_validation() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", ""); // Empty

        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidValue { var, .. } if var == "ANTHROPIC_API_KEY"
        ));
    }

    #[test]
    #[serial]
    fn test_config_negative_timeout() {
        setup_test_env();

        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-key");
        env::set_var("REQUEST_TIMEOUT_MS", "-1000"); // Negative

        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidValue { var, .. } if var == "REQUEST_TIMEOUT_MS"
        ));
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            api_key: SecretString::new("test-key"),
            database_path: "/path/to/db".to_string(),
            log_level: "debug".to_string(),
            request_timeout_ms: 5000,
            request_timeout_deep_ms: 10000,
            request_timeout_maximum_ms: 20000,
            max_retries: 2,
            model: "test-model".to_string(),
        };

        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_config_debug_redacts_api_key() {
        let config = Config {
            api_key: SecretString::new("super-secret-key"),
            database_path: "/path/to/db".to_string(),
            log_level: "debug".to_string(),
            request_timeout_ms: 5000,
            request_timeout_deep_ms: 10000,
            request_timeout_maximum_ms: 20000,
            max_retries: 2,
            model: "test-model".to_string(),
        };

        let debug = format!("{config:?}");
        // API key should be redacted
        assert!(!debug.contains("super-secret-key"));
        assert!(debug.contains("<REDACTED>"));
        // Other fields should still be visible
        assert!(debug.contains("/path/to/db"));
    }

    #[test]
    fn test_parse_env_u64_with_value() {
        env::set_var("TEST_U64", "12345");
        let result = parse_env_u64("TEST_U64", 0);
        assert_eq!(result.unwrap(), 12345);
        env::remove_var("TEST_U64");
    }

    #[test]
    fn test_parse_env_u64_default() {
        env::remove_var("TEST_U64_MISSING");
        let result = parse_env_u64("TEST_U64_MISSING", 999);
        assert_eq!(result.unwrap(), 999);
    }

    #[test]
    fn test_parse_env_u64_invalid() {
        env::set_var("TEST_U64_INVALID", "abc");
        let result = parse_env_u64("TEST_U64_INVALID", 0);
        assert!(result.is_err());
        env::remove_var("TEST_U64_INVALID");
    }

    #[test]
    fn test_parse_env_u32_with_value() {
        env::set_var("TEST_U32", "42");
        let result = parse_env_u32("TEST_U32", 0);
        assert_eq!(result.unwrap(), 42);
        env::remove_var("TEST_U32");
    }

    #[test]
    fn test_parse_env_u32_default() {
        env::remove_var("TEST_U32_MISSING");
        let result = parse_env_u32("TEST_U32_MISSING", 100);
        assert_eq!(result.unwrap(), 100);
    }

    #[test]
    fn test_parse_env_u32_invalid() {
        env::set_var("TEST_U32_INVALID", "xyz");
        let result = parse_env_u32("TEST_U32_INVALID", 0);
        assert!(result.is_err());
        env::remove_var("TEST_U32_INVALID");
    }
}
