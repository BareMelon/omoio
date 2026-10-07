//! Packages the user already has, installed onto RPCS3's virtual hard drive.
//!
//! This is the same thing RPCS3's own "Install Packages" does, and it covers
//! homebrew, demos, and anything else that ships as a .pkg. Omoio never fetches
//! a package: the user picks a file they already have, and RPCS3 does the work.
//! A package that needs a key to read is refused by RPCS3, and we pass its
//! answer on rather than trying anything of our own.
//!
//! Installed titles land in `dev_hdd0/game/<TITLE_ID>/`, each with its own
//! PARAM.SFO, which is where the list below comes from.

use crate::core::sfo::{Category, Sfo};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// One title sitting on the virtual hard drive.
#[derive(Debug, Clone, Serialize)]
pub struct Installed {
    pub title_id: String,
    pub title: String,
    pub version: String,
    pub size_bytes: u64,
}

fn game_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(super::data_dir(app)?.join("dev_hdd0").join("game"))
}

/// RPCS3 keeps its own bookkeeping in here alongside the games. The name is
/// written with a fullwidth dollar sign so it cannot collide with a title id.
fn is_rpcs3s_own(name: &str) -> bool {
    name.starts_with('＄') || name.starts_with('$')
}

/// A title id is one folder name. Anything with a separator in it is a bug
/// somewhere above, and building a path from it would reach outside the game
/// folder, so it is refused before any path exists.
fn is_plain_folder_name(title_id: &str) -> bool {
    !title_id.is_empty()
        && !title_id.contains(['/', '\\', ':'])
        && title_id != "."
        && title_id != ".."
}

/// What is installed, in name order.
///
/// Update data is left out. An update to a disc game lands here too, under the
/// game's own id with `CATEGORY` of `GD`, and it belongs to that game in the
/// library rather than being a title of its own.
pub fn installed(app: &AppHandle) -> Result<Vec<Installed>, String> {
    let dir = game_dir(app)?;
    let Ok(entries) = std::fs::read_dir(&dir) else {
        // Nothing installed yet, which is not a failure worth reporting.
        return Ok(Vec::new());
    };

    let mut found: Vec<Installed> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let path = entry.path();
            let folder = path.file_name()?.to_str()?.to_string();
            (!is_rpcs3s_own(&folder)).then(|| read_title(&path, &folder))?
        })
        .collect();

    found.sort_by_key(|title| title.title.to_lowercase());
    Ok(found)
}

/// `None` for update data. A folder with no readable PARAM.SFO is still listed,
/// by its id: it is taking up room either way, and hiding it would leave the
/// user no way to remove it.
fn read_title(path: &Path, folder: &str) -> Option<Installed> {
    let sfo = std::fs::read(path.join("PARAM.SFO"))
        .ok()
        .and_then(|bytes| Sfo::parse(&bytes).ok());

    if sfo.as_ref().and_then(|s| s.category()) == Some(Category::GameData) {
        return None;
    }

    Some(Installed {
        title_id: sfo
            .as_ref()
            .and_then(|s| s.title_id())
            .unwrap_or(folder)
            .to_string(),
        title: sfo
            .as_ref()
            .and_then(|s| s.title())
            .unwrap_or(folder)
            .to_string(),
        version: sfo
            .as_ref()
            .and_then(|s| s.app_version())
            .unwrap_or_default()
            .to_string(),
        size_bytes: tree_size(path),
    })
}

pub fn install(app: &AppHandle, package: &Path) -> Result<(), String> {
    if !package.exists() {
        return Err("That file isn't there any more.".to_string());
    }
    super::launch::install_package(app, package)
}

/// Takes a title back off the virtual hard drive.
pub fn remove(app: &AppHandle, title_id: &str) -> Result<(), String> {
    if !is_plain_folder_name(title_id) {
        return Err("Couldn't remove that.".to_string());
    }
    let path = game_dir(app)?.join(title_id);
    if !path.is_dir() {
        return Ok(());
    }
    std::fs::remove_dir_all(&path).map_err(|_| "Couldn't remove that.".to_string())
}

fn tree_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => tree_size(&entry.path()),
            Ok(_) => entry.metadata().map(|m| m.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpcs3s_own_folders_are_not_titles() {
        // The one it actually writes, seen on a real install.
        assert!(is_rpcs3s_own("＄locks"));
        assert!(is_rpcs3s_own("$locks"));
        assert!(!is_rpcs3s_own("BCES00850"));
        assert!(!is_rpcs3s_own("NPEA00243"));
    }

    #[test]
    fn only_a_plain_folder_name_can_be_removed() {
        for id in ["BCES00850", "NPEA00243", "TEST12345"] {
            assert!(is_plain_folder_name(id), "{id} should be allowed");
        }
        for bad in ["", ".", "..", "../..", "a/b", "a\\b", "C:", "BCES00850/x"] {
            assert!(!is_plain_folder_name(bad), "{bad:?} should be refused");
        }
    }
}
