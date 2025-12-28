//! Anthropic client configuration.
//!
//! This module provides:
//! - Client configuration with defaults
//! - Mode-specific configuration
//! - Thinking budget presets

#![allow(clippy::missing_const_for_fn)]

use super::types::ThinkingConfig;

/// Default base URL for Anthropic API.
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
/// Default timeout in milliseconds.
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Default maximum retries.
pub const DEFAULT_MAX_RETRIES: u32 = 3;
/// Default retry delay in milliseconds.
pub const DEFAULT_RETRY_DELAY_MS: u64 = 1_000;
/// Default model.
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
/// Default max tokens.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Client configuration for the Anthropic API.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL for the API.
    pub base_url: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Initial retry delay in milliseconds.
    pub retry_delay_ms: u64,
}

impl ClientConfig {
    /// Create a new client configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set base URL.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set timeout in milliseconds.
    #[must_use]
    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set maximum retries.
    #[must_use]
    pub const fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set retry delay in milliseconds.
    #[must_use]
    pub const fn with_retry_delay_ms(mut self, retry_delay_ms: u64) -> Self {
        self.retry_delay_ms = retry_delay_ms;
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_delay_ms: DEFAULT_RETRY_DELAY_MS,
        }
    }
}

/// Configuration for a specific reasoning mode.
#[derive(Debug, Clone)]
pub struct ModeConfig {
    /// Model identifier.
    pub model: String,
    /// Temperature for sampling.
    pub temperature: Option<f64>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Extended thinking configuration.
    pub thinking: Option<ThinkingConfig>,
    /// Whether to use streaming.
    pub streaming: bool,
}

impl ModeConfig {
    /// Create a new mode configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set model.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens.
    #[must_use]
    pub const fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set thinking configuration.
    #[must_use]
    pub fn with_thinking(mut self, thinking: ThinkingConfig) -> Self {
        self.thinking = Some(thinking);
        self
    }

    /// Enable or disable streaming.
    #[must_use]
    pub const fn with_streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    /// Create a fast mode config (no thinking).
    #[must_use]
    pub fn fast() -> Self {
        Self::new()
    }

    /// Create a standard mode config (4096 thinking budget).
    #[must_use]
    pub fn standard() -> Self {
        Self::new().with_thinking(ThinkingConfig::standard())
    }

    /// Create a deep mode config (8192 thinking budget).
    #[must_use]
    pub fn deep() -> Self {
        Self::new().with_thinking(ThinkingConfig::deep())
    }

    /// Create a maximum mode config (16384 thinking budget).
    #[must_use]
    pub fn maximum() -> Self {
        Self::new().with_thinking(ThinkingConfig::maximum())
    }
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            temperature: None,
            max_tokens: DEFAULT_MAX_TOKENS,
            thinking: None,
            streaming: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anthropic::types::{
        DEEP_THINKING_BUDGET, MAXIMUM_THINKING_BUDGET, STANDARD_THINKING_BUDGET,
    };

    // ClientConfig tests
    #[test]
    fn test_client_config_defaults() {
        let config = ClientConfig::new();
        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        assert_eq!(config.timeout_ms, DEFAULT_TIMEOUT_MS);
        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(config.retry_delay_ms, DEFAULT_RETRY_DELAY_MS);
    }

    #[test]
    fn test_client_config_with_base_url() {
        let config = ClientConfig::new().with_base_url("http://localhost:8080");
        assert_eq!(config.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_client_config_with_timeout_ms() {
        let config = ClientConfig::new().with_timeout_ms(30_000);
        assert_eq!(config.timeout_ms, 30_000);
    }

    #[test]
    fn test_client_config_with_max_retries() {
        let config = ClientConfig::new().with_max_retries(5);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_config_with_retry_delay_ms() {
        let config = ClientConfig::new().with_retry_delay_ms(2_000);
        assert_eq!(config.retry_delay_ms, 2_000);
    }

    #[test]
    fn test_client_config_builder_chain() {
        let config = ClientConfig::new()
            .with_base_url("http://localhost")
            .with_timeout_ms(10_000)
            .with_max_retries(2)
            .with_retry_delay_ms(500);

        assert_eq!(config.base_url, "http://localhost");
        assert_eq!(config.timeout_ms, 10_000);
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.retry_delay_ms, 500);
    }

    #[test]
    fn test_client_config_clone() {
        let config1 = ClientConfig::new().with_timeout_ms(5_000);
        let config2 = config1.clone();
        assert_eq!(config1.timeout_ms, config2.timeout_ms);
    }

    #[test]
    fn test_client_config_debug() {
        let config = ClientConfig::new();
        let debug = format!("{:?}", config);
        assert!(debug.contains("ClientConfig"));
        assert!(debug.contains("base_url"));
    }

    // ModeConfig tests
    #[test]
    fn test_mode_config_defaults() {
        let config = ModeConfig::new();
        assert_eq!(config.model, DEFAULT_MODEL);
        assert!(config.temperature.is_none());
        assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
        assert!(config.thinking.is_none());
        assert!(!config.streaming);
    }

    #[test]
    fn test_mode_config_with_model() {
        let config = ModeConfig::new().with_model("claude-opus-4-20250514");
        assert_eq!(config.model, "claude-opus-4-20250514");
    }

    #[test]
    fn test_mode_config_with_temperature() {
        let config = ModeConfig::new().with_temperature(0.7);
        assert_eq!(config.temperature, Some(0.7));
    }

    #[test]
    fn test_mode_config_with_max_tokens() {
        let config = ModeConfig::new().with_max_tokens(2048);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_mode_config_with_thinking() {
        let config = ModeConfig::new().with_thinking(ThinkingConfig::deep());
        assert!(config.thinking.is_some());
        assert_eq!(config.thinking.unwrap().budget_tokens, DEEP_THINKING_BUDGET);
    }

    #[test]
    fn test_mode_config_with_streaming() {
        let config = ModeConfig::new().with_streaming(true);
        assert!(config.streaming);
    }

    #[test]
    fn test_mode_config_fast() {
        let config = ModeConfig::fast();
        assert!(config.thinking.is_none());
    }

    #[test]
    fn test_mode_config_standard() {
        let config = ModeConfig::standard();
        assert!(config.thinking.is_some());
        assert_eq!(
            config.thinking.unwrap().budget_tokens,
            STANDARD_THINKING_BUDGET
        );
    }

    #[test]
    fn test_mode_config_deep() {
        let config = ModeConfig::deep();
        assert!(config.thinking.is_some());
        assert_eq!(config.thinking.unwrap().budget_tokens, DEEP_THINKING_BUDGET);
    }

    #[test]
    fn test_mode_config_maximum() {
        let config = ModeConfig::maximum();
        assert!(config.thinking.is_some());
        assert_eq!(
            config.thinking.unwrap().budget_tokens,
            MAXIMUM_THINKING_BUDGET
        );
    }

    #[test]
    fn test_mode_config_builder_chain() {
        let config = ModeConfig::new()
            .with_model("claude-3")
            .with_temperature(0.5)
            .with_max_tokens(1000)
            .with_streaming(true)
            .with_thinking(ThinkingConfig::standard());

        assert_eq!(config.model, "claude-3");
        assert_eq!(config.temperature, Some(0.5));
        assert_eq!(config.max_tokens, 1000);
        assert!(config.streaming);
        assert!(config.thinking.is_some());
    }

    #[test]
    fn test_mode_config_clone() {
        let config1 = ModeConfig::deep();
        let config2 = config1.clone();
        assert_eq!(
            config1.thinking.as_ref().map(|t| t.budget_tokens),
            config2.thinking.as_ref().map(|t| t.budget_tokens)
        );
    }

    #[test]
    fn test_mode_config_debug() {
        let config = ModeConfig::new();
        let debug = format!("{:?}", config);
        assert!(debug.contains("ModeConfig"));
        assert!(debug.contains("model"));
    }
}
