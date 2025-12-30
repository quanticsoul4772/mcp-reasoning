//! Anthropic API request and response types.
//!
//! This module provides:
//! - Request types for the Messages API
//! - Response types including content blocks
//! - Extended thinking configuration
//! - Vision content support
//! - Streaming event types

#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::derive_partial_eq_without_eq)]

use serde::{Deserialize, Serialize};

/// Request to the Anthropic Messages API.
#[derive(Debug, Clone, Serialize)]
pub struct ApiRequest {
    /// Model identifier (e.g., "claude-sonnet-4-20250514").
    pub model: String,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature for sampling (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Conversation messages.
    pub messages: Vec<ApiMessage>,
    /// Extended thinking configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Tool definitions for agentic reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Enable streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl ApiRequest {
    /// Create a new API request with required fields.
    #[must_use]
    pub fn new(model: impl Into<String>, max_tokens: u32, messages: Vec<ApiMessage>) -> Self {
        Self {
            model: model.into(),
            max_tokens,
            temperature: None,
            system: None,
            messages,
            thinking: None,
            tools: None,
            tool_choice: None,
            stream: None,
        }
    }

    /// Set temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set system prompt.
    #[must_use]
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set thinking configuration.
    #[must_use]
    pub fn with_thinking(mut self, thinking: ThinkingConfig) -> Self {
        self.thinking = Some(thinking);
        self
    }

    /// Set tools.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set tool choice.
    #[must_use]
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Enable streaming.
    #[must_use]
    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiMessage {
    /// Role: "user" or "assistant".
    pub role: String,
    /// Message content.
    pub content: MessageContent,
}

impl ApiMessage {
    /// Create a user message with text content.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create an assistant message with text content.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a user message with multiple content parts (for vision).
    #[must_use]
    pub fn user_multipart(parts: Vec<ContentPart>) -> Self {
        Self {
            role: "user".to_string(),
            content: MessageContent::Parts(parts),
        }
    }
}

/// Message content - either simple text or multipart.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Multi-part content (for images).
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Get content length in characters.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Parts(parts) => parts.iter().map(ContentPart::len).sum(),
        }
    }

    /// Check if content is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Content part for multimodal messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ContentPart {
    /// Text content.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// Image content.
    #[serde(rename = "image")]
    Image {
        /// Image source.
        source: ImageSource,
    },
}

impl ContentPart {
    /// Create a text content part.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image content part from base64 data.
    #[must_use]
    pub fn image_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
        }
    }

    /// Create an image content part from URL.
    #[must_use]
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Url { url: url.into() },
        }
    }

    /// Get content length for size validation.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Text { text } => text.len(),
            Self::Image { source } => source.len(),
        }
    }

    /// Check if content is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Image source for vision content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ImageSource {
    /// Base64 encoded image data.
    #[serde(rename = "base64")]
    Base64 {
        /// MIME type (e.g., "image/png").
        media_type: String,
        /// Base64 encoded data.
        data: String,
    },
    /// URL to image.
    #[serde(rename = "url")]
    Url {
        /// Image URL.
        url: String,
    },
}

impl ImageSource {
    /// Get length for size validation.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Base64 { data, .. } => data.len(),
            Self::Url { url } => url.len(),
        }
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Extended thinking configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingConfig {
    /// Type - always "enabled".
    #[serde(rename = "type")]
    pub type_: String,
    /// Token budget for thinking (minimum 1024).
    pub budget_tokens: u32,
}

/// Minimum allowed thinking budget.
pub const MIN_THINKING_BUDGET: u32 = 1024;
/// Standard thinking budget (4096 tokens).
pub const STANDARD_THINKING_BUDGET: u32 = 4096;
/// Deep thinking budget (8192 tokens).
pub const DEEP_THINKING_BUDGET: u32 = 8192;
/// Maximum thinking budget (16384 tokens).
pub const MAXIMUM_THINKING_BUDGET: u32 = 16384;

impl ThinkingConfig {
    /// Create thinking config with specified budget.
    /// Budget is clamped to minimum of 1024.
    #[must_use]
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            type_: "enabled".to_string(),
            budget_tokens: budget_tokens.max(MIN_THINKING_BUDGET),
        }
    }

    /// Standard budget (4096) for reflection/analysis modes.
    #[must_use]
    pub fn standard() -> Self {
        Self::enabled(STANDARD_THINKING_BUDGET)
    }

    /// Deep budget (8192) for complex decision/evidence modes.
    #[must_use]
    pub fn deep() -> Self {
        Self::enabled(DEEP_THINKING_BUDGET)
    }

    /// Maximum budget (16384) for counterfactual/mcts modes.
    #[must_use]
    pub fn maximum() -> Self {
        Self::enabled(MAXIMUM_THINKING_BUDGET)
    }
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Tool definition for agentic reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON Schema for input parameters.
    pub input_schema: serde_json::Value,
}

impl ToolDefinition {
    /// Create a new tool definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Tool choice configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ToolChoice {
    /// Let model decide.
    #[serde(rename = "auto")]
    Auto,
    /// Require tool use.
    #[serde(rename = "any")]
    Any,
    /// Use specific tool.
    #[serde(rename = "tool")]
    Specific {
        /// Tool name.
        name: String,
    },
}

impl ToolChoice {
    /// Create auto choice.
    #[must_use]
    pub const fn auto() -> Self {
        Self::Auto
    }

    /// Create any choice.
    #[must_use]
    pub const fn any() -> Self {
        Self::Any
    }

    /// Create specific tool choice.
    #[must_use]
    pub fn specific(name: impl Into<String>) -> Self {
        Self::Specific { name: name.into() }
    }
}

/// Response from the Anthropic Messages API.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    /// Unique message ID.
    pub id: String,
    /// Content blocks in the response.
    pub content: Vec<ContentBlock>,
    /// Model used.
    pub model: String,
    /// Token usage.
    pub usage: ApiUsage,
    /// Reason the response stopped.
    pub stop_reason: String,
}

/// Content block in an API response.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text content.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// Thinking content (extended thinking).
    #[serde(rename = "thinking")]
    Thinking {
        /// The thinking content.
        thinking: String,
    },
    /// Tool use block.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Tool use ID.
        id: String,
        /// Tool name.
        name: String,
        /// Tool input.
        input: serde_json::Value,
    },
}

impl ContentBlock {
    /// Get text content if this is a text block.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Get thinking content if this is a thinking block.
    #[must_use]
    pub fn as_thinking(&self) -> Option<&str> {
        match self {
            Self::Thinking { thinking } => Some(thinking),
            _ => None,
        }
    }

    /// Get tool use if this is a tool use block.
    #[must_use]
    pub fn as_tool_use(&self) -> Option<(&str, &str, &serde_json::Value)> {
        match self {
            Self::ToolUse { id, name, input } => Some((id, name, input)),
            _ => None,
        }
    }
}

/// Token usage in API response.
#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
pub struct ApiUsage {
    /// Input tokens consumed.
    pub input_tokens: u32,
    /// Output tokens generated.
    pub output_tokens: u32,
}

impl ApiUsage {
    /// Create new usage.
    #[must_use]
    pub const fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    /// Get total tokens.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Processed reasoning response.
#[derive(Debug, Clone)]
pub struct ReasoningResponse {
    /// Raw text from the response.
    pub raw_text: String,
    /// Parsed JSON (if extraction succeeded).
    pub parsed: Option<serde_json::Value>,
    /// Token usage.
    pub usage: ApiUsage,
    /// Extended thinking content (if present).
    pub thinking: Option<String>,
    /// Tool use blocks (if any).
    pub tool_uses: Vec<ToolUseResult>,
}

impl ReasoningResponse {
    /// Create a new reasoning response.
    #[must_use]
    pub fn new(raw_text: impl Into<String>, usage: ApiUsage) -> Self {
        Self {
            raw_text: raw_text.into(),
            parsed: None,
            usage,
            thinking: None,
            tool_uses: Vec::new(),
        }
    }

    /// Set parsed JSON.
    #[must_use]
    pub fn with_parsed(mut self, parsed: serde_json::Value) -> Self {
        self.parsed = Some(parsed);
        self
    }

    /// Set thinking content.
    #[must_use]
    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.thinking = Some(thinking.into());
        self
    }

    /// Add tool use result.
    #[must_use]
    pub fn with_tool_use(mut self, tool_use: ToolUseResult) -> Self {
        self.tool_uses.push(tool_use);
        self
    }
}

/// Tool use result from API response.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolUseResult {
    /// Tool use ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Tool input.
    pub input: serde_json::Value,
}

impl ToolUseResult {
    /// Create new tool use result.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
        }
    }
}

/// Streaming event from the API.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// Message started.
    MessageStart {
        /// Message ID.
        message_id: String,
    },
    /// Content block started.
    ContentBlockStart {
        /// Block index.
        index: usize,
        /// Block type.
        block_type: String,
    },
    /// Text delta received.
    TextDelta {
        /// Block index.
        index: usize,
        /// Text content.
        text: String,
    },
    /// Thinking delta received.
    ThinkingDelta {
        /// Thinking content.
        thinking: String,
    },
    /// Content block finished.
    ContentBlockStop {
        /// Block index.
        index: usize,
    },
    /// Message finished.
    MessageStop {
        /// Stop reason.
        stop_reason: String,
        /// Token usage.
        usage: ApiUsage,
    },
    /// Error occurred.
    Error {
        /// Error message.
        error: String,
    },
}

/// API error response body.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorBody {
    /// Error type.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error details.
    pub error: ApiErrorDetails,
}

/// API error details.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorDetails {
    /// Error type.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message.
    pub message: String,
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
    use serde_json::json;

    // ApiRequest tests
    #[test]
    fn test_api_request_new() {
        let messages = vec![ApiMessage::user("Hello")];
        let req = ApiRequest::new("claude-3", 1000, messages);

        assert_eq!(req.model, "claude-3");
        assert_eq!(req.max_tokens, 1000);
        assert!(req.temperature.is_none());
        assert!(req.system.is_none());
        assert_eq!(req.messages.len(), 1);
    }

    #[test]
    fn test_api_request_with_temperature() {
        let req = ApiRequest::new("claude-3", 1000, vec![]).with_temperature(0.7);
        assert_eq!(req.temperature, Some(0.7));
    }

    #[test]
    fn test_api_request_with_system() {
        let req = ApiRequest::new("claude-3", 1000, vec![]).with_system("You are helpful");
        assert_eq!(req.system, Some("You are helpful".to_string()));
    }

    #[test]
    fn test_api_request_with_thinking() {
        let req = ApiRequest::new("claude-3", 1000, vec![]).with_thinking(ThinkingConfig::deep());
        assert!(req.thinking.is_some());
        assert_eq!(req.thinking.unwrap().budget_tokens, DEEP_THINKING_BUDGET);
    }

    #[test]
    fn test_api_request_with_tools() {
        let tool = ToolDefinition::new("test", "desc", json!({}));
        let req = ApiRequest::new("claude-3", 1000, vec![]).with_tools(vec![tool]);
        assert!(req.tools.is_some());
        assert_eq!(req.tools.unwrap().len(), 1);
    }

    #[test]
    fn test_api_request_with_tool_choice() {
        let req = ApiRequest::new("claude-3", 1000, vec![]).with_tool_choice(ToolChoice::auto());
        assert!(req.tool_choice.is_some());
    }

    #[test]
    fn test_api_request_with_streaming() {
        let req = ApiRequest::new("claude-3", 1000, vec![]).with_streaming(true);
        assert_eq!(req.stream, Some(true));
    }

    #[test]
    fn test_api_request_serialization() {
        let req = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hi")]);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("claude-3"));
        assert!(json.contains("1000"));
    }

    // ApiMessage tests
    #[test]
    fn test_api_message_user() {
        let msg = ApiMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, MessageContent::Text("Hello".to_string()));
    }

    #[test]
    fn test_api_message_assistant() {
        let msg = ApiMessage::assistant("Hi there");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, MessageContent::Text("Hi there".to_string()));
    }

    #[test]
    fn test_api_message_user_multipart() {
        let parts = vec![
            ContentPart::text("Describe this"),
            ContentPart::image_url("http://example.com/image.png"),
        ];
        let msg = ApiMessage::user_multipart(parts);
        assert_eq!(msg.role, "user");
        match msg.content {
            MessageContent::Parts(p) => assert_eq!(p.len(), 2),
            MessageContent::Text(_) => panic!("Expected Parts"),
        }
    }

    // MessageContent tests
    #[test]
    fn test_message_content_text_len() {
        let content = MessageContent::Text("Hello".to_string());
        assert_eq!(content.len(), 5);
        assert!(!content.is_empty());
    }

    #[test]
    fn test_message_content_parts_len() {
        let content =
            MessageContent::Parts(vec![ContentPart::text("Hi"), ContentPart::text("There")]);
        assert_eq!(content.len(), 7);
    }

    #[test]
    fn test_message_content_empty() {
        let content = MessageContent::Text(String::new());
        assert!(content.is_empty());
    }

    // ContentPart tests
    #[test]
    fn test_content_part_text() {
        let part = ContentPart::text("Hello");
        assert_eq!(part.len(), 5);
        assert!(!part.is_empty());
    }

    #[test]
    fn test_content_part_image_base64() {
        let part = ContentPart::image_base64("image/png", "abc123");
        assert_eq!(part.len(), 6); // base64 data length
    }

    #[test]
    fn test_content_part_image_url() {
        let part = ContentPart::image_url("http://example.com/img.png");
        assert!(!part.is_empty());
    }

    #[test]
    fn test_content_part_text_serialization() {
        let part = ContentPart::text("Hello");
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_content_part_image_base64_serialization() {
        let part = ContentPart::image_base64("image/png", "abc123");
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"image\""));
        assert!(json.contains("\"type\":\"base64\""));
        assert!(json.contains("\"media_type\":\"image/png\""));
    }

    #[test]
    fn test_content_part_image_url_serialization() {
        let part = ContentPart::image_url("http://example.com/img.png");
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"image\""));
        assert!(json.contains("\"type\":\"url\""));
    }

    // ImageSource tests
    #[test]
    fn test_image_source_base64_len() {
        let source = ImageSource::Base64 {
            media_type: "image/png".to_string(),
            data: "abc123".to_string(),
        };
        assert_eq!(source.len(), 6);
        assert!(!source.is_empty());
    }

    #[test]
    fn test_image_source_url_len() {
        let source = ImageSource::Url {
            url: "http://example.com".to_string(),
        };
        assert_eq!(source.len(), 18);
    }

    // ThinkingConfig tests
    #[test]
    fn test_thinking_config_enabled() {
        let config = ThinkingConfig::enabled(2048);
        assert_eq!(config.type_, "enabled");
        assert_eq!(config.budget_tokens, 2048);
    }

    #[test]
    fn test_thinking_config_minimum_enforced() {
        let config = ThinkingConfig::enabled(500);
        assert_eq!(config.budget_tokens, MIN_THINKING_BUDGET);
    }

    #[test]
    fn test_thinking_config_standard() {
        let config = ThinkingConfig::standard();
        assert_eq!(config.budget_tokens, STANDARD_THINKING_BUDGET);
    }

    #[test]
    fn test_thinking_config_deep() {
        let config = ThinkingConfig::deep();
        assert_eq!(config.budget_tokens, DEEP_THINKING_BUDGET);
    }

    #[test]
    fn test_thinking_config_maximum() {
        let config = ThinkingConfig::maximum();
        assert_eq!(config.budget_tokens, MAXIMUM_THINKING_BUDGET);
    }

    #[test]
    fn test_thinking_config_default() {
        let config = ThinkingConfig::default();
        assert_eq!(config.budget_tokens, STANDARD_THINKING_BUDGET);
    }

    #[test]
    fn test_thinking_config_serialization() {
        let config = ThinkingConfig::standard();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"enabled\""));
        assert!(json.contains("\"budget_tokens\":4096"));
    }

    // ToolDefinition tests
    #[test]
    fn test_tool_definition_new() {
        let schema = json!({"type": "object"});
        let tool = ToolDefinition::new("test_tool", "A test tool", schema.clone());
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
        assert_eq!(tool.input_schema, schema);
    }

    // ToolChoice tests
    #[test]
    fn test_tool_choice_auto() {
        let choice = ToolChoice::auto();
        assert!(matches!(choice, ToolChoice::Auto));
    }

    #[test]
    fn test_tool_choice_any() {
        let choice = ToolChoice::any();
        assert!(matches!(choice, ToolChoice::Any));
    }

    #[test]
    fn test_tool_choice_specific() {
        let choice = ToolChoice::specific("my_tool");
        match choice {
            ToolChoice::Specific { name } => assert_eq!(name, "my_tool"),
            _ => panic!("Expected Specific"),
        }
    }

    #[test]
    fn test_tool_choice_serialization() {
        let auto = serde_json::to_string(&ToolChoice::auto()).unwrap();
        assert!(auto.contains("\"type\":\"auto\""));

        let any = serde_json::to_string(&ToolChoice::any()).unwrap();
        assert!(any.contains("\"type\":\"any\""));

        let specific = serde_json::to_string(&ToolChoice::specific("test")).unwrap();
        assert!(specific.contains("\"type\":\"tool\""));
        assert!(specific.contains("\"name\":\"test\""));
    }

    // ApiResponse tests
    #[test]
    fn test_api_response_deserialization() {
        let json = r#"{
            "id": "msg_123",
            "content": [{"type": "text", "text": "Hello"}],
            "model": "claude-3",
            "usage": {"input_tokens": 10, "output_tokens": 5},
            "stop_reason": "end_turn"
        }"#;
        let response: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "msg_123");
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.model, "claude-3");
        assert_eq!(response.stop_reason, "end_turn");
    }

    // ContentBlock tests
    #[test]
    fn test_content_block_text_deserialization() {
        let json = r#"{"type": "text", "text": "Hello"}"#;
        let block: ContentBlock = serde_json::from_str(json).unwrap();
        assert_eq!(block.as_text(), Some("Hello"));
    }

    #[test]
    fn test_content_block_thinking_deserialization() {
        let json = r#"{"type": "thinking", "thinking": "Let me think..."}"#;
        let block: ContentBlock = serde_json::from_str(json).unwrap();
        assert_eq!(block.as_thinking(), Some("Let me think..."));
    }

    #[test]
    fn test_content_block_tool_use_deserialization() {
        let json = r#"{"type": "tool_use", "id": "tu_1", "name": "test", "input": {}}"#;
        let block: ContentBlock = serde_json::from_str(json).unwrap();
        let (id, name, input) = block.as_tool_use().unwrap();
        assert_eq!(id, "tu_1");
        assert_eq!(name, "test");
        assert_eq!(*input, json!({}));
    }

    #[test]
    fn test_content_block_as_methods() {
        let text = ContentBlock::Text {
            text: "Hello".to_string(),
        };
        assert!(text.as_text().is_some());
        assert!(text.as_thinking().is_none());
        assert!(text.as_tool_use().is_none());

        let thinking = ContentBlock::Thinking {
            thinking: "Hmm".to_string(),
        };
        assert!(thinking.as_text().is_none());
        assert!(thinking.as_thinking().is_some());
        assert!(thinking.as_tool_use().is_none());
    }

    // ApiUsage tests
    #[test]
    fn test_api_usage_new() {
        let usage = ApiUsage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_api_usage_total() {
        let usage = ApiUsage::new(100, 50);
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn test_api_usage_default() {
        let usage = ApiUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
    }

    // ReasoningResponse tests
    #[test]
    fn test_reasoning_response_new() {
        let usage = ApiUsage::new(10, 20);
        let response = ReasoningResponse::new("Hello", usage);
        assert_eq!(response.raw_text, "Hello");
        assert!(response.parsed.is_none());
        assert!(response.thinking.is_none());
        assert!(response.tool_uses.is_empty());
    }

    #[test]
    fn test_reasoning_response_with_parsed() {
        let usage = ApiUsage::new(10, 20);
        let response = ReasoningResponse::new("Hello", usage).with_parsed(json!({"key": "value"}));
        assert!(response.parsed.is_some());
    }

    #[test]
    fn test_reasoning_response_with_thinking() {
        let usage = ApiUsage::new(10, 20);
        let response = ReasoningResponse::new("Hello", usage).with_thinking("I'm thinking...");
        assert_eq!(response.thinking, Some("I'm thinking...".to_string()));
    }

    #[test]
    fn test_reasoning_response_with_tool_use() {
        let usage = ApiUsage::new(10, 20);
        let tool_use = ToolUseResult::new("id1", "tool1", json!({}));
        let response = ReasoningResponse::new("Hello", usage).with_tool_use(tool_use);
        assert_eq!(response.tool_uses.len(), 1);
    }

    // ToolUseResult tests
    #[test]
    fn test_tool_use_result_new() {
        let result = ToolUseResult::new("id1", "my_tool", json!({"arg": "value"}));
        assert_eq!(result.id, "id1");
        assert_eq!(result.name, "my_tool");
        assert_eq!(result.input, json!({"arg": "value"}));
    }

    // StreamEvent tests
    #[test]
    fn test_stream_event_message_start() {
        let event = StreamEvent::MessageStart {
            message_id: "msg_1".to_string(),
        };
        match event {
            StreamEvent::MessageStart { message_id } => assert_eq!(message_id, "msg_1"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_text_delta() {
        let event = StreamEvent::TextDelta {
            index: 0,
            text: "Hello".to_string(),
        };
        match event {
            StreamEvent::TextDelta { index, text } => {
                assert_eq!(index, 0);
                assert_eq!(text, "Hello");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_message_stop() {
        let event = StreamEvent::MessageStop {
            stop_reason: "end_turn".to_string(),
            usage: ApiUsage::new(10, 20),
        };
        match event {
            StreamEvent::MessageStop { stop_reason, usage } => {
                assert_eq!(stop_reason, "end_turn");
                assert_eq!(usage.total(), 30);
            }
            _ => panic!("Wrong variant"),
        }
    }

    // ApiErrorBody tests
    #[test]
    fn test_api_error_body_deserialization() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "Invalid request"
            }
        }"#;
        let error: ApiErrorBody = serde_json::from_str(json).unwrap();
        assert_eq!(error.error_type, "error");
        assert_eq!(error.error.error_type, "invalid_request_error");
        assert_eq!(error.error.message, "Invalid request");
    }
}
