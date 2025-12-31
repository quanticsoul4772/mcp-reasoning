//! Response metadata for tool discoverability.
//!
//! This module provides rich metadata in tool responses to help AI agents:
//! - Predict execution times and avoid timeouts
//! - Discover next logical tools to call
//! - Find relevant workflow presets
//! - Learn optimal tool composition patterns
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::metadata::{ResponseMetadata, TimingMetadata, ConfidenceLevel};
//!
//! let metadata = ResponseMetadata {
//!     timing: TimingMetadata {
//!         estimated_duration_ms: 12000,
//!         confidence: ConfidenceLevel::High,
//!         will_timeout_on_factory: false,
//!         factory_timeout_ms: 30000,
//!     },
//!     suggestions: Default::default(),
//!     context: Default::default(),
//! };
//! ```

mod builder;
mod preset_index;
mod suggestions;
mod timing;
mod timing_defaults;

pub use builder::{MetadataBuilder, MetadataRequest};
pub use preset_index::{PresetIndex, PresetMetadata};
pub use suggestions::{ResultContext, SuggestionEngine};
pub use timing::{ComplexityMetrics, TimingDatabase};
pub use timing_defaults::get_default_timing;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response metadata for tool discoverability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct ResponseMetadata {
    /// Timing predictions and timeout analysis.
    pub timing: TimingMetadata,
    /// Tool and preset suggestions.
    pub suggestions: SuggestionMetadata,
    /// Execution context information.
    pub context: ContextMetadata,
}

/// Timing predictions and timeout analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct TimingMetadata {
    /// Estimated duration in milliseconds.
    pub estimated_duration_ms: u64,
    /// Confidence level of the estimate.
    pub confidence: ConfidenceLevel,
    /// Whether this will timeout on Factory (30s limit).
    pub will_timeout_on_factory: bool,
    /// Factory's MCP client timeout (typically 30000ms).
    pub factory_timeout_ms: u64,
}

/// Tool and preset suggestions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
pub struct SuggestionMetadata {
    /// Suggested next tools to call.
    pub next_tools: Vec<ToolSuggestion>,
    /// Relevant workflow presets.
    pub relevant_presets: Vec<PresetSuggestion>,
}

/// Execution context information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
pub struct ContextMetadata {
    /// Mode used for this execution.
    pub mode_used: String,
    /// Thinking budget level if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<String>,
    /// Session state information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_state: Option<String>,
}

/// Suggested tool to call next.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct ToolSuggestion {
    /// Tool name.
    pub tool: String,
    /// Reason for suggestion.
    pub reason: String,
    /// Estimated duration in milliseconds.
    pub estimated_duration_ms: u64,
}

/// Suggested preset workflow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct PresetSuggestion {
    /// Preset identifier.
    pub preset_id: String,
    /// Human-readable description.
    pub description: String,
    /// Estimated duration in milliseconds.
    pub estimated_duration_ms: u64,
}

/// Confidence level for timing estimates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ConfidenceLevel {
    /// High confidence (>100 samples).
    High,
    /// Medium confidence (10-100 samples).
    Medium,
    /// Low confidence (<10 samples or estimation).
    #[default]
    Low,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_metadata_serialization() {
        let metadata = ResponseMetadata {
            timing: TimingMetadata {
                estimated_duration_ms: 12000,
                confidence: ConfidenceLevel::High,
                will_timeout_on_factory: false,
                factory_timeout_ms: 30000,
            },
            suggestions: SuggestionMetadata {
                next_tools: vec![ToolSuggestion {
                    tool: "reasoning_decision".into(),
                    reason: "Synthesize perspectives".into(),
                    estimated_duration_ms: 15000,
                }],
                relevant_presets: vec![],
            },
            context: ContextMetadata {
                mode_used: "linear".into(),
                thinking_budget: None,
                session_state: None,
            },
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ResponseMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(metadata, deserialized);
    }

    #[test]
    fn test_confidence_level_ordering() {
        assert_eq!(ConfidenceLevel::default(), ConfidenceLevel::Low);
    }
}
