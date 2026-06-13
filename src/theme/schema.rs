use serde::{Deserialize, Serialize};

use super::visuals::parse_hex_color;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    Dark,
    Light,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Theme {
    pub name: String,
    pub variant: ThemeVariant,
    pub author: String,
    pub colors: ThemeColors,
    pub vim_modes: VimModeColors,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ThemeColors {
    pub background: String,
    pub background_alt: String,
    pub surface: String,
    pub border: String,
    pub text: String,
    pub text_muted: String,
    pub accent_primary: String,
    pub accent_secondary: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub cursor: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VimModeColors {
    pub normal: String,
    pub insert: String,
    pub visual: String,
    pub command: String,
    pub replace: String,
}

impl Theme {
    pub fn slug(&self) -> String {
        self.name
            .to_lowercase()
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn fallback_dark() -> Self {
        Self {
            name: "NeoNote Dark".to_string(),
            variant: ThemeVariant::Dark,
            author: "NeoNote".to_string(),
            colors: ThemeColors {
                background: "#111318".to_string(),
                background_alt: "#181b22".to_string(),
                surface: "#222631".to_string(),
                border: "#343a46".to_string(),
                text: "#e6e8ef".to_string(),
                text_muted: "#9aa3b2".to_string(),
                accent_primary: "#6ea8fe".to_string(),
                accent_secondary: "#8fd7c7".to_string(),
                success: "#8bd17c".to_string(),
                warning: "#f5c56b".to_string(),
                error: "#ff7b86".to_string(),
                cursor: "#f2d16b".to_string(),
            },
            vim_modes: VimModeColors {
                normal: "#6ea8fe".to_string(),
                insert: "#8bd17c".to_string(),
                visual: "#b38cff".to_string(),
                command: "#f5c56b".to_string(),
                replace: "#ff7b86".to_string(),
            },
        }
    }

    pub fn mode_color(&self, mode: &str) -> Option<egui::Color32> {
        let value = match mode {
            "NORMAL" => &self.vim_modes.normal,
            "INSERT" => &self.vim_modes.insert,
            "VISUAL" => &self.vim_modes.visual,
            "COMMAND" => &self.vim_modes.command,
            "REPLACE" => &self.vim_modes.replace,
            _ => &self.colors.accent_primary,
        };
        parse_hex_color(value)
    }
}
