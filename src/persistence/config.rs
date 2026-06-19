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
    pub enable_cursor_trail: bool,
    #[serde(default = "default_true")]
    pub enable_cursor_blink: bool,
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
            enable_cursor_trail: true,
            enable_cursor_blink: true,
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
        fs::read_to_string(paths.config_path())
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &AppDataPaths) -> std::io::Result<()> {
        fs::write(paths.config_path(), serde_json::to_string_pretty(self)?)
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
    }
}
