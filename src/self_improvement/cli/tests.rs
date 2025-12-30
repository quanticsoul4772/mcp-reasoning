//! Tests for CLI module.

use std::time::Duration;

use super::commands::SelfImproveCommands;
use super::duration::{format_duration, parse_duration};
use super::errors::CommandParseError;
use super::help::help_text;
use super::output_types::*;

#[test]
fn test_parse_status() {
    let args = vec!["status".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    assert!(matches!(cmd, SelfImproveCommands::Status));
}

#[test]
fn test_parse_history_default() {
    let args = vec!["history".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::History { limit, outcome } => {
            assert_eq!(limit, 10);
            assert!(outcome.is_none());
        }
        _ => panic!("Expected History command"),
    }
}

#[test]
fn test_parse_history_with_options() {
    let args = vec![
        "history".to_string(),
        "--limit".to_string(),
        "20".to_string(),
        "--outcome".to_string(),
        "success".to_string(),
    ];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::History { limit, outcome } => {
            assert_eq!(limit, 20);
            assert_eq!(outcome, Some("success".to_string()));
        }
        _ => panic!("Expected History command"),
    }
}

#[test]
fn test_parse_diagnostics() {
    let args = vec!["diagnostics".to_string(), "--verbose".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Diagnostics { verbose } => {
            assert!(verbose);
        }
        _ => panic!("Expected Diagnostics command"),
    }
}

#[test]
fn test_parse_pause() {
    let args = vec!["pause".to_string(), "2h".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Pause { duration } => {
            assert_eq!(duration, "2h");
        }
        _ => panic!("Expected Pause command"),
    }
}

#[test]
fn test_parse_approve() {
    let args = vec!["approve".to_string(), "diag-123".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Approve { diagnosis_id } => {
            assert_eq!(diagnosis_id, "diag-123");
        }
        _ => panic!("Expected Approve command"),
    }
}

#[test]
fn test_parse_reject_with_reason() {
    let args = vec![
        "reject".to_string(),
        "diag-456".to_string(),
        "Risk".to_string(),
        "too".to_string(),
        "high".to_string(),
    ];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Reject {
            diagnosis_id,
            reason,
        } => {
            assert_eq!(diagnosis_id, "diag-456");
            assert_eq!(reason, Some("Risk too high".to_string()));
        }
        _ => panic!("Expected Reject command"),
    }
}

#[test]
fn test_parse_unknown_command() {
    let args = vec!["unknown".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::UnknownCommand(_))));
}

#[test]
fn test_parse_missing_command() {
    let args: Vec<String> = vec![];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::MissingCommand)));
}

#[test]
fn test_parse_duration() {
    assert_eq!(parse_duration("100ms").unwrap(), Duration::from_millis(100));
    assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(5 * 60));
    assert_eq!(
        parse_duration("2h").unwrap(),
        Duration::from_secs(2 * 60 * 60)
    );
    assert_eq!(
        parse_duration("1d").unwrap(),
        Duration::from_secs(24 * 60 * 60)
    );
}

#[test]
fn test_parse_duration_error() {
    assert!(parse_duration("").is_err());
    assert!(parse_duration("abc").is_err());
    assert!(parse_duration("10x").is_err());
}

#[test]
fn test_format_duration() {
    assert_eq!(format_duration(Duration::from_secs(30)), "30s");
    assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
    assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
    assert_eq!(format_duration(Duration::from_secs(3660)), "1h 1m");
    assert_eq!(format_duration(Duration::from_secs(86400)), "1d");
    assert_eq!(format_duration(Duration::from_secs(90000)), "1d 1h");
}

// ========== Additional tests for 100% coverage ==========

// Parse config command
#[test]
fn test_parse_config() {
    let args = vec!["config".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    assert!(matches!(cmd, SelfImproveCommands::Config));
}

// Parse circuit-breaker command
#[test]
fn test_parse_circuit_breaker() {
    let args = vec!["circuit-breaker".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    assert!(matches!(cmd, SelfImproveCommands::CircuitBreaker));
}

// Parse cb shorthand
#[test]
fn test_parse_cb() {
    let args = vec!["cb".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    assert!(matches!(cmd, SelfImproveCommands::CircuitBreaker));
}

// Parse baselines command
#[test]
fn test_parse_baselines() {
    let args = vec!["baselines".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    assert!(matches!(cmd, SelfImproveCommands::Baselines));
}

// Parse rollback command
#[test]
fn test_parse_rollback() {
    let args = vec!["rollback".to_string(), "action-123".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Rollback { action_id } => {
            assert_eq!(action_id, "action-123");
        }
        _ => panic!("Expected Rollback command"),
    }
}

// Parse rollback missing action_id
#[test]
fn test_parse_rollback_missing_id() {
    let args = vec!["rollback".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::MissingValue(_))));
}

// Parse pause missing duration
#[test]
fn test_parse_pause_missing_duration() {
    let args = vec!["pause".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::MissingValue(_))));
}

// Parse approve missing diagnosis_id
#[test]
fn test_parse_approve_missing_id() {
    let args = vec!["approve".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::MissingValue(_))));
}

// Parse reject missing diagnosis_id
#[test]
fn test_parse_reject_missing_id() {
    let args = vec!["reject".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::MissingValue(_))));
}

// Parse reject without reason
#[test]
fn test_parse_reject_no_reason() {
    let args = vec!["reject".to_string(), "diag-123".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Reject {
            diagnosis_id,
            reason,
        } => {
            assert_eq!(diagnosis_id, "diag-123");
            assert!(reason.is_none());
        }
        _ => panic!("Expected Reject command"),
    }
}

// Parse diagnostics short form
#[test]
fn test_parse_diag() {
    let args = vec!["diag".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Diagnostics { verbose } => {
            assert!(!verbose);
        }
        _ => panic!("Expected Diagnostics command"),
    }
}

// Parse diagnostics with -v
#[test]
fn test_parse_diagnostics_short_verbose() {
    let args = vec!["diagnostics".to_string(), "-v".to_string()];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::Diagnostics { verbose } => {
            assert!(verbose);
        }
        _ => panic!("Expected Diagnostics command"),
    }
}

// Parse history with short flags
#[test]
fn test_parse_history_short_flags() {
    let args = vec![
        "history".to_string(),
        "-l".to_string(),
        "5".to_string(),
        "-o".to_string(),
        "failed".to_string(),
    ];
    let cmd = SelfImproveCommands::parse(&args).unwrap();
    match cmd {
        SelfImproveCommands::History { limit, outcome } => {
            assert_eq!(limit, 5);
            assert_eq!(outcome, Some("failed".to_string()));
        }
        _ => panic!("Expected History command"),
    }
}

// Parse history missing --limit value
#[test]
fn test_parse_history_missing_limit_value() {
    let args = vec!["history".to_string(), "--limit".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::MissingValue(_))));
}

// Parse history missing --outcome value
#[test]
fn test_parse_history_missing_outcome_value() {
    let args = vec!["history".to_string(), "--outcome".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::MissingValue(_))));
}

// Parse history invalid --limit value
#[test]
fn test_parse_history_invalid_limit() {
    let args = vec![
        "history".to_string(),
        "--limit".to_string(),
        "abc".to_string(),
    ];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(
        result,
        Err(CommandParseError::InvalidValue { .. })
    ));
}

// Parse history unknown flag
#[test]
fn test_parse_history_unknown_flag() {
    let args = vec!["history".to_string(), "--unknown".to_string()];
    let result = SelfImproveCommands::parse(&args);
    assert!(matches!(result, Err(CommandParseError::UnknownFlag(_))));
}

// CommandParseError Display tests
#[test]
fn test_command_parse_error_display() {
    let err = CommandParseError::MissingCommand;
    assert!(err.to_string().contains("No command provided"));

    let err = CommandParseError::UnknownCommand("foo".into());
    assert!(err.to_string().contains("Unknown command: 'foo'"));

    let err = CommandParseError::UnknownFlag("--bar".into());
    assert!(err.to_string().contains("Unknown flag: '--bar'"));

    let err = CommandParseError::MissingValue("--limit".into());
    assert!(err.to_string().contains("Missing value for '--limit'"));

    let err = CommandParseError::InvalidValue {
        flag: "--limit".into(),
        value: "abc".into(),
    };
    assert!(err
        .to_string()
        .contains("Invalid value 'abc' for '--limit'"));
}

// Test CommandParseError is Error
#[test]
fn test_command_parse_error_is_error() {
    let err: Box<dyn std::error::Error> = Box::new(CommandParseError::MissingCommand);
    assert!(err.to_string().contains("No command"));
}

// Test help_text function
#[test]
fn test_help_text() {
    let help = help_text();
    assert!(help.contains("status"));
    assert!(help.contains("history"));
    assert!(help.contains("diagnostics"));
    assert!(help.contains("circuit-breaker"));
    assert!(help.contains("baselines"));
    assert!(help.contains("pause"));
    assert!(help.contains("rollback"));
    assert!(help.contains("approve"));
    assert!(help.contains("reject"));
}

// format_duration edge cases
#[test]
fn test_format_duration_exact_minutes() {
    assert_eq!(format_duration(Duration::from_secs(60)), "1m");
    assert_eq!(format_duration(Duration::from_secs(120)), "2m");
}

#[test]
fn test_format_duration_exact_hours() {
    assert_eq!(format_duration(Duration::from_secs(7200)), "2h");
}

#[test]
fn test_format_duration_exact_days() {
    assert_eq!(format_duration(Duration::from_secs(172_800)), "2d");
}

// Output types serialization tests
#[test]
fn test_status_output_serialization() {
    let status = StatusOutput {
        enabled: true,
        paused: false,
        pause_remaining: None,
        circuit_breaker_state: "closed".into(),
        total_invocations: 1000,
        total_diagnoses: 10,
        total_actions: 5,
        pending_diagnoses: 2,
        current_error_rate: 0.05,
        current_latency_p95: 150,
        current_quality_score: 0.9,
        last_cycle_at: Some("2024-01-01T00:00:00Z".into()),
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("enabled"));
    assert!(json.contains("total_invocations"));
}

#[test]
fn test_history_output_serialization() {
    let history = HistoryOutput {
        actions: vec![ActionHistoryEntry {
            id: "action-1".into(),
            diagnosis_id: "diag-1".into(),
            action_type: "config_adjust".into(),
            outcome: "success".into(),
            execution_time_ms: 100,
            created_at: "2024-01-01T00:00:00Z".into(),
            reward: Some(0.5),
        }],
        total_count: 1,
    };
    let json = serde_json::to_string(&history).unwrap();
    assert!(json.contains("action-1"));
    assert!(json.contains("config_adjust"));
}

#[test]
fn test_diagnostics_output_serialization() {
    let diagnostics = DiagnosticsOutput {
        health: HealthDiagnostics {
            status: "healthy".into(),
            score: 0.95,
            issues: vec![],
        },
        recent_errors: vec![],
        resources: ResourceDiagnostics {
            memory_mb: 100.0,
            active_connections: 5,
            queue_depth: 10,
        },
        performance: PerformanceDiagnostics {
            avg_cycle_time_ms: 50.0,
            avg_analysis_time_ms: 30.0,
            avg_execution_time_ms: 20.0,
        },
    };
    let json = serde_json::to_string(&diagnostics).unwrap();
    assert!(json.contains("healthy"));
    assert!(json.contains("memory_mb"));
}

#[test]
fn test_config_output_serialization() {
    let config = ConfigOutput {
        monitor: MonitorConfigOutput {
            check_interval_secs: 60,
            min_samples: 100,
            error_rate_threshold: 0.1,
            latency_threshold_ms: 500,
            quality_threshold: 0.8,
        },
        analyzer: AnalyzerConfigOutput {
            model: "claude-3-haiku".into(),
            max_tokens: 1000,
            min_severity: "warning".into(),
        },
        executor: ExecutorConfigOutput {
            cooldown_secs: 300,
            rate_limit_per_hour: 10,
            auto_approve: false,
        },
        learner: LearnerConfigOutput {
            observation_window_secs: 3600,
            min_samples: 50,
        },
        circuit_breaker: CircuitBreakerConfigOutput {
            failure_threshold: 3,
            reset_timeout_secs: 60,
            half_open_max: 1,
        },
        overrides: vec![ConfigOverrideOutput {
            key: "timeout".into(),
            value: "60000".into(),
            applied_by: Some("action-1".into()),
            updated_at: "2024-01-01T00:00:00Z".into(),
        }],
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("check_interval_secs"));
    assert!(json.contains("claude-3-haiku"));
}

#[test]
fn test_circuit_breaker_output_serialization() {
    let cb = CircuitBreakerOutput {
        state: "closed".into(),
        consecutive_failures: 0,
        last_failure_at: None,
        reset_in: None,
        total_trips: 0,
    };
    let json = serde_json::to_string(&cb).unwrap();
    assert!(json.contains("closed"));
    assert!(json.contains("consecutive_failures"));
}

#[test]
fn test_baselines_output_serialization() {
    let baselines = BaselinesOutput {
        global: GlobalBaselinesOutput {
            error_rate: 0.05,
            latency_p95_ms: 100,
            quality_score: 0.9,
            sample_count: 1000,
            is_valid: true,
        },
        tools: vec![ToolBaselinesOutput {
            tool_name: "reasoning_linear".into(),
            error_rate: 0.03,
            latency_ms: 80.0,
            quality_score: 0.92,
            sample_count: 500,
        }],
    };
    let json = serde_json::to_string(&baselines).unwrap();
    assert!(json.contains("error_rate"));
    assert!(json.contains("reasoning_linear"));
}

// parse_duration invalid number
#[test]
fn test_parse_duration_invalid_number() {
    assert!(parse_duration("xxs").is_err());
    assert!(parse_duration("-5s").is_err());
}
