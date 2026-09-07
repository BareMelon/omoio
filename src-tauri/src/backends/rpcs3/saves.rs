//! Copies of a game's saved games, kept outside the emulator.
//!
//! RPCS3 keeps saves under `dev_hdd0/home/<user>/savedata/`, one folder per
//! game plus whatever variants a game makes of its own. That was read off a
//! real install rather than assumed.
//!
//! Backups live in Omoio's own folder, not inside RPCS3, so reinstalling the
//! emulator or clearing its storage does not take someone's saves with it.
//!
//! Only saves. Trophies sit under `trophy/NPWR00160_00`, keyed by a trophy
//! identifier with no obvious link back to a title, and guessing at that
//! mapping would either miss trophies or claim the wrong ones.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    /// Seconds since the epoch, as the session log names use.
    pub made: u64,
    pub bytes: u64,
    /// How many save folders it holds, which is not always one.
    pub folders: usize,
}

fn home(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::install_dir(app)?.join("dev_hdd0").join("home"))
}

fn backups_dir(app: &AppHandle, title_id: &str) -> Result<PathBuf, String> {
    let data = app.path().data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("Omoio").join("saves").join(title_id))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Every save folder belonging to this game, as (user, folder).
///
/// A game makes more than one: LittleBigPlanet keeps `BCES00141` beside
/// whatever else it decides to write, and RPCS3 supports several users, so
/// this looks through all of them rather than assuming the first.
fn save_folders(app: &AppHandle, title_id: &str) -> Vec<(String, PathBuf)> {
    let Ok(home) = home(app) else {
        return Vec::new();
    };
    let Ok(users) = std::fs::read_dir(&home) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    for user in users.flatten() {
        let savedata = user.path().join("savedata");
        let Ok(saves) = std::fs::read_dir(&savedata) else {
            continue;
        };
        let user_name = user.file_name().to_string_lossy().into_owned();
        for save in saves.flatten() {
            let name = save.file_name().to_string_lossy().into_owned();
            // A game's own folders all begin with its title id, which is how
            // RPCS3 groups saves and their variants.
            if name.starts_with(title_id) && save.path().is_dir() {
                found.push((user_name.clone(), save.path()));
            }
        }
    }
    found.sort();
    found
}

pub fn has_saves(app: &AppHandle, title_id: &str) -> bool {
    !save_folders(app, title_id).is_empty()
}

/// Takes a copy of this game's saves. Returns `None` when there is nothing to
/// copy, which is not a failure: a game that has never been played has no
/// saves, and saying so is better than writing an empty backup.
pub fn back_up(app: &AppHandle, title_id: &str) -> Result<Option<Backup>, String> {
    let folders = save_folders(app, title_id);
    if folders.is_empty() {
        return Ok(None);
    }

    let made = now();
    let into = backups_dir(app, title_id)?.join(made.to_string());
    std::fs::create_dir_all(&into).map_err(|e| e.to_string())?;

    let mut bytes = 0;
    for (user, folder) in &folders {
        let name = folder.file_name().unwrap_or_default();
        let target = into.join(user).join(name);
        bytes += copy_tree(folder, &target)?;
    }

    Ok(Some(Backup { made, bytes, folders: folders.len() }))
}

/// What we have kept for this game, newest first.
pub fn list(app: &AppHandle, title_id: &str) -> Vec<Backup> {
    let Ok(dir) = backups_dir(app, title_id) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut backups: Vec<Backup> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let made = e.file_name().to_string_lossy().parse().ok()?;
            let (bytes, folders) = measure(&e.path());
            Some(Backup { made, bytes, folders })
        })
        .collect();
    backups.sort_by(|a, b| b.made.cmp(&a.made));
    backups
}

/// Puts a backup back, replacing whatever is there for this game.
///
/// The current saves are copied aside first. Restoring is the one action here
/// that destroys something, and doing it without a way back would make a
/// mis-click unrecoverable.
pub fn restore(app: &AppHandle, title_id: &str, made: u64) -> Result<(), String> {
    let from = backups_dir(app, title_id)?.join(made.to_string());
    if !from.is_dir() {
        return Err("That backup isn't there any more.".into());
    }

    if has_saves(app, title_id) {
        back_up(app, title_id)?;
    }

    let home = home(app)?;
    let users = std::fs::read_dir(&from).map_err(|e| e.to_string())?;
    for user in users.flatten() {
        let savedata = home.join(user.file_name()).join("savedata");
        let Ok(saves) = std::fs::read_dir(user.path()) else {
            continue;
        };
        for save in saves.flatten() {
            let target = savedata.join(save.file_name());
            // Replaced whole, so a save that shrank does not keep stale files.
            if target.exists() {
                std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
            }
            copy_tree(&save.path(), &target)?;
        }
    }
    Ok(())
}

pub fn forget(app: &AppHandle, title_id: &str, made: u64) -> Result<(), String> {
    let dir = backups_dir(app, title_id)?.join(made.to_string());
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Copies a folder and everything under it, returning the bytes copied.
fn copy_tree(from: &Path, to: &Path) -> Result<u64, String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    let mut bytes = 0;

    for entry in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let target = to.join(entry.file_name());
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => bytes += copy_tree(&entry.path(), &target)?,
            Ok(kind) if kind.is_file() => {
                bytes += std::fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }
    Ok(bytes)
}

/// Size on disk and how many save folders a backup holds.
fn measure(dir: &Path) -> (u64, usize) {
    let mut bytes = 0;
    let mut folders = 0;

    let Ok(users) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    for user in users.flatten() {
        let Ok(saves) = std::fs::read_dir(user.path()) else {
            continue;
        };
        for save in saves.flatten() {
            folders += 1;
            bytes += tree_size(&save.path());
        }
    }
    (bytes, folders)
}

fn tree_size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => stack.push(entry.path()),
                Ok(kind) if kind.is_file() => {
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
                _ => {}
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tree(PathBuf);

    impl Tree {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("omoio-saves-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Tree(root)
        }

        fn file(&self, at: &str, bytes: &[u8]) {
            let path = self.0.join(at);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn copies_a_whole_save_folder() {
        let tree = Tree::new("copy");
        tree.file("from/PARAM.SFO", &[0; 100]);
        tree.file("from/deep/inner/DATA.BIN", &[0; 250]);

        let bytes = copy_tree(&tree.0.join("from"), &tree.0.join("to")).unwrap();

        assert_eq!(bytes, 350);
        assert!(tree.0.join("to/PARAM.SFO").is_file());
        assert!(tree.0.join("to/deep/inner/DATA.BIN").is_file());
    }

    #[test]
    fn measures_a_backup_by_its_save_folders() {
        // The shape a backup is written in: <user>/<save folder>/files.
        let tree = Tree::new("measure");
        tree.file("00000001/BCES00141/DATA.BIN", &[0; 400]);
        tree.file("00000001/BCES00141-AUTO/DATA.BIN", &[0; 600]);

        let (bytes, folders) = measure(&tree.0);
        assert_eq!(bytes, 1000);
        assert_eq!(folders, 2, "each save folder counts, not each user");
    }

    #[test]
    fn an_empty_backup_measures_as_nothing() {
        let tree = Tree::new("empty");
        assert_eq!(measure(&tree.0), (0, 0));
        assert_eq!(tree_size(&tree.0.join("no-such-folder")), 0);
    }
}
