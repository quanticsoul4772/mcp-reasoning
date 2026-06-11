//! Server types and shared state.
//!
//! This module defines the core server types including application state
//! and the reasoning server wrapper.

use std::sync::Arc;

use tokio::sync::broadcast;

use super::progress::ProgressEvent;
use crate::agents::metrics::AgentMetricsCollector;
use crate::agents::{AgentRegistry, TeamRegistry};
use crate::anthropic::AnthropicClient;
use crate::config::Config;
use crate::metadata::MetadataBuilder;
use crate::metrics::MetricsCollector;
use crate::presets::PresetRegistry;
use crate::self_improvement::ManagerHandle;
use crate::skills::SkillRegistry;
use crate::storage::SqliteStorage;

/// Shared application state for all tool handlers.
///
/// This struct holds the configured components that tools need
/// to perform reasoning operations.
///
/// # Self-Improvement System
///
/// The `self_improvement` field provides access to the self-improvement system.
/// Self-improvement is ALWAYS enabled - it is a core feature, not optional.
#[derive(Clone)]
pub struct AppState {
    /// Storage backend for sessions and thoughts.
    pub storage: Arc<SqliteStorage>,
    /// Anthropic client for LLM calls.
    pub client: Arc<AnthropicClient>,
    /// Voyage AI client for embeddings/reranking (memory tools). `None` when
    /// `VOYAGE_API_KEY` is unset — the memory tools then return a config error.
    pub voyage_client: Option<Arc<crate::voyage::VoyageClient>>,
    /// Server configuration.
    pub config: Arc<Config>,
    /// Metrics collector for tracking tool usage.
    pub metrics: Arc<MetricsCollector>,
    /// Shared self-heal defect log (spec 001): records parse/schema failures of
    /// the server's own tool output and tracks recurrence across calls.
    pub defect_log: Arc<crate::self_improvement::heal::DefectLog>,
    /// Preset registry for workflow presets.
    pub presets: Arc<PresetRegistry>,
    /// Agent registry for available agents.
    pub agents: Arc<AgentRegistry>,
    /// Skill registry for composable skills.
    pub skills: Arc<SkillRegistry>,
    /// Team registry for agent team configurations.
    pub teams: Arc<TeamRegistry>,
    /// Agent metrics collector.
    pub agent_metrics: Arc<AgentMetricsCollector>,
    /// Self-improvement manager handle.
    ///
    /// This handle allows MCP tools to interact with the self-improvement system.
    /// Self-improvement is ALWAYS enabled - it is a core feature.
    pub self_improvement: Arc<ManagerHandle>,
    /// Metadata builder for enriching tool responses.
    pub metadata_builder: Arc<MetadataBuilder>,
    /// Broadcast sender for progress events.
    ///
    /// Tools use this to emit progress notifications during streaming operations.
    /// Clients can subscribe via `progress_tx.subscribe()`.
    pub progress_tx: broadcast::Sender<ProgressEvent>,
    /// In-memory activity bus for the real-time dashboard.
    ///
    /// Always present (cheap when unused); the dashboard sidecar subscribes when
    /// enabled. The same bus is injected into `metrics` so every tool call emits
    /// a completion activity. See [`crate::dashboard`].
    pub activity: crate::dashboard::ActivityBus,
}

impl AppState {
    /// Creates a new application state.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend
    /// * `client` - The Anthropic client
    /// * `config` - Server configuration
    /// * `metrics` - Shared metrics collector (used by both tools and self-improvement)
    /// * `self_improvement` - Self-improvement manager handle
    /// * `metadata_builder` - Metadata builder for tool responses
    /// * `progress_tx` - Broadcast sender for progress events
    #[must_use]
    pub fn new(
        storage: SqliteStorage,
        client: AnthropicClient,
        config: Config,
        metrics: Arc<MetricsCollector>,
        self_improvement: ManagerHandle,
        metadata_builder: MetadataBuilder,
        progress_tx: broadcast::Sender<ProgressEvent>,
    ) -> Self {
        let preset_registry = PresetRegistry::new();
        let skill_registry = SkillRegistry::with_presets(&preset_registry);
        // Build the Voyage client from config when a key is present. Construction
        // failure (not a missing key) is logged and treated as unavailable; the
        // memory tools surface a clear error rather than silently degrading.
        let voyage_client = config.voyage_api_key.as_ref().and_then(|key| {
            let vconfig = crate::anthropic::ClientConfig::default()
                .with_timeout_ms(config.request_timeout_ms)
                .with_max_retries(config.max_retries);
            match crate::voyage::VoyageClient::new(
                key.expose(),
                config.voyage_model.clone(),
                vconfig,
            ) {
                Ok(c) => Some(Arc::new(c)),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to build Voyage client; memory tools unavailable");
                    None
                }
            }
        });
        // Create the dashboard activity bus and inject it into the shared metrics
        // collector so every tool call's completion emits an activity event. The
        // bus is cheap and always present; it only does work when the dashboard
        // sidecar (off by default) has a subscriber.
        let activity = crate::dashboard::ActivityBus::new();
        metrics.set_activity(activity.clone());
        // Install the same bus as the process-global sink so cross-cutting seams
        // (Anthropic client, storage, the background loops) can emit without
        // threading the bus through their constructors.
        crate::dashboard::set_global(activity.clone());

        Self {
            storage: Arc::new(storage),
            client: Arc::new(client),
            voyage_client,
            config: Arc::new(config),
            metrics,
            defect_log: Arc::new(crate::self_improvement::heal::DefectLog::new(
                crate::self_improvement::heal::DEFAULT_RECURRENCE_THRESHOLD,
            )),
            presets: Arc::new(preset_registry),
            agents: Arc::new(AgentRegistry::new()),
            skills: Arc::new(skill_registry),
            teams: Arc::new(TeamRegistry::new()),
            agent_metrics: Arc::new(AgentMetricsCollector::new()),
            self_improvement: Arc::new(self_improvement),
            metadata_builder: Arc::new(metadata_builder),
            progress_tx,
            activity,
        }
    }

    /// Create a progress reporter for an operation.
    ///
    /// # Arguments
    ///
    /// * `token` - Unique identifier for the operation (or use request ID)
    #[must_use]
    pub fn create_progress_reporter(
        &self,
        token: impl Into<String>,
    ) -> super::progress::ProgressReporter {
        super::progress::ProgressReporter::new(token, self.progress_tx.clone())
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::unused_async
)]
mod tests {
    use super::*;
    use crate::anthropic::ClientConfig;
    use crate::config::{SecretString, SelfImprovementConfig};
    use crate::self_improvement::{SelfImprovementManager, SelfImprovementStorage};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

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

    fn mock_response(content: &str) -> CompletionResponse {
        CompletionResponse::new(content, Usage::new(100, 50))
    }

    fn create_mock_client() -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"{"summary": "Test", "confidence": 0.8, "actions": []}"#,
            ))
        });
        client
    }

    async fn create_test_handle(
        metrics: Arc<MetricsCollector>,
        storage: &SqliteStorage,
    ) -> ManagerHandle {
        let si_config = SelfImprovementConfig::default();
        let si_client = create_mock_client();
        let si_storage = Arc::new(SelfImprovementStorage::new(storage.pool.clone()));

        let (_manager, handle) =
            SelfImprovementManager::new(si_config, si_client, metrics, si_storage);
        handle
    }

    #[tokio::test]
    async fn test_app_state_new() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();
        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_handle(metrics.clone(), &storage).await;
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );
        let (progress_tx, _rx) = broadcast::channel(100);

        let state = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
            progress_tx,
        );

        // Verify all Arc wrappers are properly created
        assert!(Arc::strong_count(&state.storage) >= 1);
        assert!(Arc::strong_count(&state.client) >= 1);
        assert!(Arc::strong_count(&state.config) >= 1);
        assert!(Arc::strong_count(&state.metrics) >= 1);
        assert!(Arc::strong_count(&state.presets) >= 1);
        assert!(Arc::strong_count(&state.self_improvement) >= 1);
    }

    #[tokio::test]
    async fn test_app_state_debug() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();
        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_handle(metrics.clone(), &storage).await;
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );
        let (progress_tx, _rx) = broadcast::channel(100);

        let state = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
            progress_tx,
        );
        let debug = format!("{:?}", state);

        assert!(debug.contains("AppState"));
        assert!(debug.contains("config"));
    }

    #[tokio::test]
    async fn test_app_state_clone() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();
        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_handle(metrics.clone(), &storage).await;
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );
        let (progress_tx, _rx) = broadcast::channel(100);

        let state1 = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
            progress_tx,
        );
        let state2 = state1.clone();

        // Both should share the same Arc pointers
        assert!(Arc::ptr_eq(&state1.storage, &state2.storage));
        assert!(Arc::ptr_eq(&state1.client, &state2.client));
        assert!(Arc::ptr_eq(&state1.config, &state2.config));
        assert!(Arc::ptr_eq(&state1.metrics, &state2.metrics));
        assert!(Arc::ptr_eq(&state1.presets, &state2.presets));
        assert!(Arc::ptr_eq(
            &state1.self_improvement,
            &state2.self_improvement
        ));
    }

    #[test]
    fn test_app_state_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }

    #[test]
    fn test_app_state_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppState>();
    }
}
