pub mod schema;
pub mod visuals;

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::persistence::AppDataPaths;

pub use schema::{Theme, ThemeVariant};

#[derive(Default)]
pub struct ThemeStore {
    themes: Vec<Theme>,
    active_index: Option<usize>,
    preview_index: Option<usize>,
}

impl ThemeStore {
    pub fn load(paths: &AppDataPaths, active_name: Option<&str>) -> Self {
        let mut store = Self::default();
        for dir in built_in_theme_dirs(paths) {
            store.load_dir(dir);
        }
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
        self.preview_index
            .or(self.active_index)
            .and_then(|index| self.themes.get(index))
    }

    pub fn committed_theme(&self) -> Option<&Theme> {
        self.active_index.and_then(|index| self.themes.get(index))
    }

    pub fn all(&self) -> &[Theme] {
        &self.themes
    }

    pub fn filtered(&self, variant: Option<ThemeVariant>) -> Vec<(usize, &Theme)> {
        self.themes
            .iter()
            .enumerate()
            .filter(|(_, theme)| {
                variant
                    .map(|variant| theme.variant == variant)
                    .unwrap_or(true)
            })
            .collect()
    }

    pub fn active_slug(&self) -> Option<String> {
        self.committed_theme().map(Theme::slug)
    }

    pub fn is_active(&self, index: usize) -> bool {
        self.active_index == Some(index)
    }

    pub fn is_preview(&self, index: usize) -> bool {
        self.preview_index == Some(index)
    }

    pub fn preview(&mut self, index: usize) {
        if index < self.themes.len() {
            self.preview_index = Some(index);
        }
    }

    pub fn clear_preview(&mut self) {
        self.preview_index = None;
    }

    pub fn commit(&mut self, index: usize) -> Option<&Theme> {
        if index < self.themes.len() {
            self.active_index = Some(index);
            self.preview_index = None;
        }
        self.committed_theme()
    }

    pub fn import_theme(&mut self, source: &Path, paths: &AppDataPaths) -> Result<Theme, String> {
        let content = fs::read_to_string(source).map_err(|error| error.to_string())?;
        let theme: Theme = serde_json::from_str(&content).map_err(|error| error.to_string())?;
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("{}.json", theme.slug()));
        let destination = paths.user_theme_dir().join(file_name);
        fs::copy(source, &destination).map_err(|error| error.to_string())?;
        self.reload(paths, self.active_slug().as_deref());
        Ok(theme)
    }

    pub fn save_user_theme(
        &mut self,
        theme: &Theme,
        paths: &AppDataPaths,
    ) -> Result<PathBuf, String> {
        let path = paths
            .user_theme_dir()
            .join(format!("{}.json", theme.slug()));
        let content = serde_json::to_string_pretty(theme).map_err(|error| error.to_string())?;
        fs::write(&path, content).map_err(|error| error.to_string())?;
        self.reload(paths, Some(&theme.slug()));
        Ok(path)
    }

    pub fn reload(&mut self, paths: &AppDataPaths, active_name: Option<&str>) {
        *self = Self::load(paths, active_name);
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

fn built_in_theme_dirs(paths: &AppDataPaths) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("themes")
            .join("built-in"),
    );

    if let Ok(current_dir) = std::env::current_dir() {
        dirs.push(current_dir.join("assets").join("themes").join("built-in"));
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            dirs.push(exe_dir.join("assets").join("themes").join("built-in"));
        }
    }

    dirs.push(paths.built_in_theme_dir());
    dirs.dedup();
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commits_active_theme_by_index() {
        let mut store = ThemeStore {
            themes: vec![Theme::fallback_dark()],
            active_index: None,
            preview_index: None,
        };
        assert_eq!(
            store.commit(0).map(|theme| theme.name.as_str()),
            Some("NeoNote Dark")
        );
        assert_eq!(store.active_slug().as_deref(), Some("neonote-dark"));
    }
}
