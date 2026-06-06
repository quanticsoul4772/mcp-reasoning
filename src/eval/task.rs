//! Eval task and dataset model.
//!
//! An [`EvalTask`] is one scoreable item; a [`Dataset`] is a JSONL file of them
//! (one task per line) loaded from `eval/data/`. The fields exist to feed the
//! statistics, not as decoration:
//!
//! - `cluster_id` drives clustered standard errors when items are correlated
//!   (e.g. several questions drawn from one source);
//! - `expected_mode` lets a run validate `auto`/`meta` selection — treated as a
//!   weak prior, never as ground truth (the empirically-best mode is the
//!   authority);
//! - `answer_kind` selects the scoring strategy.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What kind of answer a task expects, which selects the scoring strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerKind {
    /// A numeric final answer, compared after numeric normalization
    /// (strip `$`, `,`, `%`, whitespace, a trailing `.`, then parse).
    Numeric,
    /// A short string answer, compared after light text normalization
    /// (trim, drop a trailing `.`, case-insensitive).
    Exact,
}

/// A single evaluation item.
// `metadata` is a `serde_json::Map` (values include `f64`), so `Eq` is not
// derivable; the lint's suggestion does not apply.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalTask {
    /// Stable identifier, unique within a dataset.
    pub id: String,
    /// Cluster this item belongs to, for clustered standard errors. `None` means
    /// the item is treated as independent.
    #[serde(default)]
    pub cluster_id: Option<String>,
    /// The prompt presented to the solver.
    pub prompt: String,
    /// The reference answer scored against.
    pub target: String,
    /// The mode expected to do best on this item (a weak label that validates
    /// `auto`/`meta`, not ground truth).
    #[serde(default)]
    pub expected_mode: Option<String>,
    /// The answer kind, selecting the scorer.
    pub answer_kind: AnswerKind,
    /// Free-form metadata (difficulty tags, provenance, etc.).
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// Errors raised while loading a [`Dataset`].
#[derive(Debug, Error)]
pub enum DatasetError {
    /// A line failed to parse as an [`EvalTask`]. `line` is 1-based.
    #[error("dataset line {line}: {source}")]
    Parse {
        /// 1-based line number of the offending record.
        line: usize,
        /// The underlying JSON error.
        source: serde_json::Error,
    },
    /// The dataset contained no tasks (every line was blank).
    #[error("dataset is empty")]
    Empty,
    /// The dataset file could not be read.
    #[error("failed to read dataset file: {0}")]
    Io(#[from] std::io::Error),
}

/// An ordered collection of [`EvalTask`]s.
// `EvalTask` is not `Eq` (see above), so neither is `Dataset`.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    tasks: Vec<EvalTask>,
}

impl Dataset {
    /// Parse a JSONL string: one [`EvalTask`] per non-blank line.
    ///
    /// Blank lines are skipped. Returns [`DatasetError::Parse`] (with the 1-based
    /// line number) on the first malformed record, or [`DatasetError::Empty`] if
    /// no tasks were found.
    pub fn from_jsonl(content: &str) -> Result<Self, DatasetError> {
        let mut tasks = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let task: EvalTask =
                serde_json::from_str(trimmed).map_err(|source| DatasetError::Parse {
                    line: idx + 1,
                    source,
                })?;
            tasks.push(task);
        }
        if tasks.is_empty() {
            return Err(DatasetError::Empty);
        }
        Ok(Self { tasks })
    }

    /// Load and parse a JSONL dataset file.
    pub fn from_jsonl_file(path: impl AsRef<std::path::Path>) -> Result<Self, DatasetError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_jsonl(&content)
    }

    /// The tasks in load order.
    #[must_use]
    pub fn tasks(&self) -> &[EvalTask] {
        &self.tasks
    }

    /// Number of tasks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Whether the dataset has no tasks. Always `false` for a value produced by
    /// [`Dataset::from_jsonl`] (which rejects empty input), but provided for
    /// completeness and to satisfy the `len`/`is_empty` convention.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}
