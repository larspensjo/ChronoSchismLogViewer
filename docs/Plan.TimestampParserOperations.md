This task involves creating a concrete implementation for the `TimestampParserOperations` trait you've already defined. This component is crucial for satisfying requirement `[CSV-Core-IgnoreTSV1]` and will complete the core, platform-agnostic logic of your application.

Here is a step-by-step plan to implement and test it.

### Next Step: Implement `CoreTimestampParser` ([CSV-Core-TSPatternV1])

**Goal:** Implement the `TimestampParserOperations` trait for a new `CoreTimestampParser` struct in `src/core/timestamp_parser.rs`.

---

#### Step 1: Add Dependencies

The implementation will rely on the `regex` crate. You need to add it to your project's `Cargo.toml`.

**In `Cargo.toml`:**

Add the `regex` dependency. Your `[dependencies]` section should look like this:

```toml
[dependencies]
commanductui = { path = "src/CommanDuctUI" }
regex = "1.10.5" # Using a recent, stable version
```

---

#### Step 2: Boilerplate and Initial Test Setup

Let's create the struct and a placeholder implementation so you can immediately run `cargo test`.

**In `src/core/timestamp_parser.rs`:**

1.  **Create the `CoreTimestampParser` struct:**
    ```rust
    pub struct CoreTimestampParser;

    impl CoreTimestampParser {
        pub fn new() -> Self {
            Self
        }
    }
    ```

2.  **Implement the trait with a `todo!()` placeholder:**
    ```rust
    impl TimestampParserOperations for CoreTimestampParser {
        fn strip_timestamps(
            &self,
            lines: &[String],
            pattern: &str,
        ) -> Result<Vec<String>, TimestampParserError> {
            todo!("Implement timestamp stripping with regex");
        }
    }
    ```

3.  **Add the test module and a failing test:**
    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[should_panic]
        fn test_strip_timestamps_panics_on_todo() {
            let parser = CoreTimestampParser::new();
            let lines = vec!["line 1".to_string()];
            let _ = parser.strip_timestamps(&lines, ".*");
        }
    }
    ```

**Build Check:** Run `cargo test`. The test will pass because it is expected to panic. Now you can remove the `#[should_panic]` attribute and the test body to begin the real implementation.

---

#### Step 3: Implement the Stripping Logic

Now, let's implement the `strip_timestamps` method using the `regex` crate.

**In `src/core/timestamp_parser.rs`:**

Replace the `todo!()` implementation with the following logic:

1.  **Import the `Regex` type:**
    ```rust
    use regex::Regex;
    ```

2.  **Implement the `strip_timestamps` method:**
    ```rust
    impl TimestampParserOperations for CoreTimestampParser {
        fn strip_timestamps(
            &self,
            lines: &[String],
            pattern: &str,
        ) -> Result<Vec<String>, TimestampParserError> {
            if pattern.is_empty() {
                // If the pattern is empty, no stripping is needed. Return the original lines.
                return Ok(lines.to_vec());
            }

            // Attempt to compile the regex pattern.
            let regex = Regex::new(pattern).map_err(|e| {
                TimestampParserError::invalid_pattern(pattern, e.to_string())
            })?;

            // Process each line, replacing any matches with an empty string.
            let stripped_lines = lines
                .iter()
                .map(|line| regex.replace_all(line, "").to_string())
                .collect();

            Ok(stripped_lines)
        }
    }
    ```

---

#### Step 4: Write Comprehensive Unit Tests

Now, let's write tests to cover the main success and failure cases.

**In `src/core/timestamp_parser.rs` inside the `tests` module:**

Replace your placeholder test with these:
```rust
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
        // Regex to match a timestamp in brackets, followed by a space
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
        let invalid_pattern = "["; // Unmatched opening bracket

        let result = parser.strip_timestamps(&lines, invalid_pattern);

        assert!(result.is_err());
        match result.unwrap_err() {
            TimestampParserError::InvalidPattern { pattern, .. } => {
                assert_eq!(pattern, invalid_pattern);
            }
            _ => panic!("Expected InvalidPattern error"),
        }
    }

    #[test]
    fn test_empty_pattern_returns_original_lines() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["line 1".to_string(), "line 2".to_string()];

        let result = parser.strip_timestamps(&lines, "").unwrap();

        assert_eq!(result, lines);
    }

    #[test]
    fn test_no_matches_returns_original_lines() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["line 1".to_string(), "another line".to_string()];
        let pattern = "xyz"; // A pattern that won't match

        let result = parser.strip_timestamps(&lines, pattern).unwrap();

        assert_eq!(result, lines);
    }
}
```

### Completed File (`src/core/timestamp_parser.rs`)

For your convenience, here is the complete code for the file after these changes:

```rust
use regex::Regex;
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

pub struct CoreTimestampParser;

impl CoreTimestampParser {
    pub fn new() -> Self {
        Self
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

        let regex = Regex::new(pattern)
            .map_err(|e| TimestampParserError::invalid_pattern(pattern, e.to_string()))?;

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
            _ => panic!("Expected InvalidPattern error"),
        }
    }

    #[test]
    fn test_empty_pattern_returns_original_lines() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["line 1".to_string(), "line 2".to_string()];

        let result = parser.strip_timestamps(&lines, "").unwrap();

        assert_eq!(result, lines);
    }

    #[test]
    fn test_no_matches_returns_original_lines() {
        let parser = CoreTimestampParser::new();
        let lines = vec!["line 1".to_string(), "another line".to_string()];
        let pattern = "xyz";

        let result = parser.strip_timestamps(&lines, pattern).unwrap();

        assert_eq!(result, lines);
    }
}
```

### What's Next?

Once you complete this, **Phase 2 (Core Logic) will be finished!** All the platform-agnostic business logic will be implemented and unit-tested.

The next major phase will be **Phase 3: Application Logic (The "Presenter")**, where you will start connecting your `core` services to the `CommanDuctUI` framework..
