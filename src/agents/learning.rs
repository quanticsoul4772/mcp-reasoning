//! Agent learning and performance analysis.
//!
//! Tracks agent performance over time and provides optimization suggestions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Performance record for an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentPerformance {
    /// Total invocations.
    pub total_invocations: u64,
    /// Successful invocations.
    pub successful_invocations: u64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Average confidence score.
    pub avg_confidence: f64,
    /// Most used skills.
    pub skill_usage: HashMap<String, u64>,
}

impl AgentPerformance {
    /// Success rate (0.0-1.0).
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_invocations == 0 {
            0.0
        } else {
            self.successful_invocations as f64 / self.total_invocations as f64
        }
    }

    /// Record an invocation.
    pub fn record(&mut self, success: bool, latency_ms: u64, confidence: f64) {
        self.total_invocations += 1;
        if success {
            self.successful_invocations += 1;
        }

        // Running average
        let n = self.total_invocations as f64;
        self.avg_latency_ms = self.avg_latency_ms * (n - 1.0) / n + latency_ms as f64 / n;
        self.avg_confidence = self.avg_confidence * (n - 1.0) / n + confidence / n;
    }

    /// Record skill usage.
    pub fn record_skill(&mut self, skill_id: &str) {
        *self.skill_usage.entry(skill_id.to_string()).or_insert(0) += 1;
    }
}

/// Tracks performance across all agents.
#[derive(Debug, Default)]
pub struct AgentLearner {
    performance: HashMap<String, AgentPerformance>,
}

impl AgentLearner {
    /// Create a new learner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an agent invocation result.
    pub fn record(&mut self, agent_id: &str, success: bool, latency_ms: u64, confidence: f64) {
        self.performance
            .entry(agent_id.to_string())
            .or_default()
            .record(success, latency_ms, confidence);
    }

    /// Record a skill usage.
    pub fn record_skill_usage(&mut self, agent_id: &str, skill_id: &str) {
        self.performance
            .entry(agent_id.to_string())
            .or_default()
            .record_skill(skill_id);
    }

    /// Get performance for an agent.
    #[must_use]
    pub fn get_performance(&self, agent_id: &str) -> Option<&AgentPerformance> {
        self.performance.get(agent_id)
    }

    /// Get performance for all agents.
    #[must_use]
    pub fn all_performance(&self) -> &HashMap<String, AgentPerformance> {
        &self.performance
    }

    /// Compare agents by success rate.
    #[must_use]
    pub fn rank_by_success_rate(&self) -> Vec<(&str, f64)> {
        let mut ranked: Vec<(&str, f64)> = self
            .performance
            .iter()
            .map(|(id, perf)| (id.as_str(), perf.success_rate()))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_performance_default() {
        let perf = AgentPerformance::default();
        assert_eq!(perf.total_invocations, 0);
        assert_eq!(perf.success_rate(), 0.0);
    }

    #[test]
    fn test_agent_performance_record() {
        let mut perf = AgentPerformance::default();
        perf.record(true, 100, 0.9);
        perf.record(true, 200, 0.8);
        perf.record(false, 300, 0.3);

        assert_eq!(perf.total_invocations, 3);
        assert_eq!(perf.successful_invocations, 2);
        assert!((perf.success_rate() - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_agent_performance_record_skill() {
        let mut perf = AgentPerformance::default();
        perf.record_skill("code-review");
        perf.record_skill("code-review");
        perf.record_skill("debug-analysis");

        assert_eq!(perf.skill_usage["code-review"], 2);
        assert_eq!(perf.skill_usage["debug-analysis"], 1);
    }

    #[test]
    fn test_learner_record_and_get() {
        let mut learner = AgentLearner::new();
        learner.record("analyst", true, 150, 0.85);
        learner.record("analyst", false, 250, 0.4);

        let perf = learner.get_performance("analyst").unwrap();
        assert_eq!(perf.total_invocations, 2);
        assert_eq!(perf.successful_invocations, 1);
    }

    #[test]
    fn test_learner_get_unknown() {
        let learner = AgentLearner::new();
        assert!(learner.get_performance("unknown").is_none());
    }

    #[test]
    fn test_learner_rank_by_success_rate() {
        let mut learner = AgentLearner::new();
        learner.record("good", true, 100, 0.9);
        learner.record("good", true, 100, 0.9);
        learner.record("bad", true, 100, 0.5);
        learner.record("bad", false, 100, 0.3);

        let ranked = learner.rank_by_success_rate();
        assert_eq!(ranked[0].0, "good");
        assert!((ranked[0].1 - 1.0).abs() < f64::EPSILON);
        assert!((ranked[1].1 - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_learner_all_performance() {
        let mut learner = AgentLearner::new();
        learner.record("a1", true, 100, 0.9);
        learner.record("a2", true, 200, 0.8);

        assert_eq!(learner.all_performance().len(), 2);
    }

    #[test]
    fn test_performance_serialize() {
        let mut perf = AgentPerformance::default();
        perf.record(true, 100, 0.9);
        let json = serde_json::to_string(&perf).unwrap();
        assert!(json.contains("\"total_invocations\":1"));
    }
}
