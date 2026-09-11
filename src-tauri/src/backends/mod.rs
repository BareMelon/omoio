//! Everything specific to one emulator lives behind `EmulatorBackend`.
//!
//! Each game records its console, and the console picks the backend. Import,
//! launch and cover art go through here, so adding an emulator means adding an
//! implementation and a line in `all`, not touching the library or a screen.

pub mod rpcs3;

use crate::core::console::{Console, Features};
use crate::core::library::Game;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

pub trait EmulatorBackend: Sync {
    fn console(&self) -> Console;

    fn features(&self) -> Features;

    /// Whether this looks like one of this console's dumps. It must not read
    /// or measure the dump: every backend is asked about every import.
    fn recognises(&self, path: &Path) -> bool;

    fn identify(&self, path: &Path) -> Result<Game, String>;

    /// The picture the dump itself ships, if it has one.
    fn icon(&self, game: &Game) -> Option<PathBuf>;

    /// Gets things ready before a game starts, the controller first of all.
    fn prepare(&self, app: &AppHandle);

    /// Sizes the picture for this machine if it has not been. Returns the
    /// scale that applies, or `None` when the user already chose their own.
    fn tune_picture(
        &self,
        app: &AppHandle,
        display_height: u32,
        graphics_memory: u64,
    ) -> Result<Option<u32>, String>;

    /// Starts the game and returns the emulator's process id.
    fn launch(&self, app: &AppHandle, game: &Game) -> Result<u32, String>;
}

/// Every emulator Omoio can run, one per console.
pub fn all() -> &'static [&'static dyn EmulatorBackend] {
    &[&rpcs3::Rpcs3]
}

pub fn for_console(console: Console) -> Option<&'static dyn EmulatorBackend> {
    all().iter().copied().find(|backend| backend.console() == console)
}

/// Works out which console a dump is for and reads it with that emulator.
pub fn identify(path: &Path) -> Result<Game, String> {
    match all().iter().find(|backend| backend.recognises(path)) {
        Some(backend) => backend.identify(path),
        None => Err(unknown_dump(all().iter().map(|backend| backend.console().short()))),
    }
}

/// Says which consoles Omoio takes, so an unknown folder gets an answer that
/// helps rather than one that only says no.
fn unknown_dump<'a>(consoles: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = consoles.collect();
    let list = match names.as_slice() {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    format!("This doesn't look like a game Omoio can play. It takes {list} games.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_dump_is_told_which_consoles_are_taken() {
        assert_eq!(
            unknown_dump(["PS3"].into_iter()),
            "This doesn't look like a game Omoio can play. It takes PS3 games."
        );
        assert_eq!(
            unknown_dump(["PS3", "Wii U"].into_iter()),
            "This doesn't look like a game Omoio can play. It takes PS3 and Wii U games."
        );
        assert_eq!(
            unknown_dump(["PS3", "Wii U", "PS2"].into_iter()),
            "This doesn't look like a game Omoio can play. It takes PS3, Wii U and PS2 games."
        );
    }

    #[test]
    fn every_console_has_at_most_one_emulator() {
        let mut seen = std::collections::HashSet::new();
        for backend in all() {
            assert!(seen.insert(backend.console()), "{:?} twice", backend.console());
        }
        assert!(for_console(Console::Ps3).is_some());
    }
}
