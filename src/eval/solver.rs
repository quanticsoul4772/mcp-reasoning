//! Solvers — adapters that run a reasoning mode to a final answer.
//!
//! A [`Solver`] wraps the **real** mode path (e.g. [`LinearMode::process`]); it
//! does not reimplement a parallel "canonical sequence," or the harness would be
//! measuring a construction that the server never runs. The client is injected
//! via the existing trait DI, so a solver can be driven by the real
//! `AnthropicClient` (pointed at the live API or a wiremock server) or by a
//! trait mock, with no change to the solver itself.
//!
//! [`LinearSolver`] is the v1 adapter (linear is the simplest single-answer
//! path). Further per-mode adapters follow the same shape: hold the mode, call
//! its real `process`, return the conclusion text as [`SolverOutput::text`].

use async_trait::async_trait;
use thiserror::Error;

use crate::error::ModeError;
use crate::eval::task::EvalTask;
use crate::modes::LinearMode;
use crate::traits::{AnthropicClientTrait, StorageTrait};

/// The output of running a solver on one task: the text to be scored, plus
/// which mode produced it and the mode's self-reported confidence (when any).
#[derive(Debug, Clone, PartialEq)]
pub struct SolverOutput {
    /// The mode's conclusion text — what the scorer extracts an answer from.
    pub text: String,
    /// The mode that produced this output (e.g. `"linear"`).
    pub mode: String,
    /// The mode's self-reported confidence, if it exposes one.
    pub confidence: Option<f64>,
}

/// Errors raised while solving a task.
#[derive(Debug, Error)]
pub enum SolverError {
    /// The underlying mode failed (API error, parse failure, etc.).
    #[error("mode execution failed: {0}")]
    Mode(#[from] ModeError),
}

/// Runs a reasoning mode to a final answer for an [`EvalTask`].
#[async_trait]
pub trait Solver: Send + Sync {
    /// Solve `task`, returning the conclusion text to be scored.
    async fn solve(&self, task: &EvalTask) -> Result<SolverOutput, SolverError>;

    /// The mode this solver runs (e.g. `"linear"`).
    fn mode(&self) -> &str;
}

/// Solver wrapping the real [`LinearMode`] path.
pub struct LinearSolver<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    mode: LinearMode<S, C>,
}

impl<S, C> LinearSolver<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Build a linear solver over the given storage and client.
    pub fn new(storage: S, client: C) -> Self {
        Self {
            mode: LinearMode::new(storage, client),
        }
    }
}

#[async_trait]
impl<S, C> Solver for LinearSolver<S, C>
where
    S: StorageTrait + Send + Sync,
    C: AnthropicClientTrait + Send + Sync,
{
    async fn solve(&self, task: &EvalTask) -> Result<SolverOutput, SolverError> {
        // Each task is scored independently: no session threading, no prior
        // context, no confidence threshold — the eval measures the mode's
        // single-shot answer to the prompt.
        let response = self.mode.process(&task.prompt, None, None).await?;
        Ok(SolverOutput {
            text: response.content,
            mode: "linear".to_string(),
            confidence: Some(response.confidence),
        })
    }

    // The trait returns `&str` (other solvers borrow a runtime field); this impl
    // happens to return a literal, which is correct, not a missed `'static`.
    #[allow(clippy::unnecessary_literal_bound)]
    fn mode(&self) -> &str {
        "linear"
    }
}

/// Deterministic in-memory solver for tests: returns a canned output per task
/// id (falling back to a default), or a configured error for selected ids.
#[derive(Debug, Clone, Default)]
pub struct MockSolver {
    outputs: std::collections::HashMap<String, String>,
    errors: std::collections::HashSet<String>,
    default_text: String,
    mode: String,
}

impl MockSolver {
    /// Create a mock solver returning `default_text` for any unconfigured task.
    pub fn new(default_text: impl Into<String>) -> Self {
        Self {
            outputs: std::collections::HashMap::new(),
            errors: std::collections::HashSet::new(),
            default_text: default_text.into(),
            mode: "mock".to_string(),
        }
    }

    /// Configure the output text for a specific task id.
    #[must_use]
    pub fn with_output(mut self, task_id: impl Into<String>, text: impl Into<String>) -> Self {
        self.outputs.insert(task_id.into(), text.into());
        self
    }

    /// Configure a task id to fail (solver error) instead of producing output.
    #[must_use]
    pub fn with_error(mut self, task_id: impl Into<String>) -> Self {
        self.errors.insert(task_id.into());
        self
    }

    /// Override the reported mode label.
    #[must_use]
    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
        self
    }
}

#[async_trait]
impl Solver for MockSolver {
    async fn solve(&self, task: &EvalTask) -> Result<SolverOutput, SolverError> {
        if self.errors.contains(&task.id) {
            return Err(SolverError::Mode(ModeError::ApiUnavailable {
                message: format!("mock solver error for task {}", task.id),
            }));
        }
        let text = self
            .outputs
            .get(&task.id)
            .cloned()
            .unwrap_or_else(|| self.default_text.clone());
        Ok(SolverOutput {
            text,
            mode: self.mode.clone(),
            confidence: None,
        })
    }

    fn mode(&self) -> &str {
        &self.mode
    }
}
