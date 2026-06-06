//! Programmatic scoring — the anchor signal, and the only signal allowed into
//! the self-improvement loop.
//!
//! [`ExactMatch`] follows the lm-eval two-filter pattern: a **strict** filter
//! first constrains the model to a terminal answer format (`#### <answer>`, the
//! GSM8K delimiter) and extracts from it; if that fails, a **flexible** filter
//! falls back to the last number (numeric) or last non-blank line (exact). Both
//! sides normalize before comparing.
//!
//! The extraction-failure rate is tracked as a **first-class metric** (see
//! [`Score::extraction_failed`]): a rising invalid rate depresses scores and
//! corrupts deltas while masquerading as a quality regression — exactly the
//! artifact that would poison the self-improvement sensor — so it must be
//! surfaced, not silently folded into "wrong."

use crate::eval::task::{AnswerKind, EvalTask};

/// The result of scoring one task's output.
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    /// `1.0` if the extracted answer matched the target, else `0.0`.
    pub value: f64,
    /// The answer extracted from the output, if any.
    pub extracted: Option<String>,
    /// `true` when no answer could be extracted at all. Such an item scores
    /// `0.0` *and* counts toward the extraction-failure rate, so the two failure
    /// modes (wrong answer vs unparseable output) stay distinguishable.
    pub extraction_failed: bool,
}

/// Scores a model output against a task's target.
///
/// `Send + Sync` so a `&dyn Scorer` can be held across the runner's `.await`s.
pub trait Scorer: Send + Sync {
    /// Score `output` for `task`.
    fn score(&self, task: &EvalTask, output: &str) -> Score;
}

/// The terminal-format sentinel the strict filter looks for.
const TERMINAL_MARKER: &str = "####";

/// Exact-match scorer with two-filter extraction.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExactMatch;

impl ExactMatch {
    /// Create an exact-match scorer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract a candidate answer: strict (`#### <answer>`) first, then flexible.
    fn extract(output: &str, kind: AnswerKind) -> Option<String> {
        if let Some(marker) = output.rfind(TERMINAL_MARKER) {
            let after = &output[marker + TERMINAL_MARKER.len()..];
            let strict = match kind {
                AnswerKind::Numeric => extract_numbers(after).into_iter().next(),
                AnswerKind::Exact => first_nonblank_line(after),
            };
            if strict.is_some() {
                return strict;
            }
        }
        // Flexible fallback over the whole output.
        match kind {
            AnswerKind::Numeric => extract_numbers(output).into_iter().next_back(),
            AnswerKind::Exact => last_nonblank_line(output),
        }
    }
}

impl Scorer for ExactMatch {
    fn score(&self, task: &EvalTask, output: &str) -> Score {
        Self::extract(output, task.answer_kind).map_or(
            Score {
                value: 0.0,
                extracted: None,
                extraction_failed: true,
            },
            |answer| {
                let correct = match task.answer_kind {
                    AnswerKind::Numeric => numeric_equal(&answer, &task.target),
                    AnswerKind::Exact => exact_equal(&answer, &task.target),
                };
                Score {
                    value: f64::from(u8::from(correct)),
                    extracted: Some(answer),
                    extraction_failed: false,
                }
            },
        )
    }
}

/// First non-blank line of `s`, trimmed.
fn first_nonblank_line(s: &str) -> Option<String> {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(ToString::to_string)
}

/// Last non-blank line of `s`, trimmed.
fn last_nonblank_line(s: &str) -> Option<String> {
    s.lines()
        .map(str::trim)
        .rev()
        .find(|l| !l.is_empty())
        .map(ToString::to_string)
}

/// Extract numeric tokens (optional leading sign, digits, thousands commas, one
/// decimal point) in left-to-right order.
fn extract_numbers(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut seen_dot = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let next_is_digit = chars.peek().is_some_and(char::is_ascii_digit);
        let ends_with_digit = cur.chars().next_back().is_some_and(|l| l.is_ascii_digit());
        if c.is_ascii_digit()
            || (c == ',' && ends_with_digit)
            || ((c == '-' || c == '+') && cur.is_empty() && next_is_digit)
        {
            cur.push(c);
        } else if c == '.' && !seen_dot && ends_with_digit && next_is_digit {
            cur.push(c);
            seen_dot = true;
        } else {
            push_number(&mut cur, &mut out);
            seen_dot = false;
        }
    }
    push_number(&mut cur, &mut out);
    out
}

/// Flush `cur` into `out` if it holds at least one digit, then clear it.
fn push_number(cur: &mut String, out: &mut Vec<String>) {
    if cur.chars().any(|c| c.is_ascii_digit()) {
        out.push(std::mem::take(cur));
    } else {
        cur.clear();
    }
}

/// Strip currency/grouping/whitespace and a trailing period from a numeric token.
fn normalize_numeric(s: &str) -> String {
    let stripped: String = s
        .chars()
        .filter(|c| !matches!(c, '$' | ',' | '%' | ' ' | '\t'))
        .collect();
    stripped.trim_end_matches('.').to_string()
}

/// Compare two numeric strings: parse both after normalization and compare with
/// a small tolerance; fall back to normalized string equality if either side
/// does not parse.
fn numeric_equal(a: &str, b: &str) -> bool {
    let (na, nb) = (normalize_numeric(a), normalize_numeric(b));
    match (na.parse::<f64>(), nb.parse::<f64>()) {
        (Ok(x), Ok(y)) => (x - y).abs() < 1e-9,
        _ => na == nb,
    }
}

/// Compare two short strings: trim, drop a trailing period, case-insensitive.
fn exact_equal(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().trim_end_matches('.').trim().to_lowercase();
    norm(a) == norm(b)
}
