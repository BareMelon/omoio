//! The Skylanders menu: an Omoio window over the running game, opened with
//! the Guide button and used with the pad alone. It shows the figures on the
//! portal, the saved ones, and every character the game reads sorted by
//! element, and puts one on or takes one off through the game's emulator.
//!
//! Figures are the user's own files, copied into Omoio's figures folder from
//! Settings, or new ones of any character, which the emulator's own figure
//! maker makes into the same folder. Omoio never writes figure data itself.

use crate::backends::EmulatorBackend;
use crate::core::console::Console;
use crate::core::figures::{self, Character, Element, Kind};
use crate::core::settings::Settings;
use crate::session::Session;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "portal";

/// What the emulators take as a figure file: Cemu's and RPCS3's own filters.
const EXTENSIONS: [&str; 4] = ["sky", "bin", "dump", "dmp"];

/// How many recently used figures are remembered, to list them first.
const RECENT: usize = 30;

/// The kind of pad that opened the menu, so its buttons are named as printed
/// on that pad.
static FAMILY: Mutex<String> = Mutex::new(String::new());

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().data_dir().map_err(|e| e.to_string())?.join("Omoio"))
}

/// Where the user's figure files are kept.
pub fn folder(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("figures"))
}

fn is_figure(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.iter().any(|known| known.eq_ignore_ascii_case(e)))
}

#[derive(serde::Serialize)]
pub struct Figure {
    /// The file's name without its extension.
    pub name: String,
    pub path: String,
    /// The character, for a figure Omoio had the emulator make. A file the
    /// user brought has none, since Omoio doesn't read inside figure files.
    pub id: Option<u16>,
    pub variant: Option<u16>,
    pub element: Option<Element>,
    pub kind: Option<Kind>,
}

/// Which character each figure Omoio had made is, by file name.
fn made_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("made-figures.json"))
}

fn made_list(app: &AppHandle) -> BTreeMap<String, [u16; 2]> {
    made_file(app)
        .ok()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Remembers which character a new figure is, so the menu files it under
/// its element and uses it again rather than making another.
pub fn made(app: &AppHandle, figure: &Path, character: &Character) {
    let (Ok(file), Some(name)) = (made_file(app), figure.file_name()) else {
        return;
    };
    let mut list = made_list(app);
    list.insert(name.to_string_lossy().into_owned(), [character.id, character.variant]);
    if let Ok(text) = serde_json::to_string(&list) {
        let _ = std::fs::write(file, text);
    }
}

/// The user's figure files, the ones used lately first, the rest by name.
pub fn list(app: &AppHandle) -> Vec<Figure> {
    let Ok(dir) = folder(app) else {
        return Vec::new();
    };
    let recent = data_dir(app)
        .map(|d| Settings::load(&d.join("settings.json")).recent_figures)
        .unwrap_or_default();
    let made = made_list(app);
    let mut found: Vec<Figure> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && is_figure(path))
                .map(|path| {
                    let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let character = made.get(&file).copied();
                    Figure {
                        name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                        path: path.to_string_lossy().into_owned(),
                        id: character.map(|[id, _]| id),
                        variant: character.map(|[_, variant]| variant),
                        element: character.and_then(|[id, _]| figures::element(id)),
                        kind: character.map(|[id, _]| figures::kind(id)),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort_by_cached_key(|figure| {
        let file = Path::new(&figure.path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        (
            recent.iter().position(|used| *used == file).unwrap_or(usize::MAX),
            figure.name.to_lowercase(),
        )
    });
    found
}

/// Copies figure files the user picked into the figures folder. A file of the
/// same name already there is kept. Returns how many were added.
pub fn add(app: &AppHandle, paths: &[String]) -> Result<usize, String> {
    let dir = folder(app)?;
    std::fs::create_dir_all(&dir).map_err(|_| "Couldn't make the figures folder.".to_string())?;
    let mut added = 0;
    for path in paths {
        let from = Path::new(path);
        let Some(name) = from.file_name() else {
            continue;
        };
        let to = dir.join(name);
        if !is_figure(from) || to.exists() {
            continue;
        }
        std::fs::copy(from, &to).map_err(|_| "Couldn't copy that figure file.".to_string())?;
        added += 1;
    }
    Ok(added)
}

/// The characters an emulator's figure maker offered, kept with the
/// emulator's version so a newer emulator is asked again.
#[derive(serde::Serialize, serde::Deserialize)]
struct Kept {
    version: String,
    characters: Vec<Character>,
}

/// Every character the running game's emulator can make a figure of. Read
/// from the emulator the first time and kept, since the list only changes
/// with a new emulator.
pub fn characters(
    app: &AppHandle,
    backend: &dyn EmulatorBackend,
    console: Console,
    pid: u32,
) -> Result<Vec<Character>, String> {
    let version = backend.detect_version(app).unwrap_or_default();
    let key = serde_json::to_string(&console).unwrap_or_default().replace('"', "");
    let file = data_dir(app)?.join(format!("characters-{key}.json"));
    let kept = std::fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str::<Kept>(&text).ok())
        .filter(|kept| kept.version == version && !kept.characters.is_empty());
    if let Some(kept) = kept {
        return Ok(kept.characters);
    }
    let characters = backend.portal_characters(pid)?;
    if let Ok(text) = serde_json::to_string(&Kept { version, characters: characters.clone() }) {
        let _ = std::fs::write(&file, text);
    }
    Ok(characters)
}

/// Where a new figure of a character is kept: the figures folder, under the
/// character's name.
pub fn new_figure(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let dir = folder(app)?;
    std::fs::create_dir_all(&dir).map_err(|_| "Couldn't make the figures folder.".to_string())?;
    Ok(dir.join(figures::free_name(name, |file| dir.join(file).exists())))
}

/// Remembers a figure as just used, so the menu lists it first.
pub fn used(app: &AppHandle, figure: &str) {
    let (Ok(dir), Some(name)) = (data_dir(app), Path::new(figure).file_name()) else {
        return;
    };
    let name = name.to_string_lossy().into_owned();
    let file = dir.join("settings.json");
    let mut settings = Settings::load(&file);
    settings.recent_figures.retain(|known| *known != name);
    settings.recent_figures.insert(0, name);
    settings.recent_figures.truncate(RECENT);
    let _ = settings.save(&file);
}

/// Skylanders games are the ones with a portal to fill.
fn wants_portal(title: &str) -> bool {
    title.to_lowercase().contains("skylanders")
}

#[derive(Clone, serde::Serialize)]
struct MenuState {
    open: bool,
    family: String,
}

pub fn family() -> String {
    let family = FAMILY.lock().unwrap().clone();
    if family.is_empty() {
        "generic".to_string()
    } else {
        family
    }
}

fn tell(app: &AppHandle, open: bool) {
    let _ = app.emit_to(LABEL, "portal-menu", MenuState { open, family: family() });
}

fn is_open(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

/// Shows the menu over the whole screen Omoio is on. Made the first time and
/// kept, hidden, after that, so opening it again is instant.
fn open(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show().map_err(|e| e.to_string())?;
        let _ = window.set_focus();
        tell(app, true);
        return Ok(());
    }
    let main = app.get_webview_window("main").ok_or("Omoio's window isn't there.")?;
    let monitor = main
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No screen found for the menu.")?;
    let scale = monitor.scale_factor();
    let (position, size) = (monitor.position(), monitor.size());
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(PathBuf::from("portal.html")))
        .title("Portal")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(true)
        .position(position.x as f64 / scale, position.y as f64 / scale)
        .inner_size(size.width as f64 / scale, size.height as f64 / scale)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Hides the menu. The game underneath carries on.
pub fn close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
        tell(app, false);
    }
}

/// The pad button that opens the menu, as a place on the pad.
pub fn button(app: &AppHandle) -> String {
    data_dir(app)
        .map(|dir| Settings::load(&dir.join("settings.json")).portal_button)
        .unwrap_or_else(|_| "Guide".to_string())
}

/// Makes `place` the button that opens the menu.
pub fn set_button(app: &AppHandle, place: &str) -> Result<(), String> {
    if !crate::core::pad_layout::INPUTS.contains(&place) {
        return Err("That isn't a button Omoio knows.".to_string());
    }
    let file = data_dir(app)?.join("settings.json");
    let mut settings = Settings::load(&file);
    settings.portal_button = place.to_string();
    settings.save(&file)
}

/// While a Skylanders game runs, the chosen button on any pad opens or
/// closes the menu. Read twenty times a second, since a press lasts about a
/// tenth of one, and the choice is read again every two seconds so one made
/// while playing counts. It ends with the game and takes the menu with it.
pub fn watch(app: AppHandle, pid: u32) {
    std::thread::spawn(move || {
        let mut was = false;
        let mut wanted = button(&app);
        let mut ticks = 0u32;
        loop {
            std::thread::sleep(Duration::from_millis(50));
            ticks = ticks.wrapping_add(1);
            if ticks % 40 == 0 {
                wanted = button(&app);
            }
            let session = app.state::<Session>();
            if session.pid() != Some(pid) {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(window) = handle.get_webview_window(LABEL) {
                        let _ = window.close();
                    }
                });
                return;
            }
            let Some(playing) = session.playing() else {
                continue;
            };
            if !wants_portal(&playing.title) {
                return;
            }
            let pressing = crate::pads::connected()
                .into_iter()
                .find(|pad| crate::pads::held(&pad.device).is_some_and(|held| held.iter().any(|h| *h == wanted)));
            let now = pressing.is_some();
            if now && !was {
                if let Some(pad) = pressing {
                    *FAMILY.lock().unwrap() = pad.family;
                }
                let handle = app.clone();
                // Windows are made and shown on the main thread.
                let _ = app.run_on_main_thread(move || {
                    if is_open(&handle) {
                        close(&handle);
                    } else {
                        let _ = open(&handle);
                    }
                });
            }
            was = now;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figure_files_are_told_by_the_emulators_own_extensions() {
        assert!(is_figure(Path::new("Whirlwind.sky")));
        assert!(is_figure(Path::new("Spyro.BIN")));
        assert!(is_figure(Path::new("x.dump")));
        assert!(!is_figure(Path::new("notes.txt")));
        assert!(!is_figure(Path::new("no extension")));
    }

    #[test]
    fn only_skylanders_games_open_the_menu() {
        assert!(wants_portal("Skylanders SWAP Force"));
        assert!(!wants_portal("LittleBigPlanet 3"));
    }
}
