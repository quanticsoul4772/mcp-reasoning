//! Integrity guard (FR-010, D6).
//!
//! The self-heal repair action must never modify the acceptance/measurement
//! surface — a fix that could edit its own tests, metrics, eval scorer, sensor,
//! or circuit breaker could game its own success signal (reward hacking is the
//! default failure mode). This guard names the protected surface and rejects any
//! candidate edit that touches it.

/// Repo-relative path prefixes (dirs end with `/`) and files the repair action
/// must never modify.
#[must_use]
pub fn protected_paths() -> &'static [&'static str] {
    &[
        "tests/",
        "src/metrics/",
        "src/eval/",
        "src/self_improvement/sensor.rs",
        "src/self_improvement/circuit_breaker.rs",
        "src/self_improvement/allowlist.rs",
    ]
}

/// True if `path` lies in the protected acceptance/measurement surface and may
/// not be auto-modified. `path` is treated as repo-relative; back-slashes are
/// normalized so Windows paths match.
#[must_use]
pub fn is_protected(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    protected_paths().iter().any(|p| {
        p.strip_suffix('/').map_or_else(
            || norm == *p || norm.ends_with(&format!("/{p}")),
            |dir| norm == dir || norm.starts_with(p),
        )
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn protects_measurement_surface() {
        assert!(is_protected("src/metrics/mod.rs"));
        assert!(is_protected("tests/integration/heal_detection.rs"));
        assert!(is_protected("src/eval/scorer.rs"));
        assert!(is_protected("src/self_improvement/sensor.rs"));
        assert!(is_protected("src/self_improvement/circuit_breaker.rs"));
        assert!(is_protected("src/self_improvement/allowlist.rs"));
    }

    #[test]
    fn allows_ordinary_production_paths() {
        assert!(!is_protected("src/modes/core.rs"));
        assert!(!is_protected("src/self_improvement/monitor.rs"));
        assert!(!is_protected("src/server/params.rs"));
    }

    #[test]
    fn normalizes_backslashes() {
        assert!(is_protected("src\\metrics\\mod.rs"));
    }
}
