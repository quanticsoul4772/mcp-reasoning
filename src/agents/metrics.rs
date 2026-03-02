//! Agent-specific metrics recording and querying.
//!
//! Tracks agent invocations, skill usage, and team performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// An agent metric event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricEvent {
    /// Agent ID.
    pub agent_id: String,
    /// Operation type.
    pub operation: AgentOperation,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Confidence score if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Skill used if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    /// Team ID if part of a team execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// Timestamp.
    pub timestamp: u64,
}

/// Type of agent operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentOperation {
    /// Agent invocation.
    Invoke,
    /// Skill execution.
    SkillRun,
    /// Team execution.
    TeamRun,
    /// Task decomposition.
    Decompose,
}

impl AgentMetricEvent {
    /// Create a new metric event.
    #[must_use]
    pub fn new(
        agent_id: impl Into<String>,
        operation: AgentOperation,
        latency_ms: u64,
        success: bool,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            operation,
            latency_ms,
            success,
            confidence: None,
            skill_id: None,
            team_id: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Set confidence.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Set skill ID.
    #[must_use]
    pub fn with_skill(mut self, skill_id: impl Into<String>) -> Self {
        self.skill_id = Some(skill_id.into());
        self
    }

    /// Set team ID.
    #[must_use]
    pub fn with_team(mut self, team_id: impl Into<String>) -> Self {
        self.team_id = Some(team_id.into());
        self
    }
}

/// Summary of agent metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetricsSummary {
    /// Total events recorded.
    pub total_events: usize,
    /// Events by agent.
    pub by_agent: HashMap<String, AgentStats>,
    /// Events by operation type.
    pub by_operation: HashMap<String, u64>,
}

/// Statistics for a single agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStats {
    /// Total invocations.
    pub invocations: u64,
    /// Successful invocations.
    pub successes: u64,
    /// Average latency.
    pub avg_latency_ms: f64,
}

/// Collects agent metrics.
#[derive(Debug, Default)]
pub struct AgentMetricsCollector {
    events: Mutex<Vec<AgentMetricEvent>>,
}

impl AgentMetricsCollector {
    /// Create a new collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an agent metric event.
    pub fn record(&self, event: AgentMetricEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }

    /// Get a summary of all metrics.
    #[must_use]
    pub fn summary(&self) -> AgentMetricsSummary {
        let events = match self.events.lock() {
            Ok(e) => e,
            Err(_) => return AgentMetricsSummary::default(),
        };

        let mut summary = AgentMetricsSummary {
            total_events: events.len(),
            ..Default::default()
        };

        for event in events.iter() {
            let stats = summary.by_agent.entry(event.agent_id.clone()).or_default();
            stats.invocations += 1;
            if event.success {
                stats.successes += 1;
            }
            let n = stats.invocations as f64;
            stats.avg_latency_ms =
                stats.avg_latency_ms * (n - 1.0) / n + event.latency_ms as f64 / n;

            let op_key = format!("{:?}", event.operation).to_lowercase();
            *summary.by_operation.entry(op_key).or_insert(0) += 1;
        }
        drop(events);

        summary
    }

    /// Get events for a specific agent.
    #[must_use]
    pub fn events_for(&self, agent_id: &str) -> Vec<AgentMetricEvent> {
        self.events.lock().map_or_else(
            |_| vec![],
            |events| {
                events
                    .iter()
                    .filter(|e| e.agent_id == agent_id)
                    .cloned()
                    .collect()
            },
        )
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
    fn test_agent_metric_event_new() {
        let event = AgentMetricEvent::new("analyst", AgentOperation::Invoke, 150, true);
        assert_eq!(event.agent_id, "analyst");
        assert!(event.success);
        assert!(event.confidence.is_none());
    }

    #[test]
    fn test_agent_metric_event_with_fields() {
        let event = AgentMetricEvent::new("analyst", AgentOperation::SkillRun, 200, true)
            .with_confidence(0.85)
            .with_skill("code-review")
            .with_team("code-review-team");
        assert_eq!(event.confidence, Some(0.85));
        assert_eq!(event.skill_id, Some("code-review".to_string()));
        assert_eq!(event.team_id, Some("code-review-team".to_string()));
    }

    #[test]
    fn test_collector_record_and_summary() {
        let collector = AgentMetricsCollector::new();
        collector.record(AgentMetricEvent::new(
            "analyst",
            AgentOperation::Invoke,
            100,
            true,
        ));
        collector.record(AgentMetricEvent::new(
            "analyst",
            AgentOperation::Invoke,
            200,
            false,
        ));
        collector.record(AgentMetricEvent::new(
            "explorer",
            AgentOperation::SkillRun,
            150,
            true,
        ));

        let summary = collector.summary();
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.by_agent["analyst"].invocations, 2);
        assert_eq!(summary.by_agent["analyst"].successes, 1);
        assert_eq!(summary.by_agent["explorer"].invocations, 1);
    }

    #[test]
    fn test_collector_events_for() {
        let collector = AgentMetricsCollector::new();
        collector.record(AgentMetricEvent::new(
            "analyst",
            AgentOperation::Invoke,
            100,
            true,
        ));
        collector.record(AgentMetricEvent::new(
            "explorer",
            AgentOperation::Invoke,
            200,
            true,
        ));

        let analyst_events = collector.events_for("analyst");
        assert_eq!(analyst_events.len(), 1);
        assert_eq!(analyst_events[0].agent_id, "analyst");
    }

    #[test]
    fn test_collector_empty_summary() {
        let collector = AgentMetricsCollector::new();
        let summary = collector.summary();
        assert_eq!(summary.total_events, 0);
        assert!(summary.by_agent.is_empty());
    }

    #[test]
    fn test_metric_event_serialize() {
        let event = AgentMetricEvent::new("analyst", AgentOperation::Invoke, 100, true);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"analyst\""));
        assert!(json.contains("\"invoke\""));
    }
}
