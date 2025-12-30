//! Circuit breaker for self-improvement safety.
//!
//! Prevents runaway changes by halting after consecutive failures.

use std::time::{Duration, Instant};

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, operations proceed normally.
    Closed,
    /// Circuit is open, operations are blocked.
    Open,
    /// Circuit is half-open, allowing test operations.
    HalfOpen,
}

/// Configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before tripping.
    pub failure_threshold: u32,
    /// Cooldown duration before attempting recovery.
    pub cooldown_duration: Duration,
    /// Number of successes in half-open state to close.
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            cooldown_duration: Duration::from_secs(300), // 5 minutes
            success_threshold: 2,
        }
    }
}

/// Circuit breaker for the self-improvement system.
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_failure: Option<Instant>,
    total_failures: u64,
    total_successes: u64,
    trips: u64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    #[must_use]
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure: None,
            total_failures: 0,
            total_successes: 0,
            trips: 0,
        }
    }

    /// Create a circuit breaker with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get current state.
    #[must_use]
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Check if operations are allowed.
    #[must_use]
    pub fn is_allowed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if cooldown has elapsed
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() >= self.config.cooldown_duration {
                        self.transition_to(CircuitState::HalfOpen);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true, // Allow test operations
        }
    }

    /// Record a successful operation.
    pub fn record_success(&mut self) {
        self.total_successes += 1;
        self.consecutive_failures = 0;
        self.consecutive_successes += 1;

        match self.state {
            CircuitState::HalfOpen => {
                if self.consecutive_successes >= self.config.success_threshold {
                    self.transition_to(CircuitState::Closed);
                }
            }
            CircuitState::Closed => {
                // Already closed, nothing to do
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                self.transition_to(CircuitState::HalfOpen);
            }
        }
    }

    /// Record a failed operation.
    pub fn record_failure(&mut self) {
        self.total_failures += 1;
        self.consecutive_failures += 1;
        self.consecutive_successes = 0;
        self.last_failure = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                if self.consecutive_failures >= self.config.failure_threshold {
                    self.transition_to(CircuitState::Open);
                    self.trips += 1;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state trips the circuit
                self.transition_to(CircuitState::Open);
                self.trips += 1;
            }
            CircuitState::Open => {
                // Already open, refresh the timer
            }
        }
    }

    /// Manually reset the circuit breaker.
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.consecutive_failures = 0;
        self.consecutive_successes = 0;
        self.last_failure = None;
    }

    /// Force the circuit open.
    pub fn trip(&mut self) {
        self.transition_to(CircuitState::Open);
        self.last_failure = Some(Instant::now());
        self.trips += 1;
    }

    /// Get statistics.
    #[must_use]
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            state: self.state,
            consecutive_failures: self.consecutive_failures,
            consecutive_successes: self.consecutive_successes,
            total_failures: self.total_failures,
            total_successes: self.total_successes,
            trips: self.trips,
            time_in_state: self.last_failure.map(|t| t.elapsed()),
        }
    }

    /// Get remaining cooldown time if circuit is open.
    #[must_use]
    pub fn remaining_cooldown(&self) -> Option<Duration> {
        if self.state != CircuitState::Open {
            return None;
        }

        self.last_failure
            .map(|last| self.config.cooldown_duration.saturating_sub(last.elapsed()))
    }

    fn transition_to(&mut self, new_state: CircuitState) {
        if self.state != new_state {
            self.state = new_state;
            match new_state {
                CircuitState::Closed => {
                    self.consecutive_failures = 0;
                    self.consecutive_successes = 0;
                }
                CircuitState::HalfOpen => {
                    self.consecutive_successes = 0;
                }
                CircuitState::Open => {
                    // Keep failure count for diagnostics
                }
            }
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Circuit breaker statistics.
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Current state.
    pub state: CircuitState,
    /// Current consecutive failures.
    pub consecutive_failures: u32,
    /// Current consecutive successes.
    pub consecutive_successes: u32,
    /// Total failures recorded.
    pub total_failures: u64,
    /// Total successes recorded.
    pub total_successes: u64,
    /// Number of times circuit has tripped.
    pub trips: u64,
    /// Time since last state change (if available).
    pub time_in_state: Option<Duration>,
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

    fn create_fast_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 2,
            cooldown_duration: Duration::from_millis(100),
            success_threshold: 1,
        }
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::with_defaults();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_allows_when_closed() {
        let mut cb = CircuitBreaker::with_defaults();
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_trips_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_blocks_when_open() {
        let mut cb = CircuitBreaker::new(create_fast_config());

        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_transitions_to_half_open() {
        let mut cb = CircuitBreaker::new(create_fast_config());

        cb.record_failure();
        cb.record_failure();

        // Wait for cooldown
        std::thread::sleep(Duration::from_millis(150));

        assert!(cb.is_allowed());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_closes_after_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_millis(10),
            success_threshold: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(20));
        assert!(cb.is_allowed());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reopens_on_half_open_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_millis(10),
            success_threshold: 2,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure();
        std::thread::sleep(Duration::from_millis(20));
        let _ = cb.is_allowed(); // Transition to half-open

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_success_resets_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.stats().consecutive_failures, 1);
    }

    #[test]
    fn test_circuit_breaker_manual_reset() {
        let mut cb = CircuitBreaker::new(create_fast_config());

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_manual_trip() {
        let mut cb = CircuitBreaker::with_defaults();

        cb.trip();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_stats() {
        let mut cb = CircuitBreaker::with_defaults();

        cb.record_success();
        cb.record_success();
        cb.record_failure();

        let stats = cb.stats();
        assert_eq!(stats.total_successes, 2);
        assert_eq!(stats.total_failures, 1);
        assert_eq!(stats.consecutive_failures, 1);
    }

    #[test]
    fn test_circuit_breaker_trip_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_millis(10),
            success_threshold: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure(); // Trip 1
        std::thread::sleep(Duration::from_millis(20));
        let _ = cb.is_allowed();
        cb.record_failure(); // Trip 2

        assert_eq!(cb.stats().trips, 2);
    }

    #[test]
    fn test_remaining_cooldown() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_secs(60),
            ..Default::default()
        };
        let mut cb = CircuitBreaker::new(config);

        // No cooldown when closed
        assert!(cb.remaining_cooldown().is_none());

        cb.record_failure();

        // Should have remaining cooldown when open
        let remaining = cb.remaining_cooldown();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > Duration::from_secs(50));
    }

    #[test]
    fn test_circuit_breaker_requires_multiple_successes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_millis(10),
            success_threshold: 3,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure();
        std::thread::sleep(Duration::from_millis(20));
        let _ = cb.is_allowed();

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_default_config_values() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.cooldown_duration, Duration::from_secs(300));
        assert_eq!(config.success_threshold, 2);
    }
}
