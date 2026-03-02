//! Agent metrics storage operations.
//!
//! Provides CRUD operations for agent invocations, messages, and discovered skills.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::{StoredAgentInvocation, StoredAgentMessage, StoredDiscoveredSkill};

// SQL constants for agent invocations
const INSERT_AGENT_INVOCATION: &str = "\
    INSERT INTO agent_invocations (id, agent_id, session_id, operation, task, skill_id, team_id, \
    latency_ms, success, confidence, created_at) \
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))";

const SELECT_AGENT_INVOCATIONS_BY_AGENT: &str = "\
    SELECT id, agent_id, session_id, operation, task, skill_id, team_id, \
    latency_ms, success, confidence, created_at \
    FROM agent_invocations WHERE agent_id = ? ORDER BY created_at DESC";

const SELECT_RECENT_AGENT_INVOCATIONS: &str = "\
    SELECT id, agent_id, session_id, operation, task, skill_id, team_id, \
    latency_ms, success, confidence, created_at \
    FROM agent_invocations ORDER BY created_at DESC LIMIT ?";

// SQL constants for agent messages
const INSERT_AGENT_MESSAGE: &str = "\
    INSERT INTO agent_messages (id, session_id, from_agent, to_agent, content, message_type, \
    created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))";

const SELECT_AGENT_MESSAGES_BY_SESSION: &str = "\
    SELECT id, session_id, from_agent, to_agent, content, message_type, created_at \
    FROM agent_messages WHERE session_id = ? ORDER BY created_at ASC";

// SQL constants for discovered skills
const INSERT_DISCOVERED_SKILL: &str = "\
    INSERT INTO discovered_skills (id, tool_chain, occurrences, avg_success_rate, materialized, \
    skill_id, discovered_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))";

const SELECT_DISCOVERED_SKILLS: &str = "\
    SELECT id, tool_chain, occurrences, avg_success_rate, materialized, skill_id, discovered_at \
    FROM discovered_skills ORDER BY occurrences DESC";

const UPDATE_DISCOVERED_SKILL_MATERIALIZED: &str = "\
    UPDATE discovered_skills SET materialized = 1, skill_id = ? WHERE id = ?";

impl SqliteStorage {
    /// Save an agent invocation record.
    pub async fn save_agent_invocation(
        &self,
        invocation: &StoredAgentInvocation,
    ) -> Result<(), StorageError> {
        let success_i32: i32 = i32::from(invocation.success);

        sqlx::query(INSERT_AGENT_INVOCATION)
            .bind(&invocation.id)
            .bind(&invocation.agent_id)
            .bind(&invocation.session_id)
            .bind(&invocation.operation)
            .bind(&invocation.task)
            .bind(&invocation.skill_id)
            .bind(&invocation.team_id)
            .bind(invocation.latency_ms)
            .bind(success_i32)
            .bind(invocation.confidence)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("INSERT agent_invocations", format!("{e}")))?;

        Ok(())
    }

    /// Get invocations for a specific agent.
    pub async fn get_agent_invocations(
        &self,
        agent_id: &str,
    ) -> Result<Vec<StoredAgentInvocation>, StorageError> {
        let rows = sqlx::query(SELECT_AGENT_INVOCATIONS_BY_AGENT)
            .bind(agent_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Self::query_error("SELECT agent_invocations", format!("{e}")))?;

        Ok(rows.iter().map(Self::row_to_agent_invocation).collect())
    }

    /// Get recent agent invocations.
    pub async fn get_recent_agent_invocations(
        &self,
        limit: u32,
    ) -> Result<Vec<StoredAgentInvocation>, StorageError> {
        let rows = sqlx::query(SELECT_RECENT_AGENT_INVOCATIONS)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Self::query_error("SELECT agent_invocations", format!("{e}")))?;

        Ok(rows.iter().map(Self::row_to_agent_invocation).collect())
    }

    /// Save an agent message.
    pub async fn save_agent_message(
        &self,
        message: &StoredAgentMessage,
    ) -> Result<(), StorageError> {
        sqlx::query(INSERT_AGENT_MESSAGE)
            .bind(&message.id)
            .bind(&message.session_id)
            .bind(&message.from_agent)
            .bind(&message.to_agent)
            .bind(&message.content)
            .bind(&message.message_type)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("INSERT agent_messages", format!("{e}")))?;

        Ok(())
    }

    /// Get messages for a session.
    pub async fn get_agent_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredAgentMessage>, StorageError> {
        let rows = sqlx::query(SELECT_AGENT_MESSAGES_BY_SESSION)
            .bind(session_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Self::query_error("SELECT agent_messages", format!("{e}")))?;

        Ok(rows.iter().map(Self::row_to_agent_message).collect())
    }

    /// Save a discovered skill pattern.
    pub async fn save_discovered_skill(
        &self,
        skill: &StoredDiscoveredSkill,
    ) -> Result<(), StorageError> {
        let materialized_i32: i32 = i32::from(skill.materialized);
        let tool_chain_json =
            serde_json::to_string(&skill.tool_chain).map_err(|e| StorageError::Internal {
                message: format!("Failed to serialize tool chain: {e}"),
            })?;

        sqlx::query(INSERT_DISCOVERED_SKILL)
            .bind(&skill.id)
            .bind(&tool_chain_json)
            .bind(skill.occurrences)
            .bind(skill.avg_success_rate)
            .bind(materialized_i32)
            .bind(&skill.skill_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("INSERT discovered_skills", format!("{e}")))?;

        Ok(())
    }

    /// Get all discovered skills.
    pub async fn get_discovered_skills(&self) -> Result<Vec<StoredDiscoveredSkill>, StorageError> {
        let rows = sqlx::query(SELECT_DISCOVERED_SKILLS)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Self::query_error("SELECT discovered_skills", format!("{e}")))?;

        rows.iter().map(Self::row_to_discovered_skill).collect()
    }

    /// Mark a discovered skill as materialized.
    pub async fn materialize_discovered_skill(
        &self,
        id: &str,
        skill_id: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(UPDATE_DISCOVERED_SKILL_MATERIALIZED)
            .bind(skill_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE discovered_skills", format!("{e}")))?;

        Ok(())
    }

    // Row converters

    fn row_to_agent_invocation(row: &sqlx::sqlite::SqliteRow) -> StoredAgentInvocation {
        let success: i32 = row.get("success");

        StoredAgentInvocation {
            id: row.get("id"),
            agent_id: row.get("agent_id"),
            session_id: row.get("session_id"),
            operation: row.get("operation"),
            task: row.get("task"),
            skill_id: row.get("skill_id"),
            team_id: row.get("team_id"),
            latency_ms: row.get("latency_ms"),
            success: success != 0,
            confidence: row.get("confidence"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_agent_message(row: &sqlx::sqlite::SqliteRow) -> StoredAgentMessage {
        StoredAgentMessage {
            id: row.get("id"),
            session_id: row.get("session_id"),
            from_agent: row.get("from_agent"),
            to_agent: row.get("to_agent"),
            content: row.get("content"),
            message_type: row.get("message_type"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_discovered_skill(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<StoredDiscoveredSkill, StorageError> {
        let materialized: i32 = row.get("materialized");
        let tool_chain_json: String = row.get("tool_chain");
        let tool_chain: Vec<String> =
            serde_json::from_str(&tool_chain_json).map_err(|e| StorageError::Internal {
                message: format!("Failed to parse tool chain JSON: {e}"),
            })?;

        Ok(StoredDiscoveredSkill {
            id: row.get("id"),
            tool_chain,
            occurrences: row.get("occurrences"),
            avg_success_rate: row.get("avg_success_rate"),
            materialized: materialized != 0,
            skill_id: row.get("skill_id"),
            discovered_at: row.get("discovered_at"),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_save_and_get_agent_invocation() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("session-1")
            .await
            .expect("create session");

        let invocation = StoredAgentInvocation {
            id: "inv-1".to_string(),
            agent_id: "analyst".to_string(),
            session_id: "session-1".to_string(),
            operation: "invoke".to_string(),
            task: "Review code quality".to_string(),
            skill_id: Some("code-review".to_string()),
            team_id: None,
            latency_ms: 1500,
            success: true,
            confidence: Some(0.85),
            created_at: String::new(),
        };

        storage
            .save_agent_invocation(&invocation)
            .await
            .expect("save invocation");

        let results = storage
            .get_agent_invocations("analyst")
            .await
            .expect("get invocations");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent_id, "analyst");
        assert_eq!(results[0].task, "Review code quality");
        assert!(results[0].success);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_recent_agent_invocations() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("session-1")
            .await
            .expect("create session");

        for i in 0..5 {
            let invocation = StoredAgentInvocation {
                id: format!("inv-{i}"),
                agent_id: "analyst".to_string(),
                session_id: "session-1".to_string(),
                operation: "invoke".to_string(),
                task: format!("Task {i}"),
                skill_id: None,
                team_id: None,
                latency_ms: 100 * (i + 1),
                success: true,
                confidence: None,
                created_at: String::new(),
            };
            storage
                .save_agent_invocation(&invocation)
                .await
                .expect("save");
        }

        let results = storage
            .get_recent_agent_invocations(3)
            .await
            .expect("get recent");
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    #[serial]
    async fn test_save_and_get_agent_message() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("session-1")
            .await
            .expect("create session");

        let message = StoredAgentMessage {
            id: "msg-1".to_string(),
            session_id: "session-1".to_string(),
            from_agent: "analyst".to_string(),
            to_agent: Some("explorer".to_string()),
            content: "Please review this pattern".to_string(),
            message_type: "request".to_string(),
            created_at: String::new(),
        };

        storage
            .save_agent_message(&message)
            .await
            .expect("save message");

        let results = storage
            .get_agent_messages("session-1")
            .await
            .expect("get messages");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].from_agent, "analyst");
        assert_eq!(results[0].to_agent, Some("explorer".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_save_and_get_discovered_skill() {
        let storage = test_storage().await;

        let skill = StoredDiscoveredSkill {
            id: "disc-1".to_string(),
            tool_chain: vec!["linear".to_string(), "tree".to_string()],
            occurrences: 10,
            avg_success_rate: 0.9,
            materialized: false,
            skill_id: None,
            discovered_at: String::new(),
        };

        storage
            .save_discovered_skill(&skill)
            .await
            .expect("save discovered skill");

        let results = storage
            .get_discovered_skills()
            .await
            .expect("get discovered skills");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool_chain, vec!["linear", "tree"]);
        assert_eq!(results[0].occurrences, 10);
    }

    #[tokio::test]
    #[serial]
    async fn test_materialize_discovered_skill() {
        let storage = test_storage().await;

        let skill = StoredDiscoveredSkill {
            id: "disc-2".to_string(),
            tool_chain: vec!["divergent".to_string(), "reflection".to_string()],
            occurrences: 15,
            avg_success_rate: 0.85,
            materialized: false,
            skill_id: None,
            discovered_at: String::new(),
        };

        storage.save_discovered_skill(&skill).await.expect("save");

        storage
            .materialize_discovered_skill("disc-2", "discovered-dive-refl")
            .await
            .expect("materialize");

        let results = storage.get_discovered_skills().await.expect("get");
        assert_eq!(results.len(), 1);
        assert!(results[0].materialized);
        assert_eq!(
            results[0].skill_id,
            Some("discovered-dive-refl".to_string())
        );
    }
}
