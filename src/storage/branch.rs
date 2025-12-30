//! Branch storage operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::{BranchStatus, StoredBranch};

impl SqliteStorage {
    /// Save a branch to the database.
    pub async fn save_branch(&self, branch: &StoredBranch) -> Result<(), StorageError> {
        let created_at_str = branch.created_at.to_rfc3339();
        let status_str = branch.status.as_str();

        sqlx::query(
            "INSERT INTO branches (id, session_id, parent_branch_id, content, score, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&branch.id)
        .bind(&branch.session_id)
        .bind(&branch.parent_branch_id)
        .bind(&branch.content)
        .bind(branch.score)
        .bind(status_str)
        .bind(&created_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("INSERT branches", format!("{e}")))?;

        Ok(())
    }

    /// Get a branch by ID.
    pub async fn get_branch(&self, id: &str) -> Result<Option<StoredBranch>, StorageError> {
        let row = sqlx::query(
            "SELECT id, session_id, parent_branch_id, content, score, status, created_at
             FROM branches WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT branches", format!("{e}")))?;

        match row {
            Some(row) => {
                let branch = Self::row_to_branch(&row)?;
                Ok(Some(branch))
            }
            None => Ok(None),
        }
    }

    /// Get all branches for a session.
    pub async fn get_branches(&self, session_id: &str) -> Result<Vec<StoredBranch>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, parent_branch_id, content, score, status, created_at
             FROM branches WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT branches", format!("{e}")))?;

        let mut branches = Vec::with_capacity(rows.len());
        for row in &rows {
            branches.push(Self::row_to_branch(row)?);
        }

        Ok(branches)
    }

    /// Update branch status.
    pub async fn update_branch_status(
        &self,
        id: &str,
        status: BranchStatus,
    ) -> Result<(), StorageError> {
        let status_str = status.as_str();

        let result = sqlx::query("UPDATE branches SET status = ? WHERE id = ?")
            .bind(status_str)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE branches", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Branch not found: {id}"),
            });
        }

        Ok(())
    }

    /// Convert a database row to a `StoredBranch`.
    fn row_to_branch(row: &sqlx::sqlite::SqliteRow) -> Result<StoredBranch, StorageError> {
        let id: String = row.get("id");
        let session_id: String = row.get("session_id");
        let parent_branch_id: Option<String> = row.get("parent_branch_id");
        let content: String = row.get("content");
        let score: f64 = row.get("score");
        let status_str: String = row.get("status");
        let created_at_str: String = row.get("created_at");

        let status = BranchStatus::from_str(&status_str).unwrap_or_default();
        let created_at = Self::parse_datetime(&created_at_str)?;

        let mut branch = StoredBranch::new(&id, &session_id, &content)
            .with_score(score)
            .with_status(status);
        branch.created_at = created_at;

        if let Some(p) = parent_branch_id {
            branch = branch.with_parent(p);
        }

        Ok(branch)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_save_branch() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let branch = StoredBranch::new("b-1", "sess-123", "Branch content");
        let result = storage.save_branch(&branch).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_branch() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let branch = StoredBranch::new("b-1", "sess-123", "Branch content").with_score(0.75);
        storage.save_branch(&branch).await.expect("save");

        let fetched = storage.get_branch("b-1").await;
        assert!(fetched.is_ok());
        let fetched = fetched.expect("fetch").expect("branch exists");
        assert_eq!(fetched.id, "b-1");
        assert!((fetched.score - 0.75).abs() < f64::EPSILON);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_branches() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let branch1 = StoredBranch::new("b-1", "sess-123", "First");
        let branch2 = StoredBranch::new("b-2", "sess-123", "Second");

        storage.save_branch(&branch1).await.expect("save 1");
        storage.save_branch(&branch2).await.expect("save 2");

        let branches = storage.get_branches("sess-123").await;
        assert!(branches.is_ok());
        let branches = branches.expect("branches");
        assert_eq!(branches.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_branch_status() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let branch = StoredBranch::new("b-1", "sess-123", "Content");
        storage.save_branch(&branch).await.expect("save");

        let result = storage
            .update_branch_status("b-1", BranchStatus::Completed)
            .await;
        assert!(result.is_ok());

        let fetched = storage
            .get_branch("b-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.status, BranchStatus::Completed);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_branch_status_not_found() {
        let storage = test_storage().await;

        let result = storage
            .update_branch_status("nonexistent", BranchStatus::Completed)
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Internal { .. })));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_branch_with_parent() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        // Create parent branch
        let parent = StoredBranch::new("parent-1", "sess-123", "Parent content");
        storage.save_branch(&parent).await.expect("save parent");

        // Create child branch with parent
        let child = StoredBranch::new("child-1", "sess-123", "Child content")
            .with_parent("parent-1".to_string());
        storage.save_branch(&child).await.expect("save child");

        // Retrieve and verify parent is preserved
        let fetched = storage
            .get_branch("child-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.id, "child-1");
        assert_eq!(fetched.parent_branch_id, Some("parent-1".to_string()));
    }
}
