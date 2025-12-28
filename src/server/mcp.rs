//! Main MCP server orchestration.
//!
//! This module provides the main entry point for running the MCP reasoning server.

use std::sync::Arc;

use crate::anthropic::{AnthropicClient, ClientConfig};
use crate::config::Config;
use crate::error::AppError;
use crate::storage::SqliteStorage;

use super::tools::ReasoningServer;
use super::transport::StdioTransport;
use super::types::AppState;

/// Main MCP server that orchestrates all components.
///
/// This struct provides the main entry point for the MCP reasoning server,
/// handling initialization of storage, client, and transport.
#[derive(Debug)]
pub struct McpServer {
    /// Server configuration.
    config: Config,
}

impl McpServer {
    /// Creates a new MCP server with the given configuration.
    #[must_use]
    pub const fn new(config: Config) -> Self {
        Self { config }
    }

    /// Runs the server using stdio transport.
    ///
    /// This function initializes all components and starts serving requests
    /// over stdin/stdout. It blocks until the client disconnects or an error occurs.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage initialization fails
    /// - Anthropic client creation fails
    /// - Server encounters a runtime error
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub async fn run_stdio(&self) -> Result<(), AppError> {
        // Initialize storage
        let storage = SqliteStorage::new(&self.config.database_path).await?;

        // Create Anthropic client
        let client_config = ClientConfig::default()
            .with_timeout_ms(self.config.request_timeout_ms)
            .with_max_retries(self.config.max_retries);
        let client = AnthropicClient::new(self.config.api_key.expose(), client_config)?;

        // Create app state
        let state = AppState::new(storage, client, self.config.clone());

        // Create reasoning server
        let server = ReasoningServer::new(Arc::new(state));

        // Run with stdio transport
        let transport = StdioTransport::new();
        let running = transport.serve(server).await?;

        // Wait for server to complete
        let _ = running.waiting().await;

        Ok(())
    }

    /// Returns the server configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SecretString;

    fn test_config() -> Config {
        Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    #[test]
    fn test_mcp_server_new() {
        let config = test_config();
        let server = McpServer::new(config);
        assert_eq!(server.config().max_retries, 3);
    }

    #[test]
    fn test_mcp_server_debug() {
        let config = test_config();
        let server = McpServer::new(config);
        let debug = format!("{server:?}");
        assert!(debug.contains("McpServer"));
    }

    #[test]
    fn test_mcp_server_config_accessor() {
        let mut config = test_config();
        config.database_path = "/tmp/test.db".to_string();
        config.max_retries = 5;
        config.request_timeout_ms = 60000;
        let server = McpServer::new(config);
        assert_eq!(server.config().database_path, "/tmp/test.db");
        assert_eq!(server.config().request_timeout_ms, 60000);
    }
}
