use std::fs;

use serde::{Deserialize, Serialize};

use super::AppDataPaths;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionState {
    #[serde(default)]
    pub window: WindowState,
    #[serde(default)]
    pub sidebar: SidebarState,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub active_document: Option<std::path::PathBuf>,
    #[serde(default)]
    pub opened_files: Vec<SessionFile>,
}

fn default_version() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionFile {
    pub path: Option<std::path::PathBuf>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub viewport_top_line: usize,
    #[serde(default)]
    pub is_dirty: bool,
    #[serde(default)]
    pub unsaved_content: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct SidebarState {
    pub open: bool,
    pub width: f32,
    pub active_tab: SidebarTab,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SidebarTab {
    #[default]
    Explorer,
    Recent,
    Bookmarks,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            window: WindowState {
                x: 100,
                y: 100,
                width: 1280,
                height: 800,
            },
            sidebar: SidebarState {
                open: true,
                width: 220.0,
                active_tab: SidebarTab::Explorer,
            },
            version: 1,
            active_document: None,
            opened_files: Vec::new(),
        }
    }
}

impl SessionState {
    pub fn load_or_default(paths: &AppDataPaths) -> Self {
        fs::read_to_string(paths.session_path())
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &AppDataPaths) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(paths.session_path(), json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_round_trips() {
        let session = SessionState::default();
        let json = serde_json::to_string(&session).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.window.width, 1280);
        assert!(matches!(parsed.sidebar.active_tab, SidebarTab::Explorer));
    }
}
