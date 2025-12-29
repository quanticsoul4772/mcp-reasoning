//! Secret string wrapper for sensitive data.
//!
//! This module provides a wrapper type that prevents accidental logging
//! of sensitive data like API keys.

use std::fmt;

/// A wrapper for sensitive strings that redacts the value in Debug/Display output.
///
/// This type is designed to wrap sensitive data like API keys to prevent
/// accidental logging or exposure through debug output.
///
/// # Example
///
/// ```
/// use mcp_reasoning::config::SecretString;
///
/// let secret = SecretString::new("sk-ant-api-key-123");
/// assert_eq!(format!("{:?}", secret), "<REDACTED>");
/// assert_eq!(secret.expose(), "sk-ant-api-key-123");
/// ```
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    /// Creates a new `SecretString` from any string-like value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Exposes the underlying secret value.
    ///
    /// Use this method only when you need to actually use the secret,
    /// such as when making API calls.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Returns true if the secret is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the length of the secret.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<REDACTED>")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<REDACTED>")
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SecretString {}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_new() {
        let secret = SecretString::new("api-key-123");
        assert_eq!(secret.expose(), "api-key-123");
    }

    #[test]
    fn test_secret_string_from_string() {
        let secret: SecretString = String::from("api-key-123").into();
        assert_eq!(secret.expose(), "api-key-123");
    }

    #[test]
    fn test_secret_string_from_str() {
        let secret: SecretString = "api-key-123".into();
        assert_eq!(secret.expose(), "api-key-123");
    }

    #[test]
    fn test_secret_string_debug_redacted() {
        let secret = SecretString::new("super-secret-key");
        let debug = format!("{secret:?}");
        assert_eq!(debug, "<REDACTED>");
        assert!(!debug.contains("super-secret-key"));
    }

    #[test]
    fn test_secret_string_display_redacted() {
        let secret = SecretString::new("super-secret-key");
        let display = format!("{secret}");
        assert_eq!(display, "<REDACTED>");
        assert!(!display.contains("super-secret-key"));
    }

    #[test]
    fn test_secret_string_clone() {
        let secret = SecretString::new("api-key");
        let cloned = secret.clone();
        assert_eq!(secret.expose(), cloned.expose());
    }

    #[test]
    fn test_secret_string_eq() {
        let secret1 = SecretString::new("same-key");
        let secret2 = SecretString::new("same-key");
        let secret3 = SecretString::new("different-key");
        assert_eq!(secret1, secret2);
        assert_ne!(secret1, secret3);
    }

    #[test]
    fn test_secret_string_is_empty() {
        let empty = SecretString::new("");
        let not_empty = SecretString::new("key");
        assert!(empty.is_empty());
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn test_secret_string_len() {
        let secret = SecretString::new("12345");
        assert_eq!(secret.len(), 5);
    }

    #[test]
    fn test_secret_string_expose_returns_original() {
        let original = "sk-ant-api03-verylongapikey123456789";
        let secret = SecretString::new(original);
        assert_eq!(secret.expose(), original);
    }
}
