//! Streaming support for the Anthropic API.
//!
//! This module provides:
//! - Server-Sent Events (SSE) parsing
//! - Stream event types
//! - Accumulator for building complete responses

#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::match_same_arms)]

use serde::Deserialize;

use super::types::{ApiUsage, StreamEvent};
use crate::error::AnthropicError;

/// Parse a Server-Sent Event line into a `StreamEvent`.
#[must_use]
pub fn parse_sse_line(line: &str) -> Option<Result<StreamEvent, AnthropicError>> {
    // Skip empty lines and comments
    let line = line.trim();
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Extract data from "data: {...}" format
    if let Some(data) = line.strip_prefix("data: ") {
        if data == "[DONE]" {
            return None;
        }
        return Some(parse_event_data(data));
    }

    None
}

/// Parse the JSON data from an SSE event.
fn parse_event_data(data: &str) -> Result<StreamEvent, AnthropicError> {
    let event: RawStreamEvent =
        serde_json::from_str(data).map_err(|e| AnthropicError::UnexpectedResponse {
            message: format!("Failed to parse stream event: {e}"),
        })?;

    match event.type_.as_deref().unwrap_or("") {
        "message_start" => {
            let message_id = event
                .message
                .map(|m| m.id)
                .unwrap_or_else(|| "unknown".to_string());
            Ok(StreamEvent::MessageStart { message_id })
        }
        "content_block_start" => {
            let index = event.index.unwrap_or(0);
            let block_type = event
                .content_block
                .map(|b| b.type_)
                .unwrap_or_else(|| "text".to_string());
            Ok(StreamEvent::ContentBlockStart { index, block_type })
        }
        "content_block_delta" => {
            let index = event.index.unwrap_or(0);
            if let Some(delta) = event.delta {
                match delta.type_.as_deref().unwrap_or("") {
                    "text_delta" => {
                        let text = delta.text.unwrap_or_default();
                        Ok(StreamEvent::TextDelta { index, text })
                    }
                    "thinking_delta" => {
                        let thinking = delta.thinking.unwrap_or_default();
                        Ok(StreamEvent::ThinkingDelta { thinking })
                    }
                    // Ignore unknown delta types (signature_delta, input_json_delta, missing type, etc.)
                    _ => Ok(StreamEvent::Ignored),
                }
            } else {
                Err(AnthropicError::UnexpectedResponse {
                    message: "Missing delta in content_block_delta".to_string(),
                })
            }
        }
        "content_block_stop" => {
            let index = event.index.unwrap_or(0);
            Ok(StreamEvent::ContentBlockStop { index })
        }
        "message_delta" => {
            // This contains usage update, but we handle it in message_stop
            Ok(StreamEvent::ContentBlockStop { index: 0 })
        }
        "message_stop" => {
            let stop_reason = event
                .message
                .as_ref()
                .and_then(|m| m.stop_reason.clone())
                .or_else(|| event.delta.and_then(|d| d.stop_reason))
                .unwrap_or_else(|| "end_turn".to_string());

            let usage = event
                .usage
                .map(|u| ApiUsage::new(u.input_tokens.unwrap_or(0), u.output_tokens.unwrap_or(0)))
                .unwrap_or_default();

            Ok(StreamEvent::MessageStop { stop_reason, usage })
        }
        "error" => {
            let error = event
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Unknown error".to_string());
            Ok(StreamEvent::Error { error })
        }
        "ping" => Ok(StreamEvent::Ping),
        // Handle missing or empty type field gracefully
        "" => Ok(StreamEvent::Ignored),
        other => Err(AnthropicError::UnexpectedResponse {
            message: format!("Unknown event type: {other}"),
        }),
    }
}

/// Raw stream event from the API.
#[derive(Debug, Deserialize)]
struct RawStreamEvent {
    #[serde(rename = "type", default)]
    type_: Option<String>,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    message: Option<RawMessage>,
    #[serde(default)]
    content_block: Option<RawContentBlock>,
    #[serde(default)]
    delta: Option<RawDelta>,
    #[serde(default)]
    usage: Option<RawUsage>,
    #[serde(default)]
    error: Option<RawError>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    id: String,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawContentBlock {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Deserialize)]
struct RawDelta {
    #[serde(rename = "type", default)]
    type_: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawUsage {
    #[serde(default)]
    input_tokens: Option<u32>,
    #[serde(default)]
    output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawError {
    message: String,
}

/// Accumulator for building a complete response from stream events.
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    message_id: Option<String>,
    text_blocks: Vec<String>,
    thinking: Option<String>,
    usage: ApiUsage,
    stop_reason: Option<String>,
    current_text: String,
    current_thinking: String,
}

impl StreamAccumulator {
    /// Create a new accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream event.
    pub fn process(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::MessageStart { message_id } => {
                self.message_id = Some(message_id);
            }
            StreamEvent::ContentBlockStart {
                index: _,
                block_type: _,
            } => {
                // Block started, prepare for content
            }
            StreamEvent::TextDelta { index: _, text } => {
                self.current_text.push_str(&text);
            }
            StreamEvent::ThinkingDelta { thinking } => {
                self.current_thinking.push_str(&thinking);
            }
            StreamEvent::ContentBlockStop { index: _ } => {
                if !self.current_text.is_empty() {
                    self.text_blocks
                        .push(std::mem::take(&mut self.current_text));
                }
                if !self.current_thinking.is_empty() {
                    self.thinking = Some(std::mem::take(&mut self.current_thinking));
                }
            }
            StreamEvent::MessageStop { stop_reason, usage } => {
                self.stop_reason = Some(stop_reason);
                self.usage = usage;
            }
            StreamEvent::Error { error: _ } => {
                // Error event - caller should handle
            }
            StreamEvent::Ping => {
                // Keep-alive event - no action needed
            }
            StreamEvent::Ignored => {
                // Unknown delta type - no action needed
            }
        }
    }

    /// Get the accumulated text.
    #[must_use]
    pub fn text(&self) -> String {
        self.text_blocks.join("\n")
    }

    /// Get the accumulated thinking.
    #[must_use]
    pub fn thinking(&self) -> Option<&str> {
        self.thinking.as_deref()
    }

    /// Get the message ID.
    #[must_use]
    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    /// Get the stop reason.
    #[must_use]
    pub fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }

    /// Get the usage.
    #[must_use]
    pub const fn usage(&self) -> &ApiUsage {
        &self.usage
    }

    /// Check if the stream is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.stop_reason.is_some()
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

    // SSE line parsing tests
    #[test]
    fn test_parse_sse_empty_line() {
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line("   ").is_none());
    }

    #[test]
    fn test_parse_sse_comment() {
        assert!(parse_sse_line(": this is a comment").is_none());
    }

    #[test]
    fn test_parse_sse_done() {
        assert!(parse_sse_line("data: [DONE]").is_none());
    }

    #[test]
    fn test_parse_sse_message_start() {
        let line = r#"data: {"type": "message_start", "message": {"id": "msg_123"}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::MessageStart { message_id } => {
                assert_eq!(message_id, "msg_123");
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_sse_content_block_start() {
        let line = r#"data: {"type": "content_block_start", "index": 0, "content_block": {"type": "text"}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::ContentBlockStart { index, block_type } => {
                assert_eq!(index, 0);
                assert_eq!(block_type, "text");
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_sse_text_delta() {
        let line = r#"data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::TextDelta { index, text } => {
                assert_eq!(index, 0);
                assert_eq!(text, "Hello");
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_sse_thinking_delta() {
        let line = r#"data: {"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "Hmm..."}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::ThinkingDelta { thinking } => {
                assert_eq!(thinking, "Hmm...");
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_sse_content_block_stop() {
        let line = r#"data: {"type": "content_block_stop", "index": 0}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::ContentBlockStop { index } => {
                assert_eq!(index, 0);
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_sse_message_stop() {
        let line =
            r#"data: {"type": "message_stop", "usage": {"input_tokens": 10, "output_tokens": 20}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::MessageStop { stop_reason, usage } => {
                assert_eq!(stop_reason, "end_turn");
                assert_eq!(usage.input_tokens, 10);
                assert_eq!(usage.output_tokens, 20);
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_sse_error() {
        let line = r#"data: {"type": "error", "error": {"message": "Something went wrong"}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::Error { error } => {
                assert_eq!(error, "Something went wrong");
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_parse_sse_ping() {
        let line = r#"data: {"type": "ping"}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().unwrap(), StreamEvent::Ping));
    }

    #[test]
    fn test_parse_sse_unknown_delta_type_ignored() {
        // signature_delta and other unknown delta types should be ignored, not error
        let line = r#"data: {"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": "abc"}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().unwrap(), StreamEvent::Ignored));
    }

    #[test]
    fn test_parse_sse_missing_type_field() {
        // Events without a type field should be ignored
        let line = r#"data: {"index": 0, "delta": {"text": "hello"}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().unwrap(), StreamEvent::Ignored));
    }

    #[test]
    fn test_parse_sse_invalid_json() {
        let line = "data: not valid json";
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_parse_sse_unknown_event_type() {
        let line = r#"data: {"type": "unknown_type"}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_parse_sse_unknown_delta_type() {
        // Unknown delta types are now ignored instead of erroring
        let line = r#"data: {"type": "content_block_delta", "index": 0, "delta": {"type": "unknown_delta"}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert!(matches!(result.unwrap().unwrap(), StreamEvent::Ignored));
    }

    #[test]
    fn test_parse_sse_missing_delta() {
        let line = r#"data: {"type": "content_block_delta", "index": 0}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    // StreamAccumulator tests
    #[test]
    fn test_accumulator_new() {
        let acc = StreamAccumulator::new();
        assert!(acc.message_id().is_none());
        assert!(acc.text().is_empty());
        assert!(acc.thinking().is_none());
        assert!(!acc.is_complete());
    }

    #[test]
    fn test_accumulator_message_start() {
        let mut acc = StreamAccumulator::new();
        acc.process(StreamEvent::MessageStart {
            message_id: "msg_123".to_string(),
        });
        assert_eq!(acc.message_id(), Some("msg_123"));
    }

    #[test]
    fn test_accumulator_text_deltas() {
        let mut acc = StreamAccumulator::new();
        acc.process(StreamEvent::ContentBlockStart {
            index: 0,
            block_type: "text".to_string(),
        });
        acc.process(StreamEvent::TextDelta {
            index: 0,
            text: "Hello".to_string(),
        });
        acc.process(StreamEvent::TextDelta {
            index: 0,
            text: " World".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStop { index: 0 });

        assert_eq!(acc.text(), "Hello World");
    }

    #[test]
    fn test_accumulator_thinking_deltas() {
        let mut acc = StreamAccumulator::new();
        acc.process(StreamEvent::ContentBlockStart {
            index: 0,
            block_type: "thinking".to_string(),
        });
        acc.process(StreamEvent::ThinkingDelta {
            thinking: "Let me ".to_string(),
        });
        acc.process(StreamEvent::ThinkingDelta {
            thinking: "think...".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStop { index: 0 });

        assert_eq!(acc.thinking(), Some("Let me think..."));
    }

    #[test]
    fn test_accumulator_multiple_text_blocks() {
        let mut acc = StreamAccumulator::new();

        // First text block
        acc.process(StreamEvent::ContentBlockStart {
            index: 0,
            block_type: "text".to_string(),
        });
        acc.process(StreamEvent::TextDelta {
            index: 0,
            text: "First".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStop { index: 0 });

        // Second text block
        acc.process(StreamEvent::ContentBlockStart {
            index: 1,
            block_type: "text".to_string(),
        });
        acc.process(StreamEvent::TextDelta {
            index: 1,
            text: "Second".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStop { index: 1 });

        assert_eq!(acc.text(), "First\nSecond");
    }

    #[test]
    fn test_accumulator_message_stop() {
        let mut acc = StreamAccumulator::new();
        acc.process(StreamEvent::MessageStop {
            stop_reason: "end_turn".to_string(),
            usage: ApiUsage::new(100, 50),
        });

        assert!(acc.is_complete());
        assert_eq!(acc.stop_reason(), Some("end_turn"));
        assert_eq!(acc.usage().input_tokens, 100);
        assert_eq!(acc.usage().output_tokens, 50);
    }

    #[test]
    fn test_accumulator_full_stream() {
        let mut acc = StreamAccumulator::new();

        acc.process(StreamEvent::MessageStart {
            message_id: "msg_abc".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStart {
            index: 0,
            block_type: "thinking".to_string(),
        });
        acc.process(StreamEvent::ThinkingDelta {
            thinking: "Thinking...".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStop { index: 0 });
        acc.process(StreamEvent::ContentBlockStart {
            index: 1,
            block_type: "text".to_string(),
        });
        acc.process(StreamEvent::TextDelta {
            index: 1,
            text: "The answer is 42.".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStop { index: 1 });
        acc.process(StreamEvent::MessageStop {
            stop_reason: "end_turn".to_string(),
            usage: ApiUsage::new(10, 20),
        });

        assert_eq!(acc.message_id(), Some("msg_abc"));
        assert_eq!(acc.thinking(), Some("Thinking..."));
        assert_eq!(acc.text(), "The answer is 42.");
        assert_eq!(acc.stop_reason(), Some("end_turn"));
        assert!(acc.is_complete());
    }

    #[test]
    fn test_accumulator_error_event() {
        let mut acc = StreamAccumulator::new();
        acc.process(StreamEvent::Error {
            error: "Test error".to_string(),
        });
        // Error event doesn't change state, caller should handle
        assert!(!acc.is_complete());
    }

    #[test]
    fn test_accumulator_default() {
        let acc = StreamAccumulator::default();
        assert!(acc.text().is_empty());
        assert!(!acc.is_complete());
    }

    #[test]
    fn test_accumulator_debug() {
        let acc = StreamAccumulator::new();
        let debug = format!("{:?}", acc);
        assert!(debug.contains("StreamAccumulator"));
    }

    #[test]
    fn test_parse_sse_non_data_line() {
        // Lines that don't start with "data: " should return None
        assert!(parse_sse_line("event: message").is_none());
        assert!(parse_sse_line("id: 123").is_none());
        assert!(parse_sse_line("retry: 1000").is_none());
        assert!(parse_sse_line("random text").is_none());
    }

    #[test]
    fn test_parse_sse_content_block_stop_event() {
        let line = r#"data: {"type": "content_block_stop", "index": 0}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());

        match result.unwrap().unwrap() {
            StreamEvent::ContentBlockStop { index } => {
                assert_eq!(index, 0);
            }
            e => panic!("Wrong event type: {e:?}"),
        }
    }

    #[test]
    fn test_accumulator_content_block_stop() {
        let mut acc = StreamAccumulator::new();
        acc.process(StreamEvent::MessageStart {
            message_id: "msg_123".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStart {
            index: 0,
            block_type: "text".to_string(),
        });
        acc.process(StreamEvent::TextDelta {
            index: 0,
            text: "Hello".to_string(),
        });
        acc.process(StreamEvent::ContentBlockStop { index: 0 });

        // After content block stop, text should still be available
        assert_eq!(acc.text(), "Hello");
    }
}
