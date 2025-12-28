//! Anthropic API client.
//!
//! This module provides:
//! - Direct Claude API integration
//! - Retry logic with exponential backoff
//! - Extended thinking support
//! - Streaming response handling
//!
//! # Architecture
//!
//! The client uses `reqwest` for HTTP and supports:
//! - Request validation with size limits
//! - Retry with exponential backoff
//! - Server-Sent Events for streaming
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::anthropic::{ApiMessage, ApiRequest, ClientConfig};
//! use mcp_reasoning::doctest_helpers::{MockClient, block_on};
//! use mcp_reasoning::traits::{AnthropicClientTrait, Message, CompletionConfig};
//!
//! // In production, use AnthropicClient::with_api_key("sk-ant-xxx")
//! // For doctests, we use MockClient to avoid real API calls
//! let client = MockClient::with_response(r#"{"result": "Hello!"}"#);
//!
//! block_on(async {
//!     let messages = vec![Message::user("Hello")];
//!     let config = CompletionConfig::new().with_max_tokens(1000);
//!     let response = client.complete(messages, config).await.unwrap();
//!     assert!(!response.content.is_empty());
//! });
//! ```

mod client;
mod config;
mod streaming;
mod types;

pub use client::{AnthropicClient, MAX_CONTENT_LENGTH, MAX_MESSAGES, MAX_REQUEST_BYTES};
pub use config::{
    ClientConfig, ModeConfig, DEFAULT_BASE_URL, DEFAULT_MAX_RETRIES, DEFAULT_MAX_TOKENS,
    DEFAULT_MODEL, DEFAULT_RETRY_DELAY_MS, DEFAULT_TIMEOUT_MS,
};
pub use streaming::{parse_sse_line, StreamAccumulator};
pub use types::{
    ApiErrorBody, ApiErrorDetails, ApiMessage, ApiRequest, ApiResponse, ApiUsage, ContentBlock,
    ContentPart, ImageSource, MessageContent, ReasoningResponse, StreamEvent, ThinkingConfig,
    ToolChoice, ToolDefinition, ToolUseResult, DEEP_THINKING_BUDGET, MAXIMUM_THINKING_BUDGET,
    MIN_THINKING_BUDGET, STANDARD_THINKING_BUDGET,
};
