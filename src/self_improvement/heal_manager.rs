//! Owning wrapper for the propose cycle (spec 001 T024b): holds the cycle's
//! dependencies and runs it on demand via [`HealManager::tick`].

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::StorageError;
use crate::metrics::MetricsCollector;
use crate::self_improvement::heal::DefectLog;
use crate::self_improvement::heal_cycle::{run_propose_cycle, ProposeCycleSummary};
use crate::self_improvement::repair::CommandRunner;
use crate::self_improvement::storage::SelfImprovementStorage;
use crate::traits::AnthropicClientTrait;

/// Owns the dependencies of the propose cycle and runs it on demand.
///
/// Constructed (only when the propose path is explicitly enabled) at startup with
/// a real workspace + `SystemCommandRunner`; a background task calls [`tick`] on an
/// interval. Reading the live [`DefectLog`] each tick means newly-recurring defects
/// are picked up without restarting.
///
/// [`tick`]: HealManager::tick
pub struct HealManager<C, R> {
    client: C,
    runner: R,
    storage: Arc<SelfImprovementStorage>,
    defect_log: Arc<DefectLog>,
    metrics: Arc<MetricsCollector>,
    workspace: PathBuf,
    max_proposals: usize,
}

impl<C, R> HealManager<C, R>
where
    C: AnthropicClientTrait,
    R: CommandRunner,
{
    /// Assemble a heal manager.
    #[must_use]
    pub fn new(
        client: C,
        runner: R,
        storage: Arc<SelfImprovementStorage>,
        defect_log: Arc<DefectLog>,
        metrics: Arc<MetricsCollector>,
        workspace: PathBuf,
        max_proposals: usize,
    ) -> Self {
        Self {
            client,
            runner,
            storage,
            defect_log,
            metrics,
            workspace,
            max_proposals,
        }
    }

    /// Run one propose cycle over the defects currently marked `Recurring`. A
    /// no-op (empty summary, no side effects) when nothing is recurring.
    ///
    /// # Errors
    /// Propagates a storage failure from the cycle; LLM/repair failures are counted
    /// in the returned summary, not propagated.
    pub async fn tick(&self) -> Result<ProposeCycleSummary, StorageError> {
        let recurring = self.defect_log.recurring();
        if recurring.is_empty() {
            return Ok(ProposeCycleSummary::default());
        }
        // Most recent model-version change (spec 002, FR-005); a defect whose window
        // overlaps it routes to drift rather than the repair path.
        let latest_model_change = self
            .metrics
            .model_version_changes()
            .iter()
            .map(|c| c.at_millis)
            .max();
        run_propose_cycle(
            &self.client,
            &self.runner,
            &self.storage,
            &recurring,
            &self.workspace,
            self.max_proposals,
            latest_model_change,
        )
        .await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::self_improvement::heal::{FailureClass, DEFAULT_RECURRENCE_THRESHOLD};
    use crate::self_improvement::repair::testutil::{failing, passing, ScriptedRunner};
    use crate::self_improvement::repair::CommandOutput;
    use crate::storage::SqliteStorage;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};
    use serial_test::serial;

    const LOCALIZE: &str =
        r#"{"component": "reasoning_linear/linear", "source_hint": "src/modes/linear.rs"}"#;
    const SYNTH: &str = r##"{"test_name": "heal_repro_parse", "test_path": "tests/heal_repro_parse.rs", "test_code": "#[test]\nfn heal_repro_parse() { assert!(false); }\n"}"##;
    const FIX: &str = r#"{"change_summary": "broaden the JSON parser", "files": [{"path": "src/modes/linear.rs", "contents": "// fixed\n"}]}"#;

    fn staged_client() -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(move |messages, _| {
            let text: String = messages.iter().map(|m| m.content.clone()).collect();
            let body = if text.contains("REPRODUCES") {
                SYNTH
            } else if text.contains("source_hint") {
                LOCALIZE
            } else {
                FIX
            };
            Ok(CompletionResponse::new(body, Usage::new(10, 10)))
        });
        client
    }

    fn ok() -> CommandOutput {
        CommandOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    fn full_admissible_runner() -> ScriptedRunner {
        ScriptedRunner::new(vec![
            failing(),
            ok(),
            passing(),
            passing(),
            ok(),
            ok(),
            ok(),
            ok(),
            CommandOutput {
                status: 0,
                stdout: "https://github.com/o/r/pull/5\n".to_string(),
                stderr: String::new(),
            },
        ])
    }

    async fn storage() -> Arc<SelfImprovementStorage> {
        let sqlite = SqliteStorage::new_in_memory().await.expect("storage");
        Arc::new(SelfImprovementStorage::new(sqlite.pool.clone()))
    }

    #[tokio::test]
    #[serial]
    async fn tick_proposes_for_a_stable_path_recurring_defect() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let log = Arc::new(DefectLog::new(DEFAULT_RECURRENCE_THRESHOLD));
        let metrics = Arc::new(MetricsCollector::new());
        // Three observations of the SAME input → Recurring AND propose-eligible.
        for t in 1..=3 {
            log.observe(
                &metrics,
                "reasoning_linear/linear",
                FailureClass::Parse,
                "bad",
                t,
            );
        }
        assert_eq!(log.recurring().len(), 1);

        let mgr = HealManager::new(
            staged_client(),
            full_admissible_runner(),
            Arc::clone(&s),
            Arc::clone(&log),
            Arc::clone(&metrics),
            dir.path().to_path_buf(),
            5,
        );
        let summary = mgr.tick().await.unwrap();
        assert_eq!(summary.proposed, 1);
    }

    #[tokio::test]
    #[serial]
    async fn tick_is_a_noop_with_no_recurring_defects() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let log = Arc::new(DefectLog::new(3));
        let metrics = Arc::new(MetricsCollector::new());
        let mgr = HealManager::new(
            staged_client(),
            ScriptedRunner::new(vec![]),
            Arc::clone(&s),
            Arc::clone(&log),
            Arc::clone(&metrics),
            dir.path().to_path_buf(),
            5,
        );
        let summary = mgr.tick().await.unwrap();
        assert_eq!(summary, ProposeCycleSummary::default());
    }
}
