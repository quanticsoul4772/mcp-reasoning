//! Enhanced error messages with contextual alternatives.
//!
//! This module provides error enhancement for better agent experience:
//! - Categorizes errors for machine parsing
//! - Suggests alternative approaches on failure
//! - Provides context-aware recovery guidance

use serde::{Deserialize, Serialize};

/// Metrics about request complexity for error context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Length of content in bytes.
    pub content_length: usize,
    /// Depth of operation (e.g., graph depth, iteration count).
    pub operation_depth: Option<u32>,
    /// Branching factor (e.g., num_perspectives, num_branches).
    pub branching_factor: Option<u32>,
}

impl ComplexityMetrics {
    /// Build metrics from a request's content length (the common case).
    #[must_use]
    pub const fn from_content(content_length: usize) -> Self {
        Self {
            content_length,
            operation_depth: None,
            branching_factor: None,
        }
    }

    /// Attach a branching factor (e.g. `num_branches` / `num_perspectives`).
    #[must_use]
    pub const fn with_branching(mut self, branching_factor: Option<u32>) -> Self {
        self.branching_factor = branching_factor;
        self
    }
}

/// Enhanced error with recovery suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedError {
    /// Original error message.
    pub error: String,
    /// Error category for machine parsing.
    pub category: ErrorCategory,
    /// Suggested alternatives.
    pub alternatives: Vec<Alternative>,
    /// Context that helps with recovery.
    pub context: Option<ErrorContext>,
}

/// Category of error for machine parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Request timeout.
    Timeout,
    /// Rate limit exceeded.
    RateLimit,
    /// Authentication failure.
    Authentication,
    /// Invalid request parameters.
    InvalidRequest,
    /// API unavailable.
    ApiUnavailable,
    /// Storage/database error.
    Storage,
    /// Other/unknown error.
    Other,
}

/// A suggested alternative approach.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    /// Alternative tool or approach.
    pub suggestion: String,
    /// Why this might work better.
    pub reason: String,
    /// Estimated duration if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,
}

/// Context about the failed operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Tool that failed.
    pub failed_tool: String,
    /// Operation that failed (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_operation: Option<String>,
    /// Request complexity metrics.
    pub complexity: ComplexityMetrics,
    /// Timeout used in milliseconds.
    pub timeout_ms: u64,
}

/// Enhances errors with contextual alternatives.
pub struct ErrorEnhancer;

impl Default for ErrorEnhancer {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorEnhancer {
    /// Create a new error enhancer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Enhance an error with contextual alternatives.
    #[must_use]
    pub fn enhance(&self, error_message: &str, context: ErrorContext) -> EnhancedError {
        let category = self.categorize_error(error_message);
        let alternatives = self.generate_alternatives(&category, &context);

        EnhancedError {
            error: error_message.to_string(),
            category,
            alternatives,
            context: Some(context),
        }
    }

    /// Categorize an error based on its message.
    fn categorize_error(&self, error_message: &str) -> ErrorCategory {
        let msg = error_message.to_lowercase();

        if msg.contains("timeout") || msg.contains("timed out") {
            ErrorCategory::Timeout
        } else if msg.contains("rate limit") || msg.contains("too many requests") {
            ErrorCategory::RateLimit
        } else if msg.contains("authentication") || msg.contains("api key") {
            ErrorCategory::Authentication
        } else if msg.contains("invalid")
            || msg.contains("bad request")
            || msg.contains("missing required")
            || msg.contains("required but not provided")
            || msg.contains("not found")
        {
            // Incorrect-parameter failures: bad value/operation, a missing or
            // required field, or a bad id (session/branch/thought "not found").
            ErrorCategory::InvalidRequest
        } else if msg.contains("unavailable") || msg.contains("service error") {
            ErrorCategory::ApiUnavailable
        } else if msg.contains("storage") || msg.contains("database") {
            ErrorCategory::Storage
        } else {
            ErrorCategory::Other
        }
    }

    /// Generate alternatives based on error category and context.
    fn generate_alternatives(
        &self,
        category: &ErrorCategory,
        context: &ErrorContext,
    ) -> Vec<Alternative> {
        match category {
            ErrorCategory::Timeout => self.timeout_alternatives(context),
            ErrorCategory::RateLimit => self.rate_limit_alternatives(),
            ErrorCategory::ApiUnavailable => self.unavailable_alternatives(),
            ErrorCategory::InvalidRequest => self.invalid_request_alternatives(context),
            ErrorCategory::Authentication => self.authentication_alternatives(),
            _ => vec![],
        }
    }

    /// Generate alternatives for authentication errors.
    fn authentication_alternatives(&self) -> Vec<Alternative> {
        vec![
            Alternative {
                suggestion: "Verify ANTHROPIC_API_KEY is set and valid".into(),
                reason: "Authentication failed — the API key may be missing, expired, or malformed"
                    .into(),
                estimated_duration_ms: None,
            },
            Alternative {
                suggestion: "Re-export the key and restart the server".into(),
                reason: "The server reads ANTHROPIC_API_KEY at startup".into(),
                estimated_duration_ms: None,
            },
        ]
    }

    /// Generate alternatives for timeout errors.
    fn timeout_alternatives(&self, ctx: &ErrorContext) -> Vec<Alternative> {
        let mut alts = vec![];

        // Suggest faster tool if available
        match ctx.failed_tool.as_str() {
            "reasoning_divergent" => {
                alts.push(Alternative {
                    suggestion: "Use reasoning_linear instead".into(),
                    reason: "Completes faster than divergent mode".into(),
                    estimated_duration_ms: Some(12_000),
                });
            }
            "reasoning_graph" => {
                alts.push(Alternative {
                    suggestion: "Use reasoning_tree with 2-3 branches".into(),
                    reason: "Similar exploration but faster execution".into(),
                    estimated_duration_ms: Some(18_000),
                });
            }
            "reasoning_mcts" => {
                alts.push(Alternative {
                    suggestion: "Use reasoning_tree for simpler exploration".into(),
                    reason: "Tree exploration without Monte Carlo simulation".into(),
                    estimated_duration_ms: Some(15_000),
                });
            }
            _ => {}
        }

        // Suggest breaking down if content is large
        if ctx.complexity.content_length > 10_000 {
            alts.push(Alternative {
                suggestion: "Break content into 2-3 smaller reasoning_linear calls".into(),
                reason: format!(
                    "Content length {} bytes is high. Splitting may help.",
                    ctx.complexity.content_length
                ),
                estimated_duration_ms: Some(8_000 * 3),
            });
        }

        // Suggest reducing branching if the fan-out is wide (num_branches /
        // num_perspectives). Each branch is an additional generation step.
        if let Some(branching) = ctx.complexity.branching_factor {
            if branching > 4 {
                alts.push(Alternative {
                    suggestion: "Reduce branching (num_branches / num_perspectives) to 2-3".into(),
                    reason: format!(
                        "Branching factor {branching} is high. Each branch adds a generation step."
                    ),
                    estimated_duration_ms: None,
                });
            }
        }

        // Always suggest auto mode
        alts.push(Alternative {
            suggestion: "Use reasoning_auto to select optimal mode".into(),
            reason: "Automatically routes to the best mode for complexity".into(),
            estimated_duration_ms: Some(15_000),
        });

        // Suggest longer timeout if current is short
        if ctx.timeout_ms < 60_000 {
            alts.push(Alternative {
                suggestion: "Request longer timeout".into(),
                reason: format!(
                    "Current timeout ({}ms) may be too short. Try 60s or 120s.",
                    ctx.timeout_ms
                ),
                estimated_duration_ms: None,
            });
        }

        alts
    }

    /// Generate alternatives for rate limit errors.
    fn rate_limit_alternatives(&self) -> Vec<Alternative> {
        vec![
            Alternative {
                suggestion: "Wait and retry".into(),
                reason: "Rate limit will reset after a short wait".into(),
                estimated_duration_ms: None,
            },
            Alternative {
                suggestion: "Use reasoning_checkpoint to save progress".into(),
                reason: "Save current state before retrying".into(),
                estimated_duration_ms: Some(100),
            },
        ]
    }

    /// Generate alternatives for API unavailable errors.
    fn unavailable_alternatives(&self) -> Vec<Alternative> {
        vec![
            Alternative {
                suggestion: "Retry with exponential backoff".into(),
                reason: "API may be temporarily unavailable".into(),
                estimated_duration_ms: None,
            },
            Alternative {
                suggestion: "Check reasoning_metrics for historical patterns".into(),
                reason: "Review past successful patterns".into(),
                estimated_duration_ms: None,
            },
        ]
    }

    /// Generate alternatives for invalid request errors.
    fn invalid_request_alternatives(&self, ctx: &ErrorContext) -> Vec<Alternative> {
        let mut alts = vec![];

        // Incorrect parameters: show the correct call with example values so the
        // caller can copy a working invocation instead of guessing the schema.
        if let Some(example) = Self::example_call(&ctx.failed_tool, ctx.failed_operation.as_deref())
        {
            alts.push(Alternative {
                suggestion: "Retry with corrected parameters".into(),
                reason: format!("Example: {example}"),
                estimated_duration_ms: None,
            });
        }

        if ctx.complexity.content_length > 50_000 {
            alts.push(Alternative {
                suggestion: "Reduce content length".into(),
                reason: format!(
                    "Content length {} bytes may exceed limits",
                    ctx.complexity.content_length
                ),
                estimated_duration_ms: None,
            });
        }

        if let Some(depth) = ctx.complexity.operation_depth {
            if depth > 10 {
                alts.push(Alternative {
                    suggestion: "Reduce operation depth".into(),
                    reason: format!("Depth {} may be too deep, try reducing", depth),
                    estimated_duration_ms: None,
                });
            }
        }

        alts
    }

    /// A correctly-shaped example invocation for a tool/operation, used to show
    /// "the correct call with example values" on an incorrect-parameter failure.
    /// Returns `None` for tools we have no canonical example for, so the caller
    /// never sees a misleading example.
    fn example_call(failed_tool: &str, failed_operation: Option<&str>) -> Option<String> {
        let op = failed_operation.unwrap_or("");
        let args = match (failed_tool, op) {
            ("reasoning_linear", _) => r#"{ "content": "Explain X step by step" }"#,
            ("reasoning_tree", "focus") => {
                r#"{ "operation": "focus", "session_id": "<id from create>", "branch_id": "<id from list>" }"#
            }
            ("reasoning_tree", "complete") => {
                r#"{ "operation": "complete", "session_id": "<id from create>", "branch_id": "<id from list>", "completed": true }"#
            }
            ("reasoning_tree", "list" | "summarize") => {
                r#"{ "operation": "list", "session_id": "<id from create>" }"#
            }
            ("reasoning_tree", _) => {
                r#"{ "operation": "create", "content": "Compare options A and B", "num_branches": 3 }"#
            }
            ("reasoning_divergent", _) => {
                r#"{ "content": "Critique this plan", "num_perspectives": 4 }"#
            }
            ("reasoning_reflection", "evaluate") => {
                r#"{ "operation": "evaluate", "session_id": "<id from a prior session>" }"#
            }
            ("reasoning_reflection", _) => {
                r#"{ "operation": "process", "content": "Reasoning to refine" }"#
            }
            ("reasoning_auto", _) => r#"{ "content": "Problem to route" }"#,
            ("reasoning_meta", _) => r#"{ "content": "Problem to classify and route" }"#,
            ("reasoning_confidence_route", _) => {
                r#"{ "content": "Problem to route by confidence" }"#
            }
            ("reasoning_checkpoint", _) => {
                r#"{ "operation": "create", "session_id": "<id from a prior session>" }"#
            }
            ("reasoning_graph", _) => {
                r#"{ "operation": "init", "content": "Problem to explore as a graph" }"#
            }
            ("reasoning_detect", _) => {
                r#"{ "operation": "biases", "content": "Argument to analyze" }"#
            }
            ("reasoning_decision", _) => {
                r#"{ "operation": "weighted", "options": ["A", "B"], "criteria": ["cost", "speed"] }"#
            }
            ("reasoning_evidence", _) => {
                r#"{ "operation": "assess", "content": "Claim and supporting evidence" }"#
            }
            ("reasoning_timeline", _) => {
                r#"{ "operation": "create", "content": "Sequence of events to analyze" }"#
            }
            ("reasoning_mcts", _) => {
                r#"{ "operation": "explore", "content": "Decision space to search" }"#
            }
            ("reasoning_counterfactual", _) => {
                r#"{ "content": "Causal scenario: what if X had not happened?" }"#
            }
            _ => return None,
        };
        Some(format!("{failed_tool} {args}"))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    clippy::default_constructed_unit_structs
)]
mod tests {
    use super::*;

    fn create_test_context(tool: &str) -> ErrorContext {
        ErrorContext {
            failed_tool: tool.into(),
            failed_operation: None,
            complexity: ComplexityMetrics::default(),
            timeout_ms: 30_000,
        }
    }

    #[test]
    fn test_complexity_metrics_default() {
        let metrics = ComplexityMetrics::default();
        assert_eq!(metrics.content_length, 0);
        assert!(metrics.operation_depth.is_none());
        assert!(metrics.branching_factor.is_none());
    }

    #[test]
    fn test_error_enhancer_new() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Some error", ctx);
        assert_eq!(enhanced.error, "Some error");
    }

    #[test]
    fn test_error_enhancer_default() {
        let enhancer = ErrorEnhancer::default();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("timeout occurred", ctx);
        assert_eq!(enhanced.category, ErrorCategory::Timeout);
    }

    #[test]
    fn test_categorize_timeout() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_divergent");
        let enhanced = enhancer.enhance("Request timeout after 30000ms", ctx);
        assert_eq!(enhanced.category, ErrorCategory::Timeout);
    }

    #[test]
    fn test_categorize_rate_limit() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Rate limited: too many requests", ctx);
        assert_eq!(enhanced.category, ErrorCategory::RateLimit);
    }

    #[test]
    fn test_categorize_authentication() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Authentication failed: invalid API key", ctx);
        assert_eq!(enhanced.category, ErrorCategory::Authentication);
    }

    #[test]
    fn test_categorize_invalid_request() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Invalid request parameters", ctx);
        assert_eq!(enhanced.category, ErrorCategory::InvalidRequest);
    }

    #[test]
    fn test_categorize_api_unavailable() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Service unavailable", ctx);
        assert_eq!(enhanced.category, ErrorCategory::ApiUnavailable);
    }

    #[test]
    fn test_categorize_storage() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Database connection error", ctx);
        assert_eq!(enhanced.category, ErrorCategory::Storage);
    }

    #[test]
    fn test_categorize_other() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Some unknown error", ctx);
        assert_eq!(enhanced.category, ErrorCategory::Other);
    }

    #[test]
    fn test_timeout_alternatives_divergent() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_divergent");
        let enhanced = enhancer.enhance("Request timeout", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("reasoning_linear")));
        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("reasoning_auto")));
    }

    #[test]
    fn test_timeout_alternatives_graph() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_graph");
        let enhanced = enhancer.enhance("Request timeout", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("reasoning_tree")));
    }

    #[test]
    fn test_timeout_alternatives_mcts() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_mcts");
        let enhanced = enhancer.enhance("Request timeout", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("reasoning_tree")));
    }

    #[test]
    fn test_timeout_alternatives_large_content() {
        let enhancer = ErrorEnhancer::new();
        let ctx = ErrorContext {
            failed_tool: "reasoning_linear".into(),
            failed_operation: None,
            complexity: ComplexityMetrics {
                content_length: 15_000,
                ..Default::default()
            },
            timeout_ms: 30_000,
        };
        let enhanced = enhancer.enhance("Request timeout", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("Break content")));
    }

    #[test]
    fn test_timeout_alternatives_short_timeout() {
        let enhancer = ErrorEnhancer::new();
        let ctx = ErrorContext {
            failed_tool: "reasoning_linear".into(),
            failed_operation: None,
            complexity: ComplexityMetrics::default(),
            timeout_ms: 10_000,
        };
        let enhanced = enhancer.enhance("Request timeout", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("longer timeout")));
    }

    #[test]
    fn test_rate_limit_alternatives() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Rate limited", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("retry")));
        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("checkpoint")));
    }

    #[test]
    fn test_unavailable_alternatives() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Service unavailable", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("backoff")));
    }

    #[test]
    fn test_invalid_request_large_content() {
        let enhancer = ErrorEnhancer::new();
        let ctx = ErrorContext {
            failed_tool: "reasoning_linear".into(),
            failed_operation: None,
            complexity: ComplexityMetrics {
                content_length: 60_000,
                ..Default::default()
            },
            timeout_ms: 30_000,
        };
        let enhanced = enhancer.enhance("Invalid request", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("Reduce content")));
    }

    #[test]
    fn test_invalid_request_deep_operation() {
        let enhancer = ErrorEnhancer::new();
        let ctx = ErrorContext {
            failed_tool: "reasoning_graph".into(),
            failed_operation: Some("generate".into()),
            complexity: ComplexityMetrics {
                content_length: 1000,
                operation_depth: Some(15),
                branching_factor: None,
            },
            timeout_ms: 30_000,
        };
        let enhanced = enhancer.enhance("Invalid request", ctx);

        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("depth")));
    }

    #[test]
    fn test_enhanced_error_serialize() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("timeout", ctx);

        let json = serde_json::to_string(&enhanced).unwrap();
        assert!(json.contains("\"error\":\"timeout\""));
        assert!(json.contains("\"category\":\"timeout\""));
        assert!(json.contains("\"alternatives\""));
    }

    #[test]
    fn test_error_context_with_operation() {
        let ctx = ErrorContext {
            failed_tool: "reasoning_graph".into(),
            failed_operation: Some("init".into()),
            complexity: ComplexityMetrics::default(),
            timeout_ms: 30_000,
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("\"failed_operation\":\"init\""));
    }

    #[test]
    fn test_alternative_without_duration() {
        let alt = Alternative {
            suggestion: "Test".into(),
            reason: "Reason".into(),
            estimated_duration_ms: None,
        };

        let json = serde_json::to_string(&alt).unwrap();
        assert!(!json.contains("estimated_duration_ms"));
    }

    #[test]
    fn test_complexity_metrics_constructors() {
        let m = ComplexityMetrics::from_content(123).with_branching(Some(4));
        assert_eq!(m.content_length, 123);
        assert_eq!(m.branching_factor, Some(4));
        assert!(m.operation_depth.is_none());
    }

    #[test]
    fn test_categorize_missing_field_is_invalid_request() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_linear");
        let enhanced = enhancer.enhance("Missing required field: content", ctx);
        assert_eq!(enhanced.category, ErrorCategory::InvalidRequest);
    }

    #[test]
    fn test_categorize_session_required_is_invalid_request() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_tree");
        let enhanced = enhancer.enhance("Session required but not provided", ctx);
        assert_eq!(enhanced.category, ErrorCategory::InvalidRequest);
    }

    #[test]
    fn test_categorize_not_found_is_invalid_request() {
        let enhancer = ErrorEnhancer::new();
        let ctx = create_test_context("reasoning_tree");
        let enhanced = enhancer.enhance("Not found: branch xyz", ctx);
        assert_eq!(enhanced.category, ErrorCategory::InvalidRequest);
    }

    #[test]
    fn test_invalid_request_includes_example_call() {
        let enhancer = ErrorEnhancer::new();
        let ctx = ErrorContext {
            failed_tool: "reasoning_tree".into(),
            failed_operation: Some("focus".into()),
            complexity: ComplexityMetrics::default(),
            timeout_ms: 30_000,
        };
        let enhanced = enhancer.enhance("Not found: branch xyz", ctx);
        let example = enhanced
            .alternatives
            .iter()
            .find(|a| a.reason.contains("Example:"))
            .expect("an example-call alternative");
        assert!(example.reason.contains("reasoning_tree"));
        assert!(example.reason.contains("branch_id"));
    }

    #[test]
    fn test_invalid_request_unknown_tool_has_no_example() {
        let enhancer = ErrorEnhancer::new();
        let ctx = ErrorContext {
            failed_tool: "reasoning_unknown".into(),
            failed_operation: None,
            complexity: ComplexityMetrics::default(),
            timeout_ms: 30_000,
        };
        let enhanced = enhancer.enhance("Not found: thing", ctx);
        assert!(!enhanced
            .alternatives
            .iter()
            .any(|a| a.reason.contains("Example:")));
    }

    #[test]
    fn test_timeout_alternatives_high_branching() {
        let enhancer = ErrorEnhancer::new();
        let ctx = ErrorContext {
            failed_tool: "reasoning_divergent".into(),
            failed_operation: None,
            complexity: ComplexityMetrics::from_content(100).with_branching(Some(8)),
            timeout_ms: 30_000,
        };
        let enhanced = enhancer.enhance("Operation timed out after 30000ms", ctx);
        assert!(enhanced
            .alternatives
            .iter()
            .any(|a| a.suggestion.contains("Reduce branching")));
    }
}
