//! Security utilities for prompt sanitization.
//!
//! This module implements input sanitization to prevent prompt injection attacks:
//! - `escape_for_prompt()` escapes format string markers and truncates long content
//! - `sanitize_multiline()` neutralizes instruction separator patterns

/// Maximum length for user-controlled content in prompts (10KB).
pub const MAX_PROMPT_CONTENT_LEN: usize = 10_000;

/// Maximum size for extracted JSON responses (100KB).
///
/// This conservative limit (vs 1MB) is intentional for security:
/// - Prevents DoS via large response processing
/// - Limits memory consumption during JSON parsing
/// - Self-improvement responses are structured and compact
/// - Typical valid responses are < 10KB
pub const MAX_JSON_SIZE: usize = 100_000;

/// Escape content for safe inclusion in prompts.
///
/// This prevents prompt injection by:
/// 1. Escaping format string markers (`{` and `}`)
/// 2. Truncating content exceeding `MAX_PROMPT_CONTENT_LEN`
///
/// Prevents prompt injection by escaping format string markers.
pub fn escape_for_prompt(content: &str) -> String {
    let mut escaped = content.replace('{', "{{").replace('}', "}}");

    // Truncate if too long
    if escaped.len() > MAX_PROMPT_CONTENT_LEN {
        escaped.truncate(MAX_PROMPT_CONTENT_LEN);
        escaped.push_str("...[truncated]");
    }

    escaped
}

/// Sanitize multiline content that could contain injection patterns.
///
/// In addition to escaping format markers, this function neutralizes
/// patterns that could be interpreted as instruction separators:
/// - `---` → `- - -`
/// - `===` → `= = =`
/// - `###` → `# # #`
///
/// This helps prevent prompt injection attacks that use visual separators
/// to make the LLM ignore previous instructions.
pub fn sanitize_multiline(content: &str) -> String {
    let escaped = escape_for_prompt(content);

    // Replace patterns that look like instruction separators
    escaped
        .replace("---", "- - -")
        .replace("===", "= = =")
        .replace("###", "# # #")
}
