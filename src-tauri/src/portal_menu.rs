//! The Skylanders menu: an Omoio window over the running game, opened with
//! the Guide button and used with the pad alone. It shows the figures on the
//! portal and the user's own figure files, and puts one on or takes one off
//! through the game's emulator.
//!
//! The figure files are the user's own, copied into Omoio's figures folder
//! from Settings. Omoio never supplies figure data or makes it itself.

use crate::core::settings::Settings;
use crate::session::Session;
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
}

/// The user's figure files, the ones used lately first, the rest by name.
pub fn list(app: &AppHandle) -> Vec<Figure> {
    let Ok(dir) = folder(app) else {
        return Vec::new();
    };
    let recent = data_dir(app)
        .map(|d| Settings::load(&d.join("settings.json")).recent_figures)
        .unwrap_or_default();
    let mut found: Vec<Figure> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && is_figure(path))
                .map(|path| Figure {
                    name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                    path: path.to_string_lossy().into_owned(),
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

/// While a Skylanders game runs, Guide on any pad opens or closes the menu.
/// Read twenty times a second, since a press lasts about a tenth of one. It
/// ends with the game and takes the menu with it.
pub fn watch(app: AppHandle, pid: u32) {
    std::thread::spawn(move || {
        let mut was = false;
        loop {
            std::thread::sleep(Duration::from_millis(50));
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
                .find(|pad| crate::pads::held(&pad.device).is_some_and(|held| held.contains(&"Guide")));
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
