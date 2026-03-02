//! Team registry for managing available agent teams.
//!
//! Follows the `AgentRegistry` / `SkillRegistry` pattern: HashMap-based
//! with built-in teams registered on construction.

use std::collections::HashMap;

use super::team::{AgentTeam, TeamMember, TeamRole, TeamTopology};

/// Registry of available agent teams.
#[derive(Debug, Default)]
pub struct TeamRegistry {
    teams: HashMap<String, AgentTeam>,
}

impl TeamRegistry {
    /// Create a new registry with built-in teams.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_builtin_teams();
        registry
    }

    /// Register the 5 built-in teams.
    fn register_builtin_teams(&mut self) {
        self.register(AgentTeam::new(
            "code-review",
            "Code Review",
            "Multi-agent code quality review",
            TeamTopology::Adversarial,
            vec![
                TeamMember::new("analyst", TeamRole::Lead).with_focus("code quality"),
                TeamMember::new("explorer", TeamRole::Reviewer).with_focus("alternatives"),
            ],
        ));

        self.register(AgentTeam::new(
            "architecture-decision",
            "Architecture Decision",
            "Collaborative architectural analysis",
            TeamTopology::Hub,
            vec![
                TeamMember::new("strategist", TeamRole::Lead),
                TeamMember::new("analyst", TeamRole::Member).with_focus("technical feasibility"),
                TeamMember::new("explorer", TeamRole::Member).with_focus("alternatives"),
            ],
        ));

        self.register(AgentTeam::new(
            "debug-investigation",
            "Debug Investigation",
            "Systematic debugging pipeline",
            TeamTopology::Sequential,
            vec![
                TeamMember::new("analyst", TeamRole::Lead).with_focus("root cause"),
                TeamMember::new("strategist", TeamRole::Member).with_focus("fix strategy"),
            ],
        ));

        self.register(AgentTeam::new(
            "research-synthesis",
            "Research Synthesis",
            "Multi-perspective research and synthesis",
            TeamTopology::Sequential,
            vec![
                TeamMember::new("explorer", TeamRole::Lead).with_focus("exploration"),
                TeamMember::new("strategist", TeamRole::Member).with_focus("evaluation"),
                TeamMember::new("analyst", TeamRole::Member).with_focus("synthesis"),
            ],
        ));

        self.register(AgentTeam::new(
            "full-analysis",
            "Full Analysis",
            "Comprehensive analysis with all agents",
            TeamTopology::Hub,
            vec![
                TeamMember::new("coordinator", TeamRole::Lead),
                TeamMember::new("analyst", TeamRole::Member).with_focus("detailed analysis"),
                TeamMember::new("strategist", TeamRole::Member).with_focus("strategic review"),
                TeamMember::new("explorer", TeamRole::Member).with_focus("exploration"),
            ],
        ));
    }

    /// Register a team.
    pub fn register(&mut self, team: AgentTeam) {
        self.teams.insert(team.id.clone(), team);
    }

    /// Get a team by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&AgentTeam> {
        self.teams.get(id)
    }

    /// List all teams.
    #[must_use]
    pub fn list(&self) -> Vec<&AgentTeam> {
        self.teams.values().collect()
    }

    /// List teams by topology.
    #[must_use]
    pub fn list_by_topology(&self, topology: &TeamTopology) -> Vec<&AgentTeam> {
        self.teams
            .values()
            .filter(|t| &t.topology == topology)
            .collect()
    }

    /// Find teams that include a specific agent.
    #[must_use]
    pub fn find_by_agent(&self, agent_id: &str) -> Vec<&AgentTeam> {
        self.teams
            .values()
            .filter(|t| t.members.iter().any(|m| m.agent_id == agent_id))
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new_has_builtin_teams() {
        let registry = TeamRegistry::new();
        assert_eq!(registry.list().len(), 5);
    }

    #[test]
    fn test_registry_get_code_review() {
        let registry = TeamRegistry::new();
        let team = registry.get("code-review").unwrap();
        assert_eq!(team.name, "Code Review");
        assert_eq!(team.topology, TeamTopology::Adversarial);
        assert_eq!(team.members.len(), 2);
        assert_eq!(team.lead(), Some("analyst"));
    }

    #[test]
    fn test_registry_get_architecture_decision() {
        let registry = TeamRegistry::new();
        let team = registry.get("architecture-decision").unwrap();
        assert_eq!(team.topology, TeamTopology::Hub);
        assert_eq!(team.members.len(), 3);
        assert_eq!(team.lead(), Some("strategist"));
    }

    #[test]
    fn test_registry_get_full_analysis() {
        let registry = TeamRegistry::new();
        let team = registry.get("full-analysis").unwrap();
        assert_eq!(team.members.len(), 4);
        assert_eq!(team.lead(), Some("coordinator"));
    }

    #[test]
    fn test_registry_get_unknown() {
        let registry = TeamRegistry::new();
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_registry_register_custom() {
        let mut registry = TeamRegistry::new();
        let initial = registry.list().len();

        registry.register(AgentTeam::new(
            "custom-team",
            "Custom",
            "A custom team",
            TeamTopology::Parallel,
            vec![TeamMember::new("analyst", TeamRole::Member)],
        ));

        assert_eq!(registry.list().len(), initial + 1);
        assert!(registry.get("custom-team").is_some());
    }

    #[test]
    fn test_registry_list_by_topology() {
        let registry = TeamRegistry::new();

        let sequential = registry.list_by_topology(&TeamTopology::Sequential);
        assert_eq!(sequential.len(), 2); // debug-investigation + research-synthesis

        let hub = registry.list_by_topology(&TeamTopology::Hub);
        assert_eq!(hub.len(), 2); // architecture-decision + full-analysis

        let adversarial = registry.list_by_topology(&TeamTopology::Adversarial);
        assert_eq!(adversarial.len(), 1); // code-review
    }

    #[test]
    fn test_registry_find_by_agent() {
        let registry = TeamRegistry::new();

        // Analyst appears in all 5 teams
        let analyst_teams = registry.find_by_agent("analyst");
        assert_eq!(analyst_teams.len(), 5);

        // Coordinator only appears in full-analysis
        let coord_teams = registry.find_by_agent("coordinator");
        assert_eq!(coord_teams.len(), 1);
        assert_eq!(coord_teams[0].id, "full-analysis");

        // Unknown agent
        let none_teams = registry.find_by_agent("unknown");
        assert!(none_teams.is_empty());
    }
}
