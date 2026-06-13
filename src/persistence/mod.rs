pub mod config;
pub mod recent_files;
pub mod session;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub use config::AppConfig;
pub use recent_files::RecentFiles;
pub use session::SessionState;

#[derive(Clone, Debug)]
pub struct AppDataPaths {
    root: PathBuf,
}

impl AppDataPaths {
    pub fn new() -> Self {
        let root = env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join("NeoNote");

        let paths = Self { root };
        paths.ensure_dirs();
        paths
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn session_path(&self) -> PathBuf {
        self.root.join("session.json")
    }

    pub fn recent_files_path(&self) -> PathBuf {
        self.root.join("recent_files.json")
    }

    pub fn built_in_theme_dir(&self) -> PathBuf {
        self.root.join("themes").join("built-in")
    }

    pub fn user_theme_dir(&self) -> PathBuf {
        self.root.join("themes").join("user")
    }

    fn ensure_dirs(&self) {
        let _ = fs::create_dir_all(self.built_in_theme_dir());
        let _ = fs::create_dir_all(self.user_theme_dir());
    }
}

impl Default for AppDataPaths {
    fn default() -> Self {
        Self::new()
    }
}
