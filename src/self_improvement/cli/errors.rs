//! CLI command parsing errors.

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
