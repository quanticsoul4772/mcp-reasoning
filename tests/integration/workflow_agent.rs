//! Agent system integration tests.
//!
//! Tests the agent layer end-to-end:
//! 1. Agent registry with built-in agents
//! 2. Agent capabilities and role filtering
//! 3. Agent team definitions and topologies
//! 4. Agent communication
//! 5. Serialization round-trips
//!
//! Note: Tests requiring LLM mocks (executor, coordinator, decomposer) are
//! in the corresponding unit tests within src/agents/*.rs since mockall
//! types are only available inside the crate.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use mcp_reasoning::agents::communication::{AgentMailbox, AgentMessage, MessageType};
use mcp_reasoning::agents::registry::AgentRegistry;
use mcp_reasoning::agents::team::{AgentTeam, TeamInfo, TeamMember, TeamRole, TeamTopology};
use mcp_reasoning::agents::types::{
    Agent, AgentCapability, AgentConfig, AgentInfo, AgentRole, AgentStatus,
};

// ============================================================================
// Agent Registry Integration Tests
// ============================================================================

#[test]
fn test_registry_has_all_four_builtin_agents() {
    let registry = AgentRegistry::new();
    let agents = registry.list();
    assert_eq!(agents.len(), 4, "Should have exactly 4 built-in agents");

    assert!(registry.get("analyst").is_some());
    assert!(registry.get("strategist").is_some());
    assert!(registry.get("explorer").is_some());
    assert!(registry.get("coordinator").is_some());
}

#[test]
fn test_agent_roles_match_capabilities() {
    let registry = AgentRegistry::new();

    // Analyst should have analytical tools
    let analyst = registry.get("analyst").unwrap();
    assert_eq!(analyst.role, AgentRole::Analyst);
    for mode in &["linear", "tree", "reflection", "detect"] {
        assert!(
            analyst.has_capability(mode),
            "Analyst should have capability: {mode}"
        );
    }

    // Strategist should have strategic tools
    let strategist = registry.get("strategist").unwrap();
    assert_eq!(strategist.role, AgentRole::Strategist);
    for mode in &["decision", "evidence", "counterfactual", "timeline"] {
        assert!(
            strategist.has_capability(mode),
            "Strategist should have capability: {mode}"
        );
    }

    // Explorer should have exploration tools
    let explorer = registry.get("explorer").unwrap();
    assert_eq!(explorer.role, AgentRole::Explorer);
    for mode in &["divergent", "mcts", "graph", "timeline"] {
        assert!(
            explorer.has_capability(mode),
            "Explorer should have capability: {mode}"
        );
    }

    // Coordinator should have orchestration tools
    let coordinator = registry.get("coordinator").unwrap();
    assert_eq!(coordinator.role, AgentRole::Coordinator);
    for mode in &["auto", "preset", "checkpoint", "metrics"] {
        assert!(
            coordinator.has_capability(mode),
            "Coordinator should have capability: {mode}"
        );
    }
}

#[test]
fn test_agents_have_correct_default_skills() {
    let registry = AgentRegistry::new();

    let analyst = registry.get("analyst").unwrap();
    assert!(analyst.default_skills.contains(&"code-review".to_string()));
    assert!(analyst
        .default_skills
        .contains(&"debug-analysis".to_string()));

    let strategist = registry.get("strategist").unwrap();
    assert!(strategist
        .default_skills
        .contains(&"strategic-decision".to_string()));
    assert!(strategist
        .default_skills
        .contains(&"evidence-conclusion".to_string()));

    let explorer = registry.get("explorer").unwrap();
    assert!(explorer
        .default_skills
        .contains(&"architecture-decision".to_string()));
}

#[test]
fn test_find_agents_by_shared_capability() {
    let registry = AgentRegistry::new();

    // Timeline is shared between strategist and explorer
    let timeline_agents = registry.find_by_capability("timeline");
    assert_eq!(
        timeline_agents.len(),
        2,
        "Strategist and Explorer both have timeline"
    );

    let agent_ids: Vec<&str> = timeline_agents.iter().map(|a| a.id.as_str()).collect();
    assert!(agent_ids.contains(&"strategist"));
    assert!(agent_ids.contains(&"explorer"));
}

#[test]
fn test_list_agents_by_role() {
    let registry = AgentRegistry::new();

    let analysts = registry.list_by_role(&AgentRole::Analyst);
    assert_eq!(analysts.len(), 1);
    assert_eq!(analysts[0].id, "analyst");

    let strategists = registry.list_by_role(&AgentRole::Strategist);
    assert_eq!(strategists.len(), 1);

    // No custom agents in default registry
    let custom = registry.list_by_role(&AgentRole::Custom("anything".to_string()));
    assert!(custom.is_empty());
}

#[test]
fn test_custom_agent_registration() {
    let mut registry = AgentRegistry::new();
    let initial_count = registry.list().len();

    registry.register(
        Agent::new(
            "security-auditor",
            "Security Auditor",
            AgentRole::Custom("security".to_string()),
            "Specialized agent for security analysis",
            vec![
                AgentCapability::mode("detect"),
                AgentCapability::mode("evidence"),
                AgentCapability::mode_with_ops(
                    "tree",
                    vec!["create".to_string(), "focus".to_string()],
                ),
            ],
        )
        .with_skills(vec!["deep-code-review".to_string()])
        .with_config(AgentConfig {
            max_steps: 8,
            confidence_threshold: 0.85,
            include_reasoning: true,
        }),
    );

    assert_eq!(registry.list().len(), initial_count + 1);

    let auditor = registry.get("security-auditor").unwrap();
    assert_eq!(auditor.role, AgentRole::Custom("security".to_string()));
    assert!(auditor.can_perform("tree", "create"));
    assert!(!auditor.can_perform("tree", "delete"));
    assert_eq!(auditor.config.max_steps, 8);
}

#[test]
fn test_agent_info_conversion() {
    let registry = AgentRegistry::new();

    for agent in registry.list() {
        let info = AgentInfo::from(agent);
        assert_eq!(info.id, agent.id);
        assert_eq!(info.name, agent.name);
        assert_eq!(info.role, agent.role.to_string());
        assert_eq!(info.capability_count, agent.capabilities.len());
        assert_eq!(info.default_skills.len(), agent.default_skills.len());
    }
}

#[test]
fn test_agent_status_serialization_roundtrip() {
    let statuses = vec![
        AgentStatus::Idle,
        AgentStatus::Planning,
        AgentStatus::Executing,
        AgentStatus::Completed,
        AgentStatus::Failed,
    ];

    for status in &statuses {
        let json = serde_json::to_string(status).unwrap();
        let back: AgentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, status);
    }
}

// ============================================================================
// Team Definition Integration Tests
// ============================================================================

#[test]
fn test_all_team_topologies() {
    let topologies = vec![
        (TeamTopology::Sequential, "sequential"),
        (TeamTopology::Parallel, "parallel"),
        (TeamTopology::Hub, "hub"),
        (TeamTopology::Adversarial, "adversarial"),
    ];

    for (topology, expected) in &topologies {
        assert_eq!(topology.to_string(), *expected);
        let json = serde_json::to_string(topology).unwrap();
        let back: TeamTopology = serde_json::from_str(&json).unwrap();
        assert_eq!(topology, &back);
    }
}

#[test]
fn test_code_review_team_structure() {
    let team = AgentTeam::new(
        "code-review",
        "Code Review",
        "Multi-perspective code review with adversarial analysis",
        TeamTopology::Adversarial,
        vec![
            TeamMember::new("analyst", TeamRole::Lead).with_focus("code structure"),
            TeamMember::new("explorer", TeamRole::Reviewer).with_focus("alternative approaches"),
        ],
    );

    assert_eq!(team.lead(), Some("analyst"));
    assert_eq!(team.members.len(), 2);
    assert_eq!(team.topology, TeamTopology::Adversarial);
    assert_eq!(team.members[0].focus, Some("code structure".to_string()));
}

#[test]
fn test_full_analysis_team() {
    let team = AgentTeam::new(
        "full-analysis",
        "Full Analysis",
        "Comprehensive analysis using all agents",
        TeamTopology::Hub,
        vec![
            TeamMember::new("coordinator", TeamRole::Lead),
            TeamMember::new("analyst", TeamRole::Member).with_focus("code quality"),
            TeamMember::new("strategist", TeamRole::Member).with_focus("architecture"),
            TeamMember::new("explorer", TeamRole::Member).with_focus("alternatives"),
        ],
    );

    assert_eq!(team.lead(), Some("coordinator"));
    assert_eq!(team.members.len(), 4);

    let info = TeamInfo::from(&team);
    assert_eq!(info.id, "full-analysis");
    assert_eq!(info.topology, "hub");
    assert_eq!(info.member_count, 4);
}

#[test]
fn test_team_without_lead() {
    let team = AgentTeam::new(
        "parallel-team",
        "Parallel Team",
        "All members equal",
        TeamTopology::Parallel,
        vec![
            TeamMember::new("analyst", TeamRole::Member),
            TeamMember::new("explorer", TeamRole::Member),
        ],
    );

    assert!(team.lead().is_none());
}

// ============================================================================
// Communication Integration Tests
// ============================================================================

#[test]
fn test_mailbox_multi_agent_communication() {
    let mut mailbox = AgentMailbox::new();

    // Analyst sends analysis to explorer
    mailbox.send(
        AgentMessage::new(
            "m1",
            "s1",
            "analyst",
            "Code quality is good",
            MessageType::Info,
        )
        .to("explorer"),
    );

    // Explorer challenges analyst
    mailbox.send(
        AgentMessage::new(
            "m2",
            "s1",
            "explorer",
            "But have you considered alternative patterns?",
            MessageType::Challenge,
        )
        .to("analyst"),
    );

    // Coordinator broadcasts synthesis
    mailbox.send(AgentMessage::new(
        "m3",
        "s1",
        "coordinator",
        "Team synthesis: code is good with room for pattern improvement",
        MessageType::Synthesis,
    ));

    assert_eq!(mailbox.len(), 3);

    // Explorer sees: direct message from analyst + broadcast
    let explorer_msgs = mailbox.messages_for("explorer");
    assert_eq!(explorer_msgs.len(), 2);

    // Analyst sees: challenge from explorer + broadcast
    let analyst_msgs = mailbox.messages_for("analyst");
    assert_eq!(analyst_msgs.len(), 2);

    // Strategist sees: only broadcast
    let strategist_msgs = mailbox.messages_for("strategist");
    assert_eq!(strategist_msgs.len(), 1);
    assert_eq!(strategist_msgs[0].message_type, MessageType::Synthesis);
}

#[test]
fn test_message_type_serialization() {
    let types = vec![
        MessageType::Request,
        MessageType::Response,
        MessageType::Info,
        MessageType::Challenge,
        MessageType::Synthesis,
    ];

    for msg_type in &types {
        let json = serde_json::to_string(msg_type).unwrap();
        let back: MessageType = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, msg_type);
    }
}

// ============================================================================
// Agent Serialization Integration Tests
// ============================================================================

#[test]
fn test_agent_round_trip_serialization() {
    let agent = Agent::new(
        "test-agent",
        "Test Agent",
        AgentRole::Custom("tester".to_string()),
        "An agent for testing",
        vec![
            AgentCapability::mode("linear"),
            AgentCapability::mode_with_ops("tree", vec!["create".to_string()]),
        ],
    )
    .with_skills(vec!["code-review".to_string()])
    .with_config(AgentConfig {
        max_steps: 5,
        confidence_threshold: 0.8,
        include_reasoning: true,
    });

    let json = serde_json::to_string(&agent).unwrap();
    let deserialized: Agent = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "test-agent");
    assert_eq!(deserialized.role, AgentRole::Custom("tester".to_string()));
    assert_eq!(deserialized.capabilities.len(), 2);
    assert_eq!(deserialized.default_skills, vec!["code-review"]);
    assert_eq!(deserialized.config.max_steps, 5);
}

#[test]
fn test_team_round_trip_serialization() {
    let team = AgentTeam::new(
        "test-team",
        "Test Team",
        "A test team",
        TeamTopology::Adversarial,
        vec![
            TeamMember::new("analyst", TeamRole::Lead).with_focus("quality"),
            TeamMember::new("explorer", TeamRole::Reviewer),
        ],
    );

    let json = serde_json::to_string(&team).unwrap();
    let deserialized: AgentTeam = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "test-team");
    assert_eq!(deserialized.topology, TeamTopology::Adversarial);
    assert_eq!(deserialized.members.len(), 2);
    assert_eq!(deserialized.lead(), Some("analyst"));
}

// ============================================================================
// Cross-System Integration: Agents + Skills
// ============================================================================

#[test]
fn test_agent_default_skills_exist_in_skill_registry() {
    use mcp_reasoning::presets::PresetRegistry;
    use mcp_reasoning::skills::registry::SkillRegistry;

    let agent_registry = AgentRegistry::new();
    let preset_registry = PresetRegistry::new();
    let skill_registry = SkillRegistry::with_presets(&preset_registry);

    // Every agent's default skills should be findable in the skill registry
    for agent in agent_registry.list() {
        for skill_id in &agent.default_skills {
            assert!(
                skill_registry.get(skill_id).is_some(),
                "Agent '{}' references skill '{}' which should exist in SkillRegistry",
                agent.id,
                skill_id
            );
        }
    }
}
