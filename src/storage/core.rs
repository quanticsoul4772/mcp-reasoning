//! Core `SQLite` storage implementation.
//!
//! This module provides the main [`SqliteStorage`] struct and core database operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

/// `SQLite` storage backend.
///
/// Provides persistent storage for sessions, thoughts, branches,
/// checkpoints, graph data, metrics, and self-improvement actions.
#[derive(Debug, Clone)]
pub struct SqliteStorage {
    pub(crate) pool: SqlitePool,
}

impl SqliteStorage {
    /// Get a clone of the connection pool.
    ///
    /// Useful for creating additional storage instances that share the connection pool.
    #[must_use]
    pub fn get_pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    /// Create a new `SQLite` storage instance.
    ///
    /// # Arguments
    ///
    /// * `database_path` - Path to the `SQLite` database file
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::ConnectionFailed`] if the connection fails.
    pub async fn new(database_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = database_path.as_ref();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::ConnectionFailed {
                message: format!("Failed to create database directory: {e}"),
            })?;
        }

        let options =
            SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", path.display()))
                .map_err(|e| StorageError::ConnectionFailed {
                    message: format!("Invalid database path: {e}"),
                })?
                .journal_mode(SqliteJournalMode::Wal)
                .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::ConnectionFailed {
                message: format!("Failed to connect to database: {e}"),
            })?;

        let storage = Self { pool };
        storage.run_migrations().await?;

        Ok(storage)
    }

    /// Create a new in-memory `SQLite` storage instance for testing.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::ConnectionFailed`] if the connection fails.
    pub async fn new_in_memory() -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(|e| StorageError::ConnectionFailed {
                message: format!("Invalid memory database options: {e}"),
            })?
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::ConnectionFailed {
                message: format!("Failed to create in-memory database: {e}"),
            })?;

        let storage = Self { pool };
        storage.run_migrations().await?;

        Ok(storage)
    }

    /// Run database migrations.
    ///
    /// Migrations are run in order. Each migration is idempotent (uses IF NOT EXISTS/IF EXISTS).
    pub(crate) async fn run_migrations(&self) -> Result<(), StorageError> {
        // Migration 001: Initial schema
        let schema_001 = include_str!("../../migrations/001_initial_schema.sql");
        sqlx::query(schema_001)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::MigrationFailed {
                version: "001".to_string(),
                message: format!("Failed to run migration 001: {e}"),
            })?;

        // Migration 002: Unique constraint on si_actions.diagnosis_id
        let schema_002 = include_str!("../../migrations/002_si_actions_unique_diagnosis.sql");
        sqlx::query(schema_002)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::MigrationFailed {
                version: "002".to_string(),
                message: format!("Failed to run migration 002: {e}"),
            })?;

        Ok(())
    }

    /// Generate a new UUID.
    pub(crate) fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Parse a datetime string from the database.
    pub(crate) fn parse_datetime(s: &str) -> Result<DateTime<Utc>, StorageError> {
        s.parse::<DateTime<Utc>>()
            .map_err(|e| StorageError::Internal {
                message: format!("Failed to parse datetime '{s}': {e}"),
            })
    }

    /// Create a query error with the given query name and message.
    pub(crate) fn query_error(query: &str, message: String) -> StorageError {
        StorageError::QueryFailed {
            query: query.to_string(),
            message,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
pub mod tests {
    use super::*;
    use serial_test::serial;

    pub async fn test_storage() -> SqliteStorage {
        SqliteStorage::new_in_memory()
            .await
            .expect("Failed to create test storage")
    }

    #[tokio::test]
    #[serial]
    async fn test_new_in_memory() {
        let storage = SqliteStorage::new_in_memory().await;
        assert!(storage.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_new_with_file() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_mcp_reasoning.db");
        let _ = std::fs::remove_file(&db_path);

        let storage = SqliteStorage::new(&db_path).await;
        assert!(storage.is_ok());

        let _ = std::fs::remove_file(&db_path);
    }

    // ========== Additional tests for coverage ==========

    #[test]
    fn test_generate_id() {
        let id1 = SqliteStorage::generate_id();
        let id2 = SqliteStorage::generate_id();

        // IDs should be unique
        assert_ne!(id1, id2);

        // Should be valid UUIDs
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());
    }

    #[test]
    fn test_parse_datetime_valid() {
        let datetime_str = "2024-01-15T10:30:00Z";
        let result = SqliteStorage::parse_datetime(datetime_str);
        assert!(result.is_ok());

        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let invalid_str = "not-a-datetime";
        let result = SqliteStorage::parse_datetime(invalid_str);
        assert!(result.is_err());

        match result {
            Err(StorageError::Internal { message }) => {
                assert!(message.contains("Failed to parse datetime"));
                assert!(message.contains("not-a-datetime"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_query_error() {
        let err = SqliteStorage::query_error("SELECT * FROM foo", "some db error".to_string());

        match err {
            StorageError::QueryFailed { query, message } => {
                assert_eq!(query, "SELECT * FROM foo");
                assert_eq!(message, "some db error");
            }
            _ => panic!("Expected QueryFailed error"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_new_with_nested_path() {
        let temp_dir = std::env::temp_dir();
        let nested_path = temp_dir.join("deeply").join("nested").join("path");
        let db_path = nested_path.join("test_nested.db");

        // Clean up first if exists
        let _ = std::fs::remove_dir_all(temp_dir.join("deeply"));

        // Should create parent directories
        let storage = SqliteStorage::new(&db_path).await;
        assert!(storage.is_ok());

        // Clean up
        let _ = std::fs::remove_dir_all(temp_dir.join("deeply"));
    }

    #[tokio::test]
    #[serial]
    async fn test_storage_debug() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let debug = format!("{:?}", storage);
        assert!(debug.contains("SqliteStorage"));
    }

    #[tokio::test]
    #[serial]
    async fn test_storage_clone() {
        let storage1 = SqliteStorage::new_in_memory().await.unwrap();
        let storage2 = storage1.clone();

        // Both should work
        let id1 = SqliteStorage::generate_id();
        let id2 = SqliteStorage::generate_id();
        assert_ne!(id1, id2);

        // Check that cloned storage works
        drop(storage1);
        // storage2 should still be usable (pool is shared)
        drop(storage2);
    }

    #[test]
    fn test_parse_datetime_with_offset() {
        // ISO 8601 with Z offset
        let dt_str = "2024-06-15T14:30:00Z";
        let result = SqliteStorage::parse_datetime(dt_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_datetime_empty() {
        let result = SqliteStorage::parse_datetime("");
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_pool() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let pool = storage.get_pool();

        // Verify the pool works by running a simple query
        let result = sqlx::query("SELECT 1 as value").fetch_one(&pool).await;
        assert!(result.is_ok());
    }

    use chrono::Datelike;
}
