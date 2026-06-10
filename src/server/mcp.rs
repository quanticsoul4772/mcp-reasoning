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
        let si_storage = Arc::new(SelfImprovementStorage::new(storage.pool.clone()));

        // Initialize self-improvement system (ALWAYS enabled - core feature)
        let si_config = SelfImprovementConfig::from_env();

        // Start from the env config; when opted in, apply recorded SI config
        // overrides over it so an approved/auto-executed config change actually
        // takes effect (bounded to allowlisted, validated fields). Off by
        // default — SI stays advisory unless SELF_IMPROVEMENT_APPLY_OVERRIDES.
        let mut config = self.config.clone();
        if si_config.apply_config_overrides {
            let applied = apply_stored_overrides(&mut config, &si_storage).await;
            if applied.is_empty() {
                tracing::info!("No applicable self-improvement config overrides found");
            } else {
                tracing::info!(
                    applied = ?applied,
                    "Applied self-improvement config overrides at startup"
                );
            }
        }

        // Initialize metrics collector (shared between MCP tools and self-improvement)
        let metrics = Arc::new(MetricsCollector::new());

        // Create Anthropic client for MCP tools. It records each call's pinned
        // model identifier into metrics so a model-version change is detected
        // (spec 001, FR-017) and the drift classifier can use it.
        let client_config = ClientConfig::default()
            .with_timeout_ms(config.request_timeout_maximum_ms) // Use maximum timeout for deep thinking modes
            .with_max_retries(config.max_retries);
        let client = AnthropicClient::new(config.api_key.expose(), client_config)?
            .with_metrics(Arc::clone(&metrics));

        let si_client_config = ClientConfig::default()
            .with_timeout_ms(config.request_timeout_maximum_ms) // Use maximum timeout for deep thinking modes
            .with_max_retries(config.max_retries);
        let si_client = AnthropicClient::new(config.api_key.expose(), si_client_config)?;

        let (si_manager, si_handle) = SelfImprovementManager::new(
            si_config.clone(),
            si_client,
            metrics.clone(),
            Arc::clone(&si_storage),
        );

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
            config.factory_timeout_ms,
        )
        .with_metrics(Arc::clone(&metrics));

        // Create the progress broadcast bus. The sender lives in AppState so modes
        // can emit milestones; the per-call MCP forwarder (tools/progress_bridge.rs)
        // subscribes its own receiver for each streaming tool call, so this startup
        // receiver is intentionally dropped.
        let (progress_tx, _progress_rx) = super::progress::create_progress_channel();

        // Create app state with shared metrics and self-improvement handle
        let state = AppState::new(
            storage,
            client,
            config.clone(),
            metrics,
            si_handle,
            metadata_builder,
            progress_tx,
        );

        // Spawn the self-heal propose loop ONLY when explicitly enabled AND a
        // workspace is configured (Constitution IV: default-off, operator opt-in).
        // This is the ONLY path that opens PRs against the repo; it never merges.
        if si_config.heal_propose_enabled {
            if let Some(workspace) = si_config
                .heal_workspace
                .clone()
                .filter(|w| !w.trim().is_empty())
            {
                let heal_client_config = ClientConfig::default()
                    .with_timeout_ms(config.request_timeout_maximum_ms)
                    .with_max_retries(config.max_retries);
                let heal_client =
                    AnthropicClient::new(config.api_key.expose(), heal_client_config)?;
                let heal_manager = crate::self_improvement::heal_manager::HealManager::new(
                    heal_client,
                    crate::self_improvement::repair::SystemCommandRunner,
                    Arc::clone(&si_storage),
                    Arc::clone(&state.defect_log),
                    Arc::clone(&state.metrics),
                    std::path::PathBuf::from(&workspace),
                    si_config.heal_max_proposals,
                );
                let interval_secs = si_config.cycle_interval_secs;
                let mut heal_shutdown = shutdown_tx.subscribe();
                tokio::spawn(async move {
                    tracing::warn!(
                        workspace = %workspace,
                        max_proposals = si_config.heal_max_proposals,
                        "Self-heal propose loop ENABLED — opens operator-reviewed PRs for recurring defects (never merges)"
                    );
                    let mut ticker =
                        tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                    loop {
                        tokio::select! {
                            _ = ticker.tick() => match heal_manager.tick().await {
                                Ok(s) if s.proposed + s.not_admissible + s.reused + s.drift + s.held_back + s.errored > 0 => {
                                    tracing::info!(
                                        proposed = s.proposed,
                                        not_admissible = s.not_admissible,
                                        reused = s.reused,
                                        drift = s.drift,
                                        held_back = s.held_back,
                                        errored = s.errored,
                                        "Self-heal propose cycle complete"
                                    );
                                }
                                Ok(_) => {}
                                Err(e) => tracing::error!(error = %e, "Self-heal propose cycle storage error"),
                            },
                            _ = heal_shutdown.changed() => {
                                if *heal_shutdown.borrow() {
                                    break;
                                }
                            }
                        }
                    }
                    tracing::info!("Self-heal propose loop stopped");
                });
            } else {
                tracing::warn!(
                    "SELF_HEAL_PROPOSE_ENABLED is set but SELF_HEAL_WORKSPACE is empty — heal propose loop NOT started"
                );
            }
        }

        // Spawn the background embedding worker when Voyage is configured, so the
        // embedding cost is paid ahead of the first search/relate instead of on
        // that call. Shares the self-improvement shutdown signal.
        if let Some(voyage) = state.voyage_client.clone() {
            let worker_storage = state.storage.clone();
            let worker_model = config.voyage_model.clone();
            let worker_shutdown = shutdown_tx.subscribe();
            tokio::spawn(async move {
                tracing::info!("Embedding worker started");
                crate::modes::memory::run_embed_worker(
                    worker_storage,
                    voyage,
                    worker_model,
                    std::time::Duration::from_secs(30),
                    worker_shutdown,
                )
                .await;
                tracing::info!("Embedding worker stopped");
            });
        }

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

/// Load persisted self-improvement config overrides and apply them over
/// `config`, returning the keys actually applied.
///
/// This is the startup bridge that lets a recorded SI recommendation reach the
/// running server (bounded by the allowlist + `Config::apply_overrides`
/// validation). A storage error is logged and treated as "nothing applied" so
/// startup never fails on it.
async fn apply_stored_overrides(
    config: &mut Config,
    si_storage: &SelfImprovementStorage,
) -> Vec<String> {
    match si_storage.get_all_config_overrides().await {
        Ok(records) => {
            let overrides: Vec<(String, serde_json::Value)> = records
                .into_iter()
                .map(|r| {
                    let value = serde_json::from_str(&r.value_json)
                        .unwrap_or(serde_json::Value::String(r.value_json));
                    (r.key, value)
                })
                .collect();
            config.apply_overrides(&overrides)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to load config overrides; using env config");
            Vec::new()
        }
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
            factory_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
            voyage_api_key: None,
            voyage_model: "voyage-4".to_string(),
            high_confidence_threshold: 0.75,
            reflection_quality_threshold: 0.8,
            mcts_quality_threshold: 0.5,
            graph_prune_threshold: 0.3,
        }
    }

    #[test]
    fn test_mcp_server_new() {
        let config = test_config();
        let server = McpServer::new(config);
        assert_eq!(server.config().max_retries, 3);
    }

    // End-to-end proof of the apply-overrides chain: a recommendation persisted
    // to `config_overrides` (via the real storage layer) is loaded and applied
    // over `Config` by the same helper the server runs at startup.
    #[tokio::test]
    async fn test_apply_stored_overrides_end_to_end() {
        use crate::self_improvement::storage::ConfigOverrideRecord;
        use crate::self_improvement::SelfImprovementStorage;
        use crate::storage::SqliteStorage;
        use chrono::Utc;

        let sqlite = SqliteStorage::new_in_memory().await.expect("storage");
        let si_storage = SelfImprovementStorage::new(sqlite.pool.clone());

        // Persist overrides exactly as a successful SI cycle would: real Config
        // keys (a timeout, a retry count, and a decision threshold) + one bogus
        // key that must be ignored.
        for (key, value_json) in [
            ("request_timeout_ms", "45000"),
            ("max_retries", "5"),
            ("mcts_quality_threshold", "0.4"),
            ("threshold:nonexistent", "0.9"),
        ] {
            si_storage
                .upsert_config_override(&ConfigOverrideRecord {
                    key: key.to_string(),
                    value_json: value_json.to_string(),
                    applied_by_action: None,
                    updated_at: Utc::now(),
                })
                .await
                .expect("seed override");
        }

        let mut config = test_config();
        let applied = apply_stored_overrides(&mut config, &si_storage).await;

        // The three real keys applied; the bogus one was skipped.
        assert_eq!(applied.len(), 3);
        assert!(!applied.iter().any(|k| k.starts_with("threshold:")));
        assert_eq!(config.request_timeout_ms, 45_000);
        assert_eq!(config.max_retries, 5);
        assert!((config.mcts_quality_threshold - 0.4).abs() < f64::EPSILON);
        // The recorded config stays valid against the same bounds.
        assert!(crate::config::validate_config(&config).is_ok());
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
