//! Turning a folder the user picked into a library entry.
//!
//! Games are referenced where they already are. An 8 GB dump is not copied,
//! so importing is immediate and a game on an external drive stays on it.

use crate::core::library::Game;
use crate::core::sfo::{Category, Sfo};
use std::path::{Path, PathBuf};

/// What went wrong, in terms the interface can say out loud without showing a
/// path or an exception.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NotAGameDump,
    UnreadableMetadata,
    IsAnUpdate,
    NoTitleId,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAGameDump => {
                write!(f, "This folder doesn't look like a PS3 game.")
            }
            Self::UnreadableMetadata => {
                write!(f, "Couldn't read PARAM.SFO. This folder doesn't look like a PS3 game dump.")
            }
            Self::IsAnUpdate => {
                write!(f, "This is a game update, not a game. Import the game itself first.")
            }
            Self::NoTitleId => {
                write!(f, "This dump is missing its title ID, so it can't be identified.")
            }
        }
    }
}

/// Where a dump keeps its PARAM.SFO. A disc dump puts it under PS3_GAME; an
/// installed PSN title keeps it at the top.
fn sfo_path(root: &Path) -> Option<PathBuf> {
    let disc = root.join("PS3_GAME").join("PARAM.SFO");
    if disc.is_file() {
        return Some(disc);
    }
    let hdd = root.join("PARAM.SFO");
    if hdd.is_file() {
        return Some(hdd);
    }
    None
}

/// The tile art shipped inside the dump itself. Using it costs nothing and
/// needs no metadata service, which is why it is the default before one exists.
pub fn icon_path(root: &Path) -> Option<PathBuf> {
    let candidates = [
        root.join("PS3_GAME").join("ICON0.PNG"),
        root.join("ICON0.PNG"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// Archives usually extract into a folder of their own name, so the dump often
/// sits one level below whatever the user picked. Looking one level down costs
/// nothing and saves them from having to find the real root themselves.
fn find_dump_root(picked: &Path) -> Option<PathBuf> {
    if sfo_path(picked).is_some() {
        return Some(picked.to_path_buf());
    }
    let mut candidate = None;
    for entry in std::fs::read_dir(picked).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() && sfo_path(&path).is_some() {
            // Only unambiguous when there is exactly one dump inside.
            if candidate.is_some() {
                return None;
            }
            candidate = Some(path);
        }
    }
    candidate
}

pub fn identify(picked: &Path) -> Result<Game, Error> {
    let root = find_dump_root(picked).ok_or(Error::NotAGameDump)?;
    let sfo_file = sfo_path(&root).ok_or(Error::NotAGameDump)?;

    let bytes = std::fs::read(&sfo_file).map_err(|_| Error::UnreadableMetadata)?;
    let sfo = Sfo::parse(&bytes).map_err(|_| Error::UnreadableMetadata)?;

    // A GD entry is an update or add-on. Letting one into the library would
    // put a second "LittleBigPlanet" beside the real one.
    if sfo.category() == Some(Category::GameData) {
        return Err(Error::IsAnUpdate);
    }

    let title_id = sfo.title_id().ok_or(Error::NoTitleId)?.to_string();
    let title = sfo
        .title()
        .map(str::to_string)
        .unwrap_or_else(|| title_id.clone());

    Ok(Game {
        title_id,
        title,
        version: sfo.app_version().map(str::to_string),
        size_bytes: directory_size(&root),
        path: root,
    })
}

/// Sums what it can and ignores what it cannot read, because a size shown in
/// the interface is never worth failing an import over.
fn directory_size(root: &Path) -> u64 {
    let mut total = 0;
    // Dumps nest past Windows' 260-character path limit, and this walk would
    // quietly skip whatever sat beyond it and report a size that was too small.
    let mut stack = vec![crate::archive::extended_length(root)];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
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

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("omoio-import-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, bytes: &[u8]) {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A PARAM.SFO with null-terminated text entries, matching real dumps.
    fn sfo(entries: &[(&str, &str)]) -> Vec<u8> {
        const HEADER: usize = 20;
        let (mut keys, mut data, mut index) = (Vec::new(), Vec::new(), Vec::new());
        for (key, value) in entries {
            let key_offset = keys.len();
            keys.extend_from_slice(key.as_bytes());
            keys.push(0);

            let data_offset = data.len();
            data.extend_from_slice(value.as_bytes());
            data.push(0);
            let len = value.len() + 1;

            index.extend_from_slice(&(key_offset as u16).to_le_bytes());
            index.extend_from_slice(&0x0204u16.to_le_bytes());
            index.extend_from_slice(&(len as u32).to_le_bytes());
            index.extend_from_slice(&(len as u32).to_le_bytes());
            index.extend_from_slice(&(data_offset as u32).to_le_bytes());
        }
        while keys.len() % 4 != 0 {
            keys.push(0);
        }

        let key_table = HEADER + index.len();
        let mut out = Vec::new();
        out.extend_from_slice(b"\0PSF");
        out.extend_from_slice(&0x0101u32.to_le_bytes());
        out.extend_from_slice(&(key_table as u32).to_le_bytes());
        out.extend_from_slice(&((key_table + keys.len()) as u32).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&keys);
        out.extend_from_slice(&data);
        out
    }

    fn disc_game() -> Vec<(&'static str, &'static str)> {
        vec![
            ("CATEGORY", "DG"),
            ("TITLE", "LittleBigPlanet"),
            ("TITLE_ID", "BCES00141"),
            ("VERSION", "01.01"),
        ]
    }

    #[test]
    fn imports_a_disc_dump() {
        let dir = TempDir::new("disc");
        dir.write("PS3_DISC.SFB", &[0; 1536]);
        dir.write("PS3_GAME/PARAM.SFO", &sfo(&disc_game()));
        dir.write("PS3_GAME/USRDIR/EBOOT.BIN", &[0; 2048]);

        let game = identify(&dir.0).unwrap();
        assert_eq!(game.title_id, "BCES00141");
        assert_eq!(game.title, "LittleBigPlanet");
        assert_eq!(game.version.as_deref(), Some("01.01"));
        assert_eq!(game.path, dir.0);
        assert_eq!(game.size_bytes, 1536 + 2048 + sfo(&disc_game()).len() as u64);
    }

    #[test]
    fn imports_an_installed_psn_title() {
        let dir = TempDir::new("hdd");
        dir.write(
            "PARAM.SFO",
            &sfo(&[
                ("CATEGORY", "HG"),
                ("TITLE", "Sackboy's Prehistoric Moves"),
                ("TITLE_ID", "NPEA00243"),
                ("APP_VER", "01.00"),
            ]),
        );
        dir.write("USRDIR/EBOOT.BIN", &[0; 16]);

        let game = identify(&dir.0).unwrap();
        assert_eq!(game.title_id, "NPEA00243");
        assert_eq!(game.version.as_deref(), Some("01.00"));
    }

    #[test]
    fn looks_one_level_down_for_the_dump() {
        // What extracting an archive normally leaves behind.
        let dir = TempDir::new("nested");
        dir.write("LittleBigPlanet (Europe)/PS3_DISC.SFB", &[0; 16]);
        dir.write("LittleBigPlanet (Europe)/PS3_GAME/PARAM.SFO", &sfo(&disc_game()));

        let game = identify(&dir.0).unwrap();
        assert_eq!(game.title_id, "BCES00141");
        assert_eq!(game.path, dir.0.join("LittleBigPlanet (Europe)"));
    }

    #[test]
    fn refuses_to_guess_between_two_nested_dumps() {
        let dir = TempDir::new("ambiguous");
        dir.write("one/PS3_GAME/PARAM.SFO", &sfo(&disc_game()));
        dir.write("two/PS3_GAME/PARAM.SFO", &sfo(&disc_game()));
        assert_eq!(identify(&dir.0).unwrap_err(), Error::NotAGameDump);
    }

    #[test]
    fn finds_the_tile_art_in_both_dump_shapes() {
        let dir = TempDir::new("icons");
        dir.write("disc/PS3_GAME/ICON0.PNG", b"png");
        dir.write("hdd/ICON0.PNG", b"png");

        assert_eq!(
            icon_path(&dir.0.join("disc")),
            Some(dir.0.join("disc").join("PS3_GAME").join("ICON0.PNG"))
        );
        assert_eq!(
            icon_path(&dir.0.join("hdd")),
            Some(dir.0.join("hdd").join("ICON0.PNG"))
        );
        assert_eq!(icon_path(&dir.0.join("nothing")), None);
    }

    #[test]
    fn tolerates_extra_files_beside_the_dump() {
        let dir = TempDir::new("extras");
        dir.write("PS3_DISC.SFB", &[0; 16]);
        dir.write("PS3_GAME/PARAM.SFO", &sfo(&disc_game()));
        dir.write("PS3_EXTRA/PARAM.SFO", &sfo(&disc_game()));
        dir.write("notes.txt", b"whatever");

        assert_eq!(identify(&dir.0).unwrap().title_id, "BCES00141");
    }

    #[test]
    fn rejects_an_update_rather_than_listing_it_as_a_game() {
        let dir = TempDir::new("update");
        dir.write(
            "PARAM.SFO",
            &sfo(&[
                ("CATEGORY", "GD"),
                ("TITLE", "LittleBigPlanet"),
                ("TITLE_ID", "BCES00141"),
                ("APP_VER", "02.00"),
            ]),
        );
        assert_eq!(identify(&dir.0).unwrap_err(), Error::IsAnUpdate);
    }

    #[test]
    fn rejects_a_dump_with_no_title_id() {
        let dir = TempDir::new("no-id");
        dir.write("PARAM.SFO", &sfo(&[("CATEGORY", "HG"), ("TITLE", "Save data")]));
        assert_eq!(identify(&dir.0).unwrap_err(), Error::NoTitleId);
    }

    #[test]
    fn rejects_a_folder_that_is_not_a_game() {
        let dir = TempDir::new("empty");
        dir.write("holiday.jpg", &[0; 8]);
        assert_eq!(identify(&dir.0).unwrap_err(), Error::NotAGameDump);
        assert_eq!(identify(Path::new("no-such-folder")).unwrap_err(), Error::NotAGameDump);
    }

    /// Checks identify() against a real dump, which is the only thing that
    /// proves it. Game files are not committed, so point this at one to run it:
    ///   set OMOIO_DUMP=D:\dumps\some-game
    ///   cargo test real_dump -- --ignored --nocapture
    #[test]
    #[ignore = "needs OMOIO_DUMP pointing at a real dump"]
    fn identifies_a_real_dump() {
        let picked = std::env::var("OMOIO_DUMP").expect("set OMOIO_DUMP");
        let game = identify(Path::new(&picked)).expect("should identify the dump");
        println!("picked: {picked}");
        println!("root:   {}", game.path.display());
        println!(
            "game:   {} {} v{} ({:.1} GB)",
            game.title_id,
            game.title,
            game.version.as_deref().unwrap_or("-"),
            game.size_bytes as f64 / 1024f64.powi(3),
        );
        assert!(!game.title_id.is_empty());
        assert!(game.size_bytes > 0);
    }

    #[test]
    fn reports_unreadable_metadata_separately_from_a_wrong_folder() {
        let dir = TempDir::new("bad-sfo");
        dir.write("PARAM.SFO", b"this is not a PARAM.SFO");
        assert_eq!(identify(&dir.0).unwrap_err(), Error::UnreadableMetadata);
    }
}
