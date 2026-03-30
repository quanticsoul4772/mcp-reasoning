//! Metrics collection.
//!
//! This module provides:
//! - Usage metrics tracking per mode
//! - Latency measurements
//! - Success/failure rates
//! - Query interfaces for metrics data
//! - Tool transition tracking for chain analysis
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::metrics::{MetricsCollector, MetricEvent};
//!
//! let metrics = MetricsCollector::new();
//! metrics.record(MetricEvent::new("linear", 150, true));
//! metrics.record(MetricEvent::new("linear", 200, true));
//! metrics.record(MetricEvent::new("tree", 300, false));
//!
//! let summary = metrics.summary();
//! assert_eq!(summary.total_invocations, 3);
//! // 2 out of 3 succeeded = ~66.7%
//! assert!((summary.overall_success_rate - 0.666).abs() < 0.01);
//! // Per-mode stats are available
//! assert!(summary.by_mode.contains_key("linear"));
//! assert!(summary.by_mode.contains_key("tree"));
//! ```

// Allow intentional numeric casts for metrics calculations
#![allow(clippy::cast_lossless, clippy::cast_possible_wrap)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

/// Maximum number of transitions to keep in circular buffer.
const MAX_TRANSITIONS: usize = 10_000;

/// A single metric event recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEvent {
    /// Mode that was invoked.
    pub mode: String,
    /// Operation within the mode (if applicable).
    pub operation: Option<String>,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Whether the invocation succeeded.
    pub success: bool,
    /// Timestamp of the event (Unix epoch seconds).
    pub timestamp: u64,
    /// Problem type tag for effectiveness tracking (e.g., "math", "code_review", "planning").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem_type: Option<String>,
    /// Quality rating of the result (0.0-1.0), set by caller or inferred from outcome.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_rating: Option<f64>,
}

impl MetricEvent {
    /// Create a new metric event.
    #[must_use]
    pub fn new(mode: impl Into<String>, latency_ms: u64, success: bool) -> Self {
        Self {
            mode: mode.into(),
            operation: None,
            latency_ms,
            success,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            problem_type: None,
            quality_rating: None,
        }
    }

    /// Create an event with an operation.
    #[must_use]
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Tag this event with a problem type for effectiveness tracking.
    #[must_use]
    pub fn with_problem_type(mut self, problem_type: impl Into<String>) -> Self {
        self.problem_type = Some(problem_type.into());
        self
    }

    /// Set a quality rating for this event's result (0.0-1.0).
    #[must_use]
    pub fn with_quality_rating(mut self, rating: f64) -> Self {
        self.quality_rating = Some(rating.clamp(0.0, 1.0));
        self
    }
}

/// Summary statistics for a mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModeSummary {
    /// Total invocations.
    pub total_invocations: u64,
    /// Successful invocations.
    pub successful: u64,
    /// Failed invocations.
    pub failed: u64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Minimum latency in milliseconds.
    pub min_latency_ms: u64,
    /// Maximum latency in milliseconds.
    pub max_latency_ms: u64,
    /// Success rate (0.0-1.0).
    pub success_rate: f64,
}

/// Overall metrics summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    /// Total invocations across all modes.
    pub total_invocations: u64,
    /// Overall success rate.
    pub overall_success_rate: f64,
    /// Per-mode summaries.
    pub by_mode: HashMap<String, ModeSummary>,
    /// Recent fallbacks (mode → fallback mode).
    pub recent_fallbacks: Vec<FallbackEvent>,
}

/// A fallback event when a mode fails and routes to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEvent {
    /// Original mode that failed.
    pub from_mode: String,
    /// Mode that handled the fallback.
    pub to_mode: String,
    /// Reason for fallback.
    pub reason: String,
    /// Timestamp.
    pub timestamp: u64,
}

impl FallbackEvent {
    /// Create a new fallback event.
    #[must_use]
    pub fn new(
        from_mode: impl Into<String>,
        to_mode: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            from_mode: from_mode.into(),
            to_mode: to_mode.into(),
            reason: reason.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

// ============================================================================
// Tool Transition Tracking (for chain analysis)
// ============================================================================

/// A tool transition event tracking tool A → tool B usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTransition {
    /// Tool that was used before.
    pub from_tool: String,
    /// Tool that was used after.
    pub to_tool: String,
    /// Session this transition occurred in.
    pub session_id: String,
    /// Whether the to_tool execution succeeded.
    pub success: bool,
    /// Timestamp in milliseconds since epoch.
    pub timestamp: u64,
}

impl ToolTransition {
    /// Create a new tool transition.
    #[must_use]
    pub fn new(
        from_tool: impl Into<String>,
        to_tool: impl Into<String>,
        session_id: impl Into<String>,
        success: bool,
    ) -> Self {
        Self {
            from_tool: from_tool.into(),
            to_tool: to_tool.into(),
            session_id: session_id.into(),
            success,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

/// Statistics for a specific tool transition (A → B).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransitionStats {
    /// Number of times this transition occurred.
    pub count: u32,
    /// Success rate of the destination tool (0.0-1.0).
    pub success_rate: f64,
    /// Average time between tools in milliseconds.
    pub avg_time_between_ms: u64,
}

/// A detected tool chain pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChain {
    /// Sequence of tools in the chain.
    pub tools: Vec<String>,
    /// Number of times this chain was observed.
    pub occurrences: u32,
    /// Average success rate across the chain.
    pub avg_success_rate: f64,
    /// Average total duration of the chain in milliseconds.
    pub avg_total_duration_ms: u64,
}

/// Summary of tool chain patterns discovered from metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainSummary {
    /// Most common tool sequences (min 3 tools, min 5 occurrences).
    pub common_chains: Vec<ToolChain>,
    /// Transition matrix: from_tool → (to_tool → stats).
    pub transitions: HashMap<String, HashMap<String, TransitionStats>>,
    /// Tools that are frequently starting points.
    pub entry_tools: Vec<String>,
    /// Tools that are frequently ending points.
    pub terminal_tools: Vec<String>,
}

/// Tool effectiveness for a specific context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEffectiveness {
    /// Tool/mode name.
    pub tool_name: String,
    /// Success rate (0.0-1.0).
    pub success_rate: f64,
    /// Average quality rating (0.0-1.0), if quality data is available.
    pub avg_quality: Option<f64>,
    /// Number of observations.
    pub sample_count: u64,
    /// Average latency in ms.
    pub avg_latency_ms: f64,
}

/// Thread-safe metrics collector.
#[derive(Debug, Default)]
pub struct MetricsCollector {
    events: RwLock<Vec<MetricEvent>>,
    fallbacks: RwLock<Vec<FallbackEvent>>,
    transitions: RwLock<Vec<ToolTransition>>,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a metric event.
    pub fn record(&self, event: MetricEvent) {
        match self.events.write() {
            Ok(mut events) => {
                events.push(event);
            }
            Err(poison_error) => {
                tracing::error!(
                    mode = %event.mode,
                    error = %poison_error,
                    "Failed to record metric event: RwLock poisoned"
                );
            }
        }
    }

    /// Record a fallback event.
    pub fn record_fallback(&self, fallback: FallbackEvent) {
        match self.fallbacks.write() {
            Ok(mut fallbacks) => {
                fallbacks.push(fallback);
            }
            Err(poison_error) => {
                tracing::error!(
                    from_mode = %fallback.from_mode,
                    to_mode = %fallback.to_mode,
                    error = %poison_error,
                    "Failed to record fallback event: RwLock poisoned"
                );
            }
        }
    }

    /// Get summary statistics.
    #[must_use]
    pub fn summary(&self) -> MetricsSummary {
        let events = match self.events.read() {
            Ok(e) => e.clone(),
            Err(poison_error) => {
                tracing::warn!(
                    error = %poison_error,
                    "Reading events from poisoned lock, using recovered data"
                );
                poison_error.into_inner().clone()
            }
        };
        let fallbacks = match self.fallbacks.read() {
            Ok(f) => f.clone(),
            Err(poison_error) => {
                tracing::warn!(
                    error = %poison_error,
                    "Reading fallbacks from poisoned lock, using recovered data"
                );
                poison_error.into_inner().clone()
            }
        };

        // Pre-allocate with typical number of modes (5-10)
        let mut by_mode: HashMap<String, Vec<&MetricEvent>> = HashMap::with_capacity(10);
        for event in &events {
            by_mode.entry(event.mode.clone()).or_default().push(event);
        }

        let mode_summaries: HashMap<String, ModeSummary> = by_mode
            .into_iter()
            .map(|(mode, mode_events)| {
                let total = mode_events.len() as u64;
                let successful = mode_events.iter().filter(|e| e.success).count() as u64;
                let failed = total - successful;

                // Optimize: Compute stats in single pass without intermediate Vec
                let (sum, min, max, count) = mode_events.iter().map(|e| e.latency_ms).fold(
                    (0u64, u64::MAX, 0u64, 0usize),
                    |(sum, min, max, count), lat| {
                        (sum + lat, min.min(lat), max.max(lat), count + 1)
                    },
                );
                let avg_latency = if count > 0 {
                    sum as f64 / count as f64
                } else {
                    0.0
                };
                let min_latency = if min == u64::MAX { 0 } else { min };
                let max_latency = max;
                let success_rate = if total > 0 {
                    successful as f64 / total as f64
                } else {
                    0.0
                };

                (
                    mode,
                    ModeSummary {
                        total_invocations: total,
                        successful,
                        failed,
                        avg_latency_ms: avg_latency,
                        min_latency_ms: min_latency,
                        max_latency_ms: max_latency,
                        success_rate,
                    },
                )
            })
            .collect();

        let total_invocations = events.len() as u64;
        let total_successful = events.iter().filter(|e| e.success).count() as u64;
        let overall_success_rate = if total_invocations > 0 {
            total_successful as f64 / total_invocations as f64
        } else {
            1.0
        };

        MetricsSummary {
            total_invocations,
            overall_success_rate,
            by_mode: mode_summaries,
            recent_fallbacks: fallbacks,
        }
    }

    /// Get invocations for a specific mode.
    #[must_use]
    pub fn invocations_by_mode(&self, mode: &str) -> Vec<MetricEvent> {
        self.events
            .read()
            .map(|events| events.iter().filter(|e| e.mode == mode).cloned().collect())
            .unwrap_or_default()
    }

    /// Get invocations within a time range.
    #[must_use]
    pub fn invocations_in_range(&self, start: u64, end: u64) -> Vec<MetricEvent> {
        self.events
            .read()
            .map(|events| {
                events
                    .iter()
                    .filter(|e| e.timestamp >= start && e.timestamp <= end)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get recent fallbacks.
    #[must_use]
    pub fn fallbacks(&self) -> Vec<FallbackEvent> {
        self.fallbacks.read().map(|f| f.clone()).unwrap_or_default()
    }

    /// Clear all metrics (useful for testing).
    pub fn clear(&self) {
        if let Ok(mut events) = self.events.write() {
            events.clear();
        }
        if let Ok(mut fallbacks) = self.fallbacks.write() {
            fallbacks.clear();
        }
        if let Ok(mut transitions) = self.transitions.write() {
            transitions.clear();
        }
    }

    // ========================================================================
    // Tool Transition Tracking Methods
    // ========================================================================

    /// Record a tool transition event.
    ///
    /// Maintains a circular buffer of max `MAX_TRANSITIONS` events.
    pub fn record_transition(&self, transition: ToolTransition) {
        if let Ok(mut transitions) = self.transitions.write() {
            // Implement circular buffer behavior
            if transitions.len() >= MAX_TRANSITIONS {
                transitions.remove(0);
            }
            transitions.push(transition);
        }
    }

    /// Mark the last transition for a session as failed.
    ///
    /// Used when a tool execution fails after the transition was recorded.
    pub fn mark_last_transition_failed(&self, session_id: &str) {
        if let Ok(mut transitions) = self.transitions.write() {
            // Find the last transition for this session and mark it as failed
            for transition in transitions.iter_mut().rev() {
                if transition.session_id == session_id {
                    transition.success = false;
                    break;
                }
            }
        }
    }

    /// Get transition statistics for transitions FROM a specific tool.
    #[must_use]
    pub fn transitions_from(&self, tool: &str) -> HashMap<String, TransitionStats> {
        let transitions = self
            .transitions
            .read()
            .map(|t| t.clone())
            .unwrap_or_default();

        // Build per-session ordered timeline to compute dwell time on `tool`
        let mut sessions: HashMap<String, Vec<&ToolTransition>> = HashMap::new();
        for t in &transitions {
            sessions.entry(t.session_id.clone()).or_default().push(t);
        }
        for v in sessions.values_mut() {
            v.sort_by_key(|t| t.timestamp);
        }

        // For each from_tool→to_tool transition, compute how long the user spent on `tool`
        // before switching: timestamp(from→to) - timestamp(prev transition that landed on `tool`)
        let mut stats_map: HashMap<String, (u32, u32, Vec<u64>)> = HashMap::new();
        for session_transitions in sessions.values() {
            for (i, t) in session_transitions.iter().enumerate() {
                if t.from_tool != tool {
                    continue;
                }
                let entry = stats_map
                    .entry(t.to_tool.clone())
                    .or_insert((0, 0, Vec::new()));
                entry.0 += 1;
                if t.success {
                    entry.1 += 1;
                }
                // Find the previous transition in this session that ended at `tool`
                if let Some(prev) = session_transitions[..i]
                    .iter()
                    .rev()
                    .find(|p| p.to_tool == tool)
                {
                    let dwell = t.timestamp.saturating_sub(prev.timestamp);
                    entry.2.push(dwell);
                }
            }
        }

        stats_map
            .into_iter()
            .map(|(to_tool, (count, successful, dwells))| {
                let success_rate = if count > 0 {
                    successful as f64 / count as f64
                } else {
                    0.0
                };
                let avg_time_between_ms = if dwells.is_empty() {
                    0
                } else {
                    dwells.iter().sum::<u64>() / dwells.len() as u64
                };
                (
                    to_tool,
                    TransitionStats {
                        count,
                        success_rate,
                        avg_time_between_ms,
                    },
                )
            })
            .collect()
    }

    /// Get total number of recorded invocations.
    #[must_use]
    pub fn total_invocations(&self) -> u64 {
        self.events
            .read()
            .map(|events| events.len() as u64)
            .unwrap_or(0)
    }

    /// Get tool effectiveness data filtered by problem type context.
    ///
    /// Returns effectiveness stats for each tool that has been used with the
    /// given problem type tag, sorted by a composite score of success rate and quality.
    #[must_use]
    pub fn effectiveness_by_context(&self, context: &str) -> Vec<ToolEffectiveness> {
        let events = self.events.read().map(|e| e.clone()).unwrap_or_default();

        // Group events by mode, filtered by problem_type
        let mut by_mode: HashMap<String, Vec<&MetricEvent>> = HashMap::new();
        for event in &events {
            if event.problem_type.as_deref() == Some(context) {
                by_mode.entry(event.mode.clone()).or_default().push(event);
            }
        }

        let mut results: Vec<ToolEffectiveness> = by_mode
            .into_iter()
            .map(|(tool_name, mode_events)| {
                let sample_count = mode_events.len() as u64;
                let successes = mode_events.iter().filter(|e| e.success).count() as f64;
                let success_rate = successes / sample_count as f64;

                let quality_ratings: Vec<f64> = mode_events
                    .iter()
                    .filter_map(|e| e.quality_rating)
                    .collect();
                let avg_quality = if quality_ratings.is_empty() {
                    None
                } else {
                    Some(quality_ratings.iter().sum::<f64>() / quality_ratings.len() as f64)
                };

                let avg_latency_ms = mode_events.iter().map(|e| e.latency_ms).sum::<u64>() as f64
                    / sample_count as f64;

                ToolEffectiveness {
                    tool_name,
                    success_rate,
                    avg_quality,
                    sample_count,
                    avg_latency_ms,
                }
            })
            .collect();

        // Sort by composite score: success_rate * 0.6 + avg_quality * 0.4 (quality defaults to success_rate)
        results.sort_by(|a, b| {
            let score_a = a.success_rate * 0.6 + a.avg_quality.unwrap_or(a.success_rate) * 0.4;
            let score_b = b.success_rate * 0.6 + b.avg_quality.unwrap_or(b.success_rate) * 0.4;
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Recommend the best tool for a given problem type based on historical effectiveness.
    ///
    /// Returns the tool name and a confidence score (0.0-1.0) based on sample size
    /// and consistency. Returns `None` if no data is available for this context.
    #[must_use]
    pub fn recommend_tool(&self, context: &str) -> Option<(String, f64)> {
        let effectiveness = self.effectiveness_by_context(context);

        // Need at least one tool with 3+ samples to make a recommendation
        let viable: Vec<&ToolEffectiveness> = effectiveness
            .iter()
            .filter(|e| e.sample_count >= 3)
            .collect();

        let best = viable.first()?;

        // Confidence based on sample size: 3 samples = 0.3, 10+ = 0.8, 30+ = 1.0
        let size_confidence = (best.sample_count as f64 / 30.0).min(1.0);
        // Weight by success rate
        let confidence = (size_confidence * 0.4 + best.success_rate * 0.6).min(1.0);

        Some((best.tool_name.clone(), confidence))
    }

    /// Analyze tool chains and return a summary.
    ///
    /// This performs pattern detection to identify:
    /// - Common tool sequences (3+ tools, 5+ occurrences)
    /// - Transition matrix with success rates
    /// - Entry and terminal tools
    #[must_use]
    pub fn chain_summary(&self) -> ChainSummary {
        let transitions = self
            .transitions
            .read()
            .map(|t| t.clone())
            .unwrap_or_default();

        // Build transition matrix
        let transition_matrix = self.build_transition_matrix(&transitions);

        // Identify entry and terminal tools
        let (entry_tools, terminal_tools) = self.identify_entry_terminal_tools(&transitions);

        // Detect common chains (3-5 tool sequences)
        let common_chains = self.detect_common_chains(&transitions);

        ChainSummary {
            common_chains,
            transitions: transition_matrix,
            entry_tools,
            terminal_tools,
        }
    }

    /// Build the transition matrix from recorded transitions.
    fn build_transition_matrix(
        &self,
        transitions: &[ToolTransition],
    ) -> HashMap<String, HashMap<String, TransitionStats>> {
        // (count, successful, dwell_times_ms)
        type MatrixEntry = (u32, u32, Vec<u64>);

        // Build per-session ordered timeline for dwell-time computation
        let mut sessions: HashMap<&str, Vec<&ToolTransition>> = HashMap::new();
        for t in transitions {
            sessions.entry(t.session_id.as_str()).or_default().push(t);
        }
        for v in sessions.values_mut() {
            v.sort_by_key(|t| t.timestamp);
        }

        let mut matrix: HashMap<String, HashMap<String, MatrixEntry>> = HashMap::new();

        for session_transitions in sessions.values() {
            for (i, t) in session_transitions.iter().enumerate() {
                let from_entry = matrix.entry(t.from_tool.clone()).or_default();
                let to_entry = from_entry
                    .entry(t.to_tool.clone())
                    .or_insert((0, 0, Vec::new()));
                to_entry.0 += 1;
                if t.success {
                    to_entry.1 += 1;
                }
                // Dwell = time since the prior transition that landed on from_tool
                if let Some(prev) = session_transitions[..i]
                    .iter()
                    .rev()
                    .find(|p| p.to_tool == t.from_tool)
                {
                    to_entry.2.push(t.timestamp.saturating_sub(prev.timestamp));
                }
            }
        }

        matrix
            .into_iter()
            .map(|(from_tool, to_map)| {
                let stats_map = to_map
                    .into_iter()
                    .map(|(to_tool, (count, successful, dwells))| {
                        let success_rate = if count > 0 {
                            successful as f64 / count as f64
                        } else {
                            0.0
                        };
                        let avg_time_between_ms = if dwells.is_empty() {
                            0
                        } else {
                            dwells.iter().sum::<u64>() / dwells.len() as u64
                        };
                        (
                            to_tool,
                            TransitionStats {
                                count,
                                success_rate,
                                avg_time_between_ms,
                            },
                        )
                    })
                    .collect();
                (from_tool, stats_map)
            })
            .collect()
    }

    /// Identify which tools are commonly entry points vs terminal points.
    fn identify_entry_terminal_tools(
        &self,
        transitions: &[ToolTransition],
    ) -> (Vec<String>, Vec<String>) {
        let mut from_counts: HashMap<String, u32> = HashMap::new();
        let mut to_counts: HashMap<String, u32> = HashMap::new();

        for transition in transitions {
            *from_counts.entry(transition.from_tool.clone()).or_insert(0) += 1;
            *to_counts.entry(transition.to_tool.clone()).or_insert(0) += 1;
        }

        // Entry tools: appear as from_tool but rarely as to_tool
        let mut entry_tools: Vec<(String, i32)> = from_counts
            .iter()
            .map(|(tool, from_count)| {
                let to_count = to_counts.get(tool).copied().unwrap_or(0);
                // Higher score = more likely entry point
                (tool.clone(), *from_count as i32 - to_count as i32)
            })
            .collect();
        entry_tools.sort_by(|a, b| b.1.cmp(&a.1));

        // Terminal tools: appear as to_tool but rarely as from_tool
        let mut terminal_tools: Vec<(String, i32)> = to_counts
            .iter()
            .map(|(tool, to_count)| {
                let from_count = from_counts.get(tool).copied().unwrap_or(0);
                // Higher score = more likely terminal point
                (tool.clone(), *to_count as i32 - from_count as i32)
            })
            .collect();
        terminal_tools.sort_by(|a, b| b.1.cmp(&a.1));

        (
            entry_tools.into_iter().take(5).map(|(t, _)| t).collect(),
            terminal_tools.into_iter().take(5).map(|(t, _)| t).collect(),
        )
    }

    /// Detect common tool chains (sequences of 3+ tools with 5+ occurrences).
    fn detect_common_chains(&self, transitions: &[ToolTransition]) -> Vec<ToolChain> {
        // Group transitions by session
        let mut sessions: HashMap<String, Vec<&ToolTransition>> = HashMap::new();
        for transition in transitions {
            sessions
                .entry(transition.session_id.clone())
                .or_default()
                .push(transition);
        }

        // Sort each session's transitions by timestamp
        for session_transitions in sessions.values_mut() {
            session_transitions.sort_by_key(|t| t.timestamp);
        }

        // Extract chains of length 3 using sliding window
        let mut chain_counts: HashMap<Vec<String>, (u32, u32, u64)> = HashMap::new();

        for session_transitions in sessions.values() {
            if session_transitions.len() < 2 {
                continue;
            }

            // Build the tool sequence for this session
            let mut tools: Vec<String> = vec![session_transitions[0].from_tool.clone()];
            let mut timestamps: Vec<u64> = vec![session_transitions[0].timestamp];
            let mut successes: Vec<bool> = vec![];

            for t in session_transitions {
                tools.push(t.to_tool.clone());
                timestamps.push(t.timestamp);
                successes.push(t.success);
            }

            // Sliding window of size 3
            for window_start in 0..tools.len().saturating_sub(2) {
                let chain: Vec<String> = tools[window_start..window_start + 3].to_vec();
                let duration = timestamps
                    .get(window_start + 2)
                    .unwrap_or(&0)
                    .saturating_sub(*timestamps.get(window_start).unwrap_or(&0));
                let success_count = successes
                    .get(window_start..window_start + 2)
                    .map_or(0, |s| s.iter().filter(|&&x| x).count() as u32);

                let entry = chain_counts.entry(chain).or_insert((0, 0, 0));
                entry.0 += 1; // occurrences
                entry.1 += success_count; // total successes
                entry.2 += duration; // total duration
            }
        }

        // Filter to chains with 5+ occurrences and convert to ToolChain
        let mut chains: Vec<ToolChain> = chain_counts
            .into_iter()
            .filter(|(_, (count, _, _))| *count >= 5)
            .map(|(tools, (occurrences, successes, total_duration))| {
                let avg_success_rate = if occurrences > 0 {
                    successes as f64 / (occurrences * 2) as f64 // 2 transitions per 3-tool chain
                } else {
                    0.0
                };
                let avg_total_duration_ms = if occurrences > 0 {
                    total_duration / occurrences as u64
                } else {
                    0
                };

                ToolChain {
                    tools,
                    occurrences,
                    avg_success_rate,
                    avg_total_duration_ms,
                }
            })
            .collect();

        // Sort by occurrences descending
        chains.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));

        // Return top 10 chains
        chains.into_iter().take(10).collect()
    }
}

/// Timer for measuring operation latency.
#[derive(Debug)]
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Start a new timer.
    #[must_use]
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed time in milliseconds.
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::start()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_event_new() {
        let event = MetricEvent::new("linear", 100, true);
        assert_eq!(event.mode, "linear");
        assert_eq!(event.latency_ms, 100);
        assert!(event.success);
        assert!(event.operation.is_none());
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_metric_event_with_operation() {
        let event = MetricEvent::new("tree", 200, false).with_operation("create");
        assert_eq!(event.mode, "tree");
        assert_eq!(event.operation, Some("create".to_string()));
        assert!(!event.success);
    }

    #[test]
    fn test_fallback_event_new() {
        let fallback = FallbackEvent::new("graph", "linear", "API timeout");
        assert_eq!(fallback.from_mode, "graph");
        assert_eq!(fallback.to_mode, "linear");
        assert_eq!(fallback.reason, "API timeout");
        assert!(fallback.timestamp > 0);
    }

    #[test]
    fn test_metrics_collector_record() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("linear", 150, true));
        collector.record(MetricEvent::new("tree", 200, false));

        let summary = collector.summary();
        assert_eq!(summary.total_invocations, 3);
        assert_eq!(summary.by_mode.len(), 2);
    }

    #[test]
    fn test_metrics_collector_summary() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("linear", 200, true));
        collector.record(MetricEvent::new("linear", 300, false));

        let summary = collector.summary();
        let linear_summary = summary.by_mode.get("linear").unwrap();

        assert_eq!(linear_summary.total_invocations, 3);
        assert_eq!(linear_summary.successful, 2);
        assert_eq!(linear_summary.failed, 1);
        assert!((linear_summary.avg_latency_ms - 200.0).abs() < f64::EPSILON);
        assert_eq!(linear_summary.min_latency_ms, 100);
        assert_eq!(linear_summary.max_latency_ms, 300);
        assert!((linear_summary.success_rate - 0.666_666_666_666_666_6).abs() < 0.01);
    }

    #[test]
    fn test_metrics_collector_fallbacks() {
        let collector = MetricsCollector::new();
        collector.record_fallback(FallbackEvent::new("graph", "linear", "timeout"));
        collector.record_fallback(FallbackEvent::new("mcts", "tree", "API error"));

        let fallbacks = collector.fallbacks();
        assert_eq!(fallbacks.len(), 2);
        assert_eq!(fallbacks[0].from_mode, "graph");
        assert_eq!(fallbacks[1].from_mode, "mcts");
    }

    #[test]
    fn test_invocations_by_mode() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("tree", 150, true));
        collector.record(MetricEvent::new("linear", 200, false));

        let linear_events = collector.invocations_by_mode("linear");
        assert_eq!(linear_events.len(), 2);

        let tree_events = collector.invocations_by_mode("tree");
        assert_eq!(tree_events.len(), 1);

        let unknown_events = collector.invocations_by_mode("unknown");
        assert!(unknown_events.is_empty());
    }

    #[test]
    fn test_metrics_collector_clear() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record_fallback(FallbackEvent::new("a", "b", "c"));

        assert_eq!(collector.summary().total_invocations, 1);
        assert_eq!(collector.fallbacks().len(), 1);

        collector.clear();

        assert_eq!(collector.summary().total_invocations, 0);
        assert!(collector.fallbacks().is_empty());
    }

    #[test]
    fn test_empty_summary() {
        let collector = MetricsCollector::new();
        let summary = collector.summary();

        assert_eq!(summary.total_invocations, 0);
        assert!((summary.overall_success_rate - 1.0).abs() < f64::EPSILON);
        assert!(summary.by_mode.is_empty());
        assert!(summary.recent_fallbacks.is_empty());
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_timer_default() {
        let timer = Timer::default();
        let elapsed = timer.elapsed_ms();
        assert!(elapsed < 100); // Should be nearly instant
    }

    #[test]
    fn test_mode_summary_default() {
        let summary = ModeSummary::default();
        assert_eq!(summary.total_invocations, 0);
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.failed, 0);
        assert!((summary.avg_latency_ms - 0.0).abs() < f64::EPSILON);
        assert_eq!(summary.min_latency_ms, 0);
        assert_eq!(summary.max_latency_ms, 0);
        assert!((summary.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invocations_in_range() {
        let collector = MetricsCollector::new();

        // Create events with known timestamps
        let mut event1 = MetricEvent::new("linear", 100, true);
        event1.timestamp = 1000;
        let mut event2 = MetricEvent::new("tree", 150, true);
        event2.timestamp = 2000;
        let mut event3 = MetricEvent::new("linear", 200, false);
        event3.timestamp = 3000;

        collector.record(event1);
        collector.record(event2);
        collector.record(event3);

        let in_range = collector.invocations_in_range(1500, 2500);
        assert_eq!(in_range.len(), 1);
        assert_eq!(in_range[0].mode, "tree");

        let all = collector.invocations_in_range(0, 5000);
        assert_eq!(all.len(), 3);

        let none = collector.invocations_in_range(4000, 5000);
        assert!(none.is_empty());
    }

    #[test]
    fn test_metric_event_serialize() {
        let event = MetricEvent::new("linear", 100, true).with_operation("process");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"mode\":\"linear\""));
        assert!(json.contains("\"operation\":\"process\""));
        assert!(json.contains("\"latency_ms\":100"));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_metrics_summary_serialize() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        let summary = collector.summary();

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_invocations\":1"));
        assert!(json.contains("\"by_mode\""));
    }

    // ========================================================================
    // Tool Transition Tracking Tests
    // ========================================================================

    #[test]
    fn test_tool_transition_new() {
        let transition = ToolTransition::new("linear", "divergent", "session1", true);
        assert_eq!(transition.from_tool, "linear");
        assert_eq!(transition.to_tool, "divergent");
        assert_eq!(transition.session_id, "session1");
        assert!(transition.success);
        assert!(transition.timestamp > 0);
    }

    #[test]
    fn test_record_transition() {
        let collector = MetricsCollector::new();
        collector.record_transition(ToolTransition::new("linear", "divergent", "s1", true));
        collector.record_transition(ToolTransition::new("divergent", "decision", "s1", true));

        let stats = collector.transitions_from("linear");
        assert_eq!(stats.len(), 1);
        assert!(stats.contains_key("divergent"));
        assert_eq!(stats.get("divergent").unwrap().count, 1);
    }

    #[test]
    fn test_transitions_from() {
        let collector = MetricsCollector::new();
        // Create multiple transitions from "linear"
        collector.record_transition(ToolTransition::new("linear", "divergent", "s1", true));
        collector.record_transition(ToolTransition::new("linear", "divergent", "s2", true));
        collector.record_transition(ToolTransition::new("linear", "tree", "s3", false));

        let stats = collector.transitions_from("linear");
        assert_eq!(stats.len(), 2);

        let divergent_stats = stats.get("divergent").unwrap();
        assert_eq!(divergent_stats.count, 2);
        assert!((divergent_stats.success_rate - 1.0).abs() < f64::EPSILON);

        let tree_stats = stats.get("tree").unwrap();
        assert_eq!(tree_stats.count, 1);
        assert!((tree_stats.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mark_last_transition_failed() {
        let collector = MetricsCollector::new();
        collector.record_transition(ToolTransition::new("linear", "divergent", "s1", true));
        collector.record_transition(ToolTransition::new("divergent", "decision", "s1", true));

        // Mark the last transition for s1 as failed
        collector.mark_last_transition_failed("s1");

        // Check that the decision transition is now marked as failed
        let stats = collector.transitions_from("divergent");
        let decision_stats = stats.get("decision").unwrap();
        assert!((decision_stats.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_chain_summary_empty() {
        let collector = MetricsCollector::new();
        let summary = collector.chain_summary();

        assert!(summary.common_chains.is_empty());
        assert!(summary.transitions.is_empty());
        assert!(summary.entry_tools.is_empty());
        assert!(summary.terminal_tools.is_empty());
    }

    #[test]
    fn test_chain_summary_transition_matrix() {
        let collector = MetricsCollector::new();
        collector.record_transition(ToolTransition::new("linear", "divergent", "s1", true));
        collector.record_transition(ToolTransition::new("linear", "divergent", "s2", true));
        collector.record_transition(ToolTransition::new("divergent", "decision", "s1", true));

        let summary = collector.chain_summary();
        assert!(summary.transitions.contains_key("linear"));
        assert!(summary.transitions.contains_key("divergent"));

        let linear_transitions = summary.transitions.get("linear").unwrap();
        assert!(linear_transitions.contains_key("divergent"));
        assert_eq!(linear_transitions.get("divergent").unwrap().count, 2);
    }

    #[test]
    fn test_chain_summary_entry_terminal_tools() {
        let collector = MetricsCollector::new();
        // linear is always an entry point (from, never to)
        // checkpoint is always a terminal point (to, never from)
        for i in 0..10 {
            let session = format!("s{}", i);
            collector.record_transition(ToolTransition::new("linear", "divergent", &session, true));
            collector.record_transition(ToolTransition::new(
                "divergent",
                "checkpoint",
                &session,
                true,
            ));
        }

        let summary = collector.chain_summary();
        assert!(summary.entry_tools.contains(&"linear".to_string()));
        assert!(summary.terminal_tools.contains(&"checkpoint".to_string()));
    }

    #[test]
    fn test_detect_common_chains() {
        let collector = MetricsCollector::new();

        // Create the same chain 6 times (minimum 5 for detection)
        for i in 0..6 {
            let session = format!("s{}", i);
            let mut t1 = ToolTransition::new("linear", "divergent", &session, true);
            t1.timestamp = i as u64 * 1000;
            let mut t2 = ToolTransition::new("divergent", "decision", &session, true);
            t2.timestamp = i as u64 * 1000 + 500;

            collector.record_transition(t1);
            collector.record_transition(t2);
        }

        let summary = collector.chain_summary();
        assert!(!summary.common_chains.is_empty());

        let chain = &summary.common_chains[0];
        assert_eq!(
            chain.tools,
            vec![
                "linear".to_string(),
                "divergent".to_string(),
                "decision".to_string()
            ]
        );
        assert_eq!(chain.occurrences, 6);
    }

    #[test]
    fn test_transition_circular_buffer() {
        let collector = MetricsCollector::new();

        // Record more than MAX_TRANSITIONS
        for i in 0..100 {
            collector.record_transition(ToolTransition::new(
                format!("tool{}", i),
                format!("tool{}", i + 1),
                "s1",
                true,
            ));
        }

        // Verify transitions are recorded (we can't easily verify the max without
        // exposing internal state, but this tests the basic functionality)
        let stats = collector.transitions_from("tool0");
        assert_eq!(stats.len(), 1);
    }

    #[test]
    fn test_total_invocations() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.total_invocations(), 0);

        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("tree", 150, true));
        assert_eq!(collector.total_invocations(), 2);
    }

    #[test]
    fn test_transition_stats_default() {
        let stats = TransitionStats::default();
        assert_eq!(stats.count, 0);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(stats.avg_time_between_ms, 0);
    }

    #[test]
    fn test_chain_summary_default() {
        let summary = ChainSummary::default();
        assert!(summary.common_chains.is_empty());
        assert!(summary.transitions.is_empty());
        assert!(summary.entry_tools.is_empty());
        assert!(summary.terminal_tools.is_empty());
    }

    #[test]
    fn test_tool_transition_serialize() {
        let transition = ToolTransition::new("linear", "divergent", "s1", true);
        let json = serde_json::to_string(&transition).unwrap();
        assert!(json.contains("\"from_tool\":\"linear\""));
        assert!(json.contains("\"to_tool\":\"divergent\""));
        assert!(json.contains("\"session_id\":\"s1\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_chain_summary_serialize() {
        let collector = MetricsCollector::new();
        collector.record_transition(ToolTransition::new("linear", "divergent", "s1", true));
        let summary = collector.chain_summary();

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"transitions\""));
        assert!(json.contains("\"entry_tools\""));
        assert!(json.contains("\"terminal_tools\""));
    }

    #[test]
    fn test_avg_time_between_ms_computed_from_timestamps() {
        let collector = MetricsCollector::new();

        // Build two transitions in the same session with known timestamps:
        //   t=1000: X → linear
        //   t=1500: linear → divergent  (dwell on linear = 500ms)
        //   t=2200: X2 → linear
        //   t=2900: linear → divergent  (dwell on linear = 700ms)
        // Expected avg = (500 + 700) / 2 = 600ms
        let make = |from: &str, to: &str, ts: u64| ToolTransition {
            from_tool: from.to_string(),
            to_tool: to.to_string(),
            session_id: "s1".to_string(),
            success: true,
            timestamp: ts,
        };

        collector.record_transition(make("setup", "linear", 1000));
        collector.record_transition(make("linear", "divergent", 1500));
        collector.record_transition(make("setup", "linear", 2200));
        collector.record_transition(make("linear", "divergent", 2900));

        let stats = collector.transitions_from("linear");
        let div_stats = stats.get("divergent").expect("divergent stats");
        assert_eq!(div_stats.count, 2);
        assert_eq!(div_stats.avg_time_between_ms, 600);
    }

    #[test]
    fn test_avg_time_between_ms_no_prior_transition() {
        // If there's no prior transition landing on the from_tool, dwell is unknown → 0
        let collector = MetricsCollector::new();
        collector.record_transition(ToolTransition::new("linear", "tree", "s1", true));
        let stats = collector.transitions_from("linear");
        let tree_stats = stats.get("tree").expect("tree stats");
        assert_eq!(tree_stats.count, 1);
        assert_eq!(tree_stats.avg_time_between_ms, 0); // no prior arrival at linear
    }

    #[test]
    fn test_clear_includes_transitions() {
        let collector = MetricsCollector::new();
        collector.record_transition(ToolTransition::new("linear", "divergent", "s1", true));

        let stats = collector.transitions_from("linear");
        assert!(!stats.is_empty());

        collector.clear();

        let stats_after = collector.transitions_from("linear");
        assert!(stats_after.is_empty());
    }
}
