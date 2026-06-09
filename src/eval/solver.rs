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

use std::collections::HashMap;

use async_trait::async_trait;
use thiserror::Error;

use crate::error::ModeError;
use crate::eval::scorer::ExactMatch;
use crate::eval::task::{AnswerKind, EvalTask};
use crate::modes::{LinearMode, ReflectionMode};
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message, StorageTrait};

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

/// Solver wrapping the real [`ReflectionMode`] path, parameterized by the
/// `reflection_quality_threshold` under test.
///
/// The threshold gates how many refinement passes run (the loop stops once a
/// pass's quality meets it), so changing it changes the final answer — making it
/// a real SI-tunable lever the loop genuinely controls (a `ThresholdAdjust`
/// action over `reflection_quality_threshold`), unlike a hardcoded token cap.
/// `max_iterations` must be > 1 or the threshold has no effect (it only gates the
/// early stop).
pub struct ReflectionSolver<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    mode: ReflectionMode<S, C>,
    quality_threshold: f64,
    max_iterations: u32,
}

impl<S, C> ReflectionSolver<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Attempts per task before giving up — recovers reflection's intermittent
    /// stochastic parse failures so they do not drop items from a measurement.
    const MAX_ATTEMPTS: u32 = 4;

    /// Build a reflection solver with the `quality_threshold` under test.
    pub fn new(storage: S, client: C, quality_threshold: f64, max_iterations: u32) -> Self {
        Self {
            mode: ReflectionMode::new(storage, client),
            quality_threshold,
            max_iterations,
        }
    }
}

#[async_trait]
impl<S, C> Solver for ReflectionSolver<S, C>
where
    S: StorageTrait + Send + Sync,
    C: AnthropicClientTrait + Send + Sync,
{
    async fn solve(&self, task: &EvalTask) -> Result<SolverOutput, SolverError> {
        // Reflection's structured-JSON output is stochastic (temperature 0.7) and
        // intermittently fails to parse on hard problems. A bounded retry recovers
        // those transient failures so items are not spuriously dropped from a
        // paired measurement — it is not masking a bug, it is handling a
        // non-deterministic model response. The client already retries API/network
        // errors underneath this.
        let mut last_err: Option<ModeError> = None;
        for _ in 0..Self::MAX_ATTEMPTS {
            match self
                .mode
                .process(
                    &task.prompt,
                    None,
                    Some(self.max_iterations),
                    Some(self.quality_threshold),
                )
                .await
            {
                Ok(response) => {
                    return Ok(SolverOutput {
                        text: response.refined_reasoning,
                        mode: "reflection".to_string(),
                        confidence: Some(response.quality_score),
                    })
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.map_or_else(
            || {
                SolverError::Mode(ModeError::ApiUnavailable {
                    message: "reflection produced no result".to_string(),
                })
            },
            SolverError::Mode,
        ))
    }

    // Literal return is correct here; see LinearSolver::mode.
    #[allow(clippy::unnecessary_literal_bound)]
    fn mode(&self) -> &str {
        "reflection"
    }
}

/// Self-consistency solver: runs linear `samples` (K) times and majority-votes
/// the extracted answers.
///
/// K is the lever. Higher K is well-established to raise math accuracy
/// (self-consistency), and — unlike the threshold-reading modes — the output
/// stays cleanly `#### <answer>`-scoreable, because each sample is a clean linear
/// solve and the majority answer is re-emitted in that format. `K = 1` is plain
/// single-shot linear.
pub struct SelfConsistencySolver<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    mode: LinearMode<S, C>,
    samples: u32,
}

impl<S, C> SelfConsistencySolver<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Build a self-consistency solver drawing `samples` (K, clamped to >= 1)
    /// linear samples per task at the given sampling `temperature`, optionally
    /// with a `prompt` override (for testing prompt as an SI lever). `K = 1`,
    /// default temp, no prompt override is plain single-shot linear.
    pub fn new(
        storage: S,
        client: C,
        samples: u32,
        temperature: f64,
        prompt: Option<String>,
    ) -> Self {
        let mut mode = LinearMode::new(storage, client).with_temperature(temperature);
        if let Some(p) = prompt {
            mode = mode.with_prompt(p);
        }
        Self {
            mode,
            samples: samples.max(1),
        }
    }
}

#[async_trait]
impl<S, C> Solver for SelfConsistencySolver<S, C>
where
    S: StorageTrait + Send + Sync,
    C: AnthropicClientTrait + Send + Sync,
{
    async fn solve(&self, task: &EvalTask) -> Result<SolverOutput, SolverError> {
        // Per-sample bounded retry so a transient (rate-limit/parse) failure does
        // not lose a whole sample — critical at low K, where one failure would
        // otherwise drop the item from a paired measurement.
        const SAMPLE_ATTEMPTS: u32 = 3;
        let mut counts: HashMap<String, u32> = HashMap::new();
        let mut total = 0u32;
        let mut last_err: Option<ModeError> = None;
        for _ in 0..self.samples {
            for _ in 0..SAMPLE_ATTEMPTS {
                match self.mode.process(&task.prompt, None, None).await {
                    Ok(resp) => {
                        if let Some(ans) = ExactMatch::extract(&resp.content, AnswerKind::Numeric) {
                            *counts.entry(ans).or_insert(0) += 1;
                            total += 1;
                        }
                        break; // sample produced a response; stop retrying it
                    }
                    Err(e) => last_err = Some(e),
                }
            }
        }
        let Some((answer, votes)) = counts.into_iter().max_by_key(|(_, c)| *c) else {
            return Err(last_err.map_or_else(
                || {
                    SolverError::Mode(ModeError::ApiUnavailable {
                        message: "self-consistency produced no parseable answer".to_string(),
                    })
                },
                SolverError::Mode,
            ));
        };
        Ok(SolverOutput {
            // Re-emit the majority answer in the scoreable terminal format.
            text: format!("#### {answer}"),
            mode: "self_consistency".to_string(),
            confidence: Some(f64::from(votes) / f64::from(total.max(1))),
        })
    }

    // Literal return is correct here; see LinearSolver::mode.
    #[allow(clippy::unnecessary_literal_bound)]
    fn mode(&self) -> &str {
        "self_consistency"
    }
}

/// Raw-completion solver: sends the prompt directly and returns the model's
/// **raw text**, with no JSON envelope.
///
/// `LinearMode` requires a `{analysis, confidence}` JSON response and errors when
/// the model returns prose — which it frequently does on hard problems, dropping
/// those items from a measurement. An eval only needs the solution text ending in
/// `#### <answer>` (the scorer extracts it from prose), so this solver bypasses
/// the JSON requirement entirely. An optional `prefix` (e.g. a retrieved worked
/// exemplar) is prepended to the task prompt.
pub struct RawSolver<C>
where
    C: AnthropicClientTrait,
{
    client: C,
    prefix: Option<String>,
    max_tokens: u32,
    temperature: f64,
}

impl<C> RawSolver<C>
where
    C: AnthropicClientTrait,
{
    /// Number of attempts per task before giving up (handles transient errors on
    /// top of the client's own retry/backoff).
    const MAX_ATTEMPTS: u32 = 4;

    /// Build a raw solver. `prefix`, when set, is prepended before the task
    /// prompt (e.g. a worked exemplar for memory injection).
    pub fn new(client: C, prefix: Option<String>, max_tokens: u32, temperature: f64) -> Self {
        Self {
            client,
            prefix,
            max_tokens,
            temperature,
        }
    }
}

#[async_trait]
impl<C> Solver for RawSolver<C>
where
    C: AnthropicClientTrait + Send + Sync,
{
    async fn solve(&self, task: &EvalTask) -> Result<SolverOutput, SolverError> {
        let content = self.prefix.as_ref().map_or_else(
            || task.prompt.clone(),
            |p| format!("{p}\n\n{}", task.prompt),
        );
        let mut last_err: Option<ModeError> = None;
        for _ in 0..Self::MAX_ATTEMPTS {
            let messages = vec![Message::user(content.clone())];
            let config = CompletionConfig::new()
                .with_max_tokens(self.max_tokens)
                .with_temperature(self.temperature as f32);
            match self.client.complete(messages, config).await {
                Ok(resp) => {
                    return Ok(SolverOutput {
                        text: resp.content,
                        mode: "raw".to_string(),
                        confidence: None,
                    })
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.map_or_else(
            || {
                SolverError::Mode(ModeError::ApiUnavailable {
                    message: "raw solver produced no result".to_string(),
                })
            },
            SolverError::Mode,
        ))
    }

    // Literal return is correct here; see LinearSolver::mode.
    #[allow(clippy::unnecessary_literal_bound)]
    fn mode(&self) -> &str {
        "raw"
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
