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
    pub(crate) async fn run_migrations(&self) -> Result<(), StorageError> {
        let schema = include_str!("../../migrations/001_initial_schema.sql");

        sqlx::query(schema).execute(&self.pool).await.map_err(|e| {
            StorageError::MigrationFailed {
                version: "001".to_string(),
                message: format!("Failed to run migrations: {e}"),
            }
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
}
