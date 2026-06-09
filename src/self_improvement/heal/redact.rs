//! Redaction + stable hashing of triggering input (FR-016, D8).
//!
//! Defect capture must never become a secrets-leak surface. We persist a stable
//! content hash (the recurrence key) plus a bounded, credential-scrubbed excerpt
//! — never the raw input, credentials, or full payloads.

/// Result of redacting a triggering input: a stable content hash for recurrence
/// matching and a bounded, secret-scrubbed excerpt safe to persist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedInput {
    /// Stable hash of the full input — usable as a recurrence-matching key.
    pub hash: String,
    /// Bounded, credential-scrubbed excerpt safe to store/log.
    pub excerpt: String,
}

/// Placeholder substituted for any token that looks like a credential.
const REDACTED: &str = "[REDACTED]";

/// FNV-1a 64-bit hash → lowercase hex. Deterministic across runs (no RNG), so it
/// is a stable recurrence key for persistence.
#[must_use]
fn fnv1a_hex(s: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// True if a token looks like a credential/secret and must be scrubbed.
fn looks_secret(tok: &str) -> bool {
    let lower = tok.to_ascii_lowercase();
    let key_like = tok.len() >= 24
        && tok.chars().any(|c| c.is_ascii_digit())
        && tok
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    lower.starts_with("sk-ant")
        || lower.starts_with("pa-")
        || lower.starts_with("bearer")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("authorization")
        || lower.contains("secret")
        || key_like
}

/// Redact `input` for safe persistence: hash the full input (recurrence key),
/// scrub credential-shaped tokens, and cap the excerpt to `max_len` characters.
#[must_use]
pub fn redact(input: &str, max_len: usize) -> RedactedInput {
    let hash = fnv1a_hex(input);
    let scrubbed: String = input
        .split_whitespace()
        .map(|t| if looks_secret(t) { REDACTED } else { t })
        .collect::<Vec<_>>()
        .join(" ");
    let excerpt = if scrubbed.chars().count() > max_len {
        scrubbed.chars().take(max_len).collect::<String>() + " …"
    } else {
        scrubbed
    };
    RedactedInput { hash, excerpt }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_anthropic_key() {
        let r = redact("solve this sk-ant-abc123DEF456ghi789 now", 200);
        assert!(r.excerpt.contains(REDACTED));
        assert!(!r.excerpt.contains("sk-ant-abc123"));
    }

    #[test]
    fn scrubs_bearer_and_authorization() {
        let r = redact("Authorization: Bearer tok_9999 payload", 200);
        assert!(r.excerpt.contains(REDACTED));
        assert!(!r.excerpt.to_lowercase().contains("bearer tok"));
    }

    #[test]
    fn scrubs_long_key_like_token() {
        let r = redact("prefix ABCD1234abcd5678EFGH9012ij value", 200);
        assert!(r.excerpt.contains(REDACTED));
    }

    #[test]
    fn keeps_ordinary_words() {
        let r = redact("the function returned malformed json output", 200);
        assert!(!r.excerpt.contains(REDACTED));
        assert!(r.excerpt.contains("malformed"));
    }

    #[test]
    fn hash_is_stable_and_excerpt_bounded() {
        let a = redact("same input here", 200);
        let b = redact("same input here", 200);
        assert_eq!(a.hash, b.hash);
        let long = "word ".repeat(100);
        let r = redact(&long, 10);
        assert!(r.excerpt.chars().count() <= 12);
    }
}
