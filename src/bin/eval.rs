//! Opt-in eval runner.
//!
//! Loads a JSONL dataset, runs a reasoning mode over it against the **live**
//! Anthropic API, scores each answer programmatically, and prints a report
//! (`n`, mean score, SE, extraction-failure rate, MDE) plus optional JSON.
//!
//! This binary is deliberately opt-in: it hits the live API and is never part of
//! normal CI. Per the harness plan, the first real run is where dataset adequacy
//! is judged — pre-register `n`, the metric, and the effect you care about, then
//! read the published MDE before trusting any delta.
//!
//! Usage: `eval <dataset.jsonl> [--json]`

// Enable the coverage attribute when running with nightly for llvm-cov exclusions
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::cast_precision_loss)]

use std::sync::atomic::{AtomicU32, Ordering};

use mcp_reasoning::anthropic::{AnthropicClient, ClientConfig};
use mcp_reasoning::config::Config;
use mcp_reasoning::eval::{run_eval_with_progress, Dataset, ExactMatch, LinearSolver};
use mcp_reasoning::storage::SqliteStorage;

#[cfg_attr(coverage_nightly, coverage(off))]
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(dataset_path) = args.get(1) else {
        eprintln!("usage: eval <dataset.jsonl> [--json]");
        std::process::exit(2);
    };
    let want_json = args.iter().any(|a| a == "--json");

    let dataset = match Dataset::from_jsonl_file(dataset_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("failed to load dataset '{dataset_path}': {e}");
            std::process::exit(1);
        }
    };

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("configuration error: {e}");
            std::process::exit(1);
        }
    };

    let storage = match SqliteStorage::new(&config.database_path).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("storage error: {e}");
            std::process::exit(1);
        }
    };

    let client_config = ClientConfig::new()
        .with_timeout_ms(config.request_timeout_ms)
        .with_max_retries(config.max_retries);
    let client = match AnthropicClient::new(config.api_key.expose(), client_config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("client error: {e}");
            std::process::exit(1);
        }
    };

    let solver = LinearSolver::new(storage, client);
    let scorer = ExactMatch::new();

    eprintln!(
        "running 'linear' over {} task(s) from {dataset_path} (live API)...",
        dataset.len()
    );
    // Stream a live progress counter to stderr so a long run is monitorable.
    let correct = AtomicU32::new(0);
    let outcome = run_eval_with_progress(&dataset, &solver, &scorer, |done, total, r| {
        if r.error.is_none() && r.score > 0.0 {
            correct.fetch_add(1, Ordering::Relaxed);
        }
        let acc = f64::from(correct.load(Ordering::Relaxed)) / done as f64;
        let flag = if r.extraction_failed {
            " INVALID"
        } else if r.error.is_some() {
            " ERROR"
        } else {
            ""
        };
        eprintln!(
            "[{done}/{total}] {} -> {} (running acc {acc:.3}){flag}",
            r.task_id,
            r.extracted.as_deref().unwrap_or("-"),
        );
    })
    .await;

    if want_json {
        match outcome.to_json() {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("failed to serialize outcome: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    print_report(&outcome);
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn print_report(outcome: &mcp_reasoning::eval::RunOutcome) {
    println!("mode: {}", outcome.mode);
    println!("tasks: {}", outcome.results.len());
    println!("solver errors: {}", outcome.solver_errors);
    match &outcome.report {
        Some(r) => {
            println!("n (scored): {}", r.n);
            println!("mean score: {:.4}", r.mean_score);
            println!("std error:  {:.4}", r.stderr);
            if let Some(cse) = r.clustered_stderr {
                println!("clustered SE: {cse:.4}");
            }
            println!("extraction-failure rate: {:.4}", r.extraction_failure_rate);
            println!("MDE (alpha=0.05, power=0.80): {:.4}", r.mde);
        }
        None => println!("report: insufficient scored items (need >= 2)"),
    }
}
