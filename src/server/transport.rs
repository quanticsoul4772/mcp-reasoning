//! Transport layer for MCP server.
//!
//! This module provides transport implementations for the MCP server.
//! Currently supports stdio transport (primary for Claude Code integration).

use rmcp::service::{serve_server, RoleServer, RunningService};
use rmcp::transport::io::stdio;

use super::tools::ReasoningServer;
use crate::error::AppError;

/// Configuration for transport options.
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Maximum message size in bytes.
    pub max_message_size: usize,
    /// Read timeout in milliseconds.
    pub read_timeout_ms: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            read_timeout_ms: 300_000,           // 5 minutes
        }
    }
}

/// Stdio transport handler.
///
/// Handles communication over stdin/stdout for integration with
/// Claude Code and other MCP clients.
#[derive(Debug)]
pub struct StdioTransport {
    config: TransportConfig,
}

impl StdioTransport {
    /// Creates a new stdio transport with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TransportConfig::default(),
        }
    }

    /// Creates a new stdio transport with custom configuration.
    #[must_use]
    pub const fn with_config(config: TransportConfig) -> Self {
        Self { config }
    }

    /// Returns the transport configuration.
    #[must_use]
    pub const fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// Runs the server using stdio transport.
    ///
    /// This function blocks until the client disconnects or an error occurs.
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start or encounters
    /// a communication error.
    pub async fn serve(
        self,
        server: ReasoningServer,
    ) -> Result<RunningService<RoleServer, ReasoningServer>, AppError> {
        let (stdin, stdout) = stdio();

        serve_server(server, (stdin, stdout))
            .await
            .map_err(|e| {
                AppError::Mcp(crate::error::McpError::Internal {
                    message: e.to_string(),
                })
            })
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert_eq!(config.read_timeout_ms, 300_000);
    }

    #[test]
    fn test_transport_config_clone() {
        let config = TransportConfig {
            max_message_size: 1024,
            read_timeout_ms: 1000,
        };
        let cloned = config.clone();
        assert_eq!(cloned.max_message_size, config.max_message_size);
    }

    #[test]
    fn test_transport_config_debug() {
        let config = TransportConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("TransportConfig"));
    }

    #[test]
    fn test_stdio_transport_new() {
        let transport = StdioTransport::new();
        assert_eq!(transport.config().max_message_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_stdio_transport_default() {
        let transport = StdioTransport::default();
        assert_eq!(transport.config().read_timeout_ms, 300_000);
    }

    #[test]
    fn test_stdio_transport_with_config() {
        let config = TransportConfig {
            max_message_size: 5000,
            read_timeout_ms: 10_000,
        };
        let transport = StdioTransport::with_config(config);
        assert_eq!(transport.config().max_message_size, 5000);
        assert_eq!(transport.config().read_timeout_ms, 10_000);
    }

    #[test]
    fn test_stdio_transport_debug() {
        let transport = StdioTransport::new();
        let debug = format!("{transport:?}");
        assert!(debug.contains("StdioTransport"));
    }
}
