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
//! ```ignore
//! use mcp_reasoning::anthropic::{AnthropicClient, ApiRequest, ApiMessage};
//!
//! let client = AnthropicClient::with_api_key("sk-ant-xxx")?;
//! let request = ApiRequest::new("claude-3", 1000, vec![ApiMessage::user("Hello")]);
//! let response = client.complete(request).await?;
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
