//! Auto reasoning mode.
//!
//! This mode analyzes content and selects the optimal reasoning mode.
//! It acts as a router to other modes based on content characteristics.

#![allow(clippy::missing_const_for_fn)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{get_prompt_for_mode, ReasoningMode};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

/// Response from auto mode selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoResponse {
    /// Unique thought identifier.
    pub thought_id: String,
    /// Session ID.
    pub session_id: String,
    /// The selected reasoning mode.
    pub selected_mode: ReasoningMode,
    /// Reasoning for the selection.
    pub reasoning: String,
    /// Content characteristics that influenced selection.
    pub characteristics: Vec<String>,
    /// Suggested parameters for the selected mode.
    pub suggested_parameters: HashMap<String, serde_json::Value>,
    /// Alternative mode recommendation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative_mode: Option<AlternativeMode>,
}

impl AutoResponse {
    /// Create a new auto response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        selected_mode: ReasoningMode,
        reasoning: impl Into<String>,
        characteristics: Vec<String>,
        suggested_parameters: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            selected_mode,
            reasoning: reasoning.into(),
            characteristics,
            suggested_parameters,
            alternative_mode: None,
        }
    }

    /// Add an alternative mode.
    #[must_use]
    pub fn with_alternative(mut self, alternative: AlternativeMode) -> Self {
        self.alternative_mode = Some(alternative);
        self
    }
}

/// Alternative mode recommendation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlternativeMode {
    /// The alternative mode.
    pub mode: ReasoningMode,
    /// Why this is a second-best choice.
    pub reason: String,
}

impl AlternativeMode {
    /// Create a new alternative mode.
    #[must_use]
    pub fn new(mode: ReasoningMode, reason: impl Into<String>) -> Self {
        Self {
            mode,
            reason: reason.into(),
        }
    }
}

/// Auto reasoning mode.
///
/// Analyzes content and selects the optimal reasoning mode.
pub struct AutoMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> AutoMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new auto mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Analyze content and select the optimal reasoning mode.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to analyze
    /// * `session_id` - Optional session ID for context continuity
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn select(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<AutoResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;
        let prompt = get_prompt_for_mode(ReasoningMode::Auto, None);

        let user_message = format!("{prompt}\n\nAnalyze this content:\n{content}");
        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.5);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse selected mode
        let selected_mode = Self::parse_mode(&json, "selected_mode")?;

        // Parse reasoning
        let reasoning = json
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        // Parse characteristics
        let characteristics =
            Self::parse_string_array(&json, "characteristics").unwrap_or_default();

        // Parse suggested parameters
        let suggested_parameters = json
            .get("suggested_parameters")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        // Parse alternative mode
        let alternative_mode = Self::parse_alternative(&json);

        // Generate thought ID and save
        let thought_id = generate_thought_id();
        let thought = Thought::new(&thought_id, &session.id, content, "auto", 0.9);
        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        let mut response = AutoResponse::new(
            thought_id,
            session.id,
            selected_mode,
            reasoning,
            characteristics,
            suggested_parameters,
        );

        if let Some(alt) = alternative_mode {
            response = response.with_alternative(alt);
        }

        Ok(response)
    }

    /// Parse a reasoning mode from JSON.
    fn parse_mode(json: &serde_json::Value, key: &str) -> Result<ReasoningMode, ModeError> {
        let mode_str =
            json.get(key)
                .and_then(|v| v.as_str())
                .ok_or_else(|| ModeError::MissingField {
                    field: key.to_string(),
                })?;

        mode_str
            .parse::<ReasoningMode>()
            .map_err(|_| ModeError::InvalidValue {
                field: key.to_string(),
                reason: format!("Unknown mode: {mode_str}"),
            })
    }

    /// Parse an array of strings from JSON.
    fn parse_string_array(json: &serde_json::Value, key: &str) -> Option<Vec<String>> {
        json.get(key).and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect()
            })
        })
    }

    /// Parse alternative mode from JSON.
    fn parse_alternative(json: &serde_json::Value) -> Option<AlternativeMode> {
        let alt_str = json.get("alternative_mode").and_then(|v| v.as_str())?;

        // Parse format: "mode_name: reason" or just "mode_name"
        let (mode_str, reason) = alt_str.find(':').map_or_else(
            || (alt_str.trim(), String::new()),
            |pos| {
                let (mode, rest) = alt_str.split_at(pos);
                (mode.trim(), rest[1..].trim().to_string())
            },
        );

        let mode = mode_str.parse::<ReasoningMode>().ok()?;
        Some(AlternativeMode::new(mode, reason))
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

impl<S, C> std::fmt::Debug for AutoMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn mock_auto_response() -> String {
        r#"{
            "selected_mode": "linear",
            "reasoning": "The content requires step-by-step analysis",
            "characteristics": ["Sequential problem", "Clear steps", "Single path"],
            "suggested_parameters": {
                "min_confidence": 0.7,
                "max_steps": 5
            },
            "alternative_mode": "tree: Could also explore multiple approaches"
        }"#
        .to_string()
    }

    // AlternativeMode tests
    #[test]
    fn test_alternative_mode_new() {
        let alt = AlternativeMode::new(ReasoningMode::Tree, "Good for exploration");
        assert_eq!(alt.mode, ReasoningMode::Tree);
        assert_eq!(alt.reason, "Good for exploration");
    }

    #[test]
    fn test_alternative_mode_serialize() {
        let alt = AlternativeMode::new(ReasoningMode::Linear, "Sequential");
        let json = serde_json::to_string(&alt).unwrap();
        assert!(json.contains("\"mode\":\"linear\""));
        assert!(json.contains("\"reason\":\"Sequential\""));
    }

    // AutoResponse tests
    #[test]
    fn test_auto_response_new() {
        let response = AutoResponse::new(
            "t-1",
            "s-1",
            ReasoningMode::Linear,
            "Step-by-step needed",
            vec!["Characteristic".to_string()],
            HashMap::new(),
        );
        assert_eq!(response.thought_id, "t-1");
        assert_eq!(response.selected_mode, ReasoningMode::Linear);
        assert!(response.alternative_mode.is_none());
    }

    #[test]
    fn test_auto_response_with_alternative() {
        let alt = AlternativeMode::new(ReasoningMode::Divergent, "For multiple views");
        let response = AutoResponse::new(
            "t-1",
            "s-1",
            ReasoningMode::Linear,
            "Reason",
            vec![],
            HashMap::new(),
        )
        .with_alternative(alt);

        assert!(response.alternative_mode.is_some());
        assert_eq!(
            response.alternative_mode.unwrap().mode,
            ReasoningMode::Divergent
        );
    }

    #[test]
    fn test_auto_response_with_parameters() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), serde_json::json!("value"));

        let response =
            AutoResponse::new("t-1", "s-1", ReasoningMode::Tree, "Reason", vec![], params);

        assert_eq!(
            response.suggested_parameters.get("key"),
            Some(&serde_json::json!("value"))
        );
    }

    // AutoMode select tests
    #[tokio::test]
    async fn test_auto_select_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_auto_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = AutoMode::new(mock_storage, mock_client);
        let result = mode.select("Analyze this step by step", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.selected_mode, ReasoningMode::Linear);
        assert!(!response.reasoning.is_empty());
        assert_eq!(response.characteristics.len(), 3);
        assert!(response.alternative_mode.is_some());
    }

    #[tokio::test]
    async fn test_auto_select_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = AutoMode::new(mock_storage, mock_client);
        let result = mode.select("", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "content"
        ));
    }

    #[tokio::test]
    async fn test_auto_select_missing_mode() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"reasoning": "Some reasoning"}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = AutoMode::new(mock_storage, mock_client);
        let result = mode.select("Content", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "selected_mode"
        ));
    }

    #[tokio::test]
    async fn test_auto_select_invalid_mode() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"selected_mode": "invalid_mode", "reasoning": "Test"}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = AutoMode::new(mock_storage, mock_client);
        let result = mode.select("Content", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::InvalidValue { field, .. }) if field == "selected_mode"
        ));
    }

    #[tokio::test]
    async fn test_auto_select_api_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiUnavailable {
                message: "API error".to_string(),
            })
        });

        let mode = AutoMode::new(mock_storage, mock_client);
        let result = mode.select("Content", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_auto_select_all_modes() {
        // Test that all modes can be selected
        let modes = [
            ("linear", ReasoningMode::Linear),
            ("tree", ReasoningMode::Tree),
            ("divergent", ReasoningMode::Divergent),
            ("reflection", ReasoningMode::Reflection),
            ("checkpoint", ReasoningMode::Checkpoint),
            ("graph", ReasoningMode::Graph),
            ("detect", ReasoningMode::Detect),
            ("decision", ReasoningMode::Decision),
            ("evidence", ReasoningMode::Evidence),
            ("timeline", ReasoningMode::Timeline),
            ("mcts", ReasoningMode::Mcts),
            ("counterfactual", ReasoningMode::Counterfactual),
        ];

        for (mode_str, expected_mode) in modes {
            let mut mock_storage = MockStorageTrait::new();
            let mut mock_client = MockAnthropicClientTrait::new();

            mock_storage
                .expect_get_or_create_session()
                .returning(|_| Ok(Session::new("test-session")));
            mock_storage.expect_save_thought().returning(|_| Ok(()));

            let response_json =
                format!(r#"{{"selected_mode": "{mode_str}", "reasoning": "Test"}}"#);
            mock_client.expect_complete().returning(move |_, _| {
                Ok(CompletionResponse::new(
                    response_json.clone(),
                    Usage::new(50, 100),
                ))
            });

            let mode = AutoMode::new(mock_storage, mock_client);
            let result = mode.select("Content", None).await;

            assert!(result.is_ok(), "Failed for mode: {mode_str}");
            assert_eq!(result.unwrap().selected_mode, expected_mode);
        }
    }

    #[tokio::test]
    async fn test_auto_select_no_alternative() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"selected_mode": "linear", "reasoning": "Test"}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = AutoMode::new(mock_storage, mock_client);
        let result = mode.select("Content", None).await;

        assert!(result.is_ok());
        assert!(result.unwrap().alternative_mode.is_none());
    }

    #[test]
    fn test_parse_alternative_with_reason() {
        let json = serde_json::json!({
            "alternative_mode": "tree: Good for exploration"
        });
        let alt = AutoMode::<MockStorageTrait, MockAnthropicClientTrait>::parse_alternative(&json);
        assert!(alt.is_some());
        let alt = alt.unwrap();
        assert_eq!(alt.mode, ReasoningMode::Tree);
        assert_eq!(alt.reason, "Good for exploration");
    }

    #[test]
    fn test_parse_alternative_without_reason() {
        let json = serde_json::json!({
            "alternative_mode": "divergent"
        });
        let alt = AutoMode::<MockStorageTrait, MockAnthropicClientTrait>::parse_alternative(&json);
        assert!(alt.is_some());
        let alt = alt.unwrap();
        assert_eq!(alt.mode, ReasoningMode::Divergent);
        assert!(alt.reason.is_empty());
    }

    #[test]
    fn test_parse_alternative_invalid_mode() {
        let json = serde_json::json!({
            "alternative_mode": "invalid"
        });
        let alt = AutoMode::<MockStorageTrait, MockAnthropicClientTrait>::parse_alternative(&json);
        assert!(alt.is_none());
    }

    #[test]
    fn test_auto_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = AutoMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("AutoMode"));
    }
}
