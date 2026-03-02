//! Agent registry for managing available agents.
//!
//! Follows the `PresetRegistry` pattern: HashMap-based with built-in
//! agents registered on construction.

use std::collections::HashMap;

use super::types::{Agent, AgentCapability, AgentConfig, AgentRole};

/// Registry of available agents.
#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Agent>,
}

impl AgentRegistry {
    /// Create a new registry with built-in agents.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_builtin_agents();
        registry
    }

    /// Register the 4 built-in agents.
    fn register_builtin_agents(&mut self) {
        // 1. Analyst
        self.register(
            Agent::new(
                "analyst",
                "Analyst",
                AgentRole::Analyst,
                "Analytical agent for code review, debugging, and systematic analysis",
                vec![
                    AgentCapability::mode("linear"),
                    AgentCapability::mode("tree"),
                    AgentCapability::mode("reflection"),
                    AgentCapability::mode("detect"),
                ],
            )
            .with_skills(vec![
                "code-review".to_string(),
                "debug-analysis".to_string(),
            ]),
        );

        // 2. Strategist
        self.register(
            Agent::new(
                "strategist",
                "Strategist",
                AgentRole::Strategist,
                "Strategic agent for decisions, evidence evaluation, and causal analysis",
                vec![
                    AgentCapability::mode("decision"),
                    AgentCapability::mode("evidence"),
                    AgentCapability::mode("counterfactual"),
                    AgentCapability::mode("timeline"),
                ],
            )
            .with_skills(vec![
                "strategic-decision".to_string(),
                "evidence-conclusion".to_string(),
            ]),
        );

        // 3. Explorer
        self.register(
            Agent::new(
                "explorer",
                "Explorer",
                AgentRole::Explorer,
                "Exploratory agent for divergent thinking, search, and graph analysis",
                vec![
                    AgentCapability::mode("divergent"),
                    AgentCapability::mode("mcts"),
                    AgentCapability::mode("graph"),
                    AgentCapability::mode("timeline"),
                ],
            )
            .with_skills(vec!["architecture-decision".to_string()]),
        );

        // 4. Coordinator
        self.register(
            Agent::new(
                "coordinator",
                "Coordinator",
                AgentRole::Coordinator,
                "Coordination agent that orchestrates other agents and manages workflows",
                vec![
                    AgentCapability::mode("auto"),
                    AgentCapability::mode("preset"),
                    AgentCapability::mode("checkpoint"),
                    AgentCapability::mode("metrics"),
                ],
            )
            .with_config(AgentConfig {
                max_steps: 15,
                confidence_threshold: 0.6,
                include_reasoning: true,
            }),
        );
    }

    /// Register an agent.
    pub fn register(&mut self, agent: Agent) {
        self.agents.insert(agent.id.clone(), agent);
    }

    /// Get an agent by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Agent> {
        self.agents.get(id)
    }

    /// List all agents.
    #[must_use]
    pub fn list(&self) -> Vec<&Agent> {
        self.agents.values().collect()
    }

    /// List agents by role.
    #[must_use]
    pub fn list_by_role(&self, role: &AgentRole) -> Vec<&Agent> {
        self.agents.values().filter(|a| &a.role == role).collect()
    }

    /// Find agents that can use a specific mode.
    #[must_use]
    pub fn find_by_capability(&self, mode: &str) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|a| a.has_capability(mode))
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new_has_builtin_agents() {
        let registry = AgentRegistry::new();
        assert_eq!(registry.list().len(), 4);
    }

    #[test]
    fn test_registry_get_analyst() {
        let registry = AgentRegistry::new();
        let agent = registry.get("analyst").unwrap();
        assert_eq!(agent.name, "Analyst");
        assert_eq!(agent.role, AgentRole::Analyst);
        assert!(agent.has_capability("linear"));
        assert!(agent.has_capability("tree"));
        assert!(agent.has_capability("reflection"));
        assert!(agent.has_capability("detect"));
    }

    #[test]
    fn test_registry_get_strategist() {
        let registry = AgentRegistry::new();
        let agent = registry.get("strategist").unwrap();
        assert_eq!(agent.role, AgentRole::Strategist);
        assert!(agent.has_capability("decision"));
        assert!(agent.has_capability("evidence"));
    }

    #[test]
    fn test_registry_get_explorer() {
        let registry = AgentRegistry::new();
        let agent = registry.get("explorer").unwrap();
        assert_eq!(agent.role, AgentRole::Explorer);
        assert!(agent.has_capability("divergent"));
        assert!(agent.has_capability("mcts"));
    }

    #[test]
    fn test_registry_get_coordinator() {
        let registry = AgentRegistry::new();
        let agent = registry.get("coordinator").unwrap();
        assert_eq!(agent.role, AgentRole::Coordinator);
        assert_eq!(agent.config.max_steps, 15);
    }

    #[test]
    fn test_registry_get_unknown() {
        let registry = AgentRegistry::new();
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_registry_register_custom() {
        let mut registry = AgentRegistry::new();
        let initial = registry.list().len();

        registry.register(Agent::new(
            "custom-agent",
            "Custom",
            AgentRole::Custom("special".to_string()),
            "A custom agent",
            vec![AgentCapability::mode("linear")],
        ));

        assert_eq!(registry.list().len(), initial + 1);
        assert!(registry.get("custom-agent").is_some());
    }

    #[test]
    fn test_registry_list_by_role() {
        let registry = AgentRegistry::new();
        let analysts = registry.list_by_role(&AgentRole::Analyst);
        assert_eq!(analysts.len(), 1);
        assert_eq!(analysts[0].id, "analyst");
    }

    #[test]
    fn test_registry_find_by_capability() {
        let registry = AgentRegistry::new();

        let timeline_agents = registry.find_by_capability("timeline");
        assert_eq!(timeline_agents.len(), 2); // strategist + explorer

        let linear_agents = registry.find_by_capability("linear");
        assert_eq!(linear_agents.len(), 1); // analyst only
    }

    #[test]
    fn test_registry_default_skills() {
        let registry = AgentRegistry::new();
        let analyst = registry.get("analyst").unwrap();
        assert!(analyst.default_skills.contains(&"code-review".to_string()));
        assert!(analyst
            .default_skills
            .contains(&"debug-analysis".to_string()));
    }
}
