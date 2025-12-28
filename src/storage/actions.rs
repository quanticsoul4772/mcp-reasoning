//! Self-improvement actions storage operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use chrono::Utc;
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::{ActionStatus, StoredSelfImprovementAction};

impl SqliteStorage {
    /// Save a self-improvement action to the database.
    pub async fn save_action(
        &self,
        action: &StoredSelfImprovementAction,
    ) -> Result<(), StorageError> {
        let created_at_str = action.created_at.to_rfc3339();
        let completed_at_str = action.completed_at.map(|dt| dt.to_rfc3339());
        let status_str = action.status.as_str();

        sqlx::query(
            "INSERT INTO self_improvement_actions (id, action_type, parameters, status, result, created_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&action.id)
        .bind(&action.action_type)
        .bind(&action.parameters)
        .bind(status_str)
        .bind(&action.result)
        .bind(&created_at_str)
        .bind(&completed_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("INSERT self_improvement_actions", format!("{e}")))?;

        Ok(())
    }

    /// Get an action by ID.
    pub async fn get_action(
        &self,
        id: &str,
    ) -> Result<Option<StoredSelfImprovementAction>, StorageError> {
        let row = sqlx::query(
            "SELECT id, action_type, parameters, status, result, created_at, completed_at
             FROM self_improvement_actions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT self_improvement_actions", format!("{e}")))?;

        match row {
            Some(row) => {
                let action = Self::row_to_action(&row)?;
                Ok(Some(action))
            }
            None => Ok(None),
        }
    }

    /// Get actions by status.
    pub async fn get_actions_by_status(
        &self,
        status: ActionStatus,
    ) -> Result<Vec<StoredSelfImprovementAction>, StorageError> {
        let status_str = status.as_str();

        let rows = sqlx::query(
            "SELECT id, action_type, parameters, status, result, created_at, completed_at
             FROM self_improvement_actions WHERE status = ? ORDER BY created_at ASC",
        )
        .bind(status_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT self_improvement_actions", format!("{e}")))?;

        let mut actions = Vec::with_capacity(rows.len());
        for row in &rows {
            actions.push(Self::row_to_action(row)?);
        }

        Ok(actions)
    }

    /// Update action status.
    pub async fn update_action_status(
        &self,
        id: &str,
        status: ActionStatus,
    ) -> Result<(), StorageError> {
        let status_str = status.as_str();

        let result = sqlx::query("UPDATE self_improvement_actions SET status = ? WHERE id = ?")
            .bind(status_str)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE self_improvement_actions", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Action not found: {id}"),
            });
        }

        Ok(())
    }

    /// Complete an action with result.
    pub async fn complete_action(&self, id: &str, result: &str) -> Result<(), StorageError> {
        let completed_at = Utc::now().to_rfc3339();
        let status_str = ActionStatus::Completed.as_str();

        let query_result = sqlx::query(
            "UPDATE self_improvement_actions SET status = ?, result = ?, completed_at = ? WHERE id = ?",
        )
        .bind(status_str)
        .bind(result)
        .bind(&completed_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("UPDATE self_improvement_actions", format!("{e}")))?;

        if query_result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Action not found: {id}"),
            });
        }

        Ok(())
    }

    /// Fail an action with error result.
    pub async fn fail_action(&self, id: &str, error: &str) -> Result<(), StorageError> {
        let completed_at = Utc::now().to_rfc3339();
        let status_str = ActionStatus::Failed.as_str();

        let query_result = sqlx::query(
            "UPDATE self_improvement_actions SET status = ?, result = ?, completed_at = ? WHERE id = ?",
        )
        .bind(status_str)
        .bind(error)
        .bind(&completed_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("UPDATE self_improvement_actions", format!("{e}")))?;

        if query_result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Action not found: {id}"),
            });
        }

        Ok(())
    }

    /// Convert a database row to a `StoredSelfImprovementAction`.
    fn row_to_action(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<StoredSelfImprovementAction, StorageError> {
        let id: String = row.get("id");
        let action_type: String = row.get("action_type");
        let parameters: String = row.get("parameters");
        let status_str: String = row.get("status");
        let result: Option<String> = row.get("result");
        let created_at_str: String = row.get("created_at");
        let completed_at_str: Option<String> = row.get("completed_at");

        let status = ActionStatus::from_str(&status_str).unwrap_or_default();
        let created_at = Self::parse_datetime(&created_at_str)?;
        let completed_at = completed_at_str
            .map(|s| Self::parse_datetime(&s))
            .transpose()?;

        Ok(StoredSelfImprovementAction {
            id,
            action_type,
            parameters,
            status,
            result,
            created_at,
            completed_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_save_action() {
        let storage = test_storage().await;

        let action =
            StoredSelfImprovementAction::new("a-1", "adjust_temperature", r#"{"temp": 0.8}"#);
        let result = storage.save_action(&action).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_action() {
        let storage = test_storage().await;

        let action =
            StoredSelfImprovementAction::new("a-1", "adjust_temperature", r#"{"temp": 0.8}"#);
        storage.save_action(&action).await.expect("save");

        let fetched = storage.get_action("a-1").await;
        assert!(fetched.is_ok());
        let fetched = fetched.expect("fetch").expect("action exists");
        assert_eq!(fetched.id, "a-1");
        assert_eq!(fetched.action_type, "adjust_temperature");
        assert_eq!(fetched.status, ActionStatus::Pending);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_action_not_found() {
        let storage = test_storage().await;
        let result = storage.get_action("nonexistent").await;

        assert!(result.is_ok());
        assert!(result.expect("result").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_actions_by_status() {
        let storage = test_storage().await;

        let action1 = StoredSelfImprovementAction::new("a-1", "action1", "{}");
        let action2 = StoredSelfImprovementAction::new("a-2", "action2", "{}")
            .with_status(ActionStatus::Executing);
        let action3 = StoredSelfImprovementAction::new("a-3", "action3", "{}");

        storage.save_action(&action1).await.expect("save 1");
        storage.save_action(&action2).await.expect("save 2");
        storage.save_action(&action3).await.expect("save 3");

        let actions = storage.get_actions_by_status(ActionStatus::Pending).await;
        assert!(actions.is_ok());
        let actions = actions.expect("actions");
        assert_eq!(actions.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_action_status() {
        let storage = test_storage().await;

        let action = StoredSelfImprovementAction::new("a-1", "action", "{}");
        storage.save_action(&action).await.expect("save");

        let result = storage
            .update_action_status("a-1", ActionStatus::Executing)
            .await;
        assert!(result.is_ok());

        let fetched = storage
            .get_action("a-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.status, ActionStatus::Executing);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_action_status_not_found() {
        let storage = test_storage().await;
        let result = storage
            .update_action_status("nonexistent", ActionStatus::Completed)
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Internal { .. })));
    }

    #[tokio::test]
    #[serial]
    async fn test_complete_action() {
        let storage = test_storage().await;

        let action = StoredSelfImprovementAction::new("a-1", "action", "{}");
        storage.save_action(&action).await.expect("save");

        let result = storage.complete_action("a-1", r#"{"success": true}"#).await;
        assert!(result.is_ok());

        let fetched = storage
            .get_action("a-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.status, ActionStatus::Completed);
        assert_eq!(fetched.result, Some(r#"{"success": true}"#.to_string()));
        assert!(fetched.completed_at.is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_complete_action_not_found() {
        let storage = test_storage().await;
        let result = storage.complete_action("nonexistent", "{}").await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Internal { .. })));
    }

    #[tokio::test]
    #[serial]
    async fn test_fail_action() {
        let storage = test_storage().await;

        let action = StoredSelfImprovementAction::new("a-1", "action", "{}");
        storage.save_action(&action).await.expect("save");

        let result = storage.fail_action("a-1", "Something went wrong").await;
        assert!(result.is_ok());

        let fetched = storage
            .get_action("a-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.status, ActionStatus::Failed);
        assert_eq!(fetched.result, Some("Something went wrong".to_string()));
        assert!(fetched.completed_at.is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_fail_action_not_found() {
        let storage = test_storage().await;
        let result = storage.fail_action("nonexistent", "error").await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Internal { .. })));
    }
}
