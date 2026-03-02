//! Agent team definitions and topologies.
//!
//! Teams coordinate multiple agents on complex tasks.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How agents in a team interact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamTopology {
    /// Agents execute in sequence: A -> B -> C.
    Sequential,
    /// All agents run simultaneously on subtasks.
    Parallel,
    /// One coordinator delegates to specialists.
    Hub,
    /// Agents challenge each other's conclusions.
    Adversarial,
}

impl std::fmt::Display for TeamTopology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequential => write!(f, "sequential"),
            Self::Parallel => write!(f, "parallel"),
            Self::Hub => write!(f, "hub"),
            Self::Adversarial => write!(f, "adversarial"),
        }
    }
}

impl TeamTopology {
    /// Parse a topology from a string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sequential" => Some(Self::Sequential),
            "parallel" => Some(Self::Parallel),
            "hub" => Some(Self::Hub),
            "adversarial" => Some(Self::Adversarial),
            _ => None,
        }
    }
}

/// Role of an agent within a team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    /// Leads the team and coordinates work.
    Lead,
    /// Contributes specialized analysis.
    Member,
    /// Reviews and challenges other agents' work.
    Reviewer,
}

/// A member of an agent team.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    /// The agent ID.
    pub agent_id: String,
    /// Role within the team.
    pub team_role: TeamRole,
    /// Optional focus area for this member.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

impl TeamMember {
    /// Create a new team member.
    #[must_use]
    pub fn new(agent_id: impl Into<String>, team_role: TeamRole) -> Self {
        Self {
            agent_id: agent_id.into(),
            team_role,
            focus: None,
        }
    }

    /// Set a focus area.
    #[must_use]
    pub fn with_focus(mut self, focus: impl Into<String>) -> Self {
        self.focus = Some(focus.into());
        self
    }
}

/// An agent team definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeam {
    /// Unique team identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of the team's purpose.
    pub description: String,
    /// Team interaction topology.
    pub topology: TeamTopology,
    /// Team members.
    pub members: Vec<TeamMember>,
}

impl AgentTeam {
    /// Create a new agent team.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        topology: TeamTopology,
        members: Vec<TeamMember>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            topology,
            members,
        }
    }

    /// Get the lead agent ID if one exists.
    #[must_use]
    pub fn lead(&self) -> Option<&str> {
        self.members
            .iter()
            .find(|m| m.team_role == TeamRole::Lead)
            .map(|m| m.agent_id.as_str())
    }
}

/// Summary info for listing teams.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamInfo {
    /// Team ID.
    pub id: String,
    /// Team name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Topology.
    pub topology: String,
    /// Number of members.
    pub member_count: usize,
}

impl From<&AgentTeam> for TeamInfo {
    fn from(team: &AgentTeam) -> Self {
        Self {
            id: team.id.clone(),
            name: team.name.clone(),
            description: team.description.clone(),
            topology: team.topology.to_string(),
            member_count: team.members.len(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_team_topology_display() {
        assert_eq!(TeamTopology::Sequential.to_string(), "sequential");
        assert_eq!(TeamTopology::Parallel.to_string(), "parallel");
        assert_eq!(TeamTopology::Hub.to_string(), "hub");
        assert_eq!(TeamTopology::Adversarial.to_string(), "adversarial");
    }

    #[test]
    fn test_team_member_new() {
        let member = TeamMember::new("analyst", TeamRole::Lead);
        assert_eq!(member.agent_id, "analyst");
        assert_eq!(member.team_role, TeamRole::Lead);
        assert!(member.focus.is_none());
    }

    #[test]
    fn test_team_member_with_focus() {
        let member = TeamMember::new("analyst", TeamRole::Member).with_focus("security analysis");
        assert_eq!(member.focus, Some("security analysis".to_string()));
    }

    #[test]
    fn test_agent_team_new() {
        let team = AgentTeam::new(
            "code-review",
            "Code Review",
            "Reviews code quality",
            TeamTopology::Adversarial,
            vec![
                TeamMember::new("analyst", TeamRole::Lead),
                TeamMember::new("explorer", TeamRole::Reviewer),
            ],
        );
        assert_eq!(team.id, "code-review");
        assert_eq!(team.members.len(), 2);
    }

    #[test]
    fn test_agent_team_lead() {
        let team = AgentTeam::new(
            "t1",
            "T1",
            "test",
            TeamTopology::Hub,
            vec![
                TeamMember::new("analyst", TeamRole::Lead),
                TeamMember::new("explorer", TeamRole::Member),
            ],
        );
        assert_eq!(team.lead(), Some("analyst"));
    }

    #[test]
    fn test_agent_team_no_lead() {
        let team = AgentTeam::new(
            "t1",
            "T1",
            "test",
            TeamTopology::Parallel,
            vec![TeamMember::new("analyst", TeamRole::Member)],
        );
        assert!(team.lead().is_none());
    }

    #[test]
    fn test_team_info_from() {
        let team = AgentTeam::new(
            "t1",
            "Team 1",
            "desc",
            TeamTopology::Sequential,
            vec![TeamMember::new("a1", TeamRole::Lead)],
        );
        let info = TeamInfo::from(&team);
        assert_eq!(info.id, "t1");
        assert_eq!(info.topology, "sequential");
        assert_eq!(info.member_count, 1);
    }

    #[test]
    fn test_team_serialize() {
        let team = AgentTeam::new(
            "t1",
            "T1",
            "desc",
            TeamTopology::Hub,
            vec![TeamMember::new("a1", TeamRole::Lead)],
        );
        let json = serde_json::to_string(&team).unwrap();
        assert!(json.contains("\"hub\""));
        assert!(json.contains("\"lead\""));
    }
}
