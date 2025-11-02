use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

/// Resolves the user-local settings directory, ensuring it exists per [CSV-Tech-SettingsPersistenceV1].
pub fn get_base_app_config_local_dir(app_name: &str) -> Option<PathBuf> {
    log::trace!("[CSV-Tech-SettingsPersistenceV1] Resolving config directory for '{app_name}'");
    ProjectDirs::from("", "", app_name).and_then(|proj_dirs| {
        let config_path = proj_dirs.config_local_dir();
        if !config_path.exists() {
            if let Err(err) = fs::create_dir_all(config_path) {
                log::error!(
                    "[CSV-Tech-SettingsPersistenceV1] Failed to create config dir {config_path:?}: {err}"
                );
                return None;
            }
            log::debug!(
                "[CSV-Tech-SettingsPersistenceV1] Created config dir: {config_path:?}"
            );
        } else {
            log::trace!(
                "[CSV-Tech-SettingsPersistenceV1] Config dir already exists: {config_path:?}"
            );
        }
        Some(config_path.to_path_buf())
    })
}
