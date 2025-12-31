//! Main MCP server orchestration.
//!
//! This module provides the main entry point for running the MCP reasoning server.

use std::sync::Arc;

use tokio::sync::watch;

use crate::anthropic::{AnthropicClient, ClientConfig};
use crate::config::{Config, SelfImprovementConfig};
use crate::error::AppError;
use crate::metrics::MetricsCollector;
use crate::self_improvement::{SelfImprovementManager, SelfImprovementStorage};
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
    /// The self-improvement system is automatically started as a background task.
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

        // Create Anthropic client for MCP tools
        let client_config = ClientConfig::default()
            .with_timeout_ms(self.config.request_timeout_maximum_ms) // Use maximum timeout for deep thinking modes
            .with_max_retries(self.config.max_retries);
        let client = AnthropicClient::new(self.config.api_key.expose(), client_config)?;

        // Initialize metrics collector (shared between MCP tools and self-improvement)
        let metrics = Arc::new(MetricsCollector::new());

        // Initialize self-improvement system (ALWAYS enabled - core feature)
        let si_config = SelfImprovementConfig::from_env();
        let si_client_config = ClientConfig::default()
            .with_timeout_ms(self.config.request_timeout_maximum_ms) // Use maximum timeout for deep thinking modes
            .with_max_retries(self.config.max_retries);
        let si_client = AnthropicClient::new(self.config.api_key.expose(), si_client_config)?;
        let si_storage = Arc::new(SelfImprovementStorage::new(storage.pool.clone()));

        let (si_manager, si_handle) =
            SelfImprovementManager::new(si_config.clone(), si_client, metrics.clone(), si_storage);

        // Create shutdown channel for self-improvement manager
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Spawn self-improvement background task
        tokio::spawn(async move {
            tracing::info!(
                cycle_interval_secs = si_config.cycle_interval_secs,
                min_invocations = si_config.min_invocations_for_analysis,
                require_approval = si_config.require_approval,
                "Self-improvement system started"
            );
            si_manager.run(shutdown_rx).await;
            tracing::info!("Self-improvement system stopped");
        });

        // Initialize metadata builder for tool response enrichment
        let timing_db = Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
            storage.clone(),
        )));
        let preset_index = Arc::new(crate::metadata::PresetIndex::build());
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            timing_db,
            preset_index,
            30_000, // Factory timeout (30s) - TODO: make configurable
        );

        // Create app state with shared metrics and self-improvement handle
        let state = AppState::new(
            storage,
            client,
            self.config.clone(),
            metrics,
            si_handle,
            metadata_builder,
        );

        // Create reasoning server
        let server = ReasoningServer::new(Arc::new(state));

        // Run with stdio transport
        let transport = StdioTransport::new();
        let running = transport.serve(server).await?;

        // Wait for server to complete
        let _ = running.waiting().await;

        // Signal self-improvement system to shutdown
        let _ = shutdown_tx.send(true);

        Ok(())
    }

    /// Returns the server configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;
    use crate::config::SecretString;

    fn test_config() -> Config {
        Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            request_timeout_deep_ms: 60000,
            request_timeout_maximum_ms: 120000,
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
