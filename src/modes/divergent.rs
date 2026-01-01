//! Divergent reasoning mode.
//!
//! This mode generates multiple distinct perspectives on a topic.
//! It supports:
//! - Basic perspective generation (3-5 perspectives)
//! - `challenge_assumptions` flag to identify hidden assumptions
//! - `force_rebellion` flag for maximum contrarian thinking

#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::similar_names)]
#![allow(clippy::cast_precision_loss)]

use serde::{Deserialize, Serialize};

use crate::anthropic::StreamAccumulator;
use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{get_prompt_for_mode, Operation, ReasoningMode};
use crate::server::{ProgressMilestone, ProgressReporter};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

/// A single perspective from divergent reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Perspective {
    /// Name or label for this perspective.
    pub viewpoint: String,
    /// The reasoning from this perspective.
    pub content: String,
    /// Novelty score (0.0-1.0).
    pub novelty_score: f64,
    /// What this perspective might miss.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blind_spots: Option<Vec<String>>,
}

impl Perspective {
    /// Create a new perspective.
    #[must_use]
    pub fn new(
        viewpoint: impl Into<String>,
        content: impl Into<String>,
        novelty_score: f64,
    ) -> Self {
        Self {
            viewpoint: viewpoint.into(),
            content: content.into(),
            novelty_score,
            blind_spots: None,
        }
    }

    /// Add blind spots.
    #[must_use]
    pub fn with_blind_spots(mut self, blind_spots: Vec<String>) -> Self {
        self.blind_spots = Some(blind_spots);
        self
    }
}

/// Response from divergent reasoning mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DivergentResponse {
    /// Unique thought identifier.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// List of generated perspectives.
    pub perspectives: Vec<Perspective>,
    /// Assumptions that were challenged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenged_assumptions: Option<Vec<String>>,
    /// Tensions between perspectives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tensions: Option<Vec<String>>,
    /// Synergies between perspectives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synergies: Option<Vec<String>>,
    /// Unified synthesis from all perspectives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesis: Option<String>,
}

impl DivergentResponse {
    /// Create a new divergent response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        perspectives: Vec<Perspective>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            perspectives,
            challenged_assumptions: None,
            tensions: None,
            synergies: None,
            synthesis: None,
        }
    }

    /// Add challenged assumptions.
    #[must_use]
    pub fn with_challenged_assumptions(mut self, assumptions: Vec<String>) -> Self {
        self.challenged_assumptions = Some(assumptions);
        self
    }

    /// Add tensions.
    #[must_use]
    pub fn with_tensions(mut self, tensions: Vec<String>) -> Self {
        self.tensions = Some(tensions);
        self
    }

    /// Add synergies.
    #[must_use]
    pub fn with_synergies(mut self, synergies: Vec<String>) -> Self {
        self.synergies = Some(synergies);
        self
    }

    /// Add synthesis.
    #[must_use]
    pub fn with_synthesis(mut self, synthesis: impl Into<String>) -> Self {
        self.synthesis = Some(synthesis.into());
        self
    }
}

/// Divergent reasoning mode.
///
/// Generates multiple distinct perspectives on a topic.
pub struct DivergentMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> DivergentMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new divergent mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Generate multiple perspectives on the content.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to analyze from multiple perspectives
    /// * `session_id` - Optional session ID for context continuity
    /// * `num_perspectives` - Number of perspectives to generate (2-5)
    /// * `challenge_assumptions` - Whether to identify and challenge hidden assumptions
    /// * `force_rebellion` - Enable maximum contrarian thinking
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn process(
        &self,
        content: &str,
        session_id: Option<String>,
        num_perspectives: Option<u32>,
        challenge_assumptions: bool,
        force_rebellion: bool,
    ) -> Result<DivergentResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;
        let num_perspectives = num_perspectives.unwrap_or(3).clamp(2, 5);

        // Select prompt based on force_rebellion
        let operation = if force_rebellion {
            Some(Operation::ForceRebellion)
        } else {
            None
        };
        let prompt = get_prompt_for_mode(ReasoningMode::Divergent, operation.as_ref());

        let user_message = if challenge_assumptions {
            format!(
                "{prompt}\n\nIMPORTANT: Also identify and challenge hidden assumptions.\n\nGenerate {num_perspectives} perspectives for:\n{content}"
            )
        } else {
            format!("{prompt}\n\nGenerate {num_perspectives} perspectives for:\n{content}")
        };

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.9)
            .with_deep_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse perspectives
        let perspectives = Self::parse_perspectives(&json, force_rebellion)?;

        // Parse optional fields
        let challenged_assumptions = Self::parse_string_array(&json, "assumptions_identified")
            .or_else(|| Self::parse_string_array(&json, "challenged_assumptions"));

        let tensions = Self::parse_string_array(&json, "tensions");
        let synergies = Self::parse_string_array(&json, "synergies");

        let synthesis = json
            .get("synthesis")
            .or_else(|| json.get("strongest_challenge"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Generate thought ID and save
        let thought_id = generate_thought_id();
        let avg_novelty = if perspectives.is_empty() {
            0.5
        } else {
            perspectives.iter().map(|p| p.novelty_score).sum::<f64>() / perspectives.len() as f64
        };

        let thought = Thought::new(&thought_id, &session.id, content, "divergent", avg_novelty);
        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        // Build response
        let mut response = DivergentResponse::new(&thought_id, &session.id, perspectives);

        if let Some(assumptions) = challenged_assumptions {
            response = response.with_challenged_assumptions(assumptions);
        }
        if let Some(t) = tensions {
            response = response.with_tensions(t);
        }
        if let Some(s) = synergies {
            response = response.with_synergies(s);
        }
        if let Some(syn) = synthesis {
            response = response.with_synthesis(syn);
        }

        Ok(response)
    }

    /// Generate multiple perspectives on the content using streaming.
    ///
    /// This method uses streaming API calls with progress reporting for
    /// reduced perceived latency on long-running operations.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to analyze from multiple perspectives
    /// * `session_id` - Optional session ID for context continuity
    /// * `num_perspectives` - Number of perspectives to generate (2-5)
    /// * `challenge_assumptions` - Whether to identify and challenge hidden assumptions
    /// * `force_rebellion` - Enable maximum contrarian thinking
    /// * `progress` - Optional progress reporter for streaming updates
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn process_streaming(
        &self,
        content: &str,
        session_id: Option<String>,
        num_perspectives: Option<u32>,
        challenge_assumptions: bool,
        force_rebellion: bool,
        progress: Option<&ProgressReporter>,
    ) -> Result<DivergentResponse, ModeError> {
        validate_content(content)?;

        if let Some(p) = progress {
            p.report_milestone(ProgressMilestone::RequestPrepared);
        }

        let session = self.get_or_create_session(session_id).await?;
        let num_perspectives = num_perspectives.unwrap_or(3).clamp(2, 5);

        // Select prompt based on force_rebellion
        let operation = if force_rebellion {
            Some(Operation::ForceRebellion)
        } else {
            None
        };
        let prompt = get_prompt_for_mode(ReasoningMode::Divergent, operation.as_ref());

        let user_message = if challenge_assumptions {
            format!(
                "{prompt}\n\nIMPORTANT: Also identify and challenge hidden assumptions.\n\nGenerate {num_perspectives} perspectives for:\n{content}"
            )
        } else {
            format!("{prompt}\n\nGenerate {num_perspectives} perspectives for:\n{content}")
        };

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.9)
            .with_deep_thinking();

        if let Some(p) = progress {
            p.report_milestone(ProgressMilestone::ApiCallStarted);
        }

        // Use streaming API
        let mut rx = self.client.complete_streaming(messages, config).await?;

        if let Some(p) = progress {
            p.report_milestone(ProgressMilestone::StreamingStarted);
        }

        // Accumulate the stream
        let mut accumulator = StreamAccumulator::new();
        while let Some(event_result) = rx.recv().await {
            let event = event_result?;
            accumulator.process(event);
        }

        if let Some(p) = progress {
            p.report_milestone(ProgressMilestone::ProcessingResponse);
        }

        let response_text = accumulator.text();
        let json = extract_json(&response_text)?;

        // Parse perspectives
        let perspectives = Self::parse_perspectives(&json, force_rebellion)?;

        // Parse optional fields
        let challenged_assumptions = Self::parse_string_array(&json, "assumptions_identified")
            .or_else(|| Self::parse_string_array(&json, "challenged_assumptions"));

        let tensions = Self::parse_string_array(&json, "tensions");
        let synergies = Self::parse_string_array(&json, "synergies");

        let synthesis = json
            .get("synthesis")
            .or_else(|| json.get("strongest_challenge"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Generate thought ID and save
        let thought_id = generate_thought_id();
        let avg_novelty = if perspectives.is_empty() {
            0.5
        } else {
            perspectives.iter().map(|p| p.novelty_score).sum::<f64>() / perspectives.len() as f64
        };

        let thought = Thought::new(&thought_id, &session.id, content, "divergent", avg_novelty);
        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        // Build response
        let mut response = DivergentResponse::new(&thought_id, &session.id, perspectives);

        if let Some(assumptions) = challenged_assumptions {
            response = response.with_challenged_assumptions(assumptions);
        }
        if let Some(t) = tensions {
            response = response.with_tensions(t);
        }
        if let Some(s) = synergies {
            response = response.with_synergies(s);
        }
        if let Some(syn) = synthesis {
            response = response.with_synthesis(syn);
        }

        if let Some(p) = progress {
            p.report_milestone(ProgressMilestone::Complete);
        }

        Ok(response)
    }

    /// Parse perspectives from JSON response.
    fn parse_perspectives(
        json: &serde_json::Value,
        force_rebellion: bool,
    ) -> Result<Vec<Perspective>, ModeError> {
        let perspectives_key = if force_rebellion {
            "contrarian_perspectives"
        } else {
            "perspectives"
        };

        let perspectives_json = json
            .get(perspectives_key)
            .or_else(|| json.get("perspectives"))
            .ok_or_else(|| ModeError::MissingField {
                field: "perspectives".to_string(),
            })?;

        let perspectives_arr =
            perspectives_json
                .as_array()
                .ok_or_else(|| ModeError::InvalidValue {
                    field: "perspectives".to_string(),
                    reason: "expected array".to_string(),
                })?;

        if perspectives_arr.is_empty() {
            return Err(ModeError::InvalidValue {
                field: "perspectives".to_string(),
                reason: "at least one perspective required".to_string(),
            });
        }

        let mut perspectives = Vec::new();
        for (i, p) in perspectives_arr.iter().enumerate() {
            let viewpoint = p
                .get("name")
                .or_else(|| p.get("viewpoint"))
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("Perspective {}", i + 1))
                .to_string();

            let content = p
                .get("viewpoint")
                .or_else(|| p.get("content"))
                .or_else(|| p.get("argument"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let novelty_score = p
                .get("novelty_score")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.5)
                .clamp(0.0, 1.0);

            let blind_spots = p.get("blind_spots").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
            });

            let mut perspective = Perspective::new(viewpoint, content, novelty_score);
            if let Some(spots) = blind_spots {
                perspective = perspective.with_blind_spots(spots);
            }
            perspectives.push(perspective);
        }

        Ok(perspectives)
    }

    /// Parse an array of strings from JSON.
    fn parse_string_array(json: &serde_json::Value, key: &str) -> Option<Vec<String>> {
        json.get(key).and_then(|v| {
            // Handle both array of strings and array of objects with "assumption" key
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        // Try as string first
                        item.as_str()
                            .map(String::from)
                            // Or as object with "assumption" key
                            .or_else(|| {
                                item.get("assumption")
                                    .and_then(|a| a.as_str())
                                    .map(String::from)
                            })
                    })
                    .collect()
            })
        })
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

impl<S, C> std::fmt::Debug for DivergentMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DivergentMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .finish()
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
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn mock_perspectives_response(num: usize) -> String {
        let perspectives: Vec<String> = (0..num)
            .map(|i| {
                format!(
                    r#"{{"name": "Perspective {}", "viewpoint": "Analysis from perspective {}", "novelty_score": {}, "blind_spots": ["Blind spot {}"]}}"#,
                    i + 1,
                    i + 1,
                    0.5 + (i as f64 * 0.1),
                    i + 1
                )
            })
            .collect();

        format!(
            r#"{{"perspectives": [{}], "tensions": ["Tension between A and B"], "synergies": ["Synergy between B and C"], "synthesis": "Combined insight from all perspectives"}}"#,
            perspectives.join(", ")
        )
    }

    fn mock_rebellion_response() -> String {
        r#"{
            "assumptions_identified": [
                {"assumption": "Hidden assumption 1", "why_questioned": "Reason 1"},
                {"assumption": "Hidden assumption 2", "why_questioned": "Reason 2"}
            ],
            "contrarian_perspectives": [
                {"name": "Contrarian 1", "challenge": "Challenges X", "argument": "Contrarian argument", "evidence": "Supporting evidence"}
            ],
            "radical_perspective": {"name": "Radical view", "thesis": "Radical claim", "implications": "What follows"},
            "strongest_challenge": "The most compelling challenge"
        }"#.to_string()
    }

    // Perspective tests
    #[test]
    fn test_perspective_new() {
        let p = Perspective::new("Optimist", "Everything will be fine", 0.7);
        assert_eq!(p.viewpoint, "Optimist");
        assert_eq!(p.content, "Everything will be fine");
        assert!((p.novelty_score - 0.7).abs() < f64::EPSILON);
        assert!(p.blind_spots.is_none());
    }

    #[test]
    fn test_perspective_with_blind_spots() {
        let p = Perspective::new("Optimist", "Content", 0.5)
            .with_blind_spots(vec!["Risk 1".to_string(), "Risk 2".to_string()]);
        assert_eq!(p.blind_spots.unwrap().len(), 2);
    }

    #[test]
    fn test_perspective_serialize() {
        let p = Perspective::new("Optimist", "Content", 0.5);
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"viewpoint\":\"Optimist\""));
        assert!(json.contains("\"novelty_score\":0.5"));
        // blind_spots should be omitted when None
        assert!(!json.contains("blind_spots"));
    }

    #[test]
    fn test_perspective_deserialize() {
        let json = r#"{"viewpoint": "Test", "content": "Content", "novelty_score": 0.8}"#;
        let p: Perspective = serde_json::from_str(json).unwrap();
        assert_eq!(p.viewpoint, "Test");
        assert!((p.novelty_score - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_perspective_clone() {
        let p = Perspective::new("Test", "Content", 0.5);
        let cloned = p.clone();
        assert_eq!(p, cloned);
    }

    // DivergentResponse tests
    #[test]
    fn test_divergent_response_new() {
        let perspectives = vec![Perspective::new("P1", "C1", 0.5)];
        let response = DivergentResponse::new("t-1", "s-1", perspectives);
        assert_eq!(response.thought_id, "t-1");
        assert_eq!(response.session_id, "s-1");
        assert_eq!(response.perspectives.len(), 1);
        assert!(response.challenged_assumptions.is_none());
        assert!(response.synthesis.is_none());
    }

    #[test]
    fn test_divergent_response_with_all_fields() {
        let perspectives = vec![Perspective::new("P1", "C1", 0.5)];
        let response = DivergentResponse::new("t-1", "s-1", perspectives)
            .with_challenged_assumptions(vec!["A1".to_string()])
            .with_tensions(vec!["T1".to_string()])
            .with_synergies(vec!["S1".to_string()])
            .with_synthesis("Combined insight");

        assert!(response.challenged_assumptions.is_some());
        assert!(response.tensions.is_some());
        assert!(response.synergies.is_some());
        assert_eq!(response.synthesis, Some("Combined insight".to_string()));
    }

    #[test]
    fn test_divergent_response_serialize_omits_none() {
        let perspectives = vec![Perspective::new("P1", "C1", 0.5)];
        let response = DivergentResponse::new("t-1", "s-1", perspectives);
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("challenged_assumptions"));
        assert!(!json.contains("tensions"));
        assert!(!json.contains("synthesis"));
    }

    // DivergentMode tests
    #[tokio::test]
    async fn test_divergent_process_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_perspectives_response(3);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode
            .process(
                "Test content",
                Some("test-session".to_string()),
                None,
                false,
                false,
            )
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.perspectives.len(), 3);
        assert!(response.tensions.is_some());
        assert!(response.synergies.is_some());
        assert!(response.synthesis.is_some());
    }

    #[tokio::test]
    async fn test_divergent_process_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("", None, None, false, false).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "content"
        ));
    }

    #[tokio::test]
    async fn test_divergent_process_custom_num_perspectives() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_perspectives_response(5);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None, Some(5), false, false).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().perspectives.len(), 5);
    }

    #[tokio::test]
    async fn test_divergent_process_with_challenge_assumptions() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = format!(
            r#"{{"perspectives": [{}], "challenged_assumptions": ["Assumption 1", "Assumption 2"]}}"#,
            r#"{"name": "P1", "viewpoint": "C1", "novelty_score": 0.5}"#
        );
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None, None, true, false).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.challenged_assumptions.is_some());
        assert_eq!(response.challenged_assumptions.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_divergent_process_with_force_rebellion() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_rebellion_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None, None, false, true).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        // Should have parsed contrarian_perspectives
        assert!(!response.perspectives.is_empty());
        // Should have challenged_assumptions from assumptions_identified
        assert!(response.challenged_assumptions.is_some());
        // Should have synthesis from strongest_challenge
        assert!(response.synthesis.is_some());
    }

    #[tokio::test]
    async fn test_divergent_process_api_error() {
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

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None, None, false, false).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_divergent_process_missing_perspectives() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"synthesis": "No perspectives provided"}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None, None, false, false).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "perspectives"
        ));
    }

    #[tokio::test]
    async fn test_divergent_process_empty_perspectives_array() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"perspectives": []}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None, None, false, false).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::InvalidValue { field, reason })
            if field == "perspectives" && reason.contains("at least one perspective")
        ));
    }

    #[tokio::test]
    async fn test_divergent_process_clamps_novelty_score() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        // Response with out-of-range novelty score
        let response_json =
            r#"{"perspectives": [{"name": "P1", "viewpoint": "C1", "novelty_score": 1.5}]}"#;
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.to_string(),
                Usage::new(50, 100),
            ))
        });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None, None, false, false).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        // Should be clamped to 1.0
        assert!((response.perspectives[0].novelty_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_divergent_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = DivergentMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("DivergentMode"));
    }

    // ============================================================================
    // Streaming tests
    // ============================================================================

    #[tokio::test]
    async fn test_divergent_process_streaming_success() {
        use crate::anthropic::{ApiUsage, StreamEvent};
        use tokio::sync::mpsc;

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_perspectives_response(3);
        mock_client
            .expect_complete_streaming()
            .returning(move |_, _| {
                let response_text = response_json.clone();
                let (tx, rx) = mpsc::channel(32);

                // Spawn task to send stream events
                tokio::spawn(async move {
                    let _ = tx
                        .send(Ok(StreamEvent::MessageStart {
                            message_id: "msg_123".to_string(),
                        }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::ContentBlockStart {
                            index: 0,
                            block_type: "text".to_string(),
                        }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::TextDelta {
                            index: 0,
                            text: response_text,
                        }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::ContentBlockStop { index: 0 }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::MessageStop {
                            stop_reason: "end_turn".to_string(),
                            usage: ApiUsage::new(100, 200),
                        }))
                        .await;
                });

                Ok(rx)
            });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode
            .process_streaming(
                "Test content",
                Some("test-session".to_string()),
                None,
                false,
                false,
                None,
            )
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.perspectives.len(), 3);
    }

    #[tokio::test]
    async fn test_divergent_process_streaming_with_progress() {
        use crate::anthropic::{ApiUsage, StreamEvent};
        use crate::server::{ProgressEvent, ProgressReporter};
        use tokio::sync::{broadcast, mpsc};

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_perspectives_response(3);
        mock_client
            .expect_complete_streaming()
            .returning(move |_, _| {
                let response_text = response_json.clone();
                let (tx, rx) = mpsc::channel(32);

                tokio::spawn(async move {
                    let _ = tx
                        .send(Ok(StreamEvent::MessageStart {
                            message_id: "msg_123".to_string(),
                        }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::ContentBlockStart {
                            index: 0,
                            block_type: "text".to_string(),
                        }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::TextDelta {
                            index: 0,
                            text: response_text,
                        }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::ContentBlockStop { index: 0 }))
                        .await;
                    let _ = tx
                        .send(Ok(StreamEvent::MessageStop {
                            stop_reason: "end_turn".to_string(),
                            usage: ApiUsage::new(100, 200),
                        }))
                        .await;
                });

                Ok(rx)
            });

        let mode = DivergentMode::new(mock_storage, mock_client);

        // Create progress reporter manually
        let (progress_tx, mut progress_rx) = broadcast::channel::<ProgressEvent>(100);
        let progress = ProgressReporter::new("test-token".to_string(), progress_tx);

        let result = mode
            .process_streaming(
                "Test content",
                Some("test-session".to_string()),
                None,
                false,
                false,
                Some(&progress),
            )
            .await;

        assert!(result.is_ok());

        // Verify progress events were sent
        let mut received_events = Vec::new();
        while let Ok(event) = progress_rx.try_recv() {
            received_events.push(event.progress);
        }

        // Should have received milestone progress events (5, 15, 20, 90, 100)
        assert!(
            received_events.len() >= 3,
            "Expected at least 3 progress events, got {}",
            received_events.len()
        );
    }

    #[tokio::test]
    async fn test_divergent_process_streaming_api_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client
            .expect_complete_streaming()
            .returning(|_, _| {
                Err(ModeError::ApiUnavailable {
                    message: "Streaming API error".to_string(),
                })
            });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode
            .process_streaming("Content", None, None, false, false, None)
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_divergent_process_streaming_mid_stream_error() {
        use crate::anthropic::StreamEvent;
        use tokio::sync::mpsc;

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client
            .expect_complete_streaming()
            .returning(|_, _| {
                let (tx, rx) = mpsc::channel(32);

                tokio::spawn(async move {
                    let _ = tx
                        .send(Ok(StreamEvent::MessageStart {
                            message_id: "msg_123".to_string(),
                        }))
                        .await;
                    // Send error mid-stream
                    let _ = tx
                        .send(Err(ModeError::ApiUnavailable {
                            message: "Rate limit exceeded mid-stream".to_string(),
                        }))
                        .await;
                });

                Ok(rx)
            });

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode
            .process_streaming("Content", None, None, false, false, None)
            .await;

        // Error should propagate - fail fast, fail loudly
        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_divergent_process_streaming_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = DivergentMode::new(mock_storage, mock_client);
        let result = mode
            .process_streaming("", None, None, false, false, None)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "content"
        ));
    }
}
