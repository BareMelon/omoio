//! Our own record of what the user has imported. Games are referenced where
//! they already live rather than copied, so a dump on an external drive is
//! normal and an entry whose path is missing is expected, not an error.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub title_id: String,
    pub title: String,
    /// The version of the dump itself, out of its PARAM.SFO.
    pub version: Option<String>,
    /// The official update Omoio installed, if any.
    ///
    /// An update lives in the emulator's own storage rather than in the dump,
    /// so the dump keeps saying what it always said. RPCS3's game-data folder
    /// does hold a version, but it is written for reasons of its own and did
    /// not match either the disc or anything we installed, so it is not
    /// something to read a version out of. What we put there ourselves is.
    #[serde(default)]
    pub update_version: Option<String>,
    pub path: PathBuf,
    pub size_bytes: u64,
}

impl Game {
    /// The version that actually runs, which is the update when there is one.
    pub fn running_version(&self) -> Option<&str> {
        self.update_version
            .as_deref()
            .or(self.version.as_deref())
    }

    /// Whether this entry has files behind it.
    ///
    /// A game added from the catalogue has none yet: it is a note that the user
    /// owns it, waiting for them to import it. That is a different thing from a
    /// game whose drive is unplugged, and the interface has to say so
    /// differently or the two look like the same fault.
    pub fn is_set_up(&self) -> bool {
        !self.path.as_os_str().is_empty()
    }

    /// An entry for a game the user says they own but has not imported.
    pub fn not_set_up(title_id: String, title: String) -> Self {
        Self {
            title_id,
            title,
            version: None,
            update_version: None,
            path: PathBuf::new(),
            size_bytes: 0,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    games: Vec<Game>,
}

impl Library {
    pub fn games(&self) -> &[Game] {
        &self.games
    }

    /// Re-importing a game the user already has replaces the entry instead of
    /// adding a second copy of it.
    pub fn upsert(&mut self, game: Game) {
        match self.games.iter_mut().find(|g| g.title_id == game.title_id) {
            Some(existing) => *existing = game,
            None => self.games.push(game),
        }
    }

    pub fn get_mut(&mut self, title_id: &str) -> Option<&mut Game> {
        self.games.iter_mut().find(|g| g.title_id == title_id)
    }

    pub fn remove(&mut self, title_id: &str) -> bool {
        let before = self.games.len();
        self.games.retain(|g| g.title_id != title_id);
        self.games.len() != before
    }

    /// A library we cannot read is kept aside rather than silently overwritten,
    /// so a bad file costs the user their list but never the underlying data.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str(&text) {
            Ok(library) => library,
            Err(_) => {
                let _ = std::fs::rename(path, path.with_extension("json.unreadable"));
                Self::default()
            }
        }
    }

    /// Written to a temporary file and renamed over the original, so losing
    /// power mid-write leaves the previous library intact rather than a
    /// half-written one.
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

    fn game(title_id: &str, title: &str) -> Game {
        Game {
            title_id: title_id.to_string(),
            title: title.to_string(),
            version: Some("01.00".to_string()),
            update_version: None,
            path: PathBuf::from(format!("D:\\games\\{title_id}")),
            size_bytes: 8_200_000_000,
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("omoio-lib-{}-{name}.json", std::process::id()))
    }

    #[test]
    fn reimporting_replaces_rather_than_duplicates() {
        let mut library = Library::default();
        library.upsert(game("BCES00141", "LittleBigPlanet"));
        library.upsert(game("BCES00850", "LittleBigPlanet 2"));

        let mut updated = game("BCES00141", "LittleBigPlanet");
        updated.version = Some("02.00".to_string());
        library.upsert(updated);

        assert_eq!(library.games().len(), 2);
        let entry = library.games().iter().find(|g| g.title_id == "BCES00141").unwrap();
        assert_eq!(entry.version.as_deref(), Some("02.00"));
    }

    #[test]
    fn an_installed_update_is_the_version_that_runs() {
        let mut entry = game("BCES00850", "LittleBigPlanet 2");
        assert_eq!(entry.running_version(), Some("01.00"));

        // The dump still says what it always said; the update is what runs.
        entry.update_version = Some("01.33".to_string());
        assert_eq!(entry.running_version(), Some("01.33"));
        assert_eq!(entry.version.as_deref(), Some("01.00"));
    }

    #[test]
    fn removes_by_title_id() {
        let mut library = Library::default();
        library.upsert(game("BCES00141", "LittleBigPlanet"));
        assert!(library.remove("BCES00141"));
        assert!(!library.remove("BCES00141"));
        assert!(library.games().is_empty());
    }

    #[test]
    fn survives_a_round_trip_to_disk() {
        let path = temp_path("roundtrip");
        let mut library = Library::default();
        library.upsert(game("BCES00141", "LittleBigPlanet"));
        library.save(&path).unwrap();

        let loaded = Library::load(&path);
        assert_eq!(loaded.games(), library.games());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_library_is_an_empty_one() {
        assert!(Library::load(Path::new("no-such-library.json")).games().is_empty());
    }

    #[test]
    fn an_unreadable_library_is_set_aside_not_overwritten() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "{ this is not json").unwrap();

        assert!(Library::load(&path).games().is_empty());
        let kept = path.with_extension("json.unreadable");
        assert!(kept.exists(), "the unreadable file should be kept for recovery");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&kept);
    }
}
