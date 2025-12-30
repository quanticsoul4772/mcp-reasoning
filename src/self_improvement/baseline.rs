//! Baseline calculation with EMA and rolling average.
//!
//! Provides baseline tracking for metrics comparison in the self-improvement system.
//! Uses exponential moving average (EMA) for responsiveness and rolling average
//! for stability.

use serde::{Deserialize, Serialize};

// ============================================================================
// Baseline Configuration
// ============================================================================

/// Configuration for baseline calculation.
#[derive(Debug, Clone)]
pub struct BaselineConfig {
    /// EMA smoothing factor (0.0 to 1.0, higher = more responsive).
    pub ema_alpha: f64,
    /// Rolling window size for rolling average.
    pub rolling_window_size: usize,
    /// Minimum samples before baseline is considered valid.
    pub min_samples: u64,
}

impl Default for BaselineConfig {
    fn default() -> Self {
        Self {
            ema_alpha: 0.1,           // Slow-moving EMA
            rolling_window_size: 100, // 100-sample rolling window
            min_samples: 10,          // Need at least 10 samples
        }
    }
}

// ============================================================================
// Baseline
// ============================================================================

/// Single metric baseline with EMA and rolling average.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Exponential moving average value.
    ema: f64,
    /// Rolling average value.
    rolling_avg: f64,
    /// Number of samples recorded.
    sample_count: u64,
    /// Rolling window buffer.
    #[serde(skip)]
    rolling_buffer: Vec<f64>,
    /// Rolling buffer index (circular).
    #[serde(skip)]
    rolling_index: usize,
    /// EMA smoothing factor.
    #[serde(skip)]
    ema_alpha: f64,
    /// Rolling window size.
    #[serde(skip)]
    rolling_window_size: usize,
}

impl Baseline {
    /// Create a new baseline with config.
    #[must_use]
    pub fn new(config: &BaselineConfig) -> Self {
        Self {
            ema: 0.0,
            rolling_avg: 0.0,
            sample_count: 0,
            rolling_buffer: Vec::with_capacity(config.rolling_window_size),
            rolling_index: 0,
            ema_alpha: config.ema_alpha,
            rolling_window_size: config.rolling_window_size,
        }
    }

    /// Create with default config.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(&BaselineConfig::default())
    }

    /// Update baseline with a new value.
    pub fn update(&mut self, value: f64) {
        self.sample_count += 1;

        // Update EMA
        if self.sample_count == 1 {
            self.ema = value;
        } else {
            self.ema = self.ema_alpha * value + (1.0 - self.ema_alpha) * self.ema;
        }

        // Update rolling average
        if self.rolling_buffer.len() < self.rolling_window_size {
            // Buffer not full yet
            self.rolling_buffer.push(value);
            self.rolling_avg =
                self.rolling_buffer.iter().sum::<f64>() / self.rolling_buffer.len() as f64;
        } else {
            // Buffer full, replace oldest value
            let old_value = self.rolling_buffer[self.rolling_index];
            self.rolling_buffer[self.rolling_index] = value;
            self.rolling_index = (self.rolling_index + 1) % self.rolling_window_size;

            // Efficient update: subtract old, add new
            self.rolling_avg += (value - old_value) / self.rolling_window_size as f64;
        }
    }

    /// Get the current baseline value (uses EMA by default).
    #[must_use]
    pub fn value(&self) -> f64 {
        self.ema
    }

    /// Get the EMA value.
    #[must_use]
    pub fn ema(&self) -> f64 {
        self.ema
    }

    /// Get the rolling average value.
    #[must_use]
    pub fn rolling_avg(&self) -> f64 {
        self.rolling_avg
    }

    /// Get the sample count.
    #[must_use]
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Check if baseline has enough samples to be valid.
    #[must_use]
    pub fn is_valid(&self, min_samples: u64) -> bool {
        self.sample_count >= min_samples
    }

    /// Reset the baseline.
    pub fn reset(&mut self) {
        self.ema = 0.0;
        self.rolling_avg = 0.0;
        self.sample_count = 0;
        self.rolling_buffer.clear();
        self.rolling_index = 0;
    }
}

impl Default for Baseline {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// BaselineCollection
// ============================================================================

/// Collection of baselines for all tracked metrics.
#[derive(Debug, Clone, Default)]
pub struct BaselineCollection {
    /// Error rate baseline (0.0 to 1.0).
    pub error_rate: Baseline,
    /// P95 latency baseline (milliseconds).
    pub latency_p95: Baseline,
    /// Quality score baseline (0.0 to 1.0).
    pub quality_score: Baseline,
    /// Per-tool baselines.
    pub tool_baselines: std::collections::HashMap<String, ToolBaseline>,
    /// Configuration.
    config: BaselineConfig,
}

/// Baselines for a specific tool.
#[derive(Debug, Clone, Default)]
pub struct ToolBaseline {
    /// Error rate for this tool.
    pub error_rate: Baseline,
    /// Latency for this tool.
    pub latency: Baseline,
    /// Quality score for this tool.
    pub quality: Baseline,
}

impl BaselineCollection {
    /// Create a new baseline collection with config.
    #[must_use]
    pub fn new(config: BaselineConfig) -> Self {
        Self {
            error_rate: Baseline::new(&config),
            latency_p95: Baseline::new(&config),
            quality_score: Baseline::new(&config),
            tool_baselines: std::collections::HashMap::new(),
            config,
        }
    }

    /// Create with default config.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(BaselineConfig::default())
    }

    /// Update global error rate baseline.
    pub fn update_error_rate(&mut self, rate: f64) {
        self.error_rate.update(rate.clamp(0.0, 1.0));
    }

    /// Update global latency baseline.
    pub fn update_latency(&mut self, latency_ms: i64) {
        self.latency_p95.update(latency_ms.max(0) as f64);
    }

    /// Update global quality score baseline.
    pub fn update_quality(&mut self, score: f64) {
        self.quality_score.update(score.clamp(0.0, 1.0));
    }

    /// Update tool-specific baselines.
    pub fn update_tool(
        &mut self,
        tool_name: &str,
        latency_ms: i64,
        success: bool,
        quality: Option<f64>,
    ) {
        let tool_baseline = self
            .tool_baselines
            .entry(tool_name.to_string())
            .or_insert_with(|| ToolBaseline {
                error_rate: Baseline::new(&self.config),
                latency: Baseline::new(&self.config),
                quality: Baseline::new(&self.config),
            });

        tool_baseline.latency.update(latency_ms.max(0) as f64);
        tool_baseline
            .error_rate
            .update(if success { 0.0 } else { 1.0 });
        if let Some(q) = quality {
            tool_baseline.quality.update(q.clamp(0.0, 1.0));
        }
    }

    /// Get current baseline values as a snapshot.
    #[must_use]
    pub fn snapshot(&self) -> super::types::Baselines {
        super::types::Baselines {
            error_rate: self.error_rate.value(),
            latency_p95_ms: self.latency_p95.value() as i64,
            quality_score: self.quality_score.value(),
            sample_count: self.error_rate.sample_count(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Check if all baselines have enough samples.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let min = self.config.min_samples;
        self.error_rate.is_valid(min)
            && self.latency_p95.is_valid(min)
            && self.quality_score.is_valid(min)
    }

    /// Reset all baselines.
    pub fn reset(&mut self) {
        self.error_rate.reset();
        self.latency_p95.reset();
        self.quality_score.reset();
        self.tool_baselines.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_initial_value() {
        let baseline = Baseline::with_defaults();
        assert_eq!(baseline.value(), 0.0);
        assert_eq!(baseline.sample_count(), 0);
        assert!(!baseline.is_valid(10));
    }

    #[test]
    fn test_baseline_single_update() {
        let mut baseline = Baseline::with_defaults();
        baseline.update(100.0);
        assert_eq!(baseline.value(), 100.0);
        assert_eq!(baseline.sample_count(), 1);
    }

    #[test]
    fn test_baseline_ema_calculation() {
        let config = BaselineConfig {
            ema_alpha: 0.5, // 50% weight on new value
            rolling_window_size: 10,
            min_samples: 1,
        };
        let mut baseline = Baseline::new(&config);

        baseline.update(100.0);
        assert_eq!(baseline.ema(), 100.0);

        baseline.update(200.0);
        // EMA = 0.5 * 200 + 0.5 * 100 = 150
        assert!((baseline.ema() - 150.0).abs() < f64::EPSILON);

        baseline.update(200.0);
        // EMA = 0.5 * 200 + 0.5 * 150 = 175
        assert!((baseline.ema() - 175.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_baseline_rolling_average() {
        let config = BaselineConfig {
            ema_alpha: 0.1,
            rolling_window_size: 3,
            min_samples: 1,
        };
        let mut baseline = Baseline::new(&config);

        baseline.update(10.0);
        assert!((baseline.rolling_avg() - 10.0).abs() < f64::EPSILON);

        baseline.update(20.0);
        assert!((baseline.rolling_avg() - 15.0).abs() < f64::EPSILON);

        baseline.update(30.0);
        assert!((baseline.rolling_avg() - 20.0).abs() < f64::EPSILON);

        // Window full, now oldest value gets replaced
        baseline.update(40.0);
        // New values: [40, 20, 30] -> avg = 30
        assert!((baseline.rolling_avg() - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_baseline_validity() {
        let mut baseline = Baseline::with_defaults();

        for i in 0..9 {
            baseline.update(f64::from(i));
            assert!(!baseline.is_valid(10));
        }

        baseline.update(9.0);
        assert!(baseline.is_valid(10));
    }

    #[test]
    fn test_baseline_reset() {
        let mut baseline = Baseline::with_defaults();
        baseline.update(100.0);
        baseline.update(200.0);

        baseline.reset();

        assert_eq!(baseline.value(), 0.0);
        assert_eq!(baseline.sample_count(), 0);
        assert_eq!(baseline.rolling_avg(), 0.0);
    }

    #[test]
    fn test_baseline_collection_creation() {
        let collection = BaselineCollection::with_defaults();
        assert!(!collection.is_valid());
    }

    #[test]
    fn test_baseline_collection_updates() {
        let mut collection = BaselineCollection::with_defaults();

        collection.update_error_rate(0.05);
        collection.update_latency(100);
        collection.update_quality(0.95);

        assert!((collection.error_rate.value() - 0.05).abs() < f64::EPSILON);
        assert!((collection.latency_p95.value() - 100.0).abs() < f64::EPSILON);
        assert!((collection.quality_score.value() - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_baseline_collection_tool_update() {
        let mut collection = BaselineCollection::with_defaults();

        collection.update_tool("reasoning_linear", 50, true, Some(0.9));
        collection.update_tool("reasoning_linear", 60, false, Some(0.8));

        let tool = collection.tool_baselines.get("reasoning_linear").unwrap();
        assert!(tool.latency.sample_count() == 2);
        assert!(tool.error_rate.sample_count() == 2);
        assert!(tool.quality.sample_count() == 2);
    }

    #[test]
    fn test_baseline_collection_snapshot() {
        let mut collection = BaselineCollection::with_defaults();

        for _ in 0..15 {
            collection.update_error_rate(0.02);
            collection.update_latency(50);
            collection.update_quality(0.98);
        }

        let snapshot = collection.snapshot();
        assert!(snapshot.error_rate > 0.0);
        assert!(snapshot.latency_p95_ms > 0);
        assert!(snapshot.quality_score > 0.0);
        assert_eq!(snapshot.sample_count, 15);
    }

    #[test]
    fn test_baseline_collection_reset() {
        let mut collection = BaselineCollection::with_defaults();

        collection.update_error_rate(0.05);
        collection.update_tool("test_tool", 100, true, None);

        collection.reset();

        assert_eq!(collection.error_rate.sample_count(), 0);
        assert!(collection.tool_baselines.is_empty());
    }

    #[test]
    fn test_baseline_clamping() {
        let mut collection = BaselineCollection::with_defaults();

        // Error rate should be clamped to [0, 1]
        collection.update_error_rate(1.5);
        assert!((collection.error_rate.value() - 1.0).abs() < f64::EPSILON);

        collection.update_error_rate(-0.5);
        // Second update, so EMA calculation applies

        // Quality should be clamped to [0, 1]
        let mut collection2 = BaselineCollection::with_defaults();
        collection2.update_quality(2.0);
        assert!((collection2.quality_score.value() - 1.0).abs() < f64::EPSILON);

        // Latency should not go negative
        collection2.update_latency(-100);
        assert!(collection2.latency_p95.value() >= 0.0);
    }
}
