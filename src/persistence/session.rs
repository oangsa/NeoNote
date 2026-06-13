use std::fs;

use serde::{Deserialize, Serialize};

use super::AppDataPaths;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionState {
    pub window: WindowState,
    pub sidebar: SidebarState,
    pub tabs: Vec<SessionTab>,
    pub active_tab_index: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SidebarState {
    pub open: bool,
    pub width: f32,
    pub active_tab: SidebarTab,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarTab {
    Explorer,
    Recent,
    Bookmarks,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionTab {
    pub file_path: String,
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
            tabs: Vec::new(),
            active_tab_index: 0,
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
