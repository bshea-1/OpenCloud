use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub launch_at_startup: bool,
    pub keep_running_in_background: bool,
    pub open_in_background: bool,
    pub auto_update_opencloud: bool,
    pub enable_desktop_notifications: bool,
    pub notify_when_download_completes: bool,
    pub notify_sync_issues: bool,
    pub studio_theme: String,
    pub app_language: String,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            keep_running_in_background: true,
            open_in_background: false,
            auto_update_opencloud: true,
            enable_desktop_notifications: true,
            notify_when_download_completes: true,
            notify_sync_issues: true,
            studio_theme: "obsidian".into(),
            app_language: "English (North America)".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstallSettings {
    pub install_directory: String,
    pub auto_update_apps: bool,
    pub auto_update_per_app: HashMap<String, bool>,
    pub import_previous_settings: bool,
    pub remove_previous_versions: bool,
    pub opt_in_prereleases: bool,
    pub check_interval_hours: u32,
    pub max_parallel_downloads: u32,
}

impl Default for AppInstallSettings {
    fn default() -> Self {
        let default_dir = if cfg!(target_os = "macos") {
            "/Applications".to_string()
        } else if cfg!(target_os = "windows") {
            dirs::data_local_dir()
                .map(|p| p.join("Programs").join("OpenCloud").to_string_lossy().to_string())
                .unwrap_or_else(|| "C:\\Program Files\\OpenCloud".into())
        } else {
            dirs::home_dir()
                .map(|p| p.join(".local/bin").to_string_lossy().to_string())
                .unwrap_or_else(|| "/usr/local/bin".into())
        };

        let mut auto_update_per_app = HashMap::new();
        for id in &[
            "photocraft",
            "vectorcraft",
            "filmcraft",
            "lightcraft",
            "pdfcraft",
            "effectcraft",
            "designcraft",
            "soundcraft",
            "cadcraft",
            "deckcraft",
            "gridcraft",
            "wordcraft",
        ] {
            auto_update_per_app.insert(id.to_string(), true);
        }

        Self {
            install_directory: default_dir,
            auto_update_apps: true,
            auto_update_per_app,
            import_previous_settings: true,
            remove_previous_versions: true,
            opt_in_prereleases: false,
            check_interval_hours: 6,
            max_parallel_downloads: 2,
        }
    }
}

impl AppInstallSettings {
    pub fn is_app_auto_update_enabled(&self, app_id: &str) -> bool {
        if !self.auto_update_apps {
            return false;
        }
        *self.auto_update_per_app.get(app_id).unwrap_or(&true)
    }

    pub fn set_app_auto_update(&mut self, app_id: &str, enabled: bool) {
        self.auto_update_per_app.insert(app_id.to_string(), enabled);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub cache_directory: String,
    pub clean_installers_after_install: bool,
    pub max_cache_size_gb: u32,
}

impl Default for StorageSettings {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .map(|p| p.join("opencloud").to_string_lossy().to_string())
            .unwrap_or_else(|| "/tmp/opencloud".into());

        Self {
            cache_directory: cache_dir,
            clean_installers_after_install: true,
            max_cache_size_gb: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub sync_enabled: bool,
    pub shared_assets_path: String,
    pub shared_palettes_path: String,
    pub transfer_rate_limit_mb: u32, // 0 = unlimited
}

impl Default for SyncSettings {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .map(|p| p.join("opencloud"))
            .unwrap_or_else(|| PathBuf::from("~/.config/opencloud"));

        Self {
            sync_enabled: true,
            shared_assets_path: config_dir.join("assets").to_string_lossy().to_string(),
            shared_palettes_path: config_dir.join("palettes").to_string_lossy().to_string(),
            transfer_rate_limit_mb: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub github_token: String,
    pub api_timeout_secs: u64,
    pub proxy_mode: String, // "direct", "system", "custom"
    pub proxy_url: String,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            github_token: String::new(),
            api_timeout_secs: 10,
            proxy_mode: "direct".into(),
            proxy_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub apps: AppInstallSettings,
    pub storage: StorageSettings,
    pub sync: SyncSettings,
    pub network: NetworkSettings,
}

impl AppSettings {
    pub fn config_file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("opencloud")
            .join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_file_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
                }
            }
        }
        let default_settings = Self::default();
        let _ = default_settings.save();
        default_settings
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default_and_serialization() {
        let mut settings = AppSettings::default();
        assert!(settings.general.auto_update_opencloud);
        assert!(settings.apps.auto_update_apps);
        assert!(settings.apps.import_previous_settings);
        assert!(settings.apps.remove_previous_versions);
        assert!(settings.apps.is_app_auto_update_enabled("photocraft"));

        settings.apps.set_app_auto_update("photocraft", false);
        assert!(!settings.apps.is_app_auto_update_enabled("photocraft"));

        let json = serde_json::to_string(&settings).expect("Must serialize");
        let deserialized: AppSettings = serde_json::from_str(&json).expect("Must deserialize");

        assert_eq!(settings.general.studio_theme, deserialized.general.studio_theme);
        assert_eq!(settings.general.app_language, deserialized.general.app_language);
        assert_eq!(settings.apps.auto_update_apps, deserialized.apps.auto_update_apps);
        assert_eq!(settings.apps.import_previous_settings, deserialized.apps.import_previous_settings);
        assert_eq!(settings.apps.remove_previous_versions, deserialized.apps.remove_previous_versions);
        assert_eq!(settings.storage.clean_installers_after_install, deserialized.storage.clean_installers_after_install);
    }
}
