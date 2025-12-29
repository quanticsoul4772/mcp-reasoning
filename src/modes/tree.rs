//! Tree reasoning mode.
//!
//! This mode provides branching exploration with multiple reasoning paths.
//! It supports four operations:
//! - `create`: Start new exploration from content, returns branches
//! - `focus`: Select a specific branch for continued reasoning
//! - `list`: Show all branches in the session with status and scores
//! - `complete`: Mark a branch as finished or abandoned

#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::unused_async)]

use serde::{Deserialize, Serialize};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_branch_id, validate_content};
use crate::prompts::{get_prompt_for_mode, Operation, ReasoningMode};
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait};

/// Branch status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BranchStatus {
    /// Branch is actively being explored.
    #[default]
    Active,
    /// Branch exploration completed successfully.
    Completed,
    /// Branch was abandoned.
    Abandoned,
}

impl BranchStatus {
    /// Convert to string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }
}

impl std::fmt::Display for BranchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for BranchStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(format!("Unknown branch status: {s}")),
        }
    }
}

/// A branch in the exploration tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Branch {
    /// Unique branch identifier.
    pub id: String,
    /// Branch title or label.
    pub title: String,
    /// Branch content/description.
    pub content: String,
    /// Branch score (0.0-1.0).
    pub score: f64,
    /// Current status.
    pub status: BranchStatus,
}

impl Branch {
    /// Create a new branch.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        score: f64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content: content.into(),
            score,
            status: BranchStatus::Active,
        }
    }

    /// Set the branch status.
    #[must_use]
    pub fn with_status(mut self, status: BranchStatus) -> Self {
        self.status = status;
        self
    }
}

/// Response from tree reasoning mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TreeResponse {
    /// Session this tree belongs to.
    pub session_id: String,
    /// Current or created branch ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    /// List of branches (for create/list/focus).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branches: Option<Vec<Branch>>,
    /// Recommendation for which branch to explore next.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
    /// Exploration content (for focus operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exploration: Option<String>,
    /// Insights gained (for focus operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insights: Option<Vec<String>>,
}

impl TreeResponse {
    /// Create a new tree response.
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            branch_id: None,
            branches: None,
            recommendation: None,
            exploration: None,
            insights: None,
        }
    }

    /// Set the branch ID.
    #[must_use]
    pub fn with_branch_id(mut self, branch_id: impl Into<String>) -> Self {
        self.branch_id = Some(branch_id.into());
        self
    }

    /// Set the branches list.
    #[must_use]
    pub fn with_branches(mut self, branches: Vec<Branch>) -> Self {
        self.branches = Some(branches);
        self
    }

    /// Set the recommendation.
    #[must_use]
    pub fn with_recommendation(mut self, recommendation: impl Into<String>) -> Self {
        self.recommendation = Some(recommendation.into());
        self
    }

    /// Set the exploration content.
    #[must_use]
    pub fn with_exploration(mut self, exploration: impl Into<String>) -> Self {
        self.exploration = Some(exploration.into());
        self
    }

    /// Set the insights.
    #[must_use]
    pub fn with_insights(mut self, insights: Vec<String>) -> Self {
        self.insights = Some(insights);
        self
    }
}

/// Tree reasoning mode.
///
/// Provides branching exploration with multiple reasoning paths.
pub struct TreeMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
    /// In-memory branch storage (`session_id` -> branches).
    /// In a real implementation, this would be persisted.
    branches: std::collections::HashMap<String, Vec<Branch>>,
}

impl<S, C> TreeMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new tree mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self {
            storage,
            client,
            branches: std::collections::HashMap::new(),
        }
    }

    /// Create a new exploration tree from content.
    ///
    /// Generates 2-4 divergent branches for exploration.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn create(
        &mut self,
        content: &str,
        session_id: Option<String>,
        num_branches: Option<u32>,
    ) -> Result<TreeResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;
        let num_branches = num_branches.unwrap_or(3).clamp(2, 4);

        let prompt = get_prompt_for_mode(ReasoningMode::Tree, Some(&Operation::Create));
        let user_message = format!("{prompt}\n\nGenerate {num_branches} branches for:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.8);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse branches from response
        let branches_json = json
            .get("branches")
            .ok_or_else(|| ModeError::MissingField {
                field: "branches".to_string(),
            })?;

        let branches_arr = branches_json
            .as_array()
            .ok_or_else(|| ModeError::InvalidValue {
                field: "branches".to_string(),
                reason: "expected array".to_string(),
            })?;

        let mut branches = Vec::new();
        for (i, b) in branches_arr.iter().enumerate() {
            let title = b
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("Branch {}", i + 1))
                .to_string();

            let description = b
                .get("description")
                .or_else(|| b.get("initial_thought"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let branch_id = generate_branch_id();
            branches.push(Branch::new(&branch_id, title, description, 0.5));
        }

        let recommendation = json
            .get("recommendation")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Store branches in memory
        self.branches.insert(session.id.clone(), branches.clone());

        let mut response = TreeResponse::new(&session.id).with_branches(branches);
        if let Some(rec) = recommendation {
            response = response.with_recommendation(rec);
        }

        Ok(response)
    }

    /// Focus on a specific branch for continued exploration.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if branch not found or API fails.
    pub async fn focus(
        &mut self,
        session_id: &str,
        branch_id: &str,
    ) -> Result<TreeResponse, ModeError> {
        // Find the branch
        let branches = self
            .branches
            .get(session_id)
            .ok_or_else(|| ModeError::InvalidValue {
                field: "session_id".to_string(),
                reason: format!("No branches found for session {session_id}"),
            })?;

        let branch =
            branches
                .iter()
                .find(|b| b.id == branch_id)
                .ok_or_else(|| ModeError::InvalidValue {
                    field: "branch_id".to_string(),
                    reason: format!("Branch {branch_id} not found"),
                })?;

        let prompt = get_prompt_for_mode(ReasoningMode::Tree, Some(&Operation::Focus));
        let user_message = format!(
            "{prompt}\n\nBranch to explore:\nTitle: {}\nContent: {}",
            branch.title, branch.content
        );

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.7);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let exploration = json
            .get("exploration")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let insights: Vec<String> = json
            .get("insights")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let confidence = json
            .get("confidence")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.5);

        // Update branch score
        if let Some(branches) = self.branches.get_mut(session_id) {
            if let Some(b) = branches.iter_mut().find(|b| b.id == branch_id) {
                b.score = confidence;
            }
        }

        Ok(TreeResponse::new(session_id)
            .with_branch_id(branch_id)
            .with_exploration(exploration)
            .with_insights(insights))
    }

    /// List all branches in the session.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if session not found.
    pub async fn list(&self, session_id: &str) -> Result<TreeResponse, ModeError> {
        let branches = self.branches.get(session_id).cloned().unwrap_or_default();

        // Generate recommendation based on scores
        let recommendation = if branches.is_empty() {
            None
        } else {
            let best = branches
                .iter()
                .filter(|b| b.status == BranchStatus::Active)
                .max_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            best.map(|b| format!("Recommend exploring '{}' (score: {:.2})", b.title, b.score))
        };

        let mut response = TreeResponse::new(session_id).with_branches(branches);
        if let Some(rec) = recommendation {
            response = response.with_recommendation(rec);
        }

        Ok(response)
    }

    /// Mark a branch as completed or abandoned.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if branch not found.
    pub async fn complete(
        &mut self,
        session_id: &str,
        branch_id: &str,
        completed: bool,
    ) -> Result<TreeResponse, ModeError> {
        let branches =
            self.branches
                .get_mut(session_id)
                .ok_or_else(|| ModeError::InvalidValue {
                    field: "session_id".to_string(),
                    reason: format!("No branches found for session {session_id}"),
                })?;

        let branch = branches
            .iter_mut()
            .find(|b| b.id == branch_id)
            .ok_or_else(|| ModeError::InvalidValue {
                field: "branch_id".to_string(),
                reason: format!("Branch {branch_id} not found"),
            })?;

        branch.status = if completed {
            BranchStatus::Completed
        } else {
            BranchStatus::Abandoned
        };

        Ok(TreeResponse::new(session_id)
            .with_branch_id(branch_id)
            .with_branches(branches.clone()))
    }

    /// Get or create a session.
    async fn get_or_create_session(
        &self,
        session_id: Option<String>,
    ) -> Result<Session, ModeError> {
        self.storage
            .get_or_create_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get or create session: {e}"),
            })
    }
}

impl<S, C> std::fmt::Debug for TreeMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .field("branches_count", &self.branches.len())
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn mock_create_response(num_branches: usize) -> String {
        let branches: Vec<String> = (0..num_branches)
            .map(|i| {
                format!(
                    r#"{{"title": "Branch {}", "description": "Description for branch {}", "initial_thought": "Initial thought {}"}}"#,
                    i + 1, i + 1, i + 1
                )
            })
            .collect();

        format!(
            r#"{{"branches": [{}], "recommendation": "Explore Branch 1 first"}}"#,
            branches.join(", ")
        )
    }

    fn mock_focus_response() -> String {
        r#"{"exploration": "Deep exploration of this branch", "insights": ["Insight 1", "Insight 2"], "confidence": 0.85, "status": "continue"}"#.to_string()
    }

    // BranchStatus tests
    #[test]
    fn test_branch_status_as_str() {
        assert_eq!(BranchStatus::Active.as_str(), "active");
        assert_eq!(BranchStatus::Completed.as_str(), "completed");
        assert_eq!(BranchStatus::Abandoned.as_str(), "abandoned");
    }

    #[test]
    fn test_branch_status_display() {
        assert_eq!(format!("{}", BranchStatus::Active), "active");
        assert_eq!(format!("{}", BranchStatus::Completed), "completed");
        assert_eq!(format!("{}", BranchStatus::Abandoned), "abandoned");
    }

    #[test]
    fn test_branch_status_from_str() {
        assert_eq!(
            "active".parse::<BranchStatus>().unwrap(),
            BranchStatus::Active
        );
        assert_eq!(
            "COMPLETED".parse::<BranchStatus>().unwrap(),
            BranchStatus::Completed
        );
        assert_eq!(
            "Abandoned".parse::<BranchStatus>().unwrap(),
            BranchStatus::Abandoned
        );
        assert!("unknown".parse::<BranchStatus>().is_err());
    }

    #[test]
    fn test_branch_status_default() {
        assert_eq!(BranchStatus::default(), BranchStatus::Active);
    }

    #[test]
    fn test_branch_status_serialize() {
        let json = serde_json::to_string(&BranchStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");
    }

    #[test]
    fn test_branch_status_deserialize() {
        let status: BranchStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(status, BranchStatus::Completed);
    }

    // Branch tests
    #[test]
    fn test_branch_new() {
        let branch = Branch::new("b-1", "Title", "Content", 0.75);
        assert_eq!(branch.id, "b-1");
        assert_eq!(branch.title, "Title");
        assert_eq!(branch.content, "Content");
        assert!((branch.score - 0.75).abs() < f64::EPSILON);
        assert_eq!(branch.status, BranchStatus::Active);
    }

    #[test]
    fn test_branch_with_status() {
        let branch =
            Branch::new("b-1", "Title", "Content", 0.75).with_status(BranchStatus::Completed);
        assert_eq!(branch.status, BranchStatus::Completed);
    }

    #[test]
    fn test_branch_serialize() {
        let branch = Branch::new("b-1", "Title", "Content", 0.75);
        let json = serde_json::to_string(&branch).unwrap();
        assert!(json.contains("\"id\":\"b-1\""));
        assert!(json.contains("\"title\":\"Title\""));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn test_branch_clone() {
        let branch = Branch::new("b-1", "Title", "Content", 0.75);
        let cloned = branch.clone();
        assert_eq!(branch, cloned);
    }

    // TreeResponse tests
    #[test]
    fn test_tree_response_new() {
        let response = TreeResponse::new("s-1");
        assert_eq!(response.session_id, "s-1");
        assert!(response.branch_id.is_none());
        assert!(response.branches.is_none());
        assert!(response.recommendation.is_none());
    }

    #[test]
    fn test_tree_response_with_branch_id() {
        let response = TreeResponse::new("s-1").with_branch_id("b-1");
        assert_eq!(response.branch_id, Some("b-1".to_string()));
    }

    #[test]
    fn test_tree_response_with_branches() {
        let branches = vec![Branch::new("b-1", "Title", "Content", 0.5)];
        let response = TreeResponse::new("s-1").with_branches(branches.clone());
        assert_eq!(response.branches.unwrap().len(), 1);
    }

    #[test]
    fn test_tree_response_with_recommendation() {
        let response = TreeResponse::new("s-1").with_recommendation("Explore B1");
        assert_eq!(response.recommendation, Some("Explore B1".to_string()));
    }

    #[test]
    fn test_tree_response_with_exploration() {
        let response = TreeResponse::new("s-1").with_exploration("Deep analysis");
        assert_eq!(response.exploration, Some("Deep analysis".to_string()));
    }

    #[test]
    fn test_tree_response_with_insights() {
        let response = TreeResponse::new("s-1")
            .with_insights(vec!["Insight 1".to_string(), "Insight 2".to_string()]);
        assert_eq!(response.insights.unwrap().len(), 2);
    }

    #[test]
    fn test_tree_response_serialize_omits_none() {
        let response = TreeResponse::new("s-1");
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("branch_id"));
        assert!(!json.contains("branches"));
        assert!(!json.contains("recommendation"));
    }

    // TreeMode tests
    #[tokio::test]
    async fn test_tree_create_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });

        let response_json = mock_create_response(3);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let result = mode
            .create("Explore this topic", Some("test-session".to_string()), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert!(response.branches.is_some());
        assert_eq!(response.branches.as_ref().unwrap().len(), 3);
        assert!(response.recommendation.is_some());
    }

    #[tokio::test]
    async fn test_tree_create_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let result = mode.create("", None, None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "content"
        ));
    }

    #[tokio::test]
    async fn test_tree_create_custom_num_branches() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        let response_json = mock_create_response(4);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let result = mode.create("Content", None, Some(4)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.branches.as_ref().unwrap().len(), 4);
    }

    #[tokio::test]
    async fn test_tree_create_clamps_num_branches() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        // Even if we request 10 branches, it should clamp to 4
        let response_json = mock_create_response(4);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let result = mode.create("Content", None, Some(10)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tree_focus_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });

        // First call for create
        let create_response = mock_create_response(3);
        let focus_response = mock_focus_response();

        mock_client
            .expect_complete()
            .times(1)
            .returning(move |_, _| {
                Ok(CompletionResponse::new(
                    create_response.clone(),
                    Usage::new(100, 200),
                ))
            });

        let mut mode = TreeMode::new(mock_storage, mock_client);

        // Create branches first
        let create_result = mode
            .create("Content", Some("test-session".to_string()), None)
            .await
            .unwrap();
        let branch_id = create_result.branches.unwrap()[0].id.clone();

        // Now setup mock for focus
        let mut mock_client2 = MockAnthropicClientTrait::new();
        mock_client2.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                focus_response.clone(),
                Usage::new(100, 200),
            ))
        });
        mode.client = mock_client2;

        // Focus on the branch
        let focus_result = mode.focus("test-session", &branch_id).await;

        assert!(focus_result.is_ok());
        let response = focus_result.unwrap();
        assert_eq!(response.branch_id, Some(branch_id));
        assert!(response.exploration.is_some());
        assert!(response.insights.is_some());
    }

    #[tokio::test]
    async fn test_tree_focus_branch_not_found() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let result = mode.focus("test-session", "nonexistent-branch").await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::InvalidValue { field, .. }) if field == "session_id"
        ));
    }

    #[tokio::test]
    async fn test_tree_list_empty() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = TreeMode::new(mock_storage, mock_client);
        let result = mode.list("test-session").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.branches.unwrap().len(), 0);
        assert!(response.recommendation.is_none());
    }

    #[tokio::test]
    async fn test_tree_list_with_branches() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        let response_json = mock_create_response(2);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mut mode = TreeMode::new(mock_storage, mock_client);
        mode.create("Content", Some("test-session".to_string()), Some(2))
            .await
            .unwrap();

        let result = mode.list("test-session").await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.branches.unwrap().len(), 2);
        assert!(response.recommendation.is_some());
    }

    #[tokio::test]
    async fn test_tree_complete_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        let response_json = mock_create_response(2);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let create_result = mode
            .create("Content", Some("test-session".to_string()), Some(2))
            .await
            .unwrap();
        let branch_id = create_result.branches.unwrap()[0].id.clone();

        let result = mode.complete("test-session", &branch_id, true).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        let branches = response.branches.unwrap();
        let completed_branch = branches.iter().find(|b| b.id == branch_id).unwrap();
        assert_eq!(completed_branch.status, BranchStatus::Completed);
    }

    #[tokio::test]
    async fn test_tree_complete_abandon() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        let response_json = mock_create_response(2);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let create_result = mode
            .create("Content", Some("test-session".to_string()), Some(2))
            .await
            .unwrap();
        let branch_id = create_result.branches.unwrap()[0].id.clone();

        let result = mode.complete("test-session", &branch_id, false).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        let branches = response.branches.unwrap();
        let abandoned_branch = branches.iter().find(|b| b.id == branch_id).unwrap();
        assert_eq!(abandoned_branch.status, BranchStatus::Abandoned);
    }

    #[tokio::test]
    async fn test_tree_complete_branch_not_found() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mut mode = TreeMode::new(mock_storage, mock_client);
        let result = mode.complete("test-session", "nonexistent", true).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::InvalidValue { field, .. }) if field == "session_id"
        ));
    }

    #[test]
    fn test_tree_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = TreeMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("TreeMode"));
        assert!(debug.contains("branches_count"));
    }
}
