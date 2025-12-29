//! MCP Reasoning Server binary entry point.
//!
//! This binary provides a stdio-based MCP server for structured reasoning.
//! All logs go to stderr; stdout is reserved for MCP JSON-RPC messages.
//!
//! Coverage is excluded because the main function cannot be unit tested
//! as it requires the full MCP protocol handshake over stdio.

// Enable the coverage attribute when running with nightly for llvm-cov exclusions
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use mcp_reasoning::config::Config;
use mcp_reasoning::server::McpServer;

#[cfg_attr(coverage_nightly, coverage(off))]
#[tokio::main]
async fn main() {
    // Initialize logging to stderr only (stdout is for MCP JSON-RPC)
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string())
                .parse()
                .unwrap_or_else(|_| tracing_subscriber::filter::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("mcp-reasoning starting...");

    // Load configuration from environment
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        "Configuration loaded: database={}, timeout={}ms",
        config.database_path,
        config.request_timeout_ms
    );

    // Create and run server
    let server = McpServer::new(config);
    if let Err(e) = server.run_stdio().await {
        tracing::error!("Server error: {e}");
        std::process::exit(1);
    }

    tracing::info!("mcp-reasoning shutdown complete");
}
