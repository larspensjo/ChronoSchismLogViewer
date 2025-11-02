use crate::core::path_utils;
use crate::core::settings::AppSettings;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

const SETTINGS_FILENAME: &str = "settings.json";

/// Provides persistence for `AppSettings` per [CSV-Tech-SettingsPersistenceV1].
pub trait SettingsManagerOperations: Send + Sync {
    fn save_settings(&self, app_name: &str, settings: &AppSettings) -> Result<(), std::io::Error>;
    fn load_settings(&self, app_name: &str) -> Result<AppSettings, std::io::Error>;
}

/// Filesystem-backed implementation for the settings service.
pub struct CoreSettingsManager;

impl CoreSettingsManager {
    pub fn new() -> Self {
        Self {}
    }

    fn settings_file_path(app_name: &str) -> Option<PathBuf> {
        path_utils::get_base_app_config_local_dir(app_name)
            .map(|base_dir| base_dir.join(SETTINGS_FILENAME))
    }
}

impl SettingsManagerOperations for CoreSettingsManager {
    fn save_settings(&self, app_name: &str, settings: &AppSettings) -> Result<(), std::io::Error> {
        if let Some(path) = Self::settings_file_path(app_name) {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, settings)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        }
        Ok(())
    }

    fn load_settings(&self, app_name: &str) -> Result<AppSettings, std::io::Error> {
        let Some(path) = Self::settings_file_path(app_name) else {
            return Ok(AppSettings::default());
        };

        if !path.exists() {
            log::info!(
                "[CSV-Tech-SettingsPersistenceV1] Settings file not found at {path:?}, using defaults"
            );
            return Ok(AppSettings::default());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}
