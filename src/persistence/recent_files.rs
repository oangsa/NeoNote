use std::fs;

use serde::{Deserialize, Serialize};

use super::AppDataPaths;

const MAX_RECENT_FILES: usize = 20;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecentFile {
    pub path: String,
    pub last_opened: String,
}

#[derive(Clone, Debug, Default)]
pub struct RecentFiles {
    entries: Vec<RecentFile>,
}

impl RecentFiles {
    pub fn load_or_default(paths: &AppDataPaths) -> Self {
        let mut entries = fs::read_to_string(paths.recent_files_path())
            .ok()
            .and_then(|content| serde_json::from_str::<Vec<RecentFile>>(&content).ok())
            .unwrap_or_default();

        sort_and_trim(&mut entries);
        Self { entries }
    }

    pub fn entries(&self) -> &[RecentFile] {
        &self.entries
    }

    pub fn upsert(&mut self, path: String, last_opened: String) {
        self.entries.retain(|entry| entry.path != path);
        self.entries.push(RecentFile { path, last_opened });
        sort_and_trim(&mut self.entries);
    }

    pub fn save(&self, paths: &AppDataPaths) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(&self.entries)?;
        fs::write(paths.recent_files_path(), content)
    }
}

fn sort_and_trim(entries: &mut Vec<RecentFile>) {
    entries.sort_by(|left, right| right.last_opened.cmp(&left.last_opened));
    entries.truncate(MAX_RECENT_FILES);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_files_sort_and_trim() {
        let mut recent = RecentFiles::default();
        for index in 0..25 {
            recent.upsert(
                format!("C:/file-{index}.md"),
                format!("2025-06-{:02}T10:00:00Z", index + 1),
            );
        }

        assert_eq!(recent.entries().len(), 20);
        assert_eq!(recent.entries()[0].path, "C:/file-24.md");
    }
}
