//! Shared helper that augments a tool failure message with contextual recovery
//! alternatives via [`ErrorEnhancer`].
//!
//! Handlers call [`with_recovery_suggestions`] from their `Err` branches, passing
//! their existing operation-specific hint as the base message. The enhancer's
//! category-based alternatives (timeout / rate-limit / auth / invalid /
//! unavailable) are appended **only when it produces any** — so an
//! operation-specific failure the enhancer doesn't recognize keeps just its
//! specific hint, and a timeout/API failure additionally gets recovery
//! suggestions (e.g. a faster tool to try).

use std::fmt::Write as _;

use crate::error::enhanced::{ComplexityMetrics, ErrorContext, ErrorEnhancer};

/// Append the [`ErrorEnhancer`]'s contextual alternatives to `base_message`.
///
/// `base_message` is the handler's already-formatted failure string (kept as-is).
/// `error` is the raw error text the enhancer categorizes on. `failed_tool` must
/// be the full tool name (e.g. `"reasoning_divergent"`) — the enhancer keys some
/// timeout suggestions off it. When the enhancer yields no alternatives,
/// `base_message` is returned unchanged (no empty `Suggestions:` block).
pub fn with_recovery_suggestions(
    base_message: String,
    failed_tool: &str,
    failed_operation: Option<&str>,
    error: &str,
    complexity: ComplexityMetrics,
    timeout_ms: u64,
) -> String {
    let context = ErrorContext {
        failed_tool: failed_tool.to_string(),
        failed_operation: failed_operation.map(str::to_string),
        complexity,
        timeout_ms,
    };
    let enhanced = ErrorEnhancer::new().enhance(error, context);
    if enhanced.alternatives.is_empty() {
        return base_message;
    }

    let mut message = base_message;
    message.push_str("\nSuggestions:");
    for alt in &enhanced.alternatives {
        let _ = write!(message, "\n- {}: {}", alt.suggestion, alt.reason);
        if let Some(ms) = alt.estimated_duration_ms {
            let _ = write!(message, " (~{ms} ms)");
        }
    }
    message
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_appends_faster_tool_suggestion() {
        let msg = with_recovery_suggestions(
            "divergent failed: Operation timed out after 60000ms".to_string(),
            "reasoning_divergent",
            None,
            // The real ModeError::Timeout display says "timed out", not "timeout".
            "Operation timed out after 60000ms",
            ComplexityMetrics::default(),
            60_000,
        );
        // Base hint preserved.
        assert!(msg.starts_with("divergent failed: Operation timed out"));
        // Enhancer appended a faster-tool suggestion.
        assert!(msg.contains("Suggestions:"));
        assert!(msg.contains("reasoning_linear"));
    }

    #[test]
    fn test_authentication_appends_api_key_guidance() {
        let msg = with_recovery_suggestions(
            "linear failed: auth error".to_string(),
            "reasoning_linear",
            None,
            "authentication failed: invalid api key",
            ComplexityMetrics::default(),
            30_000,
        );
        assert!(msg.contains("Suggestions:"));
        assert!(msg.contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_unavailable_appends_retry_guidance() {
        let msg = with_recovery_suggestions(
            "graph failed: service unavailable".to_string(),
            "reasoning_graph",
            Some("generate"),
            "service unavailable",
            ComplexityMetrics::default(),
            30_000,
        );
        assert!(msg.contains("Suggestions:"));
        let lower = msg.to_lowercase();
        assert!(lower.contains("retry") || lower.contains("backoff"));
    }

    #[test]
    fn test_operation_specific_error_keeps_base_unchanged() {
        // An uncategorized error ("Other") yields no alternatives, so the
        // handler's specific hint is returned verbatim.
        let base = "focus failed: use operation='list' to see valid branch_id".to_string();
        let msg = with_recovery_suggestions(
            base.clone(),
            "reasoning_tree",
            Some("focus"),
            "branch xyz not found in session",
            ComplexityMetrics::default(),
            30_000,
        );
        assert_eq!(msg, base);
        assert!(!msg.contains("Suggestions:"));
    }
}
