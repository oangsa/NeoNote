pub mod apply;
pub mod schema;
pub mod visuals;

use std::{fs, path::Path};

use crate::persistence::AppDataPaths;

pub use schema::Theme;

#[derive(Default)]
pub struct ThemeStore {
    themes: Vec<Theme>,
    active_index: Option<usize>,
}

impl ThemeStore {
    pub fn load(paths: &AppDataPaths, active_name: Option<&str>) -> Self {
        let mut store = Self::default();
        store.load_dir(Path::new("assets").join("themes").join("built-in"));
        store.load_dir(paths.user_theme_dir());

        if store.themes.is_empty() {
            store.themes.push(Theme::fallback_dark());
        }

        store.active_index = active_name
            .and_then(|name| {
                store
                    .themes
                    .iter()
                    .position(|theme| theme.slug() == name || theme.name == name)
            })
            .or(Some(0));

        store
    }

    pub fn active_theme(&self) -> Option<&Theme> {
        self.active_index.and_then(|index| self.themes.get(index))
    }

    pub fn all(&self) -> &[Theme] {
        &self.themes
    }

    fn load_dir<P: AsRef<Path>>(&mut self, dir: P) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            match fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str::<Theme>(&content).ok())
            {
                Some(theme) => self.themes.push(theme),
                None => eprintln!("Skipping malformed theme: {}", path.display()),
            }
        }
    }
}
