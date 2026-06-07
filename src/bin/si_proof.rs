//! Opt-in proof that the self-improvement loop measures and rewards a *real*
//! improvement on real problems — the end-to-end demonstration the loop never
//! had (it used to fabricate its measurement).
//!
//! What it does, against the live API:
//! 1. Builds a `ThresholdAdjust` action over `reflection_quality_threshold` and
//!    validates it through the real SI allowlist (the loop's gate on what it may
//!    tune).
//! 2. Derives a baseline and a changed `Config` via the real `apply_overrides`
//!    path, and builds two `ReflectionSolver`s differing only by that threshold.
//! 3. Runs the sensor (`measure_delta`) over a held-out AIME slice to get the
//!    measured paired delta + the lower-CB `clears_mde` verdict.
//! 4. Feeds that delta to the reconciled reward (`reward_from_measured_delta`,
//!    gated on `clears_mde`) and the divergence tripwire.
//!
//! This is opt-in and never part of CI. A small slice has little statistical
//! power (the MDE will be large), so it proves the *mechanism* works end to end,
//! not that this particular threshold change is significant.
//!
//! Usage: `si_proof [dataset.jsonl] [slice_count]`

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::too_many_lines)] // main() is a linear, readable proof script.

use mcp_reasoning::anthropic::{AnthropicClient, ClientConfig};
use mcp_reasoning::config::Config;
use mcp_reasoning::eval::{Dataset, ExactMatch, ReflectionSolver, Solver};
use mcp_reasoning::self_improvement::{
    measure_delta, ActionType, Allowlist, AllowlistConfig, CircuitBreaker, Learner,
    SelfImprovementAction,
};
use mcp_reasoning::storage::SqliteStorage;

const THRESHOLD_KEY: &str = "reflection_quality_threshold";
const BASELINE_THRESHOLD: f64 = 0.60;
const CHANGED_THRESHOLD: f64 = 0.95;
const MAX_ITERATIONS: u32 = 2;
const PRE_REGISTERED_MDE: f64 = 0.05;
const ALPHA: f64 = 0.05;
const DIVERGENCE_THRESHOLD: f64 = 0.20;

#[cfg_attr(coverage_nightly, coverage(off))]
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dataset_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "eval/data/aime_1k.jsonl".to_string());
    let slice_count: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => fail(&format!("config error: {e}")),
    };

    // --- 1. Validate the ThresholdAdjust action through the real SI allowlist ---
    let action = SelfImprovementAction::new(
        "si-proof-1",
        ActionType::ThresholdAdjust,
        "Tune reflection_quality_threshold",
        "Proof: measure the effect of raising the reflection quality threshold",
        0.1,
    )
    .with_parameters(serde_json::json!({ THRESHOLD_KEY: CHANGED_THRESHOLD }));

    let mut allowlist = Allowlist::new(AllowlistConfig::default());
    match allowlist.validate(&action) {
        Ok(()) => println!("[1] allowlist: ThresholdAdjust({THRESHOLD_KEY}) ACCEPTED"),
        Err(e) => fail(&format!("[1] allowlist REJECTED the action: {e}")),
    }

    // --- 2. Derive baseline and changed thresholds via the real override path ---
    let mut changed_config = config.clone();
    let applied = changed_config.apply_overrides(&[(
        THRESHOLD_KEY.to_string(),
        serde_json::json!(CHANGED_THRESHOLD),
    )]);
    if !applied.iter().any(|k| k == THRESHOLD_KEY) {
        fail("[2] apply_overrides did not apply the threshold key");
    }
    println!(
        "[2] apply_overrides applied {applied:?}; reflection_quality_threshold {} -> {}",
        BASELINE_THRESHOLD, changed_config.reflection_quality_threshold
    );

    // --- 3. Build two solvers differing only by the threshold; measure on AIME ---
    let held_out = match load_slice(&dataset_path, slice_count) {
        Ok(ds) => ds,
        Err(e) => fail(&format!("[3] dataset load failed: {e}")),
    };
    println!(
        "[3] held-out slice: {} items from {dataset_path} (live API, reflection x{MAX_ITERATIONS})...",
        held_out.len()
    );

    let baseline_solver = match build_solver(&config, BASELINE_THRESHOLD).await {
        Ok(s) => s,
        Err(e) => fail(&format!("baseline solver: {e}")),
    };
    let changed_solver =
        match build_solver(&config, changed_config.reflection_quality_threshold).await {
            Ok(s) => s,
            Err(e) => fail(&format!("changed solver: {e}")),
        };

    // Diagnostic: surface what the reflection solver actually returns on one item.
    if let Some(first) = held_out.tasks().first() {
        match baseline_solver.solve(first).await {
            Ok(out) => {
                let preview: String = out.text.chars().take(160).collect();
                println!("[diag] baseline solve OK on {}: \"{preview}\"...", first.id);
            }
            Err(e) => println!("[diag] baseline solve ERROR on {}: {e}", first.id),
        }
    }

    let scorer = ExactMatch::new();
    let Some(delta) = measure_delta(
        &held_out,
        &baseline_solver,
        &changed_solver,
        &scorer,
        PRE_REGISTERED_MDE,
        ALPHA,
    )
    .await
    else {
        fail("[3] measure_delta returned None (fewer than 2 paired items)");
    };

    println!(
        "[3] measured paired delta (changed - baseline): mean {:+.4}, SE {:.4}, n_paired {}",
        delta.estimate.mean, delta.estimate.stderr, delta.n_paired
    );
    println!(
        "    clears_mde (lower CB >= {PRE_REGISTERED_MDE}): {}",
        delta.clears_mde
    );

    // --- 4. Reconciled reward (gated on clears_mde) + divergence tripwire ---
    let learner = Learner::with_defaults();
    let reward = learner.reward_from_measured_delta(&delta, true);
    println!("[4] reward_from_measured_delta (gated on clears_mde): {reward:+.4}");

    let mut breaker = CircuitBreaker::with_defaults();
    let tripped = breaker.record_divergence(
        action.expected_improvement,
        delta.estimate.mean,
        DIVERGENCE_THRESHOLD,
    );
    println!(
        "[4] tripwire: proxy {:+.3} vs measured {:+.3} (threshold {DIVERGENCE_THRESHOLD}) -> {}",
        action.expected_improvement,
        delta.estimate.mean,
        if tripped { "TRIPPED" } else { "ok" }
    );

    println!(
        "\nDONE: the loop validated the action, applied the override, measured a real delta on \
         real problems, and produced a gated reward + tripwire verdict — no fabricated number."
    );
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

/// Load the first `count` tasks of a dataset as a standalone held-out slice.
#[cfg_attr(coverage_nightly, coverage(off))]
fn load_slice(path: &str, count: usize) -> Result<Dataset, String> {
    let full = Dataset::from_jsonl_file(path).map_err(|e| e.to_string())?;
    let lines: Vec<String> = full
        .tasks()
        .iter()
        .take(count)
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Dataset::from_jsonl(&lines.join("\n")).map_err(|e| e.to_string())
}

/// Build a reflection solver at `threshold` over a fresh in-memory storage + the
/// live client.
#[cfg_attr(coverage_nightly, coverage(off))]
async fn build_solver(
    config: &Config,
    threshold: f64,
) -> Result<ReflectionSolver<SqliteStorage, AnthropicClient>, String> {
    let storage = SqliteStorage::new(":memory:")
        .await
        .map_err(|e| e.to_string())?;
    let client_config = ClientConfig::new()
        .with_timeout_ms(config.request_timeout_maximum_ms)
        .with_max_retries(config.max_retries);
    let client =
        AnthropicClient::new(config.api_key.expose(), client_config).map_err(|e| e.to_string())?;
    Ok(ReflectionSolver::new(
        storage,
        client,
        threshold,
        MAX_ITERATIONS,
    ))
}
