//! Anthropic API client for self-improvement operations.

use std::sync::Arc;

use super::parsers::{
    parse_action_response, parse_diagnosis_response, parse_learning_response,
    parse_validation_response,
};
use super::prompts::{
    build_action_prompt, build_diagnosis_prompt, build_learning_prompt, build_validation_prompt,
    ACTION_SYSTEM_PROMPT, DIAGNOSIS_SYSTEM_PROMPT, LEARNING_SYSTEM_PROMPT,
    VALIDATION_SYSTEM_PROMPT,
};
use super::types::{
    DiagnosisContent, HealthContext, LearningContext, LearningSynthesis, ValidationResult,
};
use crate::error::ModeError;
use crate::self_improvement::types::{SuggestedAction, TriggerMetric};
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message};

/// Anthropic API calls for self-improvement.
///
/// The `Send + Sync` bounds ensure thread-safe sharing across async executors.
pub struct AnthropicCalls<C: AnthropicClientTrait + Send + Sync> {
    client: Arc<C>,
    /// Maximum tokens for API responses.
    pub max_tokens: u32,
}

impl<C: AnthropicClientTrait + Send + Sync> AnthropicCalls<C> {
    /// Create a new instance.
    pub fn new(client: Arc<C>, max_tokens: u32) -> Self {
        Self { client, max_tokens }
    }

    /// Generate a diagnosis from health context.
    pub async fn generate_diagnosis(
        &self,
        health: &HealthContext,
    ) -> Result<DiagnosisContent, ModeError> {
        let prompt = build_diagnosis_prompt(health);
        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::new()
            .with_max_tokens(self.max_tokens)
            .with_system_prompt(DIAGNOSIS_SYSTEM_PROMPT);

        let response = self.client.complete(messages, config).await?;

        parse_diagnosis_response(&response.content)
    }

    /// Select an action for a diagnosis.
    pub async fn select_action(
        &self,
        diagnosis: &DiagnosisContent,
        trigger: &TriggerMetric,
    ) -> Result<SuggestedAction, ModeError> {
        let prompt = build_action_prompt(diagnosis, trigger);
        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::new()
            .with_max_tokens(self.max_tokens)
            .with_system_prompt(ACTION_SYSTEM_PROMPT);

        let response = self.client.complete(messages, config).await?;

        parse_action_response(&response.content)
    }

    /// Validate a suggested action.
    pub async fn validate_decision(
        &self,
        action: &SuggestedAction,
        context: &str,
    ) -> Result<ValidationResult, ModeError> {
        let prompt = build_validation_prompt(action, context);
        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::new()
            .with_max_tokens(self.max_tokens)
            .with_system_prompt(VALIDATION_SYSTEM_PROMPT);

        let response = self.client.complete(messages, config).await?;

        parse_validation_response(&response.content)
    }

    /// Synthesize learning from an outcome.
    pub async fn synthesize_learning(
        &self,
        learning: &LearningContext,
    ) -> Result<LearningSynthesis, ModeError> {
        let prompt = build_learning_prompt(learning);
        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::new()
            .with_max_tokens(self.max_tokens)
            .with_system_prompt(LEARNING_SYSTEM_PROMPT);

        let response = self.client.complete(messages, config).await?;

        parse_learning_response(&response.content)
    }
}
