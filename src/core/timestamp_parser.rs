use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimestampParserError {
    InvalidPattern { pattern: String, message: String },
    ProcessingFailed { message: String },
}

impl TimestampParserError {
    pub fn invalid_pattern(pattern: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidPattern {
            pattern: pattern.into(),
            message: message.into(),
        }
    }

    pub fn processing_failed(message: impl Into<String>) -> Self {
        Self::ProcessingFailed {
            message: message.into(),
        }
    }
}

impl fmt::Display for TimestampParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimestampParserError::InvalidPattern { pattern, message } => {
                write!(f, "invalid timestamp pattern '{pattern}': {message}")
            }
            TimestampParserError::ProcessingFailed { message } => {
                write!(f, "failed to strip timestamps: {message}")
            }
        }
    }
}

impl Error for TimestampParserError {}

pub trait TimestampParserOperations: Send + Sync {
    fn strip_timestamps(
        &self,
        lines: &[String],
        pattern: &str,
    ) -> Result<Vec<String>, TimestampParserError>;
}
