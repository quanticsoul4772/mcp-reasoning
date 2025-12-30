//! CLI command definitions and parsing.

use super::errors::CommandParseError;

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
