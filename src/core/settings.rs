use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;

/// Snapshot of persisted fields between sessions per [CSV-Tech-SettingsPersistenceV1].
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AppSettings {
    #[serde(default)]
    left_file_path: Option<PathBuf>,
    #[serde(default)]
    right_file_path: Option<PathBuf>,
    #[serde(default)]
    timestamp_pattern: String,
    #[serde(default)]
    timestamp_history: VecDeque<String>,
}

impl AppSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_values(
        left_file_path: Option<PathBuf>,
        right_file_path: Option<PathBuf>,
        timestamp_pattern: String,
        timestamp_history: VecDeque<String>,
    ) -> Self {
        Self {
            left_file_path,
            right_file_path,
            timestamp_pattern,
            timestamp_history,
        }
    }

    pub fn left_file_path(&self) -> Option<&PathBuf> {
        self.left_file_path.as_ref()
    }

    pub fn right_file_path(&self) -> Option<&PathBuf> {
        self.right_file_path.as_ref()
    }

    pub fn timestamp_pattern(&self) -> &str {
        &self.timestamp_pattern
    }

    pub fn timestamp_history(&self) -> &VecDeque<String> {
        &self.timestamp_history
    }
}
