//! Self-improvement learning.
//!
//! Phase 4 of the 4-phase loop: Extract lessons from completed actions.

use super::executor::ExecutionResult;
use super::types::{ActionStatus, ActionType, Lesson, SelfImprovementAction};
use std::collections::HashMap;

/// Configuration for the learning system.
#[derive(Debug, Clone)]
pub struct LearnerConfig {
    /// Weight for expected vs actual improvement comparison.
    pub improvement_weight: f64,
    /// Minimum reward to consider action successful.
    pub success_threshold: f64,
    /// Maximum lessons to retain.
    pub max_lessons: usize,
}

impl Default for LearnerConfig {
    fn default() -> Self {
        Self {
            improvement_weight: 0.7,
            success_threshold: 0.0,
            max_lessons: 1000,
        }
    }
}

/// Learning result from action analysis.
#[derive(Debug, Clone)]
pub struct LearningResult {
    /// The lesson extracted.
    pub lesson: Lesson,
    /// Additional context learned.
    pub context: HashMap<String, String>,
}

/// Learner for extracting lessons from actions.
#[derive(Debug)]
pub struct Learner {
    config: LearnerConfig,
    lessons: Vec<Lesson>,
    action_type_stats: HashMap<ActionType, ActionTypeStats>,
}

/// Statistics for an action type.
#[derive(Debug, Clone, Default)]
pub struct ActionTypeStats {
    /// Total executions of this type.
    pub total_executions: u64,
    /// Successful executions.
    pub successful: u64,
    /// Average reward.
    pub avg_reward: f64,
    /// Total expected improvement.
    pub total_expected: f64,
    /// Total actual improvement.
    pub total_actual: f64,
}

impl Learner {
    /// Create a new learner.
    #[must_use]
    pub fn new(config: LearnerConfig) -> Self {
        Self {
            config,
            lessons: Vec::new(),
            action_type_stats: HashMap::new(),
        }
    }

    /// Create a learner with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(LearnerConfig::default())
    }

    /// Learn from an execution result.
    pub fn learn(&mut self, result: &ExecutionResult) -> Option<LearningResult> {
        // Only learn from completed or failed actions
        if !matches!(
            result.action.status,
            ActionStatus::Completed | ActionStatus::Failed
        ) {
            return None;
        }

        let reward = self.calculate_reward(result);
        let insight = self.generate_insight(result, reward);
        let contexts = self.identify_contexts(result);

        let lesson_id = format!(
            "lesson-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            result.action.id
        );

        let lesson = Lesson::new(&lesson_id, &result.action.id, insight.clone(), reward)
            .with_contexts(contexts.clone());

        // Update statistics
        self.update_stats(&result.action, reward);

        // Store lesson
        self.lessons.push(lesson.clone());
        self.trim_lessons();

        let mut context = HashMap::new();
        context.insert("reward".to_string(), format!("{reward:.3}"));
        context.insert(
            "action_type".to_string(),
            result.action.action_type.to_string(),
        );
        context.insert("success".to_string(), result.success.to_string());

        Some(LearningResult { lesson, context })
    }

    /// Calculate reward for an action.
    #[must_use]
    pub fn calculate_reward(&self, result: &ExecutionResult) -> f64 {
        if !result.success {
            return -0.5; // Penalty for failure
        }

        let expected = result.action.expected_improvement;
        let actual = result.measured_improvement.unwrap_or(0.0);

        if expected <= 0.0 {
            return actual; // No expectation, return raw improvement
        }

        // Calculate reward based on how well actual matched expected
        let ratio = actual / expected;

        if ratio >= 1.0 {
            // Met or exceeded expectations
            (ratio.min(2.0) - 1.0) * self.config.improvement_weight + 0.5
        } else {
            // Below expectations
            (ratio - 0.5) * self.config.improvement_weight
        }
        .clamp(-1.0, 1.0)
    }

    /// Get all lessons.
    pub fn lessons(&self) -> &[Lesson] {
        &self.lessons
    }

    /// Get lessons for a specific action type.
    pub fn lessons_by_action_type(&self, action_type: &ActionType) -> Vec<&Lesson> {
        let type_str = action_type.to_string();
        self.lessons
            .iter()
            .filter(|l| l.applicable_contexts.contains(&type_str))
            .collect()
    }

    /// Get lessons with positive reward.
    pub fn successful_lessons(&self) -> Vec<&Lesson> {
        self.lessons
            .iter()
            .filter(|l| l.reward >= self.config.success_threshold)
            .collect()
    }

    /// Get statistics for an action type.
    pub fn stats_for_type(&self, action_type: &ActionType) -> Option<&ActionTypeStats> {
        self.action_type_stats.get(action_type)
    }

    /// Get all action type statistics.
    pub fn all_stats(&self) -> &HashMap<ActionType, ActionTypeStats> {
        &self.action_type_stats
    }

    /// Get summary of learning.
    #[must_use]
    pub fn summary(&self) -> LearningSummary {
        let total_lessons = self.lessons.len();
        let avg_reward = if total_lessons > 0 {
            self.lessons.iter().map(|l| l.reward).sum::<f64>() / total_lessons as f64
        } else {
            0.0
        };

        let successful = self.lessons.iter().filter(|l| l.reward > 0.0).count();
        let failed = self.lessons.iter().filter(|l| l.reward < 0.0).count();

        LearningSummary {
            total_lessons,
            avg_reward,
            successful,
            failed,
            by_type: self.action_type_stats.clone(),
        }
    }

    fn generate_insight(&self, result: &ExecutionResult, reward: f64) -> String {
        let action_type = &result.action.action_type;
        let expected = result.action.expected_improvement;
        let actual = result.measured_improvement.unwrap_or(0.0);

        if !result.success {
            return format!("{} action failed: {}", action_type, result.message);
        }

        if actual >= expected * 1.2 {
            format!(
                "{} exceeded expectations: {:.1}% improvement vs {:.1}% expected",
                action_type,
                actual * 100.0,
                expected * 100.0
            )
        } else if actual >= expected * 0.8 {
            format!(
                "{} met expectations: {:.1}% improvement",
                action_type,
                actual * 100.0
            )
        } else if actual > 0.0 {
            format!(
                "{} underperformed: {:.1}% improvement vs {:.1}% expected (reward: {:.2})",
                action_type,
                actual * 100.0,
                expected * 100.0,
                reward
            )
        } else {
            format!(
                "{} had no measurable impact (reward: {:.2})",
                action_type, reward
            )
        }
    }

    fn identify_contexts(&self, result: &ExecutionResult) -> Vec<String> {
        let mut contexts = vec![result.action.action_type.to_string()];

        if result.success {
            contexts.push("successful".to_string());
        } else {
            contexts.push("failed".to_string());
        }

        if let Some(improvement) = result.measured_improvement {
            if improvement > 0.2 {
                contexts.push("high_impact".to_string());
            } else if improvement > 0.1 {
                contexts.push("medium_impact".to_string());
            } else {
                contexts.push("low_impact".to_string());
            }
        }

        // Add parameter-based contexts
        if let Some(params) = &result.action.parameters {
            if let Some(obj) = params.as_object() {
                for key in obj.keys() {
                    contexts.push(format!("param:{key}"));
                }
            }
        }

        contexts
    }

    fn update_stats(&mut self, action: &SelfImprovementAction, reward: f64) {
        let stats = self
            .action_type_stats
            .entry(action.action_type.clone())
            .or_default();

        stats.total_executions += 1;
        if action.status == ActionStatus::Completed {
            stats.successful += 1;
        }

        // Update rolling average reward
        let n = stats.total_executions as f64;
        stats.avg_reward = stats.avg_reward * (n - 1.0) / n + reward / n;

        stats.total_expected += action.expected_improvement;
        stats.total_actual += action.actual_improvement.unwrap_or(0.0);
    }

    fn trim_lessons(&mut self) {
        while self.lessons.len() > self.config.max_lessons {
            self.lessons.remove(0);
        }
    }
}

impl Default for Learner {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Summary of learning activity.
#[derive(Debug, Clone)]
pub struct LearningSummary {
    /// Total lessons learned.
    pub total_lessons: usize,
    /// Average reward across all lessons.
    pub avg_reward: f64,
    /// Number of successful lessons.
    pub successful: usize,
    /// Number of failed lessons.
    pub failed: usize,
    /// Stats by action type.
    pub by_type: HashMap<ActionType, ActionTypeStats>,
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

    fn create_execution_result(
        success: bool,
        expected: f64,
        actual: Option<f64>,
    ) -> ExecutionResult {
        let mut action = SelfImprovementAction::new(
            "test-action",
            ActionType::ConfigAdjust,
            "Test action",
            "Testing",
            expected,
        );

        if success {
            action.complete(actual.unwrap_or(0.0));
        } else {
            action.fail();
        }

        ExecutionResult {
            action,
            success,
            message: "Test".to_string(),
            measured_improvement: actual,
        }
    }

    #[test]
    fn test_learner_new() {
        let learner = Learner::with_defaults();
        assert!(learner.lessons().is_empty());
    }

    #[test]
    fn test_calculate_reward_success_exceeded() {
        let learner = Learner::with_defaults();
        let result = create_execution_result(true, 0.1, Some(0.2));

        let reward = learner.calculate_reward(&result);
        assert!(reward > 0.5);
    }

    #[test]
    fn test_calculate_reward_success_met() {
        let learner = Learner::with_defaults();
        let result = create_execution_result(true, 0.1, Some(0.1));

        let reward = learner.calculate_reward(&result);
        assert!(reward >= 0.0);
    }

    #[test]
    fn test_calculate_reward_success_underperformed() {
        let learner = Learner::with_defaults();
        let result = create_execution_result(true, 0.2, Some(0.05));

        let reward = learner.calculate_reward(&result);
        assert!(reward < 0.5);
    }

    #[test]
    fn test_calculate_reward_failure() {
        let learner = Learner::with_defaults();
        let result = create_execution_result(false, 0.1, None);

        let reward = learner.calculate_reward(&result);
        assert!((reward - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_learn_creates_lesson() {
        let mut learner = Learner::with_defaults();
        let result = create_execution_result(true, 0.1, Some(0.15));

        let learning = learner.learn(&result);
        assert!(learning.is_some());

        let learning = learning.unwrap();
        assert!(!learning.lesson.id.is_empty());
        assert!(learning.lesson.reward > 0.0);
    }

    #[test]
    fn test_learn_skips_non_terminal() {
        let mut learner = Learner::with_defaults();
        let mut action =
            SelfImprovementAction::new("test", ActionType::ConfigAdjust, "Test", "Test", 0.1);
        action.approve(); // Not completed or failed

        let result = ExecutionResult {
            action,
            success: true,
            message: "Test".to_string(),
            measured_improvement: Some(0.1),
        };

        let learning = learner.learn(&result);
        assert!(learning.is_none());
    }

    #[test]
    fn test_lessons_stored() {
        let mut learner = Learner::with_defaults();

        for i in 0..5 {
            let mut action = SelfImprovementAction::new(
                format!("action-{i}"),
                ActionType::ConfigAdjust,
                "Test",
                "Test",
                0.1,
            );
            action.complete(0.1);

            let result = ExecutionResult {
                action,
                success: true,
                message: "Test".to_string(),
                measured_improvement: Some(0.1),
            };

            learner.learn(&result);
        }

        assert_eq!(learner.lessons().len(), 5);
    }

    #[test]
    fn test_lessons_trimmed() {
        let config = LearnerConfig {
            max_lessons: 3,
            ..Default::default()
        };
        let mut learner = Learner::new(config);

        for i in 0..5 {
            let mut action = SelfImprovementAction::new(
                format!("action-{i}"),
                ActionType::ConfigAdjust,
                "Test",
                "Test",
                0.1,
            );
            action.complete(0.1);

            let result = ExecutionResult {
                action,
                success: true,
                message: "Test".to_string(),
                measured_improvement: Some(0.1),
            };

            learner.learn(&result);
        }

        assert_eq!(learner.lessons().len(), 3);
    }

    #[test]
    fn test_stats_updated() {
        let mut learner = Learner::with_defaults();
        let result = create_execution_result(true, 0.1, Some(0.15));

        learner.learn(&result);

        let stats = learner.stats_for_type(&ActionType::ConfigAdjust);
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().total_executions, 1);
        assert_eq!(stats.unwrap().successful, 1);
    }

    #[test]
    fn test_successful_lessons() {
        let mut learner = Learner::with_defaults();

        // Success
        let result1 = create_execution_result(true, 0.1, Some(0.15));
        learner.learn(&result1);

        // Failure
        let result2 = create_execution_result(false, 0.1, None);
        learner.learn(&result2);

        let successful = learner.successful_lessons();
        assert_eq!(successful.len(), 1);
    }

    #[test]
    fn test_lessons_by_action_type() {
        let mut learner = Learner::with_defaults();

        // ConfigAdjust
        let mut action1 =
            SelfImprovementAction::new("action-1", ActionType::ConfigAdjust, "Test", "Test", 0.1);
        action1.complete(0.1);
        let result1 = ExecutionResult {
            action: action1,
            success: true,
            message: "Test".to_string(),
            measured_improvement: Some(0.1),
        };
        learner.learn(&result1);

        // PromptTune
        let mut action2 =
            SelfImprovementAction::new("action-2", ActionType::PromptTune, "Test", "Test", 0.1);
        action2.complete(0.1);
        let result2 = ExecutionResult {
            action: action2,
            success: true,
            message: "Test".to_string(),
            measured_improvement: Some(0.1),
        };
        learner.learn(&result2);

        let config_lessons = learner.lessons_by_action_type(&ActionType::ConfigAdjust);
        assert_eq!(config_lessons.len(), 1);

        let prompt_lessons = learner.lessons_by_action_type(&ActionType::PromptTune);
        assert_eq!(prompt_lessons.len(), 1);
    }

    #[test]
    fn test_summary() {
        let mut learner = Learner::with_defaults();

        let result1 = create_execution_result(true, 0.1, Some(0.15));
        learner.learn(&result1);

        let result2 = create_execution_result(false, 0.1, None);
        learner.learn(&result2);

        let summary = learner.summary();
        assert_eq!(summary.total_lessons, 2);
        assert_eq!(summary.successful, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn test_insight_generation() {
        let learner = Learner::with_defaults();

        // Exceeded expectations
        let result1 = create_execution_result(true, 0.1, Some(0.2));
        let reward1 = learner.calculate_reward(&result1);
        let insight1 = learner.generate_insight(&result1, reward1);
        assert!(insight1.contains("exceeded"));

        // Met expectations
        let result2 = create_execution_result(true, 0.1, Some(0.1));
        let reward2 = learner.calculate_reward(&result2);
        let insight2 = learner.generate_insight(&result2, reward2);
        assert!(insight2.contains("met"));

        // Failed
        let result3 = create_execution_result(false, 0.1, None);
        let reward3 = learner.calculate_reward(&result3);
        let insight3 = learner.generate_insight(&result3, reward3);
        assert!(insight3.contains("failed"));
    }

    #[test]
    fn test_context_identification() {
        let learner = Learner::with_defaults();

        let mut action =
            SelfImprovementAction::new("test", ActionType::ConfigAdjust, "Test", "Test", 0.1);
        action = action.with_parameters(serde_json::json!({"timeout": 30000}));
        action.complete(0.25);

        let result = ExecutionResult {
            action,
            success: true,
            message: "Test".to_string(),
            measured_improvement: Some(0.25),
        };

        let contexts = learner.identify_contexts(&result);
        assert!(contexts.contains(&"config_adjust".to_string()));
        assert!(contexts.contains(&"successful".to_string()));
        assert!(contexts.contains(&"high_impact".to_string()));
        assert!(contexts.contains(&"param:timeout".to_string()));
    }

    #[test]
    fn test_reward_clamping() {
        let learner = Learner::with_defaults();

        // Very high improvement
        let result = create_execution_result(true, 0.1, Some(1.0));
        let reward = learner.calculate_reward(&result);
        assert!(reward <= 1.0);
        assert!(reward >= -1.0);
    }
}
