//! Configuration validation.
//!
//! This module provides validation logic for configuration values,
//! ensuring they are within acceptable ranges.

use super::Config;
use crate::error::ConfigError;

/// Minimum allowed timeout in milliseconds (1 second).
pub const MIN_TIMEOUT_MS: u64 = 1000;

/// Maximum allowed timeout in milliseconds (5 minutes).
pub const MAX_TIMEOUT_MS: u64 = 300_000;

/// Maximum allowed retry count.
pub const MAX_RETRIES: u32 = 10;

/// Validate configuration values.
///
/// # Errors
///
/// Returns [`ConfigError::InvalidValue`] if any value is out of range:
/// - `ANTHROPIC_API_KEY` must not be empty
/// - `REQUEST_TIMEOUT_MS` must be between 1000 and 300000
/// - `MAX_RETRIES` must be between 0 and 10
#[must_use = "validation result should be checked"]
pub fn validate_config(config: &Config) -> Result<(), ConfigError> {
    // API key must not be empty
    if config.api_key.is_empty() {
        return Err(ConfigError::InvalidValue {
            var: "ANTHROPIC_API_KEY".into(),
            reason: "must not be empty".into(),
        });
    }

    // Timeout must be reasonable (1s to 5m)
    if config.request_timeout_ms < MIN_TIMEOUT_MS || config.request_timeout_ms > MAX_TIMEOUT_MS {
        return Err(ConfigError::InvalidValue {
            var: "REQUEST_TIMEOUT_MS".into(),
            reason: format!("must be between {MIN_TIMEOUT_MS} and {MAX_TIMEOUT_MS} ms"),
        });
    }

    // Max retries must be reasonable (0 to 10)
    if config.max_retries > MAX_RETRIES {
        return Err(ConfigError::InvalidValue {
            var: "MAX_RETRIES".into(),
            reason: format!("must be between 0 and {MAX_RETRIES}"),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SecretString;

    fn create_valid_config() -> Config {
        Config {
            api_key: SecretString::new("sk-ant-test-key"),
            database_path: "./data/reasoning.db".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    #[test]
    fn test_valid_config() {
        let config = create_valid_config();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_empty_api_key() {
        let config = Config {
            api_key: SecretString::new(""),
            database_path: "./data/reasoning.db".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { var, .. } if var == "ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_timeout_too_low() {
        let mut config = create_valid_config();
        config.request_timeout_ms = 999; // Below minimum
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { var, .. } if var == "REQUEST_TIMEOUT_MS")
        );
    }

    #[test]
    fn test_timeout_too_high() {
        let mut config = create_valid_config();
        config.request_timeout_ms = 300_001; // Above maximum
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { var, .. } if var == "REQUEST_TIMEOUT_MS")
        );
    }

    #[test]
    fn test_retries_too_high() {
        let mut config = create_valid_config();
        config.max_retries = 11; // Above maximum
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { var, .. } if var == "MAX_RETRIES"));
    }

    #[test]
    fn test_boundary_timeout_min() {
        let mut config = create_valid_config();
        config.request_timeout_ms = MIN_TIMEOUT_MS; // Exactly at minimum
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_boundary_timeout_max() {
        let mut config = create_valid_config();
        config.request_timeout_ms = MAX_TIMEOUT_MS; // Exactly at maximum
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_boundary_retries_zero() {
        let mut config = create_valid_config();
        config.max_retries = 0; // Minimum allowed
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_boundary_retries_max() {
        let mut config = create_valid_config();
        config.max_retries = MAX_RETRIES; // Maximum allowed
        assert!(validate_config(&config).is_ok());
    }
}
