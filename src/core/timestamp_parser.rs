use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::RwLock;

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

pub struct CoreTimestampParser {
    cache: RwLock<HashMap<String, Regex>>,
}

impl CoreTimestampParser {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    fn cache_len(&self) -> usize {
        self.cache.read().unwrap().len()
    }
}

impl TimestampParserOperations for CoreTimestampParser {
    fn strip_timestamps(
        &self,
        lines: &[String],
        pattern: &str,
    ) -> Result<Vec<String>, TimestampParserError> {
        if pattern.is_empty() {
            return Ok(lines.to_vec());
        }

        let regex = {
            if let Some(cached) = self.cache.read().unwrap().get(pattern) {
                cached.clone()
            } else {
                let compiled = Regex::new(pattern)
                    .map_err(|e| TimestampParserError::invalid_pattern(pattern, e.to_string()))?;
                let mut cache = self.cache.write().unwrap();
                cache.entry(pattern.to_string()).or_insert(compiled).clone()
            }
        };

        let stripped_lines = lines
            .iter()
            .map(|line| regex.replace_all(line, "").to_string())
            .collect();

        Ok(stripped_lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pattern_strips_timestamps() {
        let parser = CoreTimestampParser::new();
        let lines = vec![
            "[2023-10-27 10:00:00] INFO: System start".to_string(),
            "DEBUG: No timestamp here".to_string(),
            "[2023-10-27 10:00:01] WARN: System alert".to_string(),
        ];
        let pattern = r"\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\] ";

        let result = parser.strip_timestamps(&lines, pattern).unwrap();

        assert_eq!(result[0], "INFO: System start");
        assert_eq!(result[1], "DEBUG: No timestamp here");
        assert_eq!(result[2], "WARN: System alert");
    }

    #[test]
    fn test_invalid_regex_pattern_returns_error() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["line 1".to_string()];
        let invalid_pattern = "[";

        let result = parser.strip_timestamps(&lines, invalid_pattern);

        assert!(result.is_err());
        match result.unwrap_err() {
            TimestampParserError::InvalidPattern { pattern, .. } => {
                assert_eq!(pattern, invalid_pattern);
            }
            other => panic!("Expected InvalidPattern error, got {other:?}"),
        }
    }

    #[test]
    fn test_empty_pattern_returns_original_lines() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["line 1".to_string(), "line 2".to_string()];

        let result = parser.strip_timestamps(&lines, "").unwrap();

        assert_eq!(result, lines);
        assert_eq!(parser.cache_len(), 0);
    }

    #[test]
    fn test_no_matches_returns_original_lines() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["line 1".to_string(), "another line".to_string()];
        let pattern = "xyz";

        let result = parser.strip_timestamps(&lines, pattern).unwrap();

        assert_eq!(result, lines);
        assert_eq!(parser.cache_len(), 1);
    }

    #[test]
    fn test_regex_is_cached_after_first_use() {
        let parser = CoreTimestampParser::new();
        let lines = vec![
            "[10:00] entry".to_string(),
            "[10:01] another".to_string(),
            "no timestamp".to_string(),
        ];
        let pattern = r"\[\d{2}:\d{2}\] ";

        let result_one = parser.strip_timestamps(&lines, pattern).unwrap();
        assert_eq!(parser.cache_len(), 1);

        let result_two = parser.strip_timestamps(&lines, pattern).unwrap();
        assert_eq!(parser.cache_len(), 1, "pattern should remain cached");
        assert_eq!(result_one, result_two);
    }

    #[test]
    fn test_invalid_pattern_does_not_pollute_cache() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["entry".to_string()];

        assert!(parser.strip_timestamps(&lines, "[").is_err());
        assert_eq!(parser.cache_len(), 0);
    }
}
