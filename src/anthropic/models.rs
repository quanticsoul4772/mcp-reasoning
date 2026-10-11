//! Model resolution against the Anthropic Models API.
//!
//! Model IDs change with every release, so the configured model may be a
//! family alias (`sonnet`, `opus`, `haiku`, ...) instead of a pinned ID. At
//! startup the alias resolves to the newest listed model in that family. A full
//! ID (`claude-...`) is used as given, but only after checking it is still
//! listed, so a retired model fails at startup instead of on every call.

use serde::Deserialize;

use super::client::AnthropicClient;
use super::types::{ApiMessage, ApiRequest, ThinkingConfig, MIN_THINKING_BUDGET};
use crate::error::AnthropicError;

/// One entry from `GET /v1/models`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    /// Model identifier, e.g. `claude-sonnet-5-5`.
    pub id: String,
    /// RFC 3339 release timestamp.
    pub created_at: String,
}

/// One page of `GET /v1/models`.
#[derive(Debug, Deserialize)]
pub(super) struct ModelsPage {
    pub data: Vec<ModelInfo>,
    pub has_more: bool,
    pub last_id: Option<String>,
}

/// Pick the model for `requested` from the listed models.
///
/// A value starting with `claude-` is a full ID and must appear in `models`.
/// Anything else is a family alias and resolves to the model with the latest
/// `created_at` whose ID starts with `claude-{family}-`.
///
/// # Errors
///
/// Returns [`AnthropicError::InvalidRequest`] when the ID is not listed or the
/// family has no listed models; the message names the available models.
pub fn select_model(requested: &str, models: &[ModelInfo]) -> Result<String, AnthropicError> {
    let requested = requested.trim();
    let available = || {
        models
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    if requested.starts_with("claude-") {
        return if models.iter().any(|m| m.id == requested) {
            Ok(requested.to_string())
        } else {
            Err(AnthropicError::InvalidRequest {
                message: format!(
                    "model '{requested}' is not available (retired or misspelled); available: {}",
                    available()
                ),
            })
        };
    }

    let prefix = format!("claude-{}-", requested.to_lowercase());
    models
        .iter()
        .filter(|m| m.id.starts_with(&prefix))
        // RFC 3339 timestamps in the same format and zone sort lexically.
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
        .map(|m| m.id.clone())
        .ok_or_else(|| AnthropicError::InvalidRequest {
            message: format!(
                "no models found for family '{requested}'; available: {}",
                available()
            ),
        })
}

/// Resolve `requested` (family alias or full ID) using the Models API, then
/// probe the chosen model with this server's request format.
///
/// The probe catches a new model that rejects a request field this server
/// sends (as Sonnet 5 did with `thinking.type: enabled`) at startup, with the
/// API's own message, instead of as failures on every tool call.
///
/// # Errors
///
/// Returns an error if the Models API call fails, [`select_model`] rejects
/// the requested value, or the probe request fails.
pub async fn resolve_model(
    client: &AnthropicClient,
    requested: &str,
) -> Result<String, AnthropicError> {
    let models = client.list_models().await?;
    let model = select_model(requested, &models)?;
    probe_model(client, &model).await?;
    Ok(model)
}

/// Send one minimal request in this server's format (adaptive thinking plus
/// effort) to `model`.
///
/// # Errors
///
/// A rejected request (non-success status) becomes
/// [`AnthropicError::InvalidRequest`] naming the model, the API message, and
/// how to pin a working model. Network, auth, and rate-limit errors pass
/// through unchanged.
pub async fn probe_model(client: &AnthropicClient, model: &str) -> Result<(), AnthropicError> {
    let request = ApiRequest::new(
        model,
        PROBE_MAX_TOKENS,
        vec![ApiMessage::user("Reply with the single word OK.")],
    )
    .with_thinking(ThinkingConfig::enabled(MIN_THINKING_BUDGET));
    match client.complete(request).await {
        Ok(_) => Ok(()),
        Err(AnthropicError::UnexpectedResponse { message }) => {
            Err(AnthropicError::InvalidRequest {
                message: format!(
                    "model '{model}' rejected this server's request format: {message}.                      Update the server, or pin a model that works with ANTHROPIC_MODEL=<full id>"
                ),
            })
        }
        Err(e) => Err(e),
    }
}

/// Output cap for the startup probe; the reply is one word.
const PROBE_MAX_TOKENS: u32 = 256;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn models() -> Vec<ModelInfo> {
        [
            ("claude-haiku-5-5", "2026-10-07T18:00:00Z"),
            ("claude-sonnet-5-5", "2026-09-28T00:00:00Z"),
            ("claude-opus-5-5", "2026-09-21T16:24:00Z"),
            ("claude-sonnet-5", "2026-06-29T00:00:00Z"),
            ("claude-sonnet-4-5-20250929", "2025-09-29T00:00:00Z"),
        ]
        .into_iter()
        .map(|(id, created_at)| ModelInfo {
            id: id.into(),
            created_at: created_at.into(),
        })
        .collect()
    }

    #[test]
    fn family_alias_resolves_to_newest() {
        assert_eq!(
            select_model("sonnet", &models()).unwrap(),
            "claude-sonnet-5-5"
        );
        assert_eq!(select_model("opus", &models()).unwrap(), "claude-opus-5-5");
        assert_eq!(
            select_model("Haiku", &models()).unwrap(),
            "claude-haiku-5-5"
        );
    }

    #[test]
    fn listed_full_id_is_kept() {
        assert_eq!(
            select_model("claude-sonnet-5", &models()).unwrap(),
            "claude-sonnet-5"
        );
    }

    #[test]
    fn retired_full_id_is_rejected() {
        let err = select_model("claude-sonnet-4-20250514", &models()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("claude-sonnet-4-20250514"));
        assert!(msg.contains("claude-sonnet-5-5"));
    }

    #[test]
    fn unknown_family_is_rejected() {
        assert!(select_model("mythos", &models()).is_err());
    }

    fn mock_client(server: &wiremock::MockServer) -> AnthropicClient {
        let config = super::super::ClientConfig::default()
            .with_base_url(server.uri())
            .with_max_retries(0)
            .with_timeout_ms(5_000);
        AnthropicClient::new("test-api-key", config).unwrap()
    }

    async fn mount_models(server: &wiremock::MockServer) {
        use wiremock::matchers::{method, path};
        wiremock::Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": [
                        {"id": "claude-sonnet-5-5", "created_at": "2026-09-28T00:00:00Z"},
                        {"id": "claude-sonnet-5", "created_at": "2026-06-29T00:00:00Z"}
                    ],
                    "has_more": false,
                    "last_id": "claude-sonnet-5"
                })),
            )
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn resolve_probes_newest_with_adaptive_thinking() {
        use wiremock::matchers::{body_partial_json, method, path};
        let server = wiremock::MockServer::start().await;
        mount_models(&server).await;
        wiremock::Mock::given(method("POST"))
            .and(path("/messages"))
            .and(body_partial_json(serde_json::json!({
                "model": "claude-sonnet-5-5",
                "thinking": {"type": "adaptive"},
                "output_config": {"effort": "low"}
            })))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_1",
                    "content": [{"type": "text", "text": "OK"}],
                    "model": "claude-sonnet-5-5",
                    "usage": {"input_tokens": 5, "output_tokens": 1},
                    "stop_reason": "end_turn"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = mock_client(&server);
        assert_eq!(
            resolve_model(&client, "sonnet").await.unwrap(),
            "claude-sonnet-5-5"
        );
    }

    #[tokio::test]
    async fn resolve_fails_when_model_rejects_request_format() {
        use wiremock::matchers::{method, path};
        let server = wiremock::MockServer::start().await;
        mount_models(&server).await;
        wiremock::Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(wiremock::ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "type": "error",
                "error": {"type": "invalid_request_error",
                          "message": "\"thinking.type.adaptive\" is not supported for this model."}
            })))
            .mount(&server)
            .await;
        let client = mock_client(&server);
        let msg = resolve_model(&client, "sonnet")
            .await
            .unwrap_err()
            .to_string();
        assert!(msg.contains("claude-sonnet-5-5"), "{msg}");
        assert!(
            msg.contains("rejected this server's request format"),
            "{msg}"
        );
        assert!(msg.contains("thinking.type.adaptive"), "{msg}");
        assert!(msg.contains("ANTHROPIC_MODEL"), "{msg}");
    }

    #[tokio::test]
    async fn resolve_fails_when_models_api_fails() {
        use wiremock::matchers::{method, path};
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let client = mock_client(&server);
        let msg = resolve_model(&client, "sonnet")
            .await
            .unwrap_err()
            .to_string();
        assert!(msg.contains("Models API status 500"), "{msg}");
    }

    #[test]
    fn family_prefix_does_not_match_other_families() {
        let only_opus = vec![ModelInfo {
            id: "claude-opus-5-5".into(),
            created_at: "2026-09-21T16:24:00Z".into(),
        }];
        assert!(select_model("sonnet", &only_opus).is_err());
    }
}
