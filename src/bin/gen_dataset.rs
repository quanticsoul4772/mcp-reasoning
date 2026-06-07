//! Deterministic perturbed word-problem dataset generator (pilot).
//!
//! Implements the GSM-Symbolic / GSM-Plus approach from the dataset plan
//! (docs/design/EVAL_DATASET_PLAN.md): take a handful of multi-step word-problem
//! templates and emit many novel instances by varying the numbers, names, and
//! item nouns, plus an optional extra clause as the difficulty knob. Every
//! instance is computed, so the answer is exact-matchable with no LLM judge.
//!
//! Generation is fully deterministic (parameters are derived from the instance
//! index), so the dataset is reproducible -- no randomness, same output every
//! run. Variants of one template share a cluster id so the harness can apply a
//! clustered standard error.
//!
//! Usage: `gen_dataset [out.jsonl]`  (default: eval/data/perturbed_pilot.jsonl)

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![deny(clippy::unwrap_used, clippy::expect_used)]
// Pragmatic allows for a data-generation tool (mirrors the library's).
#![allow(
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::fmt::Write as _;

use mcp_reasoning::eval::{AnswerKind, EvalTask};

const NAMES: [&str; 10] = [
    "Ava", "Ben", "Cara", "Dan", "Elsa", "Finn", "Gia", "Hugo", "Iris", "Jon",
];

/// Number of instances generated per template.
const PER_TEMPLATE: usize = 40;

/// A word-problem template: produces `(body, answer)` for instance `i`. When
/// `hard` is true it appends one extra reasoning step (the difficulty knob).
struct Template {
    cluster: &'static str,
    expected_mode: &'static str,
    build: fn(usize, bool) -> (String, i64),
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "eval/data/perturbed_pilot.jsonl".to_string());

    let templates = [
        Template {
            cluster: "shopping",
            expected_mode: "linear",
            build: tmpl_shopping,
        },
        Template {
            cluster: "travel",
            expected_mode: "linear",
            build: tmpl_travel,
        },
        Template {
            cluster: "production",
            expected_mode: "linear",
            build: tmpl_production,
        },
        Template {
            cluster: "tickets",
            expected_mode: "linear",
            build: tmpl_tickets,
        },
        Template {
            cluster: "garden",
            expected_mode: "linear",
            build: tmpl_garden,
        },
    ];

    let mut lines: Vec<String> = Vec::new();
    for t in &templates {
        for i in 0..PER_TEMPLATE {
            // Alternate medium/hard so a single calibration run sees both bands.
            let hard = i % 2 == 1;
            let (body, answer) = (t.build)(i, hard);
            let prompt = format!(
                "{body} Show your work step by step, then end with a line '#### <answer>'."
            );
            let mut metadata = serde_json::Map::new();
            metadata.insert(
                "difficulty".to_string(),
                serde_json::Value::String(if hard { "hard" } else { "medium" }.to_string()),
            );
            let task = EvalTask {
                id: format!("{}-{i:02}", t.cluster),
                cluster_id: Some(t.cluster.to_string()),
                prompt,
                target: answer.to_string(),
                expected_mode: Some(t.expected_mode.to_string()),
                answer_kind: AnswerKind::Numeric,
                metadata,
            };
            match serde_json::to_string(&task) {
                Ok(line) => lines.push(line),
                Err(e) => {
                    eprintln!("serialization failed for {}: {e}", task.id);
                    std::process::exit(1);
                }
            }
        }
    }

    let body = format!("{}\n", lines.join("\n"));
    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("cannot create {}: {e}", parent.display());
            std::process::exit(1);
        }
    }
    if let Err(e) = std::fs::write(&out_path, &body) {
        eprintln!("cannot write {out_path}: {e}");
        std::process::exit(1);
    }

    // Self-validate: the file must parse back through the real harness loader.
    match mcp_reasoning::eval::Dataset::from_jsonl_file(&out_path) {
        Ok(ds) => println!(
            "wrote {} items to {out_path} (validated by Dataset loader)",
            ds.len()
        ),
        Err(e) => {
            eprintln!("generated file failed to load: {e}");
            std::process::exit(1);
        }
    }
}

/// Deterministic value in `[lo, hi]` from index `i` and a per-template salt so
/// templates do not share the same number pattern.
#[cfg_attr(coverage_nightly, coverage(off))]
fn pick(i: usize, salt: usize, lo: i64, hi: i64) -> i64 {
    let span = hi - lo + 1;
    let h = (i as i64)
        .wrapping_mul(7)
        .wrapping_add((salt as i64).wrapping_mul(13));
    lo + h.rem_euclid(span)
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn name(i: usize, salt: usize) -> &'static str {
    NAMES[(i + salt) % NAMES.len()]
}

// ---- templates: each returns (body, exact integer answer) ------------------

#[cfg_attr(coverage_nightly, coverage(off))]
fn tmpl_shopping(i: usize, hard: bool) -> (String, i64) {
    let who = name(i, 0);
    let q1 = pick(i, 1, 3, 7);
    let p1 = pick(i, 2, 2, 9);
    let q2 = pick(i, 3, 2, 5);
    let p2 = pick(i, 4, 3, 8);
    let budget = 150 + pick(i, 5, 0, 6) * 10;
    let mut spent = q1 * p1 + q2 * p2;
    let mut body = format!(
        "{who} has ${budget}. They buy {q1} notebooks at ${p1} each and {q2} pens at ${p2} each."
    );
    if hard {
        let q3 = pick(i, 6, 2, 4);
        let p3 = pick(i, 7, 4, 9);
        let returned = pick(i, 8, 1, (q3 - 1).max(1));
        spent += q3 * p3 - returned * p3;
        let _ = write!(
            body,
            " They also buy {q3} folders at ${p3} each, then return {returned} of the folders for a full refund."
        );
    }
    let _ = write!(body, " How many dollars does {who} have left?");
    (body, budget - spent)
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn tmpl_travel(i: usize, hard: bool) -> (String, i64) {
    let s1 = pick(i, 1, 40, 80);
    let h1 = pick(i, 2, 2, 5);
    let s2 = pick(i, 3, 50, 90);
    let h2 = pick(i, 4, 1, 4);
    let mut dist = s1 * h1 + s2 * h2;
    let mut body =
        format!("A train travels at {s1} km/h for {h1} hours, then at {s2} km/h for {h2} hours.");
    if hard {
        let back = pick(i, 5, 20, 60);
        let hb = pick(i, 6, 1, 2);
        dist -= back * hb;
        let _ = write!(
            body,
            " It then reverses and travels back at {back} km/h for {hb} hours. What is its net distance in km from the start?"
        );
    } else {
        let _ = write!(body, " What total distance in km does it cover?");
    }
    (body, dist)
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn tmpl_production(i: usize, hard: bool) -> (String, i64) {
    let r1 = pick(i, 1, 8, 20);
    let h1 = pick(i, 2, 3, 6);
    let r2 = pick(i, 3, 10, 25);
    let h2 = pick(i, 4, 2, 5);
    let mut good = r1 * h1 + r2 * h2;
    let mut body = format!(
        "A workshop makes {r1} chairs per hour for {h1} hours, then {r2} chairs per hour for {h2} hours."
    );
    if hard {
        let defects = pick(i, 5, 5, 15);
        let packed = pick(i, 6, 2, 6);
        good -= defects + packed;
        let _ = write!(
            body,
            " Then {defects} chairs are found defective and {packed} are kept as showroom samples, both removed from stock."
        );
    }
    let _ = write!(body, " How many chairs are available to sell?");
    (body, good)
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn tmpl_tickets(i: usize, hard: bool) -> (String, i64) {
    let adult = pick(i, 1, 20, 60);
    let pa = pick(i, 2, 8, 15);
    let child = pick(i, 3, 10, 40);
    let pc = pick(i, 4, 4, 8);
    let mut revenue = adult * pa + child * pc;
    let mut body = format!(
        "A theater sells {adult} adult tickets at ${pa} each and {child} child tickets at ${pc} each."
    );
    if hard {
        let comps = pick(i, 5, 3, 9);
        revenue -= comps * pa;
        let _ = write!(
            body,
            " {comps} of the adult tickets were complimentary (no charge), so subtract their value."
        );
    }
    let _ = write!(body, " What is the total ticket revenue in dollars?");
    (body, revenue)
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn tmpl_garden(i: usize, hard: bool) -> (String, i64) {
    let rows = pick(i, 1, 6, 12);
    let per = pick(i, 2, 5, 11);
    let dead = pick(i, 3, 3, 12);
    let mut plants = rows * per - dead;
    let mut body =
        format!("A garden has {rows} rows of {per} plants each. Then {dead} plants die.");
    if hard {
        let added_rows = pick(i, 4, 2, 4);
        let added_per = pick(i, 5, 4, 8);
        plants += added_rows * added_per;
        let _ = write!(
            body,
            " The gardener then plants {added_rows} new rows of {added_per} plants each."
        );
    }
    let _ = write!(body, " How many plants are in the garden now?");
    (body, plants)
}
