//! Error types for the MCP Reasoning Server.
//!
//! This module defines a hierarchical error system:
//! - [`AppError`]: Top-level application errors
//! - [`AnthropicError`]: Anthropic API specific errors
//! - [`StorageError`]: Database operation errors
//! - [`McpError`]: MCP protocol errors
//! - [`ModeError`]: Reasoning mode execution errors
//! - [`ConfigError`]: Configuration errors
//!
//! All errors implement `Send + Sync` for async compatibility.

use thiserror::Error;

/// Top-level application error.
///
/// This is the main error type returned by public API functions.
/// It wraps all subsystem errors for unified error handling.
#[derive(Debug, Error)]
pub enum AppError {
    /// Anthropic API error.
    #[error("Anthropic API error: {0}")]
    Anthropic(#[from] AnthropicError),

    /// Storage error.
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// MCP protocol error.
    #[error("MCP protocol error: {0}")]
    Mcp(#[from] McpError),

    /// Mode execution error.
    #[error("Mode execution error: {0}")]
    Mode(#[from] ModeError),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

/// Anthropic API errors.
///
/// These errors represent failures when communicating with the Anthropic API.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AnthropicError {
    /// Authentication failed due to invalid API key.
    #[error("Authentication failed: invalid API key")]
    AuthenticationFailed,

    /// Request was rate limited.
    #[error("Rate limited: retry after {retry_after_seconds}s")]
    RateLimited {
        /// Seconds to wait before retrying.
        retry_after_seconds: u64,
    },

    /// The requested model is overloaded.
    #[error("Model overloaded: {model}")]
    ModelOverloaded {
        /// The model that is overloaded.
        model: String,
    },

    /// Request timed out.
    #[error("Request timeout after {timeout_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Invalid request parameters.
    #[error("Invalid request: {message}")]
    InvalidRequest {
        /// Description of what's invalid.
        message: String,
    },

    /// Network communication error.
    #[error("Network error: {message}")]
    Network {
        /// Description of the network error.
        message: String,
    },

    /// Unexpected response from the API.
    #[error("Unexpected response: {message}")]
    UnexpectedResponse {
        /// Description of what was unexpected.
        message: String,
    },
}

impl AnthropicError {
    /// Returns true if this error is retryable.
    ///
    /// Rate limiting and model overload errors are retryable.
    /// Authentication and invalid request errors are not.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. }
                | Self::ModelOverloaded { .. }
                | Self::Timeout { .. }
                | Self::Network { .. }
        )
    }
}

/// Storage errors.
///
/// These errors represent failures in database operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Failed to connect to the database.
    #[error("Database connection failed: {message}")]
    ConnectionFailed {
        /// Description of the connection failure.
        message: String,
    },

    /// A database query failed.
    #[error("Query failed: {query} - {message}")]
    QueryFailed {
        /// The query that failed (may be truncated).
        query: String,
        /// Description of the failure.
        message: String,
    },

    /// Session not found.
    #[error("Session not found: {session_id}")]
    SessionNotFound {
        /// The session ID that was not found.
        session_id: String,
    },

    /// Thought not found.
    #[error("Thought not found: {thought_id}")]
    ThoughtNotFound {
        /// The thought ID that was not found.
        thought_id: String,
    },

    /// Database migration failed.
    #[error("Migration failed: {version} - {message}")]
    MigrationFailed {
        /// The migration version that failed.
        version: String,
        /// Description of the failure.
        message: String,
    },

    /// Internal storage error.
    #[error("Internal storage error: {message}")]
    Internal {
        /// Description of the internal error.
        message: String,
    },
}

/// MCP protocol errors.
///
/// These errors represent failures in MCP JSON-RPC communication.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum McpError {
    /// Invalid JSON-RPC request.
    #[error("Invalid JSON-RPC request: {message}")]
    InvalidRequest {
        /// Description of what's invalid.
        message: String,
    },

    /// Unknown method requested.
    #[error("Unknown method: {method}")]
    UnknownMethod {
        /// The unknown method name.
        method: String,
    },

    /// Unknown tool requested.
    #[error("Unknown tool: {tool}")]
    UnknownTool {
        /// The unknown tool name.
        tool: String,
    },

    /// Invalid parameters for a tool.
    #[error("Invalid parameters for {tool}: {message}")]
    InvalidParameters {
        /// The tool name.
        tool: String,
        /// Description of what's invalid.
        message: String,
    },

    /// Internal server error.
    #[error("Internal error: {message}")]
    Internal {
        /// Description of the internal error.
        message: String,
    },
}

/// Mode execution errors.
///
/// These errors represent failures when executing reasoning modes.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModeError {
    /// Invalid operation for the mode.
    #[error("Invalid operation {operation} for mode {mode}")]
    InvalidOperation {
        /// The mode name.
        mode: String,
        /// The invalid operation.
        operation: String,
    },

    /// Missing required field.
    #[error("Missing required field: {field}")]
    MissingField {
        /// The missing field name.
        field: String,
    },

    /// Invalid value for a field.
    #[error("Invalid value for {field}: {reason}")]
    InvalidValue {
        /// The field name.
        field: String,
        /// Why the value is invalid.
        reason: String,
    },

    /// Session is required but not provided.
    #[error("Session required but not provided")]
    SessionRequired,

    /// JSON parsing failed.
    #[error("JSON parsing failed: {message}")]
    JsonParseFailed {
        /// Description of the parsing error.
        message: String,
    },

    /// API is unavailable.
    #[error("API unavailable: {message}")]
    ApiUnavailable {
        /// Description of why the API is unavailable.
        message: String,
    },

    /// Operation timed out.
    #[error("Operation timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds.
        elapsed_ms: u64,
    },
}

/// Configuration errors.
///
/// These errors represent failures in configuration loading and validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// Required configuration is missing.
    #[error("Missing required: {var}")]
    MissingRequired {
        /// The missing variable name.
        var: String,
    },

    /// Configuration value is invalid.
    #[error("Invalid value for {var}: {reason}")]
    InvalidValue {
        /// The variable name.
        var: String,
        /// Why the value is invalid.
        reason: String,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Type assertions - verify all errors implement required traits
    assert_impl_all!(AppError: Send, Sync, std::error::Error);
    assert_impl_all!(AnthropicError: Send, Sync, std::error::Error, Clone);
    assert_impl_all!(StorageError: Send, Sync, std::error::Error, Clone);
    assert_impl_all!(McpError: Send, Sync, std::error::Error, Clone);
    assert_impl_all!(ModeError: Send, Sync, std::error::Error, Clone);
    assert_impl_all!(ConfigError: Send, Sync, std::error::Error, Clone);

    // AppError tests
    #[test]
    fn test_app_error_display_anthropic() {
        let err = AppError::Anthropic(AnthropicError::AuthenticationFailed);
        assert_eq!(
            err.to_string(),
            "Anthropic API error: Authentication failed: invalid API key"
        );
    }

    #[test]
    fn test_app_error_display_storage() {
        let err = AppError::Storage(StorageError::SessionNotFound {
            session_id: "abc123".to_string(),
        });
        assert_eq!(err.to_string(), "Storage error: Session not found: abc123");
    }

    #[test]
    fn test_app_error_display_mcp() {
        let err = AppError::Mcp(McpError::UnknownTool {
            tool: "unknown_tool".to_string(),
        });
        assert_eq!(
            err.to_string(),
            "MCP protocol error: Unknown tool: unknown_tool"
        );
    }

    #[test]
    fn test_app_error_display_mode() {
        let err = AppError::Mode(ModeError::SessionRequired);
        assert_eq!(
            err.to_string(),
            "Mode execution error: Session required but not provided"
        );
    }

    #[test]
    fn test_app_error_display_config() {
        let err = AppError::Config(ConfigError::MissingRequired {
            var: "API_KEY".to_string(),
        });
        assert_eq!(
            err.to_string(),
            "Configuration error: Missing required: API_KEY"
        );
    }

    // From impl tests
    #[test]
    fn test_app_error_from_anthropic_error() {
        let anthropic_err = AnthropicError::AuthenticationFailed;
        let app_err: AppError = anthropic_err.into();
        assert!(matches!(app_err, AppError::Anthropic(_)));
    }

    #[test]
    fn test_app_error_from_storage_error() {
        let storage_err = StorageError::SessionNotFound {
            session_id: "test".to_string(),
        };
        let app_err: AppError = storage_err.into();
        assert!(matches!(app_err, AppError::Storage(_)));
    }

    #[test]
    fn test_app_error_from_mcp_error() {
        let mcp_err = McpError::UnknownMethod {
            method: "test".to_string(),
        };
        let app_err: AppError = mcp_err.into();
        assert!(matches!(app_err, AppError::Mcp(_)));
    }

    #[test]
    fn test_app_error_from_mode_error() {
        let mode_err = ModeError::SessionRequired;
        let app_err: AppError = mode_err.into();
        assert!(matches!(app_err, AppError::Mode(_)));
    }

    #[test]
    fn test_app_error_from_config_error() {
        let config_err = ConfigError::MissingRequired {
            var: "TEST".to_string(),
        };
        let app_err: AppError = config_err.into();
        assert!(matches!(app_err, AppError::Config(_)));
    }

    // AnthropicError tests
    #[test]
    fn test_anthropic_error_display_auth_failed() {
        let err = AnthropicError::AuthenticationFailed;
        assert_eq!(err.to_string(), "Authentication failed: invalid API key");
    }

    #[test]
    fn test_anthropic_error_display_rate_limited() {
        let err = AnthropicError::RateLimited {
            retry_after_seconds: 60,
        };
        assert_eq!(err.to_string(), "Rate limited: retry after 60s");
    }

    #[test]
    fn test_anthropic_error_display_model_overloaded() {
        let err = AnthropicError::ModelOverloaded {
            model: "claude-3".to_string(),
        };
        assert_eq!(err.to_string(), "Model overloaded: claude-3");
    }

    #[test]
    fn test_anthropic_error_display_timeout() {
        let err = AnthropicError::Timeout { timeout_ms: 30000 };
        assert_eq!(err.to_string(), "Request timeout after 30000ms");
    }

    #[test]
    fn test_anthropic_error_display_invalid_request() {
        let err = AnthropicError::InvalidRequest {
            message: "bad content".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid request: bad content");
    }

    #[test]
    fn test_anthropic_error_display_network() {
        let err = AnthropicError::Network {
            message: "connection refused".to_string(),
        };
        assert_eq!(err.to_string(), "Network error: connection refused");
    }

    #[test]
    fn test_anthropic_error_display_unexpected_response() {
        let err = AnthropicError::UnexpectedResponse {
            message: "missing field".to_string(),
        };
        assert_eq!(err.to_string(), "Unexpected response: missing field");
    }

    #[test]
    fn test_anthropic_error_is_retryable_rate_limited() {
        let err = AnthropicError::RateLimited {
            retry_after_seconds: 60,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_anthropic_error_is_retryable_model_overloaded() {
        let err = AnthropicError::ModelOverloaded {
            model: "claude-3".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_anthropic_error_is_retryable_timeout() {
        let err = AnthropicError::Timeout { timeout_ms: 30000 };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_anthropic_error_is_retryable_network() {
        let err = AnthropicError::Network {
            message: "test".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_anthropic_error_not_retryable_auth_failed() {
        let err = AnthropicError::AuthenticationFailed;
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_anthropic_error_not_retryable_invalid_request() {
        let err = AnthropicError::InvalidRequest {
            message: "test".to_string(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_anthropic_error_not_retryable_unexpected_response() {
        let err = AnthropicError::UnexpectedResponse {
            message: "test".to_string(),
        };
        assert!(!err.is_retryable());
    }

    // StorageError tests
    #[test]
    fn test_storage_error_display_connection_failed() {
        let err = StorageError::ConnectionFailed {
            message: "host not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Database connection failed: host not found"
        );
    }

    #[test]
    fn test_storage_error_display_query_failed() {
        let err = StorageError::QueryFailed {
            query: "SELECT *".to_string(),
            message: "syntax error".to_string(),
        };
        assert_eq!(err.to_string(), "Query failed: SELECT * - syntax error");
    }

    #[test]
    fn test_storage_error_display_session_not_found() {
        let err = StorageError::SessionNotFound {
            session_id: "sess123".to_string(),
        };
        assert_eq!(err.to_string(), "Session not found: sess123");
    }

    #[test]
    fn test_storage_error_display_thought_not_found() {
        let err = StorageError::ThoughtNotFound {
            thought_id: "thought456".to_string(),
        };
        assert_eq!(err.to_string(), "Thought not found: thought456");
    }

    #[test]
    fn test_storage_error_display_migration_failed() {
        let err = StorageError::MigrationFailed {
            version: "001".to_string(),
            message: "syntax error".to_string(),
        };
        assert_eq!(err.to_string(), "Migration failed: 001 - syntax error");
    }

    #[test]
    fn test_storage_error_display_internal() {
        let err = StorageError::Internal {
            message: "unexpected".to_string(),
        };
        assert_eq!(err.to_string(), "Internal storage error: unexpected");
    }

    // McpError tests
    #[test]
    fn test_mcp_error_display_invalid_request() {
        let err = McpError::InvalidRequest {
            message: "missing id".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid JSON-RPC request: missing id");
    }

    #[test]
    fn test_mcp_error_display_unknown_method() {
        let err = McpError::UnknownMethod {
            method: "foo/bar".to_string(),
        };
        assert_eq!(err.to_string(), "Unknown method: foo/bar");
    }

    #[test]
    fn test_mcp_error_display_unknown_tool() {
        let err = McpError::UnknownTool {
            tool: "reasoning_foo".to_string(),
        };
        assert_eq!(err.to_string(), "Unknown tool: reasoning_foo");
    }

    #[test]
    fn test_mcp_error_display_invalid_parameters() {
        let err = McpError::InvalidParameters {
            tool: "reasoning_linear".to_string(),
            message: "missing content".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid parameters for reasoning_linear: missing content"
        );
    }

    #[test]
    fn test_mcp_error_display_internal() {
        let err = McpError::Internal {
            message: "server error".to_string(),
        };
        assert_eq!(err.to_string(), "Internal error: server error");
    }

    // ModeError tests
    #[test]
    fn test_mode_error_display_invalid_operation() {
        let err = ModeError::InvalidOperation {
            mode: "tree".to_string(),
            operation: "invalid_op".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid operation invalid_op for mode tree"
        );
    }

    #[test]
    fn test_mode_error_display_missing_field() {
        let err = ModeError::MissingField {
            field: "content".to_string(),
        };
        assert_eq!(err.to_string(), "Missing required field: content");
    }

    #[test]
    fn test_mode_error_display_invalid_value() {
        let err = ModeError::InvalidValue {
            field: "confidence".to_string(),
            reason: "must be between 0 and 1".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid value for confidence: must be between 0 and 1"
        );
    }

    #[test]
    fn test_mode_error_display_session_required() {
        let err = ModeError::SessionRequired;
        assert_eq!(err.to_string(), "Session required but not provided");
    }

    #[test]
    fn test_mode_error_display_json_parse_failed() {
        let err = ModeError::JsonParseFailed {
            message: "unexpected token".to_string(),
        };
        assert_eq!(err.to_string(), "JSON parsing failed: unexpected token");
    }

    #[test]
    fn test_mode_error_display_api_unavailable() {
        let err = ModeError::ApiUnavailable {
            message: "service down".to_string(),
        };
        assert_eq!(err.to_string(), "API unavailable: service down");
    }

    #[test]
    fn test_mode_error_display_timeout() {
        let err = ModeError::Timeout { elapsed_ms: 30000 };
        assert_eq!(err.to_string(), "Operation timed out after 30000ms");
    }

    // ConfigError tests
    #[test]
    fn test_config_error_display_missing_required() {
        let err = ConfigError::MissingRequired {
            var: "ANTHROPIC_API_KEY".to_string(),
        };
        assert_eq!(err.to_string(), "Missing required: ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_config_error_display_invalid_value() {
        let err = ConfigError::InvalidValue {
            var: "TIMEOUT_MS".to_string(),
            reason: "must be positive integer".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid value for TIMEOUT_MS: must be positive integer"
        );
    }

    // Clone tests
    #[test]
    fn test_anthropic_error_clone() {
        let err = AnthropicError::RateLimited {
            retry_after_seconds: 60,
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_storage_error_clone() {
        let err = StorageError::SessionNotFound {
            session_id: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_mcp_error_clone() {
        let err = McpError::UnknownTool {
            tool: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_mode_error_clone() {
        let err = ModeError::SessionRequired;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_config_error_clone() {
        let err = ConfigError::MissingRequired {
            var: "TEST".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // PartialEq tests
    #[test]
    fn test_anthropic_error_eq() {
        let err1 = AnthropicError::AuthenticationFailed;
        let err2 = AnthropicError::AuthenticationFailed;
        let err3 = AnthropicError::Timeout { timeout_ms: 1000 };
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_storage_error_eq() {
        let err1 = StorageError::SessionNotFound {
            session_id: "a".to_string(),
        };
        let err2 = StorageError::SessionNotFound {
            session_id: "a".to_string(),
        };
        let err3 = StorageError::SessionNotFound {
            session_id: "b".to_string(),
        };
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}
