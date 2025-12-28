//! CLI commands for the self-improvement system.
//!
//! Provides commands for monitoring, controlling, and debugging
//! the self-improvement system from the command line.

use std::time::Duration;

use serde::{Deserialize, Serialize};

// ============================================================================
// CLI Commands
// ============================================================================

/// Self-improvement CLI commands.
#[derive(Debug, Clone)]
pub enum SelfImproveCommands {
    /// Show current system status.
    Status,

    /// Show action history.
    History {
        /// Maximum number of records to show.
        limit: usize,
        /// Filter by outcome (success, failed, rolled_back).
        outcome: Option<String>,
    },

    /// Show diagnostic information.
    Diagnostics {
        /// Show verbose output.
        verbose: bool,
    },

    /// Show current configuration.
    Config,

    /// Show circuit breaker status.
    CircuitBreaker,

    /// Show current baselines.
    Baselines,

    /// Temporarily pause the system.
    Pause {
        /// Duration to pause (e.g., "1h", "30m", "2d").
        duration: String,
    },

    /// Rollback a specific action.
    Rollback {
        /// Action ID to rollback.
        action_id: String,
    },

    /// Approve a pending diagnosis.
    Approve {
        /// Diagnosis ID to approve.
        diagnosis_id: String,
    },

    /// Reject a pending diagnosis.
    Reject {
        /// Diagnosis ID to reject.
        diagnosis_id: String,
        /// Optional reason for rejection.
        reason: Option<String>,
    },
}

impl SelfImproveCommands {
    /// Parse a command from string arguments.
    pub fn parse(args: &[String]) -> Result<Self, CommandParseError> {
        if args.is_empty() {
            return Err(CommandParseError::MissingCommand);
        }

        let cmd = args[0].to_lowercase();
        match cmd.as_str() {
            "status" => Ok(Self::Status),

            "history" => {
                let mut limit = 10;
                let mut outcome = None;

                let mut i = 1;
                while i < args.len() {
                    match args[i].as_str() {
                        "--limit" | "-l" => {
                            i += 1;
                            if i >= args.len() {
                                return Err(CommandParseError::MissingValue("--limit".into()));
                            }
                            limit =
                                args[i]
                                    .parse()
                                    .map_err(|_| CommandParseError::InvalidValue {
                                        flag: "--limit".into(),
                                        value: args[i].clone(),
                                    })?;
                        }
                        "--outcome" | "-o" => {
                            i += 1;
                            if i >= args.len() {
                                return Err(CommandParseError::MissingValue("--outcome".into()));
                            }
                            outcome = Some(args[i].clone());
                        }
                        _ => {
                            return Err(CommandParseError::UnknownFlag(args[i].clone()));
                        }
                    }
                    i += 1;
                }

                Ok(Self::History { limit, outcome })
            }

            "diagnostics" | "diag" => {
                let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
                Ok(Self::Diagnostics { verbose })
            }

            "config" => Ok(Self::Config),

            "circuit-breaker" | "cb" => Ok(Self::CircuitBreaker),

            "baselines" => Ok(Self::Baselines),

            "pause" => {
                if args.len() < 2 {
                    return Err(CommandParseError::MissingValue("duration".into()));
                }
                Ok(Self::Pause {
                    duration: args[1].clone(),
                })
            }

            "rollback" => {
                if args.len() < 2 {
                    return Err(CommandParseError::MissingValue("action_id".into()));
                }
                Ok(Self::Rollback {
                    action_id: args[1].clone(),
                })
            }

            "approve" => {
                if args.len() < 2 {
                    return Err(CommandParseError::MissingValue("diagnosis_id".into()));
                }
                Ok(Self::Approve {
                    diagnosis_id: args[1].clone(),
                })
            }

            "reject" => {
                if args.len() < 2 {
                    return Err(CommandParseError::MissingValue("diagnosis_id".into()));
                }
                let reason = if args.len() > 2 {
                    Some(args[2..].join(" "))
                } else {
                    None
                };
                Ok(Self::Reject {
                    diagnosis_id: args[1].clone(),
                    reason,
                })
            }

            _ => Err(CommandParseError::UnknownCommand(cmd)),
        }
    }
}

// ============================================================================
// Command Parse Error
// ============================================================================

/// Error parsing CLI commands.
#[derive(Debug, Clone)]
pub enum CommandParseError {
    /// No command provided.
    MissingCommand,
    /// Unknown command.
    UnknownCommand(String),
    /// Unknown flag.
    UnknownFlag(String),
    /// Missing value for flag.
    MissingValue(String),
    /// Invalid value for flag.
    InvalidValue {
        /// The flag with the invalid value.
        flag: String,
        /// The invalid value that was provided.
        value: String,
    },
}

impl std::fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCommand => {
                write!(f, "No command provided. Use 'help' for available commands.")
            }
            Self::UnknownCommand(cmd) => write!(
                f,
                "Unknown command: '{cmd}'. Use 'help' for available commands."
            ),
            Self::UnknownFlag(flag) => write!(f, "Unknown flag: '{flag}'"),
            Self::MissingValue(flag) => write!(f, "Missing value for '{flag}'"),
            Self::InvalidValue { flag, value } => write!(f, "Invalid value '{value}' for '{flag}'"),
        }
    }
}

impl std::error::Error for CommandParseError {}

// ============================================================================
// Command Output Types
// ============================================================================

/// Status output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusOutput {
    /// Whether the system is enabled.
    pub enabled: bool,
    /// Whether the system is paused.
    pub paused: bool,
    /// Pause remaining duration (if paused).
    pub pause_remaining: Option<String>,
    /// Circuit breaker state.
    pub circuit_breaker_state: String,
    /// Total invocations processed.
    pub total_invocations: u64,
    /// Total diagnoses created.
    pub total_diagnoses: u64,
    /// Total actions executed.
    pub total_actions: u64,
    /// Pending diagnoses count.
    pub pending_diagnoses: u64,
    /// Current error rate.
    pub current_error_rate: f64,
    /// Current latency P95.
    pub current_latency_p95: i64,
    /// Current quality score.
    pub current_quality_score: f64,
    /// Last cycle time.
    pub last_cycle_at: Option<String>,
}

/// History output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryOutput {
    /// Action records.
    pub actions: Vec<ActionHistoryEntry>,
    /// Total count (may be more than returned).
    pub total_count: u64,
}

/// Action history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHistoryEntry {
    /// Action ID.
    pub id: String,
    /// Diagnosis ID.
    pub diagnosis_id: String,
    /// Action type.
    pub action_type: String,
    /// Outcome.
    pub outcome: String,
    /// Execution time.
    pub execution_time_ms: i64,
    /// Created at.
    pub created_at: String,
    /// Reward (if learning completed).
    pub reward: Option<f64>,
}

/// Diagnostics output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsOutput {
    /// System health.
    pub health: HealthDiagnostics,
    /// Recent errors.
    pub recent_errors: Vec<String>,
    /// Resource usage.
    pub resources: ResourceDiagnostics,
    /// Performance metrics.
    pub performance: PerformanceDiagnostics,
}

/// Health diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDiagnostics {
    /// Overall health status.
    pub status: String,
    /// Health score (0.0 to 1.0).
    pub score: f64,
    /// Issues detected.
    pub issues: Vec<String>,
}

/// Resource diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDiagnostics {
    /// Memory usage.
    pub memory_mb: f64,
    /// Active connections.
    pub active_connections: u32,
    /// Queue depth.
    pub queue_depth: u32,
}

/// Performance diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDiagnostics {
    /// Average cycle time.
    pub avg_cycle_time_ms: f64,
    /// Average analysis time.
    pub avg_analysis_time_ms: f64,
    /// Average execution time.
    pub avg_execution_time_ms: f64,
}

/// Config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOutput {
    /// Monitor configuration.
    pub monitor: MonitorConfigOutput,
    /// Analyzer configuration.
    pub analyzer: AnalyzerConfigOutput,
    /// Executor configuration.
    pub executor: ExecutorConfigOutput,
    /// Learner configuration.
    pub learner: LearnerConfigOutput,
    /// Circuit breaker configuration.
    pub circuit_breaker: CircuitBreakerConfigOutput,
    /// Applied overrides.
    pub overrides: Vec<ConfigOverrideOutput>,
}

/// Monitor config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfigOutput {
    /// Check interval.
    pub check_interval_secs: u64,
    /// Minimum samples.
    pub min_samples: u64,
    /// Error rate threshold.
    pub error_rate_threshold: f64,
    /// Latency threshold.
    pub latency_threshold_ms: i64,
    /// Quality threshold.
    pub quality_threshold: f64,
}

/// Analyzer config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfigOutput {
    /// Model used.
    pub model: String,
    /// Max tokens.
    pub max_tokens: u32,
    /// Minimum severity.
    pub min_severity: String,
}

/// Executor config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfigOutput {
    /// Cooldown between actions.
    pub cooldown_secs: u64,
    /// Rate limit per hour.
    pub rate_limit_per_hour: u32,
    /// Auto-approve enabled.
    pub auto_approve: bool,
}

/// Learner config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerConfigOutput {
    /// Observation window.
    pub observation_window_secs: u64,
    /// Minimum samples for learning.
    pub min_samples: u64,
}

/// Circuit breaker config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfigOutput {
    /// Failure threshold.
    pub failure_threshold: u32,
    /// Reset timeout.
    pub reset_timeout_secs: u64,
    /// Half-open max.
    pub half_open_max: u32,
}

/// Config override output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOverrideOutput {
    /// Override key.
    pub key: String,
    /// Override value.
    pub value: String,
    /// Applied by action ID.
    pub applied_by: Option<String>,
    /// Updated at.
    pub updated_at: String,
}

/// Circuit breaker output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerOutput {
    /// Current state.
    pub state: String,
    /// Consecutive failures.
    pub consecutive_failures: u32,
    /// Last failure time.
    pub last_failure_at: Option<String>,
    /// Time until reset (if open).
    pub reset_in: Option<String>,
    /// Total trips.
    pub total_trips: u64,
}

/// Baselines output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselinesOutput {
    /// Global baselines.
    pub global: GlobalBaselinesOutput,
    /// Per-tool baselines.
    pub tools: Vec<ToolBaselinesOutput>,
}

/// Global baselines output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBaselinesOutput {
    /// Error rate baseline.
    pub error_rate: f64,
    /// Latency P95 baseline.
    pub latency_p95_ms: i64,
    /// Quality score baseline.
    pub quality_score: f64,
    /// Sample count.
    pub sample_count: u64,
    /// Is valid.
    pub is_valid: bool,
}

/// Tool baselines output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBaselinesOutput {
    /// Tool name.
    pub tool_name: String,
    /// Error rate baseline.
    pub error_rate: f64,
    /// Latency baseline.
    pub latency_ms: f64,
    /// Quality baseline.
    pub quality_score: f64,
    /// Sample count.
    pub sample_count: u64,
}

// ============================================================================
// Duration Parsing
// ============================================================================

/// Parse a duration string (e.g., "1h", "30m", "2d").
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return Err("Empty duration string".into());
    }

    let (num_str, unit) = if s.ends_with("ms") {
        (&s[..s.len() - 2], "ms")
    } else if s.ends_with('s') {
        (&s[..s.len() - 1], "s")
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], "m")
    } else if s.ends_with('h') {
        (&s[..s.len() - 1], "h")
    } else if s.ends_with('d') {
        (&s[..s.len() - 1], "d")
    } else {
        return Err(format!("Unknown duration unit in '{s}'"));
    };

    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("Invalid number in duration: '{num_str}'"))?;

    let millis = match unit {
        "ms" => num,
        "s" => num * 1000,
        "m" => num * 60 * 1000,
        "h" => num * 60 * 60 * 1000,
        "d" => num * 24 * 60 * 60 * 1000,
        _ => return Err(format!("Unknown duration unit: '{unit}'")),
    };

    Ok(Duration::from_millis(millis))
}

/// Format a duration for display.
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let rem = secs % 60;
        if rem == 0 {
            format!("{}m", mins)
        } else {
            format!("{}m {}s", mins, rem)
        }
    } else if secs < 86400 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, mins)
        }
    } else {
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        if hours == 0 {
            format!("{}d", days)
        } else {
            format!("{}d {}h", days, hours)
        }
    }
}

// ============================================================================
// Help Text
// ============================================================================

/// Get help text for CLI commands.
pub fn help_text() -> &'static str {
    r#"Self-Improvement System Commands:

  status              Show current system status

  history [options]   Show action history
    --limit, -l N       Maximum records to show (default: 10)
    --outcome, -o TYPE  Filter by outcome (success, failed, rolled_back)

  diagnostics, diag   Show diagnostic information
    --verbose, -v       Show verbose output

  config              Show current configuration

  circuit-breaker, cb Show circuit breaker status

  baselines           Show current baselines

  pause DURATION      Temporarily pause the system
                      Duration: e.g., "1h", "30m", "2d"

  rollback ACTION_ID  Rollback a specific action

  approve DIAG_ID     Approve a pending diagnosis

  reject DIAG_ID [REASON]
                      Reject a pending diagnosis with optional reason

Examples:
  self-improve status
  self-improve history --limit 20 --outcome success
  self-improve diagnostics --verbose
  self-improve pause 1h
  self-improve approve diag-abc123
  self-improve reject diag-xyz789 "Risk too high"
"#
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(format_duration(Duration::from_secs(172800)), "2d");
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
}
