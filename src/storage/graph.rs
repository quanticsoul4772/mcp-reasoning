//! Graph node and edge storage operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::{GraphEdgeType, GraphNodeType, StoredGraphEdge, StoredGraphNode};

impl SqliteStorage {
    // ========== Graph Node Operations ==========

    /// Save a graph node to the database.
    pub async fn save_graph_node(&self, node: &StoredGraphNode) -> Result<(), StorageError> {
        let created_at_str = node.created_at.to_rfc3339();
        let node_type_str = node.node_type.as_str();
        let is_terminal_i32: i32 = i32::from(node.is_terminal);

        sqlx::query(
            "INSERT INTO graph_nodes (id, session_id, content, node_type, score, is_terminal, metadata, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&node.id)
        .bind(&node.session_id)
        .bind(&node.content)
        .bind(node_type_str)
        .bind(node.score)
        .bind(is_terminal_i32)
        .bind(&node.metadata)
        .bind(&created_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("INSERT graph_nodes", format!("{e}")))?;

        Ok(())
    }

    /// Get a graph node by ID.
    pub async fn get_graph_node(&self, id: &str) -> Result<Option<StoredGraphNode>, StorageError> {
        let row = sqlx::query(
            "SELECT id, session_id, content, node_type, score, is_terminal, metadata, created_at
             FROM graph_nodes WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT graph_nodes", format!("{e}")))?;

        match row {
            Some(row) => {
                let node = Self::row_to_graph_node(&row)?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    /// Get all graph nodes for a session.
    pub async fn get_graph_nodes(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredGraphNode>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, content, node_type, score, is_terminal, metadata, created_at
             FROM graph_nodes WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT graph_nodes", format!("{e}")))?;

        let mut nodes = Vec::with_capacity(rows.len());
        for row in &rows {
            nodes.push(Self::row_to_graph_node(row)?);
        }

        Ok(nodes)
    }

    /// Update graph node score.
    pub async fn update_graph_node_score(&self, id: &str, score: f64) -> Result<(), StorageError> {
        let result = sqlx::query("UPDATE graph_nodes SET score = ? WHERE id = ?")
            .bind(score)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE graph_nodes", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Graph node not found: {id}"),
            });
        }

        Ok(())
    }

    /// Mark graph node as terminal.
    pub async fn mark_graph_node_terminal(&self, id: &str) -> Result<(), StorageError> {
        let result = sqlx::query("UPDATE graph_nodes SET is_terminal = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE graph_nodes", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Graph node not found: {id}"),
            });
        }

        Ok(())
    }

    /// Delete a graph node.
    pub async fn delete_graph_node(&self, id: &str) -> Result<(), StorageError> {
        let result = sqlx::query("DELETE FROM graph_nodes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("DELETE graph_nodes", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Graph node not found: {id}"),
            });
        }

        Ok(())
    }

    /// Convert a database row to a `StoredGraphNode`.
    fn row_to_graph_node(row: &sqlx::sqlite::SqliteRow) -> Result<StoredGraphNode, StorageError> {
        let id: String = row.get("id");
        let session_id: String = row.get("session_id");
        let content: String = row.get("content");
        let node_type_str: String = row.get("node_type");
        let score: Option<f64> = row.get("score");
        let is_terminal: i32 = row.get("is_terminal");
        let metadata: Option<String> = row.get("metadata");
        let created_at_str: String = row.get("created_at");

        let node_type = GraphNodeType::from_str(&node_type_str).unwrap_or_default();
        let created_at = Self::parse_datetime(&created_at_str)?;

        let mut node = StoredGraphNode::new(&id, &session_id, &content).with_node_type(node_type);
        node.created_at = created_at;

        if let Some(s) = score {
            node = node.with_score(s);
        }
        if is_terminal != 0 {
            node = node.as_terminal();
        }
        if let Some(m) = metadata {
            node = node.with_metadata(m);
        }

        Ok(node)
    }

    // ========== Graph Edge Operations ==========

    /// Save a graph edge to the database.
    pub async fn save_graph_edge(&self, edge: &StoredGraphEdge) -> Result<(), StorageError> {
        let created_at_str = edge.created_at.to_rfc3339();
        let edge_type_str = edge.edge_type.as_str();

        sqlx::query(
            "INSERT INTO graph_edges (id, session_id, from_node_id, to_node_id, edge_type, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&edge.id)
        .bind(&edge.session_id)
        .bind(&edge.from_node_id)
        .bind(&edge.to_node_id)
        .bind(edge_type_str)
        .bind(&created_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("INSERT graph_edges", format!("{e}")))?;

        Ok(())
    }

    /// Get a graph edge by ID.
    pub async fn get_graph_edge(&self, id: &str) -> Result<Option<StoredGraphEdge>, StorageError> {
        let row = sqlx::query(
            "SELECT id, session_id, from_node_id, to_node_id, edge_type, created_at
             FROM graph_edges WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT graph_edges", format!("{e}")))?;

        match row {
            Some(row) => {
                let edge = Self::row_to_graph_edge(&row)?;
                Ok(Some(edge))
            }
            None => Ok(None),
        }
    }

    /// Get all graph edges for a session.
    pub async fn get_graph_edges(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredGraphEdge>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, from_node_id, to_node_id, edge_type, created_at
             FROM graph_edges WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT graph_edges", format!("{e}")))?;

        let mut edges = Vec::with_capacity(rows.len());
        for row in &rows {
            edges.push(Self::row_to_graph_edge(row)?);
        }

        Ok(edges)
    }

    /// Get edges from a specific node.
    pub async fn get_edges_from_node(
        &self,
        node_id: &str,
    ) -> Result<Vec<StoredGraphEdge>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, from_node_id, to_node_id, edge_type, created_at
             FROM graph_edges WHERE from_node_id = ? ORDER BY created_at ASC",
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT graph_edges", format!("{e}")))?;

        let mut edges = Vec::with_capacity(rows.len());
        for row in &rows {
            edges.push(Self::row_to_graph_edge(row)?);
        }

        Ok(edges)
    }

    /// Get edges to a specific node.
    pub async fn get_edges_to_node(
        &self,
        node_id: &str,
    ) -> Result<Vec<StoredGraphEdge>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, from_node_id, to_node_id, edge_type, created_at
             FROM graph_edges WHERE to_node_id = ? ORDER BY created_at ASC",
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT graph_edges", format!("{e}")))?;

        let mut edges = Vec::with_capacity(rows.len());
        for row in &rows {
            edges.push(Self::row_to_graph_edge(row)?);
        }

        Ok(edges)
    }

    /// Delete a graph edge.
    pub async fn delete_graph_edge(&self, id: &str) -> Result<(), StorageError> {
        let result = sqlx::query("DELETE FROM graph_edges WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("DELETE graph_edges", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::Internal {
                message: format!("Graph edge not found: {id}"),
            });
        }

        Ok(())
    }

    /// Convert a database row to a `StoredGraphEdge`.
    fn row_to_graph_edge(row: &sqlx::sqlite::SqliteRow) -> Result<StoredGraphEdge, StorageError> {
        let id: String = row.get("id");
        let session_id: String = row.get("session_id");
        let from_node_id: String = row.get("from_node_id");
        let to_node_id: String = row.get("to_node_id");
        let edge_type_str: String = row.get("edge_type");
        let created_at_str: String = row.get("created_at");

        let edge_type = GraphEdgeType::from_str(&edge_type_str).unwrap_or_default();
        let created_at = Self::parse_datetime(&created_at_str)?;

        let mut edge = StoredGraphEdge::new(&id, &session_id, &from_node_id, &to_node_id)
            .with_edge_type(edge_type);
        edge.created_at = created_at;

        Ok(edge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    // ========== Graph Node Tests ==========

    #[tokio::test]
    #[serial]
    async fn test_save_graph_node() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node = StoredGraphNode::new("n-1", "sess-123", "Node content");
        let result = storage.save_graph_node(&node).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_graph_node() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node = StoredGraphNode::new("n-1", "sess-123", "Node content")
            .with_node_type(GraphNodeType::Aggregation)
            .with_score(0.85);
        storage.save_graph_node(&node).await.expect("save");

        let fetched = storage.get_graph_node("n-1").await;
        assert!(fetched.is_ok());
        let fetched = fetched.expect("fetch").expect("node exists");
        assert_eq!(fetched.id, "n-1");
        assert_eq!(fetched.content, "Node content");
        assert_eq!(fetched.node_type, GraphNodeType::Aggregation);
        assert_eq!(fetched.score, Some(0.85));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_graph_node_not_found() {
        let storage = test_storage().await;
        let result = storage.get_graph_node("nonexistent").await;

        assert!(result.is_ok());
        assert!(result.expect("result").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_graph_nodes() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node1 = StoredGraphNode::new("n-1", "sess-123", "First");
        let node2 = StoredGraphNode::new("n-2", "sess-123", "Second");

        storage.save_graph_node(&node1).await.expect("save 1");
        storage.save_graph_node(&node2).await.expect("save 2");

        let nodes = storage.get_graph_nodes("sess-123").await;
        assert!(nodes.is_ok());
        let nodes = nodes.expect("nodes");
        assert_eq!(nodes.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_graph_node_score() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node = StoredGraphNode::new("n-1", "sess-123", "Content");
        storage.save_graph_node(&node).await.expect("save");

        let result = storage.update_graph_node_score("n-1", 0.95).await;
        assert!(result.is_ok());

        let fetched = storage
            .get_graph_node("n-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.score, Some(0.95));
    }

    #[tokio::test]
    #[serial]
    async fn test_update_graph_node_score_not_found() {
        let storage = test_storage().await;
        let result = storage.update_graph_node_score("nonexistent", 0.5).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Internal { .. })));
    }

    #[tokio::test]
    #[serial]
    async fn test_mark_graph_node_terminal() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node = StoredGraphNode::new("n-1", "sess-123", "Content");
        storage.save_graph_node(&node).await.expect("save");

        let result = storage.mark_graph_node_terminal("n-1").await;
        assert!(result.is_ok());

        let fetched = storage
            .get_graph_node("n-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert!(fetched.is_terminal);
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_graph_node() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node = StoredGraphNode::new("n-1", "sess-123", "Content");
        storage.save_graph_node(&node).await.expect("save");

        let result = storage.delete_graph_node("n-1").await;
        assert!(result.is_ok());

        let fetched = storage.get_graph_node("n-1").await.expect("fetch");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_graph_node_not_found() {
        let storage = test_storage().await;
        let result = storage.delete_graph_node("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Internal { .. })));
    }

    #[tokio::test]
    #[serial]
    async fn test_graph_node_with_metadata() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node = StoredGraphNode::new("n-1", "sess-123", "Content")
            .with_metadata(r#"{"key": "value"}"#)
            .as_terminal();
        storage.save_graph_node(&node).await.expect("save");

        let fetched = storage
            .get_graph_node("n-1")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.metadata, Some(r#"{"key": "value"}"#.to_string()));
        assert!(fetched.is_terminal);
    }

    // ========== Graph Edge Tests ==========

    #[tokio::test]
    #[serial]
    async fn test_save_graph_edge() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node1 = StoredGraphNode::new("n-1", "sess-123", "First");
        let node2 = StoredGraphNode::new("n-2", "sess-123", "Second");
        storage.save_graph_node(&node1).await.expect("save n1");
        storage.save_graph_node(&node2).await.expect("save n2");

        let edge = StoredGraphEdge::new("e-1", "sess-123", "n-1", "n-2");
        let result = storage.save_graph_edge(&edge).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_graph_edge() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node1 = StoredGraphNode::new("n-1", "sess-123", "First");
        let node2 = StoredGraphNode::new("n-2", "sess-123", "Second");
        storage.save_graph_node(&node1).await.expect("save n1");
        storage.save_graph_node(&node2).await.expect("save n2");

        let edge = StoredGraphEdge::new("e-1", "sess-123", "n-1", "n-2")
            .with_edge_type(GraphEdgeType::Aggregates);
        storage.save_graph_edge(&edge).await.expect("save");

        let fetched = storage.get_graph_edge("e-1").await;
        assert!(fetched.is_ok());
        let fetched = fetched.expect("fetch").expect("edge exists");
        assert_eq!(fetched.id, "e-1");
        assert_eq!(fetched.from_node_id, "n-1");
        assert_eq!(fetched.to_node_id, "n-2");
        assert_eq!(fetched.edge_type, GraphEdgeType::Aggregates);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_graph_edge_not_found() {
        let storage = test_storage().await;
        let result = storage.get_graph_edge("nonexistent").await;

        assert!(result.is_ok());
        assert!(result.expect("result").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_graph_edges() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node1 = StoredGraphNode::new("n-1", "sess-123", "First");
        let node2 = StoredGraphNode::new("n-2", "sess-123", "Second");
        let node3 = StoredGraphNode::new("n-3", "sess-123", "Third");
        storage.save_graph_node(&node1).await.expect("save n1");
        storage.save_graph_node(&node2).await.expect("save n2");
        storage.save_graph_node(&node3).await.expect("save n3");

        let edge1 = StoredGraphEdge::new("e-1", "sess-123", "n-1", "n-2");
        let edge2 = StoredGraphEdge::new("e-2", "sess-123", "n-2", "n-3");
        storage.save_graph_edge(&edge1).await.expect("save e1");
        storage.save_graph_edge(&edge2).await.expect("save e2");

        let edges = storage.get_graph_edges("sess-123").await;
        assert!(edges.is_ok());
        let edges = edges.expect("edges");
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_edges_from_node() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node1 = StoredGraphNode::new("n-1", "sess-123", "First");
        let node2 = StoredGraphNode::new("n-2", "sess-123", "Second");
        let node3 = StoredGraphNode::new("n-3", "sess-123", "Third");
        storage.save_graph_node(&node1).await.expect("save n1");
        storage.save_graph_node(&node2).await.expect("save n2");
        storage.save_graph_node(&node3).await.expect("save n3");

        let edge1 = StoredGraphEdge::new("e-1", "sess-123", "n-1", "n-2");
        let edge2 = StoredGraphEdge::new("e-2", "sess-123", "n-1", "n-3");
        storage.save_graph_edge(&edge1).await.expect("save e1");
        storage.save_graph_edge(&edge2).await.expect("save e2");

        let edges = storage.get_edges_from_node("n-1").await;
        assert!(edges.is_ok());
        let edges = edges.expect("edges");
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_edges_to_node() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node1 = StoredGraphNode::new("n-1", "sess-123", "First");
        let node2 = StoredGraphNode::new("n-2", "sess-123", "Second");
        let node3 = StoredGraphNode::new("n-3", "sess-123", "Third");
        storage.save_graph_node(&node1).await.expect("save n1");
        storage.save_graph_node(&node2).await.expect("save n2");
        storage.save_graph_node(&node3).await.expect("save n3");

        let edge1 = StoredGraphEdge::new("e-1", "sess-123", "n-1", "n-3");
        let edge2 = StoredGraphEdge::new("e-2", "sess-123", "n-2", "n-3");
        storage.save_graph_edge(&edge1).await.expect("save e1");
        storage.save_graph_edge(&edge2).await.expect("save e2");

        let edges = storage.get_edges_to_node("n-3").await;
        assert!(edges.is_ok());
        let edges = edges.expect("edges");
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_graph_edge() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let node1 = StoredGraphNode::new("n-1", "sess-123", "First");
        let node2 = StoredGraphNode::new("n-2", "sess-123", "Second");
        storage.save_graph_node(&node1).await.expect("save n1");
        storage.save_graph_node(&node2).await.expect("save n2");

        let edge = StoredGraphEdge::new("e-1", "sess-123", "n-1", "n-2");
        storage.save_graph_edge(&edge).await.expect("save");

        let result = storage.delete_graph_edge("e-1").await;
        assert!(result.is_ok());

        let fetched = storage.get_graph_edge("e-1").await.expect("fetch");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_graph_edge_not_found() {
        let storage = test_storage().await;
        let result = storage.delete_graph_edge("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::Internal { .. })));
    }
}
