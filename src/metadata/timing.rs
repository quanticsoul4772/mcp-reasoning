//! Historical timing data for duration predictions.

use crate::error::AppError;
use crate::storage::SqliteStorage;
use std::sync::Arc;

use super::{timing_defaults::get_default_timing, ConfidenceLevel};

/// Historical timing data for duration predictions.
pub struct TimingDatabase {
    storage: Arc<SqliteStorage>,
}

impl TimingDatabase {
    /// Create a new timing database.
    #[must_use]
    pub fn new(storage: Arc<SqliteStorage>) -> Self {
        Self { storage }
    }

    /// Get estimated duration for a tool/mode combination.
    ///
    /// Returns (estimated_ms, confidence_level) based on historical data.
    /// Falls back to default estimates if no history available.
    ///
    /// # Errors
    ///
    /// Returns error if database query fails.
    pub async fn estimate_duration(
        &self,
        tool: &str,
        mode: Option<&str>,
        complexity: ComplexityMetrics,
    ) -> Result<(u64, ConfidenceLevel), AppError> {
        // Try to get historical data
        if let Ok(Some((avg_ms, sample_count))) = self.get_historical_average(tool, mode).await {
            let confidence = calculate_confidence(sample_count);
            let adjusted = adjust_for_complexity(avg_ms, &complexity);
            Ok((adjusted, confidence))
        } else {
            // Fall back to defaults
            let default_ms = get_default_timing(tool, &complexity);
            Ok((default_ms, ConfidenceLevel::Low))
        }
    }

    /// Record actual execution time for learning.
    ///
    /// # Errors
    ///
    /// Returns error if database insert fails.
    pub async fn record_execution(
        &self,
        tool: &str,
        mode: Option<&str>,
        duration_ms: u64,
        complexity: ComplexityMetrics,
    ) -> Result<(), AppError> {
        let complexity_score = calculate_complexity_score(&complexity);

        sqlx::query(
            "INSERT INTO tool_timing_history 
             (tool_name, mode_name, duration_ms, complexity_score, timestamp) 
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(tool)
        .bind(mode)
        .bind(duration_ms as i64)
        .bind(complexity_score)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.storage.pool)
        .await
        .map_err(|e| AppError::Storage(crate::error::StorageError::QueryFailed {
            query: "insert_timing".into(),
            message: e.to_string(),
        }))?;

        Ok(())
    }

    /// Get historical average duration for a tool.
    async fn get_historical_average(
        &self,
        tool: &str,
        mode: Option<&str>,
    ) -> Result<Option<(u64, usize)>, AppError> {
        let result: Option<(i64, i64)> = sqlx::query_as(
            "SELECT AVG(duration_ms), COUNT(*) 
             FROM tool_timing_history 
             WHERE tool_name = ? AND (mode_name = ? OR ? IS NULL)
             AND timestamp > ?",
        )
        .bind(tool)
        .bind(mode)
        .bind(mode)
        .bind(chrono::Utc::now().timestamp() - 7 * 24 * 3600) // Last 7 days
        .fetch_optional(&self.storage.pool)
        .await
        .map_err(|e| AppError::Storage(crate::error::StorageError::QueryFailed {
            query: "get_timing_average".into(),
            message: e.to_string(),
        }))?;

        Ok(result.map(|(avg, count)| (avg as u64, count as usize)))
    }
}

/// Complexity factors affecting execution time.
#[derive(Debug, Clone, Default)]
pub struct ComplexityMetrics {
    /// Number of perspectives (for divergent mode).
    pub num_perspectives: Option<u32>,
    /// Number of branches (for tree mode).
    pub num_branches: Option<u32>,
    /// Content length in characters.
    pub content_length: usize,
    /// Thinking budget in tokens.
    pub thinking_budget: Option<u32>,
}

/// Calculate confidence level based on sample count.
fn calculate_confidence(sample_count: usize) -> ConfidenceLevel {
    match sample_count {
        100.. => ConfidenceLevel::High,
        10..=99 => ConfidenceLevel::Medium,
        _ => ConfidenceLevel::Low,
    }
}

/// Adjust historical average for current complexity.
fn adjust_for_complexity(avg_ms: u64, complexity: &ComplexityMetrics) -> u64 {
    let mut factor = 1.0;

    // Adjust for perspectives
    if let Some(perspectives) = complexity.num_perspectives {
        factor *= 1.0 + (f64::from(perspectives) - 2.0) * 0.15;
    }

    // Adjust for branches
    if let Some(branches) = complexity.num_branches {
        factor *= 1.0 + (f64::from(branches) - 2.0) * 0.12;
    }

    // Adjust for thinking budget
    if let Some(budget) = complexity.thinking_budget {
        factor *= match budget {
            16384.. => 1.3,
            8192.. => 1.2,
            _ => 1.0,
        };
    }

    (avg_ms as f64 * factor) as u64
}

/// Calculate complexity score for storage.
fn calculate_complexity_score(complexity: &ComplexityMetrics) -> i32 {
    let mut score: i32 = 0;

    if let Some(p) = complexity.num_perspectives {
        score = score.saturating_add((p * 10) as i32);
    }

    if let Some(b) = complexity.num_branches {
        score = score.saturating_add((b * 8) as i32);
    }

    if let Some(t) = complexity.thinking_budget {
        score = score.saturating_add((t / 1000) as i32);
    }

    score = score.saturating_add((complexity.content_length / 100) as i32);

    score
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_confidence() {
        assert_eq!(calculate_confidence(150), ConfidenceLevel::High);
        assert_eq!(calculate_confidence(50), ConfidenceLevel::Medium);
        assert_eq!(calculate_confidence(5), ConfidenceLevel::Low);
    }

    #[test]
    fn test_adjust_for_complexity() {
        let base = 10_000;

        // Simple case
        let simple = ComplexityMetrics::default();
        assert_eq!(adjust_for_complexity(base, &simple), base);

        // With perspectives
        let with_perspectives = ComplexityMetrics {
            num_perspectives: Some(4),
            ..Default::default()
        };
        assert!(adjust_for_complexity(base, &with_perspectives) > base);

        // With thinking budget
        let with_thinking = ComplexityMetrics {
            thinking_budget: Some(16384),
            ..Default::default()
        };
        assert!(adjust_for_complexity(base, &with_thinking) > base);
    }

    #[test]
    fn test_calculate_complexity_score() {
        let simple = ComplexityMetrics::default();
        assert_eq!(calculate_complexity_score(&simple), 0);

        let complex = ComplexityMetrics {
            num_perspectives: Some(4),
            num_branches: Some(3),
            content_length: 5000,
            thinking_budget: Some(8192),
        };
        let score = calculate_complexity_score(&complex);
        assert!(score > 50);
    }

    #[tokio::test]
    async fn test_timing_database_estimate_default() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let timing_db = TimingDatabase::new(Arc::new(storage));

        let complexity = ComplexityMetrics {
            content_length: 500,
            ..Default::default()
        };

        let (estimate, confidence) = timing_db
            .estimate_duration("reasoning_linear", None, complexity)
            .await
            .expect("estimate");

        assert!(estimate > 0);
        assert_eq!(confidence, ConfidenceLevel::Low); // No historical data
    }

    #[tokio::test]
    async fn test_timing_database_record_execution() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let timing_db = TimingDatabase::new(Arc::new(storage));

        let complexity = ComplexityMetrics {
            content_length: 1000,
            ..Default::default()
        };

        timing_db
            .record_execution("reasoning_linear", Some("linear"), 12_000, complexity)
            .await
            .expect("record");

        // Verify it was recorded (implicitly - no panic means success)
    }
}
