//! What the user has told us, kept between runs. Only the games folder for
//! now: the place archives are unpacked into.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub games_folder: Option<PathBuf>,
}

impl Settings {
    /// Unreadable settings fall back to defaults rather than stopping the app;
    /// the worst case is being asked for the games folder again.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let temp = path.with_extension("json.writing");
        std::fs::write(&temp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&temp, path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("omoio-settings-{}-{name}.json", std::process::id()))
    }

    #[test]
    fn round_trips_the_games_folder() {
        let path = temp_path("roundtrip");
        let settings = Settings {
            games_folder: Some(PathBuf::from("D:\\PS3")),
        };
        settings.save(&path).unwrap();

        assert_eq!(Settings::load(&path).games_folder, Some(PathBuf::from("D:\\PS3")));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_or_unreadable_settings_fall_back_to_defaults() {
        assert!(Settings::load(Path::new("no-such-settings.json")).games_folder.is_none());

        let path = temp_path("corrupt");
        std::fs::write(&path, "not json at all").unwrap();
        assert!(Settings::load(&path).games_folder.is_none());
        let _ = std::fs::remove_file(&path);
    }
}
