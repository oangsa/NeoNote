use std::fs;

use serde::{Deserialize, Serialize};

use super::AppDataPaths;
use crate::i18n::AppLanguage;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub active_theme: Option<String>,
    #[serde(default)]
    pub language: AppLanguage,
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub tab_size: u8,
    pub word_wrap: bool,
    #[serde(default = "default_true")]
    pub sync_clipboard: bool,
    pub startup_mode: StartupMode,
    pub window_opacity: u8,
    pub blur_behind: bool,
    pub remember_window_geometry: bool,
    pub restore_last_session: bool,
    pub show_launcher_on_startup: bool,
    #[serde(default = "default_true")]
    pub enable_mica: bool,
    #[serde(default = "default_true")]
    pub enable_animations: bool,
    #[serde(default = "default_true")]
    pub enable_cursor_glide: bool,
    #[serde(default = "default_true")]
    pub enable_smooth_scroll: bool,
    #[serde(default = "default_true")]
    pub enable_cursor_blink: bool,
    #[serde(default)]
    pub enable_cursor_smear: bool,
    pub default_open_folder: Option<String>,
    pub keybindings: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupMode {
    Windowed,
    Maximized,
    Fullscreen,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_theme: Some("neovim-dark".to_string()),
            language: AppLanguage::English,
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.4,
            tab_size: 2,
            word_wrap: false,
            sync_clipboard: true,
            startup_mode: StartupMode::Windowed,
            window_opacity: 100,
            blur_behind: false,
            remember_window_geometry: true,
            restore_last_session: true,
            show_launcher_on_startup: true,
            enable_mica: true,
            enable_animations: true,
            enable_cursor_glide: true,
            enable_smooth_scroll: true,
            enable_cursor_blink: true,
            enable_cursor_smear: false,
            default_open_folder: None,
            keybindings: serde_json::Map::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

impl AppConfig {
    pub fn load_or_default(paths: &AppDataPaths) -> Self {
        let mut config: AppConfig = fs::read_to_string(paths.config_path())
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();
        config.normalize_disabled_features();
        config
    }

    pub fn save(&self, paths: &AppDataPaths) -> std::io::Result<()> {
        fs::write(paths.config_path(), serde_json::to_string_pretty(self)?)
    }

    pub fn normalize_disabled_features(&mut self) {
        self.enable_cursor_smear = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_plan() {
        let config = AppConfig::default();
        assert_eq!(config.active_theme.as_deref(), Some("neovim-dark"));
        assert_eq!(config.language, AppLanguage::English);
        assert_eq!(config.window_opacity, 100);
        assert!(config.show_launcher_on_startup);
        assert!(config.sync_clipboard);
        assert!(!config.enable_cursor_smear);
        assert!(config.enable_cursor_glide);
        assert!(config.enable_animations);
    }

    #[test]
    fn disabled_features_normalize_cursor_smear_off() {
        let mut config = AppConfig::default();
        config.enable_cursor_smear = true;

        config.normalize_disabled_features();

        assert!(!config.enable_cursor_smear);
    }

    #[test]
    fn deserialized_true_cursor_smear_can_be_normalized_off() {
        let mut config: AppConfig = serde_json::from_str(
            r#"{
                "active_theme": "neovim-dark",
                "language": "english",
                "font_family": "JetBrains Mono",
                "font_size": 14.0,
                "line_height": 1.4,
                "tab_size": 2,
                "word_wrap": false,
                "sync_clipboard": true,
                "startup_mode": "windowed",
                "window_opacity": 100,
                "blur_behind": false,
                "remember_window_geometry": true,
                "restore_last_session": true,
                "show_launcher_on_startup": true,
                "enable_mica": true,
                "enable_animations": true,
                "enable_cursor_glide": true,
                "enable_smooth_scroll": true,
                "enable_cursor_blink": true,
                "enable_cursor_smear": true,
                "default_open_folder": null,
                "keybindings": {}
            }"#,
        )
        .expect("config should deserialize");

        config.normalize_disabled_features();

        assert!(!config.enable_cursor_smear);
    }
}
