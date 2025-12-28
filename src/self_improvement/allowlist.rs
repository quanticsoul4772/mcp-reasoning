//! Action validation against allowed types.
//!
//! Provides safety by validating actions before execution.

use super::types::{ActionType, SelfImprovementAction};
use std::collections::HashSet;

/// Validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Error message.
    pub message: String,
    /// Error code.
    pub code: ValidationErrorCode,
}

/// Validation error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationErrorCode {
    /// Action type not allowed.
    ActionTypeNotAllowed,
    /// Parameter not allowed.
    ParameterNotAllowed,
    /// Value out of bounds.
    ValueOutOfBounds,
    /// Missing required field.
    MissingRequired,
    /// Rate limit exceeded.
    RateLimitExceeded,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Configuration for the allowlist.
#[derive(Debug, Clone)]
pub struct AllowlistConfig {
    /// Allowed action types.
    pub allowed_action_types: HashSet<ActionType>,
    /// Allowed parameter keys (per action type).
    pub allowed_parameters: std::collections::HashMap<ActionType, HashSet<String>>,
    /// Maximum expected improvement.
    pub max_expected_improvement: f64,
    /// Maximum actions per hour.
    pub max_actions_per_hour: u32,
}

impl Default for AllowlistConfig {
    fn default() -> Self {
        let mut allowed_action_types = HashSet::new();
        allowed_action_types.insert(ActionType::ConfigAdjust);
        allowed_action_types.insert(ActionType::PromptTune);
        allowed_action_types.insert(ActionType::ThresholdAdjust);
        allowed_action_types.insert(ActionType::LogObservation);

        let mut allowed_parameters = std::collections::HashMap::new();

        // ConfigAdjust allowed parameters
        let mut config_params = HashSet::new();
        config_params.insert("timeout_ms".to_string());
        config_params.insert("max_retries".to_string());
        config_params.insert("request_limit".to_string());
        config_params.insert("batch_size".to_string());
        allowed_parameters.insert(ActionType::ConfigAdjust, config_params);

        // PromptTune allowed parameters
        let mut prompt_params = HashSet::new();
        prompt_params.insert("prompt_key".to_string());
        prompt_params.insert("template".to_string());
        prompt_params.insert("mode".to_string());
        allowed_parameters.insert(ActionType::PromptTune, prompt_params);

        // ThresholdAdjust allowed parameters
        let mut threshold_params = HashSet::new();
        threshold_params.insert("threshold_key".to_string());
        threshold_params.insert("value".to_string());
        threshold_params.insert("min".to_string());
        threshold_params.insert("max".to_string());
        allowed_parameters.insert(ActionType::ThresholdAdjust, threshold_params);

        Self {
            allowed_action_types,
            allowed_parameters,
            max_expected_improvement: 0.5,
            max_actions_per_hour: 10,
        }
    }
}

/// Action rate tracker.
#[derive(Debug, Default)]
struct RateTracker {
    actions: Vec<u64>,
}

impl RateTracker {
    fn record(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.actions.push(now);
        self.cleanup(now);
    }

    fn count_in_last_hour(&mut self) -> u32 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.cleanup(now);
        self.actions.len() as u32
    }

    fn cleanup(&mut self, now: u64) {
        let hour_ago = now.saturating_sub(3600);
        self.actions.retain(|&t| t > hour_ago);
    }
}

/// Allowlist for action validation.
#[derive(Debug)]
pub struct Allowlist {
    config: AllowlistConfig,
    rate_tracker: RateTracker,
}

impl Allowlist {
    /// Create a new allowlist.
    #[must_use]
    pub fn new(config: AllowlistConfig) -> Self {
        Self {
            config,
            rate_tracker: RateTracker::default(),
        }
    }

    /// Create an allowlist with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(AllowlistConfig::default())
    }

    /// Validate an action.
    pub fn validate(&mut self, action: &SelfImprovementAction) -> Result<(), ValidationError> {
        // Check action type
        if !self
            .config
            .allowed_action_types
            .contains(&action.action_type)
        {
            return Err(ValidationError {
                message: format!("Action type '{}' is not allowed", action.action_type),
                code: ValidationErrorCode::ActionTypeNotAllowed,
            });
        }

        // Check expected improvement bounds
        if action.expected_improvement > self.config.max_expected_improvement {
            return Err(ValidationError {
                message: format!(
                    "Expected improvement {:.2} exceeds maximum {:.2}",
                    action.expected_improvement, self.config.max_expected_improvement
                ),
                code: ValidationErrorCode::ValueOutOfBounds,
            });
        }

        // Check parameters
        if let Some(params) = &action.parameters {
            self.validate_parameters(&action.action_type, params)?;
        }

        // Check rate limit
        if self.rate_tracker.count_in_last_hour() >= self.config.max_actions_per_hour {
            return Err(ValidationError {
                message: format!(
                    "Rate limit exceeded: {} actions/hour",
                    self.config.max_actions_per_hour
                ),
                code: ValidationErrorCode::RateLimitExceeded,
            });
        }

        Ok(())
    }

    /// Validate and record an action (for rate limiting).
    pub fn validate_and_record(
        &mut self,
        action: &SelfImprovementAction,
    ) -> Result<(), ValidationError> {
        self.validate(action)?;
        self.rate_tracker.record();
        Ok(())
    }

    /// Check if an action type is allowed.
    #[must_use]
    pub fn is_action_type_allowed(&self, action_type: &ActionType) -> bool {
        self.config.allowed_action_types.contains(action_type)
    }

    /// Add an allowed action type.
    pub fn allow_action_type(&mut self, action_type: ActionType) {
        self.config.allowed_action_types.insert(action_type);
    }

    /// Remove an allowed action type.
    pub fn disallow_action_type(&mut self, action_type: &ActionType) {
        self.config.allowed_action_types.remove(action_type);
    }

    /// Add an allowed parameter for an action type.
    pub fn allow_parameter(&mut self, action_type: ActionType, param: impl Into<String>) {
        self.config
            .allowed_parameters
            .entry(action_type)
            .or_default()
            .insert(param.into());
    }

    /// Get current rate (actions in last hour).
    pub fn current_rate(&mut self) -> u32 {
        self.rate_tracker.count_in_last_hour()
    }

    fn validate_parameters(
        &self,
        action_type: &ActionType,
        params: &serde_json::Value,
    ) -> Result<(), ValidationError> {
        let allowed = match self.config.allowed_parameters.get(action_type) {
            Some(allowed) => allowed,
            None => return Ok(()), // No restrictions for this action type
        };

        if let Some(obj) = params.as_object() {
            for key in obj.keys() {
                if !allowed.contains(key) {
                    return Err(ValidationError {
                        message: format!(
                            "Parameter '{}' is not allowed for action type '{}'",
                            key, action_type
                        ),
                        code: ValidationErrorCode::ParameterNotAllowed,
                    });
                }
            }
        }

        Ok(())
    }
}

impl Default for Allowlist {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_action(action_type: ActionType) -> SelfImprovementAction {
        SelfImprovementAction::new("test-action", action_type, "Test action", "Testing", 0.15)
    }

    #[test]
    fn test_allowlist_default() {
        let allowlist = Allowlist::with_defaults();
        assert!(allowlist.is_action_type_allowed(&ActionType::ConfigAdjust));
        assert!(allowlist.is_action_type_allowed(&ActionType::PromptTune));
        assert!(allowlist.is_action_type_allowed(&ActionType::ThresholdAdjust));
        assert!(allowlist.is_action_type_allowed(&ActionType::LogObservation));
    }

    #[test]
    fn test_validate_allowed_action() {
        let mut allowlist = Allowlist::with_defaults();
        let action = create_test_action(ActionType::ConfigAdjust);

        let result = allowlist.validate(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_disallowed_action() {
        let mut allowlist = Allowlist::with_defaults();
        allowlist.disallow_action_type(&ActionType::ConfigAdjust);

        let action = create_test_action(ActionType::ConfigAdjust);
        let result = allowlist.validate(&action);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            ValidationErrorCode::ActionTypeNotAllowed
        );
    }

    #[test]
    fn test_validate_expected_improvement_bounds() {
        let mut allowlist = Allowlist::with_defaults();
        let mut action = create_test_action(ActionType::ConfigAdjust);
        action.expected_improvement = 0.6; // Above default max of 0.5

        // We need to bypass the clamping in new() for testing
        action = SelfImprovementAction {
            expected_improvement: 0.6,
            ..action
        };

        let result = allowlist.validate(&action);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            ValidationErrorCode::ValueOutOfBounds
        );
    }

    #[test]
    fn test_validate_allowed_parameters() {
        let mut allowlist = Allowlist::with_defaults();
        let mut action = create_test_action(ActionType::ConfigAdjust);
        action = action.with_parameters(serde_json::json!({
            "timeout_ms": 30000,
            "max_retries": 5
        }));

        let result = allowlist.validate(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_disallowed_parameter() {
        let mut allowlist = Allowlist::with_defaults();
        let mut action = create_test_action(ActionType::ConfigAdjust);
        action = action.with_parameters(serde_json::json!({
            "timeout_ms": 30000,
            "dangerous_param": "value"
        }));

        let result = allowlist.validate(&action);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            ValidationErrorCode::ParameterNotAllowed
        );
    }

    #[test]
    fn test_rate_limiting() {
        let config = AllowlistConfig {
            max_actions_per_hour: 2,
            ..Default::default()
        };
        let mut allowlist = Allowlist::new(config);
        let action = create_test_action(ActionType::LogObservation);

        // First two should succeed
        assert!(allowlist.validate_and_record(&action).is_ok());
        assert!(allowlist.validate_and_record(&action).is_ok());

        // Third should fail
        let result = allowlist.validate(&action);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            ValidationErrorCode::RateLimitExceeded
        );
    }

    #[test]
    fn test_current_rate() {
        let mut allowlist = Allowlist::with_defaults();
        let action = create_test_action(ActionType::LogObservation);

        assert_eq!(allowlist.current_rate(), 0);

        allowlist.validate_and_record(&action).unwrap();
        assert_eq!(allowlist.current_rate(), 1);

        allowlist.validate_and_record(&action).unwrap();
        assert_eq!(allowlist.current_rate(), 2);
    }

    #[test]
    fn test_allow_action_type() {
        let config = AllowlistConfig {
            allowed_action_types: HashSet::new(),
            ..Default::default()
        };
        let mut allowlist = Allowlist::new(config);

        assert!(!allowlist.is_action_type_allowed(&ActionType::ConfigAdjust));

        allowlist.allow_action_type(ActionType::ConfigAdjust);
        assert!(allowlist.is_action_type_allowed(&ActionType::ConfigAdjust));
    }

    #[test]
    fn test_allow_parameter() {
        let mut allowlist = Allowlist::with_defaults();

        // Add a custom parameter
        allowlist.allow_parameter(ActionType::ConfigAdjust, "custom_param");

        let mut action = create_test_action(ActionType::ConfigAdjust);
        action = action.with_parameters(serde_json::json!({
            "custom_param": "value"
        }));

        let result = allowlist.validate(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError {
            message: "Test error".to_string(),
            code: ValidationErrorCode::ActionTypeNotAllowed,
        };

        let display = format!("{error}");
        assert!(display.contains("ActionTypeNotAllowed"));
        assert!(display.contains("Test error"));
    }

    #[test]
    fn test_log_observation_no_params_required() {
        let mut allowlist = Allowlist::with_defaults();
        let action = create_test_action(ActionType::LogObservation);

        let result = allowlist.validate(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_tune_parameters() {
        let mut allowlist = Allowlist::with_defaults();
        let mut action = create_test_action(ActionType::PromptTune);
        action = action.with_parameters(serde_json::json!({
            "prompt_key": "linear",
            "template": "New template"
        }));

        let result = allowlist.validate(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_threshold_adjust_parameters() {
        let mut allowlist = Allowlist::with_defaults();
        let mut action = create_test_action(ActionType::ThresholdAdjust);
        action = action.with_parameters(serde_json::json!({
            "threshold_key": "confidence",
            "value": 0.85
        }));

        let result = allowlist.validate(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rate_tracker_cleanup() {
        let mut tracker = RateTracker::default();

        // Add some old actions (would be cleaned up)
        tracker.actions.push(0); // Very old
        tracker.actions.push(1); // Very old

        // Count should be 0 after cleanup
        let count = tracker.count_in_last_hour();
        assert_eq!(count, 0);
    }
}
