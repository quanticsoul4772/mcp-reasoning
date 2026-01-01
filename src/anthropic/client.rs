//! Anthropic API client with retry logic.
//!
//! This module provides:
//! - HTTP client for the Anthropic Messages API
//! - Retry logic with exponential backoff
//! - Request validation
//! - Response parsing

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::uninlined_format_args)]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;

use super::config::{ClientConfig, DEFAULT_MODEL};
use super::streaming::parse_sse_line;
use super::types::{
    ApiMessage, ApiRequest, ApiResponse, ContentBlock, ReasoningResponse, StreamEvent,
    ThinkingConfig, ToolUseResult,
};
use crate::error::{AnthropicError, ModeError};
use crate::traits::{AnthropicClientTrait, CompletionConfig, CompletionResponse, Message, Usage};

/// Maximum request size in bytes (100KB).
pub const MAX_REQUEST_BYTES: usize = 100_000;
/// Maximum number of messages per request.
pub const MAX_MESSAGES: usize = 50;
/// Maximum content length per message (50KB).
pub const MAX_CONTENT_LENGTH: usize = 50_000;

/// Anthropic API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic API client.
#[derive(Debug)]
pub struct AnthropicClient {
    client: Client,
    api_key: String,
    config: ClientConfig,
}

impl AnthropicClient {
    /// Create a new Anthropic client.
    pub fn new(api_key: impl Into<String>, config: ClientConfig) -> Result<Self, AnthropicError> {
        let timeout = Duration::from_millis(config.timeout_ms);
        let client =
            Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|e| AnthropicError::Network {
                    message: format!("Failed to create HTTP client: {e}"),
                })?;

        Ok(Self {
            client,
            api_key: api_key.into(),
            config,
        })
    }

    /// Create a client with default configuration.
    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self, AnthropicError> {
        Self::new(api_key, ClientConfig::default())
    }

    /// Get the base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Get the client configuration.
    #[must_use]
    pub const fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Send a completion request with retry logic.
    pub async fn complete(&self, request: ApiRequest) -> Result<ReasoningResponse, AnthropicError> {
        Self::validate_request(&request)?;
        self.execute_with_retry(request).await
    }

    /// Send a streaming completion request.
    ///
    /// Returns a channel receiver that yields `StreamEvent`s as they arrive.
    /// The caller should consume events until the channel closes or a
    /// `StreamEvent::Error` or `StreamEvent::MessageStop` is received.
    ///
    /// # Errors
    ///
    /// Returns `AnthropicError` if request validation fails or connection cannot be established.
    pub async fn complete_streaming(
        &self,
        request: ApiRequest,
    ) -> Result<mpsc::Receiver<Result<StreamEvent, AnthropicError>>, AnthropicError> {
        Self::validate_request(&request)?;
        let request = request.with_streaming(true);

        let (tx, rx) = mpsc::channel(32);
        let url = format!("{}/messages", self.config.base_url);

        tracing::debug!(
            url = %url,
            model = %request.model,
            max_tokens = ?request.max_tokens,
            thinking_budget = ?request.thinking.as_ref().map(|t| t.budget_tokens),
            "Starting streaming Anthropic API request"
        );

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AnthropicError::Timeout {
                        timeout_ms: self.config.timeout_ms,
                    }
                } else {
                    AnthropicError::Network {
                        message: e.to_string(),
                    }
                }
            })?;

        let status = response.status();

        // Handle error status codes - fail fast, no fallbacks
        if status.as_u16() == 401 {
            return Err(AnthropicError::AuthenticationFailed);
        }
        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(AnthropicError::RateLimited {
                retry_after_seconds: retry_after,
            });
        }
        if status.as_u16() == 529 {
            return Err(AnthropicError::ModelOverloaded {
                model: request.model.clone(),
            });
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AnthropicError::UnexpectedResponse {
                message: format!("Status {}: {}", status, body),
            });
        }

        // Spawn task to parse SSE stream and send events
        let byte_stream = response.bytes_stream();
        tokio::spawn(async move {
            let mut stream = byte_stream;
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let text = match String::from_utf8(bytes.to_vec()) {
                            Ok(t) => t,
                            Err(e) => {
                                let _ = tx
                                    .send(Err(AnthropicError::UnexpectedResponse {
                                        message: format!("Invalid UTF-8 in stream: {e}"),
                                    }))
                                    .await;
                                return;
                            }
                        };

                        buffer.push_str(&text);

                        // Process complete lines
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            if let Some(event_result) = parse_sse_line(&line) {
                                match event_result {
                                    Ok(event) => {
                                        // Check for error events - propagate immediately
                                        if let StreamEvent::Error { ref error } = event {
                                            let _ = tx
                                                .send(Err(AnthropicError::UnexpectedResponse {
                                                    message: error.clone(),
                                                }))
                                                .await;
                                            return;
                                        }

                                        if tx.send(Ok(event)).await.is_err() {
                                            // Receiver dropped, stop processing
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(e)).await;
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(AnthropicError::Network {
                                message: e.to_string(),
                            }))
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Validate request size limits.
    fn validate_request(request: &ApiRequest) -> Result<(), AnthropicError> {
        if request.messages.len() > MAX_MESSAGES {
            return Err(AnthropicError::InvalidRequest {
                message: format!(
                    "Too many messages: {} > {}",
                    request.messages.len(),
                    MAX_MESSAGES
                ),
            });
        }

        for msg in &request.messages {
            let content_len = msg.content.len();
            if content_len > MAX_CONTENT_LENGTH {
                return Err(AnthropicError::InvalidRequest {
                    message: format!(
                        "Message too large: {} > {}",
                        content_len, MAX_CONTENT_LENGTH
                    ),
                });
            }
        }

        Ok(())
    }

    /// Execute request with retry logic.
    async fn execute_with_retry(
        &self,
        request: ApiRequest,
    ) -> Result<ReasoningResponse, AnthropicError> {
        let mut last_error = None;
        let mut delay = self.config.retry_delay_ms;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                tracing::warn!(attempt, delay_ms = delay, "Retrying Anthropic request");
                tokio::time::sleep(Duration::from_millis(delay)).await;
                delay *= 2; // Exponential backoff
            }

            match self.execute_once(&request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if !e.is_retryable() {
                        return Err(e);
                    }
                    tracing::warn!(error = %e, attempt, "Retryable error occurred");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AnthropicError::Network {
            message: "Unknown error after retries".to_string(),
        }))
    }

    /// Execute a single request attempt.
    async fn execute_once(
        &self,
        request: &ApiRequest,
    ) -> Result<ReasoningResponse, AnthropicError> {
        let url = format!("{}/messages", self.config.base_url);
        let start = std::time::Instant::now();

        tracing::debug!(
            url = %url,
            model = %request.model,
            max_tokens = ?request.max_tokens,
            thinking_budget = ?request.thinking.as_ref().map(|t| t.budget_tokens),
            timeout_ms = self.config.timeout_ms,
            "Starting Anthropic API request"
        );

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                if e.is_timeout() {
                    tracing::error!(
                        url = %url,
                        elapsed_ms = elapsed_ms,
                        timeout_ms = self.config.timeout_ms,
                        "Anthropic API request timed out"
                    );
                    AnthropicError::Timeout {
                        timeout_ms: self.config.timeout_ms,
                    }
                } else {
                    tracing::error!(
                        url = %url,
                        elapsed_ms = elapsed_ms,
                        error = %e,
                        "Anthropic API request failed"
                    );
                    AnthropicError::Network {
                        message: e.to_string(),
                    }
                }
            })?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        tracing::debug!(
            url = %url,
            status = %response.status(),
            elapsed_ms = elapsed_ms,
            "Anthropic API response received"
        );

        let status = response.status();

        // Handle specific error status codes
        if status.as_u16() == 401 {
            return Err(AnthropicError::AuthenticationFailed);
        }

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(AnthropicError::RateLimited {
                retry_after_seconds: retry_after,
            });
        }

        if status.as_u16() == 529 {
            return Err(AnthropicError::ModelOverloaded {
                model: request.model.clone(),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AnthropicError::UnexpectedResponse {
                message: format!("Status {}: {}", status, body),
            });
        }

        // Parse successful response
        let body: ApiResponse =
            response
                .json()
                .await
                .map_err(|e| AnthropicError::UnexpectedResponse {
                    message: format!("Failed to parse response: {e}"),
                })?;

        Self::parse_response(body)
    }

    /// Parse API response into `ReasoningResponse`.
    fn parse_response(response: ApiResponse) -> Result<ReasoningResponse, AnthropicError> {
        let mut raw_text = String::new();
        let mut thinking = None;
        let mut tool_uses = Vec::new();

        for block in &response.content {
            match block {
                ContentBlock::Text { text } => {
                    if !raw_text.is_empty() {
                        raw_text.push('\n');
                    }
                    raw_text.push_str(text);
                }
                ContentBlock::Thinking { thinking: t } => {
                    thinking = Some(t.clone());
                }
                ContentBlock::ToolUse { id, name, input } => {
                    tool_uses.push(ToolUseResult::new(id, name, input.clone()));
                }
            }
        }

        if raw_text.is_empty() && tool_uses.is_empty() {
            return Err(AnthropicError::UnexpectedResponse {
                message: "No content in response".to_string(),
            });
        }

        let mut result = ReasoningResponse::new(&raw_text, response.usage);

        if let Some(t) = thinking {
            result = result.with_thinking(t);
        }

        for tu in tool_uses {
            result = result.with_tool_use(tu);
        }

        // Try to extract JSON from text
        if let Some(parsed) = extract_json(&raw_text) {
            result = result.with_parsed(parsed);
        }

        Ok(result)
    }
}

// ============================================================================
// AnthropicClientTrait implementations
// ============================================================================

/// Convert trait types to API types and call the underlying client.
#[async_trait]
impl AnthropicClientTrait for AnthropicClient {
    async fn complete(
        &self,
        messages: Vec<Message>,
        config: CompletionConfig,
    ) -> Result<CompletionResponse, ModeError> {
        // Convert messages to API format
        let api_messages: Vec<ApiMessage> = messages
            .into_iter()
            .map(|m| {
                if m.role == "user" {
                    ApiMessage::user(&m.content)
                } else {
                    ApiMessage::assistant(&m.content)
                }
            })
            .collect();

        // Build API request using the default model
        let max_tokens = config.max_tokens.unwrap_or(4096);
        let mut request = ApiRequest::new(DEFAULT_MODEL, max_tokens, api_messages);

        // Wire extended thinking if budget is specified
        // Note: When thinking is enabled, temperature must be 1 (API constraint)
        if let Some(budget) = config.thinking_budget {
            request = request.with_thinking(ThinkingConfig::enabled(budget));
            // Temperature is implicitly 1 when thinking is enabled
        } else if let Some(temp) = config.temperature {
            // Only set custom temperature when thinking is NOT enabled
            request = request.with_temperature(f64::from(temp));
        }

        if let Some(system) = config.system_prompt.as_ref() {
            request = request.with_system(system);
        }

        // Call the underlying API method (not the trait method)
        let response =
            Self::complete(self, request)
                .await
                .map_err(|e| ModeError::ApiUnavailable {
                    message: e.to_string(),
                })?;

        // Convert to trait response
        Ok(CompletionResponse::new(
            response.raw_text,
            Usage::new(response.usage.input_tokens, response.usage.output_tokens),
        ))
    }

    async fn complete_streaming(
        &self,
        messages: Vec<Message>,
        config: CompletionConfig,
    ) -> Result<mpsc::Receiver<Result<StreamEvent, ModeError>>, ModeError> {
        // Convert messages to API format
        let api_messages: Vec<ApiMessage> = messages
            .into_iter()
            .map(|m| {
                if m.role == "user" {
                    ApiMessage::user(&m.content)
                } else {
                    ApiMessage::assistant(&m.content)
                }
            })
            .collect();

        // Build API request using the default model
        let max_tokens = config.max_tokens.unwrap_or(4096);
        let mut request = ApiRequest::new(DEFAULT_MODEL, max_tokens, api_messages);

        // Wire extended thinking if budget is specified
        if let Some(budget) = config.thinking_budget {
            request = request.with_thinking(ThinkingConfig::enabled(budget));
        } else if let Some(temp) = config.temperature {
            request = request.with_temperature(f64::from(temp));
        }

        if let Some(system) = config.system_prompt.as_ref() {
            request = request.with_system(system);
        }

        // Call the underlying streaming API method
        let mut inner_rx = Self::complete_streaming(self, request)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: e.to_string(),
            })?;

        // Create new channel with mapped error type
        let (tx, rx) = mpsc::channel(32);

        // Spawn task to forward events with error mapping
        tokio::spawn(async move {
            while let Some(event_result) = inner_rx.recv().await {
                let mapped = event_result.map_err(|e| ModeError::ApiUnavailable {
                    message: e.to_string(),
                });
                if tx.send(mapped).await.is_err() {
                    // Receiver dropped
                    return;
                }
            }
        });

        Ok(rx)
    }
}

/// Blanket implementation for `Arc<AnthropicClient>`.
#[async_trait]
impl AnthropicClientTrait for Arc<AnthropicClient> {
    async fn complete(
        &self,
        messages: Vec<Message>,
        config: CompletionConfig,
    ) -> Result<CompletionResponse, ModeError> {
        // Explicitly call the trait method on the inner AnthropicClient
        <AnthropicClient as AnthropicClientTrait>::complete(self.as_ref(), messages, config).await
    }

    async fn complete_streaming(
        &self,
        messages: Vec<Message>,
        config: CompletionConfig,
    ) -> Result<mpsc::Receiver<Result<StreamEvent, ModeError>>, ModeError> {
        // Explicitly call the trait method on the inner AnthropicClient
        <AnthropicClient as AnthropicClientTrait>::complete_streaming(
            self.as_ref(),
            messages,
            config,
        )
        .await
    }
}

/// Extract JSON from text, handling code blocks.
fn extract_json(text: &str) -> Option<serde_json::Value> {
    // Fast path: try raw JSON parse
    if let Ok(value) = serde_json::from_str(text) {
        return Some(value);
    }

    // Fallback: extract from ```json code blocks
    if let Some(start) = text.find("```json") {
        let start = start + 7;
        if let Some(end) = text[start..].find("```") {
            let json_str = text[start..start + end].trim();
            if let Ok(value) = serde_json::from_str(json_str) {
                return Some(value);
            }
        }
    }

    // Try plain ``` blocks
    if let Some(start) = text.find("```") {
        let start = start + 3;
        // Skip language identifier if present (e.g., ```json\n or just ```\n)
        let start = text[start..].find('\n').map_or(start, |n| start + n + 1);
        if let Some(end) = text[start..].find("```") {
            let json_str = text[start..start + end].trim();
            if let Ok(value) = serde_json::from_str(json_str) {
                return Some(value);
            }
        }
    }

    None
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::unused_async
)]
mod tests {
    use super::*;
    use crate::anthropic::types::ApiMessage;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Helper to create a mock client pointing to the mock server
    async fn create_mock_client(server: &MockServer) -> AnthropicClient {
        let config = ClientConfig::default()
            .with_base_url(server.uri())
            .with_max_retries(0)
            .with_timeout_ms(5_000);
        AnthropicClient::new("test-api-key", config).unwrap()
    }

    // Helper to create a valid API response body
    fn success_response_body(text: &str) -> serde_json::Value {
        json!({
            "id": "msg_123",
            "content": [{"type": "text", "text": text}],
            "model": "claude-3",
            "usage": {"input_tokens": 10, "output_tokens": 20},
            "stop_reason": "end_turn"
        })
    }

    // AnthropicClient creation tests
    #[test]
    fn test_client_new() {
        let client = AnthropicClient::with_api_key("test-key").unwrap();
        assert_eq!(client.base_url(), "https://api.anthropic.com/v1");
    }

    #[test]
    fn test_client_with_config() {
        let config = ClientConfig::default()
            .with_base_url("http://localhost:8080")
            .with_timeout_ms(10_000);
        let client = AnthropicClient::new("test-key", config).unwrap();
        assert_eq!(client.base_url(), "http://localhost:8080");
        assert_eq!(client.config().timeout_ms, 10_000);
    }

    // Request validation tests
    #[tokio::test]
    async fn test_validate_request_too_many_messages() {
        let server = MockServer::start().await;
        let client = create_mock_client(&server).await;

        let messages: Vec<ApiMessage> = (0..=MAX_MESSAGES)
            .map(|i| ApiMessage::user(format!("Message {i}")))
            .collect();

        let request = ApiRequest::new("claude-3", 1000, messages);
        let result = client.complete(request).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AnthropicError::InvalidRequest { .. }));
        assert!(err.to_string().contains("Too many messages"));
    }

    #[tokio::test]
    async fn test_validate_request_message_too_large() {
        let server = MockServer::start().await;
        let client = create_mock_client(&server).await;

        let large_content = "x".repeat(MAX_CONTENT_LENGTH + 1);
        let messages = vec![ApiMessage::user(large_content)];

        let request = ApiRequest::new("claude-3", 1000, messages);
        let result = client.complete(request).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AnthropicError::InvalidRequest { .. }));
        assert!(err.to_string().contains("Message too large"));
    }

    // Successful request tests
    #[tokio::test]
    async fn test_complete_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .and(header("x-api-key", "test-api-key"))
            .and(header("anthropic-version", ANTHROPIC_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(success_response_body("Hello!")))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.raw_text, "Hello!");
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 20);
    }

    #[tokio::test]
    async fn test_complete_with_json_response() {
        let server = MockServer::start().await;

        let json_text = r#"{"result": "success", "data": 42}"#;
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response_body(json_text)),
            )
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Give me JSON")]);

        let result = client.complete(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.parsed.is_some());
        assert_eq!(response.parsed.unwrap()["result"], "success");
    }

    #[tokio::test]
    async fn test_complete_with_json_code_block() {
        let server = MockServer::start().await;

        let text = "Here is the JSON:\n```json\n{\"value\": 123}\n```\nDone.";
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success_response_body(text)))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("JSON please")]);

        let result = client.complete(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.parsed.is_some());
        assert_eq!(response.parsed.unwrap()["value"], 123);
    }

    #[tokio::test]
    async fn test_complete_with_thinking() {
        let server = MockServer::start().await;

        let response_body = json!({
            "id": "msg_123",
            "content": [
                {"type": "thinking", "thinking": "Let me think about this..."},
                {"type": "text", "text": "The answer is 42."}
            ],
            "model": "claude-3",
            "usage": {"input_tokens": 10, "output_tokens": 30},
            "stop_reason": "end_turn"
        });

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Think hard")])
            .with_thinking(ThinkingConfig::standard());

        let result = client.complete(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.raw_text, "The answer is 42.");
        assert!(response.thinking.is_some());
        assert_eq!(response.thinking.unwrap(), "Let me think about this...");
    }

    #[tokio::test]
    async fn test_complete_with_tool_use() {
        let server = MockServer::start().await;

        let response_body = json!({
            "id": "msg_123",
            "content": [
                {"type": "tool_use", "id": "tu_1", "name": "calculator", "input": {"operation": "add", "a": 1, "b": 2}}
            ],
            "model": "claude-3",
            "usage": {"input_tokens": 10, "output_tokens": 20},
            "stop_reason": "tool_use"
        });

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Add 1 and 2")]);

        let result = client.complete(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.tool_uses.len(), 1);
        assert_eq!(response.tool_uses[0].name, "calculator");
        assert_eq!(response.tool_uses[0].input["operation"], "add");
    }

    // Error handling tests
    #[tokio::test]
    async fn test_complete_auth_failure() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AnthropicError::AuthenticationFailed
        ));
    }

    #[tokio::test]
    async fn test_complete_rate_limited() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(
                ResponseTemplate::new(429)
                    .append_header("retry-after", "30")
                    .set_body_string("Rate limited"),
            )
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            AnthropicError::RateLimited {
                retry_after_seconds,
            } => {
                assert_eq!(retry_after_seconds, 30);
            }
            e => panic!("Wrong error type: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_complete_model_overloaded() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(529).set_body_string("Overloaded"))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-sonnet", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            AnthropicError::ModelOverloaded { model } => {
                assert_eq!(model, "claude-sonnet");
            }
            e => panic!("Wrong error type: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_complete_unexpected_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AnthropicError::UnexpectedResponse { .. }
        ));
    }

    #[tokio::test]
    async fn test_complete_empty_response() {
        let server = MockServer::start().await;

        let response_body = json!({
            "id": "msg_123",
            "content": [],
            "model": "claude-3",
            "usage": {"input_tokens": 10, "output_tokens": 0},
            "stop_reason": "end_turn"
        });

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AnthropicError::UnexpectedResponse { .. }
        ));
    }

    // Retry logic tests
    #[tokio::test]
    async fn test_retry_on_rate_limit() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = Arc::clone(&call_count);

        // Use a responder that returns 429 on first call, 200 on second
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    ResponseTemplate::new(429)
                } else {
                    ResponseTemplate::new(200).set_body_json(success_response_body("Success!"))
                }
            })
            .mount(&server)
            .await;

        let config = ClientConfig::default()
            .with_base_url(server.uri())
            .with_max_retries(1)
            .with_retry_delay_ms(10); // Fast retry for tests

        let client = AnthropicClient::new("test-key", config).unwrap();
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().raw_text, "Success!");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let server = MockServer::start().await;

        // All calls return 529
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(529))
            .mount(&server)
            .await;

        let config = ClientConfig::default()
            .with_base_url(server.uri())
            .with_max_retries(2)
            .with_retry_delay_ms(10);

        let client = AnthropicClient::new("test-key", config).unwrap();
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_err());
        // Should return the last error (ModelOverloaded)
        assert!(matches!(
            result.unwrap_err(),
            AnthropicError::ModelOverloaded { .. }
        ));
    }

    #[tokio::test]
    async fn test_no_retry_on_auth_failure() {
        let server = MockServer::start().await;

        // Auth failure should not be retried
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1) // Only called once
            .mount(&server)
            .await;

        let config = ClientConfig::default()
            .with_base_url(server.uri())
            .with_max_retries(3)
            .with_retry_delay_ms(10);

        let client = AnthropicClient::new("test-key", config).unwrap();
        let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);

        let result = client.complete(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AnthropicError::AuthenticationFailed
        ));
    }

    // JSON extraction tests
    #[test]
    fn test_extract_json_raw_valid() {
        let text = r#"{"key": "value", "num": 42}"#;
        let result = extract_json(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn test_extract_json_code_block() {
        let text = "Here is the result:\n```json\n{\"status\": \"ok\"}\n```\nDone!";
        let result = extract_json(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["status"], "ok");
    }

    #[test]
    fn test_extract_json_plain_code_block() {
        let text = "Result:\n```\n{\"value\": 123}\n```";
        let result = extract_json(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["value"], 123);
    }

    #[test]
    fn test_extract_json_invalid_returns_none() {
        let text = "This is just plain text with no JSON.";
        let result = extract_json(text);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_json_nested_code_block() {
        let text = "```json\n{\"nested\": {\"deep\": true}}\n```";
        let result = extract_json(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["nested"]["deep"], true);
    }

    #[test]
    fn test_extract_json_with_whitespace() {
        let text = "```json\n  {  \"key\"  :  \"value\"  }  \n```";
        let result = extract_json(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["key"], "value");
    }

    // Message content length tests
    #[test]
    fn test_message_content_len_text() {
        let msg = ApiMessage::user("Hello");
        assert_eq!(msg.content.len(), 5);
    }

    #[test]
    fn test_message_content_len_parts() {
        use crate::anthropic::types::ContentPart;

        let parts = vec![ContentPart::text("Hi"), ContentPart::text("There")];
        let msg = ApiMessage::user_multipart(parts);
        assert_eq!(msg.content.len(), 7);
    }

    // Client debug test
    #[test]
    fn test_client_debug() {
        let client = AnthropicClient::with_api_key("test-key").unwrap();
        let debug = format!("{:?}", client);
        assert!(debug.contains("AnthropicClient"));
    }
}
