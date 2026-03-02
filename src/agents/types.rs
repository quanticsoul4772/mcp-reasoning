//! Agent type definitions.
//!
//! Core types for the agent system including roles, capabilities,
//! configuration, and status tracking.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Role that determines an agent's primary tool access and behavior.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Analytical agent: linear, tree, reflection, detect.
    Analyst,
    /// Strategic agent: decision, evidence, counterfactual, timeline.
    Strategist,
    /// Exploratory agent: divergent, mcts, graph, timeline.
    Explorer,
    /// Coordination agent: auto, preset, checkpoint, metrics.
    Coordinator,
    /// User-defined role.
    Custom(String),
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Analyst => write!(f, "analyst"),
            Self::Strategist => write!(f, "strategist"),
            Self::Explorer => write!(f, "explorer"),
            Self::Coordinator => write!(f, "coordinator"),
            Self::Custom(name) => write!(f, "custom:{name}"),
        }
    }
}

impl AgentRole {
    /// Parse a role from a string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "analyst" => Some(Self::Analyst),
            "strategist" => Some(Self::Strategist),
            "explorer" => Some(Self::Explorer),
            "coordinator" => Some(Self::Coordinator),
            other if other.starts_with("custom:") => Some(Self::Custom(
                other.trim_start_matches("custom:").to_string(),
            )),
            _ => None,
        }
    }
}

/// A specific tool capability an agent can use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentCapability {
    /// The reasoning mode name (e.g., "linear", "tree").
    pub mode: String,
    /// Allowed operations within the mode (empty = all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<String>,
}

impl AgentCapability {
    /// Create a capability for a mode with all operations.
    #[must_use]
    pub fn mode(name: impl Into<String>) -> Self {
        Self {
            mode: name.into(),
            operations: Vec::new(),
        }
    }

    /// Create a capability for a mode with specific operations.
    #[must_use]
    pub fn mode_with_ops(name: impl Into<String>, ops: Vec<String>) -> Self {
        Self {
            mode: name.into(),
            operations: ops,
        }
    }

    /// Check if this capability allows a specific operation.
    #[must_use]
    pub fn allows_operation(&self, operation: &str) -> bool {
        self.operations.is_empty() || self.operations.iter().any(|op| op == operation)
    }
}

/// Configuration for agent behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum number of tool steps per invocation.
    pub max_steps: usize,
    /// Confidence threshold for proceeding (0.0-1.0).
    pub confidence_threshold: f64,
    /// Whether to include reasoning traces in output.
    pub include_reasoning: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            confidence_threshold: 0.7,
            include_reasoning: false,
        }
    }
}

/// Current status of an agent invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is idle.
    Idle,
    /// Agent is planning steps.
    Planning,
    /// Agent is executing a step.
    Executing,
    /// Agent completed successfully.
    Completed,
    /// Agent failed.
    Failed,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Planning => write!(f, "planning"),
            Self::Executing => write!(f, "executing"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// An agent definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique agent identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Agent's role.
    pub role: AgentRole,
    /// Description of what this agent does.
    pub description: String,
    /// Tools this agent can use.
    pub capabilities: Vec<AgentCapability>,
    /// Default skills to apply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_skills: Vec<String>,
    /// Agent configuration.
    pub config: AgentConfig,
}

impl Agent {
    /// Create a new agent.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        role: AgentRole,
        description: impl Into<String>,
        capabilities: Vec<AgentCapability>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role,
            description: description.into(),
            capabilities,
            default_skills: Vec::new(),
            config: AgentConfig::default(),
        }
    }

    /// Set default skills.
    #[must_use]
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.default_skills = skills;
        self
    }

    /// Set agent configuration.
    #[must_use]
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Check if this agent has a specific capability.
    #[must_use]
    pub fn has_capability(&self, mode: &str) -> bool {
        self.capabilities.iter().any(|c| c.mode == mode)
    }

    /// Check if this agent can perform a specific mode+operation.
    #[must_use]
    pub fn can_perform(&self, mode: &str, operation: &str) -> bool {
        self.capabilities
            .iter()
            .any(|c| c.mode == mode && c.allows_operation(operation))
    }
}

/// Summary info for listing agents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentInfo {
    /// Agent ID.
    pub id: String,
    /// Agent name.
    pub name: String,
    /// Agent role.
    pub role: String,
    /// Description.
    pub description: String,
    /// Number of capabilities.
    pub capability_count: usize,
    /// Default skill IDs.
    pub default_skills: Vec<String>,
}

impl From<&Agent> for AgentInfo {
    fn from(agent: &Agent) -> Self {
        Self {
            id: agent.id.clone(),
            name: agent.name.clone(),
            role: agent.role.to_string(),
            description: agent.description.clone(),
            capability_count: agent.capabilities.len(),
            default_skills: agent.default_skills.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_display() {
        assert_eq!(AgentRole::Analyst.to_string(), "analyst");
        assert_eq!(AgentRole::Strategist.to_string(), "strategist");
        assert_eq!(AgentRole::Explorer.to_string(), "explorer");
        assert_eq!(AgentRole::Coordinator.to_string(), "coordinator");
        assert_eq!(
            AgentRole::Custom("my-role".to_string()).to_string(),
            "custom:my-role"
        );
    }

    #[test]
    fn test_agent_role_serialize() {
        let json = serde_json::to_string(&AgentRole::Analyst).unwrap();
        assert_eq!(json, "\"analyst\"");
        let custom = serde_json::to_string(&AgentRole::Custom("x".into())).unwrap();
        assert!(custom.contains("custom"));
    }

    #[test]
    fn test_agent_role_deserialize() {
        let role: AgentRole = serde_json::from_str("\"analyst\"").unwrap();
        assert_eq!(role, AgentRole::Analyst);
    }

    #[test]
    fn test_agent_capability_mode() {
        let cap = AgentCapability::mode("linear");
        assert_eq!(cap.mode, "linear");
        assert!(cap.operations.is_empty());
        assert!(cap.allows_operation("anything"));
    }

    #[test]
    fn test_agent_capability_with_ops() {
        let cap =
            AgentCapability::mode_with_ops("tree", vec!["create".to_string(), "focus".to_string()]);
        assert_eq!(cap.mode, "tree");
        assert!(cap.allows_operation("create"));
        assert!(cap.allows_operation("focus"));
        assert!(!cap.allows_operation("delete"));
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.max_steps, 10);
        assert!((config.confidence_threshold - 0.7).abs() < f64::EPSILON);
        assert!(!config.include_reasoning);
    }

    #[test]
    fn test_agent_status_display() {
        assert_eq!(AgentStatus::Idle.to_string(), "idle");
        assert_eq!(AgentStatus::Planning.to_string(), "planning");
        assert_eq!(AgentStatus::Executing.to_string(), "executing");
        assert_eq!(AgentStatus::Completed.to_string(), "completed");
        assert_eq!(AgentStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_agent_new() {
        let agent = Agent::new(
            "test",
            "Test Agent",
            AgentRole::Analyst,
            "A test agent",
            vec![AgentCapability::mode("linear")],
        );
        assert_eq!(agent.id, "test");
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.role, AgentRole::Analyst);
        assert!(agent.default_skills.is_empty());
    }

    #[test]
    fn test_agent_with_skills() {
        let agent = Agent::new(
            "a",
            "A",
            AgentRole::Analyst,
            "desc",
            vec![AgentCapability::mode("linear")],
        )
        .with_skills(vec!["code-review".to_string()]);
        assert_eq!(agent.default_skills, vec!["code-review"]);
    }

    #[test]
    fn test_agent_with_config() {
        let config = AgentConfig {
            max_steps: 5,
            confidence_threshold: 0.9,
            include_reasoning: true,
        };
        let agent = Agent::new(
            "a",
            "A",
            AgentRole::Analyst,
            "desc",
            vec![AgentCapability::mode("linear")],
        )
        .with_config(config);
        assert_eq!(agent.config.max_steps, 5);
    }

    #[test]
    fn test_agent_has_capability() {
        let agent = Agent::new(
            "a",
            "A",
            AgentRole::Analyst,
            "desc",
            vec![
                AgentCapability::mode("linear"),
                AgentCapability::mode("tree"),
            ],
        );
        assert!(agent.has_capability("linear"));
        assert!(agent.has_capability("tree"));
        assert!(!agent.has_capability("graph"));
    }

    #[test]
    fn test_agent_can_perform() {
        let agent = Agent::new(
            "a",
            "A",
            AgentRole::Analyst,
            "desc",
            vec![
                AgentCapability::mode("linear"),
                AgentCapability::mode_with_ops(
                    "tree",
                    vec!["create".to_string(), "focus".to_string()],
                ),
            ],
        );
        assert!(agent.can_perform("linear", "process"));
        assert!(agent.can_perform("tree", "create"));
        assert!(!agent.can_perform("tree", "delete"));
        assert!(!agent.can_perform("graph", "init"));
    }

    #[test]
    fn test_agent_info_from() {
        let agent = Agent::new(
            "test",
            "Test",
            AgentRole::Explorer,
            "An explorer",
            vec![AgentCapability::mode("divergent")],
        )
        .with_skills(vec!["deep-review".to_string()]);

        let info = AgentInfo::from(&agent);
        assert_eq!(info.id, "test");
        assert_eq!(info.role, "explorer");
        assert_eq!(info.capability_count, 1);
        assert_eq!(info.default_skills, vec!["deep-review"]);
    }

    #[test]
    fn test_agent_serialize() {
        let agent = Agent::new(
            "test",
            "Test",
            AgentRole::Analyst,
            "desc",
            vec![AgentCapability::mode("linear")],
        );
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("\"id\":\"test\""));
        assert!(json.contains("\"analyst\""));
    }

    #[test]
    fn test_agent_deserialize() {
        let json = r#"{
            "id": "test",
            "name": "Test",
            "role": "analyst",
            "description": "desc",
            "capabilities": [{"mode": "linear"}],
            "config": {"max_steps": 10, "confidence_threshold": 0.7, "include_reasoning": false}
        }"#;
        let agent: Agent = serde_json::from_str(json).unwrap();
        assert_eq!(agent.id, "test");
        assert_eq!(agent.role, AgentRole::Analyst);
    }
}
