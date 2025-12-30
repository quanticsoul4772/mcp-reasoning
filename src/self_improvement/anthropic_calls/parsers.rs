//! Response parsers and helper functions for LLM responses.

use serde::Deserialize;

use super::security::MAX_JSON_SIZE;
use super::types::{DiagnosisContent, LearningSynthesis, ValidationResult};
use crate::error::ModeError;
use crate::self_improvement::types::{ConfigScope, ParamValue, ResourceType, SuggestedAction};

// ============================================================================
// Response Parsers
// ============================================================================

pub fn parse_diagnosis_response(response: &str) -> Result<DiagnosisContent, ModeError> {
    let json_str = extract_json(response)?;
    serde_json::from_str(&json_str).map_err(|e| ModeError::JsonParseFailed {
        message: format!("Failed to parse diagnosis: {e}"),
    })
}

pub fn parse_action_response(response: &str) -> Result<SuggestedAction, ModeError> {
    let json_str = extract_json(response)?;

    // Parse the intermediate representation
    let parsed: ActionResponse =
        serde_json::from_str(&json_str).map_err(|e| ModeError::JsonParseFailed {
            message: format!("Failed to parse action response: {e}"),
        })?;

    // Convert to SuggestedAction
    match parsed.action_type.as_str() {
        "adjust_param" => {
            let key = parsed.key.ok_or_else(|| ModeError::MissingField {
                field: "key".into(),
            })?;
            let old_value = parse_param_value(parsed.old_value.as_ref())?;
            let new_value = parse_param_value(parsed.new_value.as_ref())?;
            let scope = parse_scope(parsed.scope.as_ref())?;

            Ok(SuggestedAction::AdjustParam {
                key,
                old_value,
                new_value,
                scope,
            })
        }
        "scale_resource" => {
            let resource_str = parsed.resource.ok_or_else(|| ModeError::MissingField {
                field: "resource".into(),
            })?;
            let resource = parse_resource_type(&resource_str)?;
            let old_value = parsed
                .old_value
                .as_ref()
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ModeError::InvalidValue {
                    field: "old_value".into(),
                    reason: "Must be a positive integer".into(),
                })? as u32;
            let new_value = parsed
                .new_value
                .as_ref()
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ModeError::InvalidValue {
                    field: "new_value".into(),
                    reason: "Must be a positive integer".into(),
                })? as u32;

            Ok(SuggestedAction::ScaleResource {
                resource,
                old_value,
                new_value,
            })
        }
        "no_op" => {
            let reason = parsed.reason.unwrap_or_else(|| "No action needed".into());
            Ok(SuggestedAction::NoOp {
                reason,
                revisit_after: std::time::Duration::from_secs(3600),
            })
        }
        _ => Err(ModeError::InvalidValue {
            field: "action_type".into(),
            reason: format!("Unknown action type: {}", parsed.action_type),
        }),
    }
}

pub fn parse_validation_response(response: &str) -> Result<ValidationResult, ModeError> {
    let json_str = extract_json(response)?;
    serde_json::from_str(&json_str).map_err(|e| ModeError::JsonParseFailed {
        message: format!("Failed to parse validation response: {e}"),
    })
}

pub fn parse_learning_response(response: &str) -> Result<LearningSynthesis, ModeError> {
    let json_str = extract_json(response)?;
    serde_json::from_str(&json_str).map_err(|e| ModeError::JsonParseFailed {
        message: format!("Failed to parse learning response: {e}"),
    })
}

// ============================================================================
// Helper Types
// ============================================================================

/// Intermediate action response from LLM.
#[derive(Debug, Deserialize)]
pub struct ActionResponse {
    pub action_type: String,
    pub key: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub scope: Option<String>,
    pub resource: Option<String>,
    pub reason: Option<String>,
    #[allow(dead_code)]
    pub rationale: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

pub fn extract_json(text: &str) -> Result<String, ModeError> {
    // Enforce size limit first to prevent DoS via large responses
    if text.len() > MAX_JSON_SIZE {
        return Err(ModeError::InvalidValue {
            field: "response".into(),
            reason: format!(
                "Response too large: {} bytes (max: {})",
                text.len(),
                MAX_JSON_SIZE
            ),
        });
    }

    let text = text.trim();

    // If it starts with {, assume it's raw JSON
    if text.starts_with('{') {
        let mut depth = 0;
        let mut end = 0;
        for (i, c) in text.chars().enumerate() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if end > 0 {
            return Ok(text[..end].to_string());
        }
    }

    // Try to extract from markdown code block
    if let Some(start) = text.find("```json") {
        let start = start + 7;
        if let Some(end) = text[start..].find("```") {
            let json_content = text[start..start + end].trim();
            // Double-check extracted size (should already be within limit)
            if json_content.len() > MAX_JSON_SIZE {
                return Err(ModeError::InvalidValue {
                    field: "json".into(),
                    reason: format!("Extracted JSON too large: {} bytes", json_content.len()),
                });
            }
            return Ok(json_content.to_string());
        }
    }

    // Try plain code block
    if let Some(start) = text.find("```") {
        let start = start + 3;
        let start = text[start..].find('\n').map_or(start, |nl| start + nl + 1);
        if let Some(end) = text[start..].find("```") {
            let json_content = text[start..start + end].trim();
            if json_content.len() > MAX_JSON_SIZE {
                return Err(ModeError::InvalidValue {
                    field: "json".into(),
                    reason: format!("Extracted JSON too large: {} bytes", json_content.len()),
                });
            }
            return Ok(json_content.to_string());
        }
    }

    Err(ModeError::JsonParseFailed {
        message: format!(
            "Could not extract JSON from response: {}",
            &text[..text.len().min(200)]
        ),
    })
}

pub fn parse_param_value(value: Option<&serde_json::Value>) -> Result<ParamValue, ModeError> {
    let value = value.ok_or_else(|| ModeError::MissingField {
        field: "value".into(),
    })?;

    match value {
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(ParamValue::Integer)
            .or_else(|| n.as_f64().map(ParamValue::Float))
            .ok_or_else(|| ModeError::InvalidValue {
                field: "value".into(),
                reason: "Invalid number".into(),
            }),
        serde_json::Value::String(s) => {
            // Check if it looks like a duration
            if s.ends_with("ms") || s.ends_with('s') || s.ends_with('m') || s.ends_with('h') {
                if let Ok(d) = crate::self_improvement::cli::parse_duration(s) {
                    return Ok(ParamValue::DurationMs(d.as_millis() as u64));
                }
            }
            Ok(ParamValue::String(s.clone()))
        }
        serde_json::Value::Bool(b) => Ok(ParamValue::Boolean(*b)),
        _ => Err(ModeError::InvalidValue {
            field: "value".into(),
            reason: format!("Unsupported value type: {value:?}"),
        }),
    }
}

pub fn parse_scope(scope: Option<&String>) -> Result<ConfigScope, ModeError> {
    let scope_str = scope.map_or("global", String::as_str);

    let config_scope = if scope_str == "global" {
        ConfigScope::Global
    } else if let Some(mode) = scope_str.strip_prefix("mode:") {
        ConfigScope::Mode(mode.to_string())
    } else if let Some(tool) = scope_str.strip_prefix("tool:") {
        ConfigScope::Tool(tool.to_string())
    } else {
        return Err(ModeError::InvalidValue {
            field: "scope".into(),
            reason: format!("Invalid scope format: {scope_str}"),
        });
    };

    // Validate mode/tool names against known values
    config_scope
        .validate()
        .map_err(|reason| ModeError::InvalidValue {
            field: "scope".into(),
            reason,
        })?;

    Ok(config_scope)
}

pub fn parse_resource_type(resource: &str) -> Result<ResourceType, ModeError> {
    match resource.to_lowercase().as_str() {
        "max_concurrent_requests" => Ok(ResourceType::MaxConcurrentRequests),
        "connection_pool_size" => Ok(ResourceType::ConnectionPoolSize),
        "cache_size" => Ok(ResourceType::CacheSize),
        "timeout_ms" => Ok(ResourceType::TimeoutMs),
        "max_retries" => Ok(ResourceType::MaxRetries),
        "retry_delay_ms" => Ok(ResourceType::RetryDelayMs),
        _ => Err(ModeError::InvalidValue {
            field: "resource".into(),
            reason: format!("Unknown resource type: {resource}"),
        }),
    }
}
