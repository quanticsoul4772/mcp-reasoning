//! Checkpoint reasoning mode.
//!
//! This mode provides state management for non-linear exploration:
//! - `create`: Save current reasoning state with a name
//! - `list`: Show all available checkpoints for a session
//! - `restore`: Return to a saved checkpoint

#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::cast_possible_truncation)]

use serde::{Deserialize, Serialize};

use crate::error::ModeError;
use crate::modes::generate_checkpoint_id;
use crate::traits::{AnthropicClientTrait, StorageTrait, StoredCheckpoint};

/// Context captured in a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointContext {
    /// Key findings so far.
    pub key_findings: Vec<String>,
    /// What's currently being explored.
    pub current_focus: String,
    /// Questions not yet answered.
    pub open_questions: Vec<String>,
}

impl CheckpointContext {
    /// Create a new checkpoint context.
    #[must_use]
    pub fn new(
        key_findings: Vec<String>,
        current_focus: impl Into<String>,
        open_questions: Vec<String>,
    ) -> Self {
        Self {
            key_findings,
            current_focus: current_focus.into(),
            open_questions,
        }
    }
}

/// Summary of a checkpoint for listing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointSummary {
    /// Checkpoint ID.
    pub id: String,
    /// Checkpoint name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Creation timestamp (RFC 3339).
    pub created_at: String,
    /// Number of thoughts at checkpoint time.
    pub thought_count: usize,
}

impl CheckpointSummary {
    /// Create a new checkpoint summary.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        created_at: impl Into<String>,
        thought_count: usize,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            created_at: created_at.into(),
            thought_count,
        }
    }

    /// Add a description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Response from the `create` operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateResponse {
    /// The created checkpoint ID.
    pub checkpoint_id: String,
    /// Session ID.
    pub session_id: String,
    /// Checkpoint name.
    pub name: String,
    /// Resumption hint.
    pub resumption_hint: String,
}

impl CreateResponse {
    /// Create a new create response.
    #[must_use]
    pub fn new(
        checkpoint_id: impl Into<String>,
        session_id: impl Into<String>,
        name: impl Into<String>,
        resumption_hint: impl Into<String>,
    ) -> Self {
        Self {
            checkpoint_id: checkpoint_id.into(),
            session_id: session_id.into(),
            name: name.into(),
            resumption_hint: resumption_hint.into(),
        }
    }
}

/// Response from the `list` operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListResponse {
    /// Session ID.
    pub session_id: String,
    /// Available checkpoints.
    pub checkpoints: Vec<CheckpointSummary>,
}

impl ListResponse {
    /// Create a new list response.
    #[must_use]
    pub fn new(session_id: impl Into<String>, checkpoints: Vec<CheckpointSummary>) -> Self {
        Self {
            session_id: session_id.into(),
            checkpoints,
        }
    }
}

/// State restored from a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoredState {
    /// Context from the checkpoint.
    pub context: CheckpointContext,
    /// Thoughts at checkpoint time.
    pub thought_count: usize,
}

impl RestoredState {
    /// Create a new restored state.
    #[must_use]
    pub fn new(context: CheckpointContext, thought_count: usize) -> Self {
        Self {
            context,
            thought_count,
        }
    }
}

/// Response from the `restore` operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreResponse {
    /// Restored checkpoint ID.
    pub checkpoint_id: String,
    /// Session ID.
    pub session_id: String,
    /// Restored state.
    pub restored_state: RestoredState,
    /// New direction (if provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_direction: Option<String>,
}

impl RestoreResponse {
    /// Create a new restore response.
    #[must_use]
    pub fn new(
        checkpoint_id: impl Into<String>,
        session_id: impl Into<String>,
        restored_state: RestoredState,
    ) -> Self {
        Self {
            checkpoint_id: checkpoint_id.into(),
            session_id: session_id.into(),
            restored_state,
            new_direction: None,
        }
    }

    /// Add a new direction.
    #[must_use]
    pub fn with_new_direction(mut self, direction: impl Into<String>) -> Self {
        self.new_direction = Some(direction.into());
        self
    }
}

/// Checkpoint reasoning mode.
///
/// Provides state management for non-linear exploration with backtracking.
pub struct CheckpointMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    // Reserved for future AI-enhanced checkpoint features (e.g., smart restore suggestions)
    #[allow(dead_code)]
    client: C,
}

impl<S, C> CheckpointMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new checkpoint mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Create a new checkpoint.
    ///
    /// Saves the current reasoning state with a name and optional description.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to checkpoint
    /// * `name` - Name for the checkpoint
    /// * `description` - Optional description
    /// * `context` - The context to capture
    /// * `resumption_hint` - Hint for resuming from this checkpoint
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if the session doesn't exist or storage fails.
    pub async fn create(
        &self,
        session_id: &str,
        name: &str,
        description: Option<&str>,
        context: CheckpointContext,
        resumption_hint: &str,
    ) -> Result<CreateResponse, ModeError> {
        // Verify session exists
        let session = self
            .storage
            .get_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get session: {e}"),
            })?
            .ok_or_else(|| ModeError::MissingField {
                field: "session_id".to_string(),
            })?;

        // Serialize state
        let state = serde_json::json!({
            "context": context,
            "resumption_hint": resumption_hint,
        });
        let state_str = serde_json::to_string(&state).map_err(|e| ModeError::InvalidValue {
            field: "context".to_string(),
            reason: format!("Failed to serialize: {e}"),
        })?;

        // Create checkpoint
        let checkpoint_id = generate_checkpoint_id();
        let mut checkpoint = StoredCheckpoint::new(&checkpoint_id, &session.id, name, state_str);

        if let Some(desc) = description {
            checkpoint = checkpoint.with_description(desc);
        }

        self.storage
            .save_checkpoint(&checkpoint)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save checkpoint: {e}"),
            })?;

        Ok(CreateResponse::new(
            checkpoint_id,
            session.id,
            name,
            resumption_hint,
        ))
    }

    /// List all checkpoints for a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to list checkpoints for
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if the session doesn't exist or storage fails.
    pub async fn list(&self, session_id: &str) -> Result<ListResponse, ModeError> {
        // Verify session exists
        self.storage
            .get_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get session: {e}"),
            })?
            .ok_or_else(|| ModeError::MissingField {
                field: "session_id".to_string(),
            })?;

        // Get checkpoints
        let stored_checkpoints = self
            .storage
            .get_checkpoints(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get checkpoints: {e}"),
            })?;

        // Get thought counts by parsing state
        let summaries = stored_checkpoints
            .into_iter()
            .map(|cp| {
                let thought_count = serde_json::from_str::<serde_json::Value>(&cp.state)
                    .ok()
                    .and_then(|v| v.get("thought_count").and_then(serde_json::Value::as_u64))
                    .map_or(0, |n| n as usize);

                let mut summary = CheckpointSummary::new(
                    &cp.id,
                    &cp.name,
                    cp.created_at.to_rfc3339(),
                    thought_count,
                );

                if let Some(desc) = cp.description {
                    summary = summary.with_description(desc);
                }

                summary
            })
            .collect();

        Ok(ListResponse::new(session_id, summaries))
    }

    /// Restore to a checkpoint.
    ///
    /// Returns to a saved checkpoint state, optionally exploring a new direction.
    ///
    /// # Arguments
    ///
    /// * `checkpoint_id` - The checkpoint to restore
    /// * `new_direction` - Optional new direction to explore from this point
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if the checkpoint doesn't exist or storage fails.
    pub async fn restore(
        &self,
        checkpoint_id: &str,
        new_direction: Option<&str>,
    ) -> Result<RestoreResponse, ModeError> {
        // Get checkpoint
        let checkpoint = self
            .storage
            .get_checkpoint(checkpoint_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get checkpoint: {e}"),
            })?
            .ok_or_else(|| ModeError::MissingField {
                field: "checkpoint_id".to_string(),
            })?;

        // Parse state
        let state: serde_json::Value =
            serde_json::from_str(&checkpoint.state).map_err(|e| ModeError::InvalidValue {
                field: "state".to_string(),
                reason: format!("Failed to parse state: {e}"),
            })?;

        // Extract context
        let context = Self::parse_context(&state)?;

        // Get thought count
        let thought_count = self
            .storage
            .get_thoughts(&checkpoint.session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get thoughts: {e}"),
            })?
            .len();

        let restored_state = RestoredState::new(context, thought_count);

        let mut response =
            RestoreResponse::new(checkpoint_id, &checkpoint.session_id, restored_state);

        if let Some(direction) = new_direction {
            response = response.with_new_direction(direction);
        }

        Ok(response)
    }

    /// Parse context from state JSON.
    fn parse_context(state: &serde_json::Value) -> Result<CheckpointContext, ModeError> {
        let context_json = state
            .get("context")
            .ok_or_else(|| ModeError::MissingField {
                field: "context".to_string(),
            })?;

        let key_findings = context_json
            .get("key_findings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let current_focus = context_json
            .get("current_focus")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let open_questions = context_json
            .get("open_questions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(CheckpointContext::new(
            key_findings,
            current_focus,
            open_questions,
        ))
    }
}

impl<S, C> std::fmt::Debug for CheckpointMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckpointMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{MockAnthropicClientTrait, MockStorageTrait, Session};

    // CheckpointContext tests
    #[test]
    fn test_checkpoint_context_new() {
        let context = CheckpointContext::new(
            vec!["Finding 1".to_string()],
            "Current topic",
            vec!["Question 1".to_string()],
        );
        assert_eq!(context.key_findings.len(), 1);
        assert_eq!(context.current_focus, "Current topic");
        assert_eq!(context.open_questions.len(), 1);
    }

    #[test]
    fn test_checkpoint_context_serialize() {
        let context = CheckpointContext::new(vec![], "Focus", vec![]);
        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("\"current_focus\":\"Focus\""));
    }

    // CheckpointSummary tests
    #[test]
    fn test_checkpoint_summary_new() {
        let summary = CheckpointSummary::new("cp-1", "Test", "2024-01-01T00:00:00Z", 5);
        assert_eq!(summary.id, "cp-1");
        assert_eq!(summary.name, "Test");
        assert_eq!(summary.thought_count, 5);
        assert!(summary.description.is_none());
    }

    #[test]
    fn test_checkpoint_summary_with_description() {
        let summary = CheckpointSummary::new("cp-1", "Test", "2024-01-01T00:00:00Z", 5)
            .with_description("Description");
        assert_eq!(summary.description, Some("Description".to_string()));
    }

    // CreateResponse tests
    #[test]
    fn test_create_response_new() {
        let response = CreateResponse::new("cp-1", "sess-1", "Test", "Continue with...");
        assert_eq!(response.checkpoint_id, "cp-1");
        assert_eq!(response.session_id, "sess-1");
        assert_eq!(response.name, "Test");
        assert_eq!(response.resumption_hint, "Continue with...");
    }

    // ListResponse tests
    #[test]
    fn test_list_response_new() {
        let response = ListResponse::new("sess-1", vec![]);
        assert_eq!(response.session_id, "sess-1");
        assert!(response.checkpoints.is_empty());
    }

    // RestoredState tests
    #[test]
    fn test_restored_state_new() {
        let context = CheckpointContext::new(vec![], "Focus", vec![]);
        let state = RestoredState::new(context, 10);
        assert_eq!(state.thought_count, 10);
    }

    // RestoreResponse tests
    #[test]
    fn test_restore_response_new() {
        let context = CheckpointContext::new(vec![], "Focus", vec![]);
        let state = RestoredState::new(context, 5);
        let response = RestoreResponse::new("cp-1", "sess-1", state);
        assert_eq!(response.checkpoint_id, "cp-1");
        assert!(response.new_direction.is_none());
    }

    #[test]
    fn test_restore_response_with_new_direction() {
        let context = CheckpointContext::new(vec![], "Focus", vec![]);
        let state = RestoredState::new(context, 5);
        let response =
            RestoreResponse::new("cp-1", "sess-1", state).with_new_direction("Explore alternative");
        assert_eq!(
            response.new_direction,
            Some("Explore alternative".to_string())
        );
    }

    // CheckpointMode create tests
    #[tokio::test]
    async fn test_checkpoint_create_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage.expect_save_checkpoint().returning(|_| Ok(()));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let context = CheckpointContext::new(
            vec!["Finding 1".to_string()],
            "Topic A",
            vec!["Question 1".to_string()],
        );

        let result = mode
            .create(
                "sess-1",
                "Checkpoint 1",
                None,
                context,
                "Continue with Topic A",
            )
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "sess-1");
        assert_eq!(response.name, "Checkpoint 1");
        assert_eq!(response.resumption_hint, "Continue with Topic A");
    }

    #[tokio::test]
    async fn test_checkpoint_create_with_description() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage.expect_save_checkpoint().returning(|_| Ok(()));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let context = CheckpointContext::new(vec![], "Focus", vec![]);

        let result = mode
            .create(
                "sess-1",
                "Checkpoint 1",
                Some("A description"),
                context,
                "Hint",
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_checkpoint_create_session_not_found() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_session().returning(|_| Ok(None));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let context = CheckpointContext::new(vec![], "Focus", vec![]);

        let result = mode
            .create("nonexistent", "Test", None, context, "Hint")
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "session_id"
        ));
    }

    // CheckpointMode list tests
    #[tokio::test]
    async fn test_checkpoint_list_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage
            .expect_get_checkpoints()
            .returning(|session_id| {
                Ok(vec![
                    StoredCheckpoint::new("cp-1", session_id, "First", "{}"),
                    StoredCheckpoint::new("cp-2", session_id, "Second", "{}"),
                ])
            });

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let result = mode.list("sess-1").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "sess-1");
        assert_eq!(response.checkpoints.len(), 2);
        assert_eq!(response.checkpoints[0].name, "First");
        assert_eq!(response.checkpoints[1].name, "Second");
    }

    #[tokio::test]
    async fn test_checkpoint_list_empty() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage
            .expect_get_checkpoints()
            .returning(|_| Ok(vec![]));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let result = mode.list("sess-1").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.checkpoints.is_empty());
    }

    #[tokio::test]
    async fn test_checkpoint_list_session_not_found() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_session().returning(|_| Ok(None));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let result = mode.list("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "session_id"
        ));
    }

    // CheckpointMode restore tests
    #[tokio::test]
    async fn test_checkpoint_restore_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let state = serde_json::json!({
            "context": {
                "key_findings": ["Finding 1"],
                "current_focus": "Topic A",
                "open_questions": ["Question 1"]
            },
            "resumption_hint": "Continue"
        });

        mock_storage.expect_get_checkpoint().returning(move |id| {
            Ok(Some(StoredCheckpoint::new(
                id,
                "sess-1",
                "Checkpoint 1",
                serde_json::to_string(&state).unwrap(),
            )))
        });
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let result = mode.restore("cp-1", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.checkpoint_id, "cp-1");
        assert_eq!(response.session_id, "sess-1");
        assert_eq!(response.restored_state.context.current_focus, "Topic A");
        assert!(response.new_direction.is_none());
    }

    #[tokio::test]
    async fn test_checkpoint_restore_with_new_direction() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let state = serde_json::json!({
            "context": {
                "key_findings": [],
                "current_focus": "Topic A",
                "open_questions": []
            }
        });

        mock_storage.expect_get_checkpoint().returning(move |id| {
            Ok(Some(StoredCheckpoint::new(
                id,
                "sess-1",
                "Checkpoint 1",
                serde_json::to_string(&state).unwrap(),
            )))
        });
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let result = mode.restore("cp-1", Some("Try alternative")).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.new_direction, Some("Try alternative".to_string()));
    }

    #[tokio::test]
    async fn test_checkpoint_restore_not_found() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_checkpoint().returning(|_| Ok(None));

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let result = mode.restore("nonexistent", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "checkpoint_id"
        ));
    }

    #[tokio::test]
    async fn test_checkpoint_restore_invalid_state() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_checkpoint().returning(|id| {
            Ok(Some(StoredCheckpoint::new(
                id,
                "sess-1",
                "Test",
                "invalid json",
            )))
        });

        let mode = CheckpointMode::new(mock_storage, mock_client);
        let result = mode.restore("cp-1", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::InvalidValue { field, .. }) if field == "state"
        ));
    }

    #[test]
    fn test_checkpoint_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = CheckpointMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("CheckpointMode"));
    }
}
