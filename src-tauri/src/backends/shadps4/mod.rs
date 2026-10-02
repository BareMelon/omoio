//! shadPS4 as the PS4's emulator.
//!
//! Every name here was read out of shadPS4's own source at tag v.0.18.0 or
//! measured on its release. What was found, and how, is in
//! docs/what-we-verified.md.

pub mod ipc;

use crate::core::console::{Console, Features};
use crate::core::library::Game;
use crate::core::sfo::Sfo;
use crate::core::types::Progress;
use futures_util::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

const RELEASES_API: &str = "https://api.github.com/repos/shadps4-emu/shadPS4/releases/latest";
const USER_AGENT: &str = "Omoio";
const EXE: &str = "shadPS4.exe";

/// shadPS4 has no `--version` (its `--help` lists none), so the release tag
/// is kept here.
const VERSION_FILE: &str = "omoio-version.txt";

/// shadPS4 keeps its settings, saves and logs in a folder named `user` in the
/// folder it is started from (`path_util.cpp`), which Omoio makes its own.
const USER_DIR: &str = "user";

/// `usb_device_backend` in shadPS4's settings: 1 is the emulated Skylanders
/// portal, 0 the real USB devices it starts with (`UsbBackendType`).
const SKYLANDERS_PORTAL: u64 = 1;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    /// "sha256:..." for the file, which GitHub works out from the upload.
    digest: Option<String>,
}

pub fn install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let local_data = app.path().local_data_dir().map_err(|e| e.to_string())?;
    Ok(local_data.join("Omoio").join("shadps4"))
}

/// The version a release tag names. shadPS4 tags its releases "v.0.18.0".
fn version_of(tag: &str) -> String {
    tag.trim_start_matches('v').trim_start_matches('.').to_string()
}

pub fn detect_version(app: &AppHandle) -> Option<String> {
    let dir = install_dir(app).ok()?;
    if !dir.join(EXE).is_file() {
        return None;
    }
    let tag = std::fs::read_to_string(dir.join(VERSION_FILE)).ok()?;
    Some(version_of(tag.trim()))
}

fn emit(app: &AppHandle, stage: &str, bytes: u64, total: u64) {
    let _ = app.emit(
        "shadps4-install-progress",
        Progress {
            stage: stage.to_string(),
            bytes,
            total,
        },
    );
}

async fn latest_release(client: &reqwest::Client) -> Result<Release, String> {
    let response = client
        .get(RELEASES_API)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't reach GitHub. Check your internet connection and try again.".to_string())?;
    if !response.status().is_success() {
        return Err("GitHub isn't answering right now. Try again in a while.".to_string());
    }
    response
        .json()
        .await
        .map_err(|_| "GitHub answered in a form Omoio doesn't understand.".to_string())
}

/// The newest release's version, as `detect_version` reports it. GitHub's
/// "latest" leaves out shadPS4's daily pre-releases.
pub async fn newest_version() -> Result<String, String> {
    let release = latest_release(&reqwest::Client::new()).await?;
    Ok(version_of(&release.tag_name))
}

/// The Windows build among a release's files. shadPS4 ships one program,
/// built with SDL, in a zip of its own.
fn is_windows_build(name: &str) -> bool {
    name.starts_with("shadps4-win64") && name.ends_with(".zip")
}

/// Downloads the newest release from shadPS4's own GitHub page, checks it
/// against the checksum GitHub keeps for the file, and unpacks it into
/// Omoio's folder. The `user` folder beside it, with the saves, is left as it
/// is. Returns the version installed.
pub async fn install(app: AppHandle, cancel: Arc<AtomicBool>) -> Result<String, String> {
    let client = reqwest::Client::new();

    emit(&app, "checking", 0, 0);
    let release = latest_release(&client).await?;
    let asset = release
        .assets
        .iter()
        .find(|asset| is_windows_build(&asset.name))
        .ok_or("shadPS4's newest release has no Windows build.")?;
    let expected = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .ok_or("GitHub gave no checksum for shadPS4's download, so Omoio can't check it.")?
        .to_ascii_lowercase();

    let dir = install_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let zip_path = dir.join(&asset.name);

    let download = client
        .get(&asset.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't download shadPS4. Check your internet connection and try again.".to_string())?;
    let total = download.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&zip_path).await.map_err(|e| e.to_string())?;
    let mut stream = download.bytes_stream();
    let mut hasher = Sha256::new();
    let mut done: u64 = 0;
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&zip_path);
            return Err("cancelled".to_string());
        }
        let chunk = chunk.map_err(|_| "The download stopped part way. Try again.".to_string())?;
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        emit(&app, "downloading", done, total);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    emit(&app, "verifying", 0, 1);
    let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got != expected {
        let _ = std::fs::remove_file(&zip_path);
        return Err("The shadPS4 download doesn't match its checksum, so it wasn't used. Try again.".to_string());
    }

    emit(&app, "extracting", 0, 1);
    let (from, into) = (zip_path.clone(), dir.clone());
    tauri::async_runtime::spawn_blocking(move || unpack(&from, &into))
        .await
        .map_err(|e| e.to_string())??;
    let _ = std::fs::remove_file(&zip_path);

    std::fs::create_dir_all(dir.join(USER_DIR)).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(VERSION_FILE), &release.tag_name).map_err(|e| e.to_string())?;

    emit(&app, "done", 1, 1);
    detect_version(&app).ok_or_else(|| "shadPS4 unpacked, but shadPS4.exe isn't where it should be.".to_string())
}

fn unpack(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|_| "The shadPS4 download is damaged. Try again.".to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name already refuses anything that climbs out of the folder.
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let target = dest.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// An unpacked PS4 game: its program and its param.sfo, where shadPS4 itself
/// looks for them (`emulator.cpp`).
fn is_game_folder(dir: &Path) -> bool {
    dir.join("eboot.bin").is_file() && dir.join("sce_sys").join("param.sfo").is_file()
}

/// The game may sit one level below what was picked, which is where an
/// archive unpacks it. Only when there is exactly one.
fn find_root(picked: &Path) -> Option<PathBuf> {
    if is_game_folder(picked) {
        return Some(picked.to_path_buf());
    }
    let mut found = None;
    for entry in std::fs::read_dir(picked).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() && is_game_folder(&path) {
            if found.is_some() {
                return None;
            }
            found = Some(path);
        }
    }
    found
}

/// A PS4 title id is four letters and five digits, such as CUSA02180.
fn is_title_id(id: &str) -> bool {
    id.len() == 9 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

fn identify(picked: &Path) -> Result<Game, String> {
    let root = find_root(picked).ok_or("This doesn't look like an unpacked PS4 game.")?;
    let bytes = std::fs::read(root.join("sce_sys").join("param.sfo"))
        .map_err(|_| "Couldn't read param.sfo. This folder doesn't look like a PS4 game.".to_string())?;
    let sfo =
        Sfo::parse(&bytes).map_err(|_| "Couldn't read param.sfo. This folder doesn't look like a PS4 game.".to_string())?;

    // An update or add-on is laid out like a game. Letting one into the
    // library would put a second copy of the name beside the game.
    match sfo.text("CATEGORY") {
        Some("gp") => return Err("This is a game update, not a game. Import the game itself first.".to_string()),
        Some("ac") => return Err("This is add-on content, not a game. Import the game itself first.".to_string()),
        _ => {}
    }

    let title_id = sfo
        .title_id()
        .filter(|id| is_title_id(id))
        .ok_or("This game's param.sfo has no title ID, so it can't be identified.")?
        .to_ascii_uppercase();
    let title = sfo
        .title()
        .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| title_id.clone());

    Ok(Game {
        console: Console::Ps4,
        title_id,
        title,
        version: sfo.app_version().map(str::to_string),
        update_version: None,
        size_bytes: crate::import::directory_size(&root),
        path: root,
    })
}

/// Plugs the emulated Skylanders portal in for a Skylanders game, in the
/// settings shadPS4 keeps for that game alone (`custom_configs/<title id>.json`,
/// laid over its own settings). Anything else in that file is left as it is.
fn plug_in_portal(user: &Path, title_id: &str) -> Result<(), String> {
    let dir = user.join("custom_configs");
    let path = dir.join(format!("{title_id}.json"));
    let mut settings: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    if settings["Input"]["usb_device_backend"] == SKYLANDERS_PORTAL {
        return Ok(());
    }
    if !settings["Input"].is_object() {
        settings["Input"] = serde_json::json!({});
    }
    settings["Input"]["usb_device_backend"] = SKYLANDERS_PORTAL.into();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

/// shadPS4 is a console program; without this every start of it from Omoio
/// would put a console window over whatever the user is looking at.
fn command(exe: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// What the PS4's pad calls each place. shadPS4's own layout puts each on the
/// same place of the pad (`input_handler.cpp`, `GetDefaultInputConfig`); it
/// has nothing on the Guide button.
const PS4_BUTTONS: [(&str, &str); 24] = [
    ("South", "Cross"),
    ("East", "Circle"),
    ("West", "Square"),
    ("North", "Triangle"),
    ("LB", "L1"),
    ("RB", "R1"),
    ("LT", "L2"),
    ("RT", "R2"),
    ("LS", "L3"),
    ("RS", "R3"),
    ("Up", "D-pad up"),
    ("Down", "D-pad down"),
    ("Left", "D-pad left"),
    ("Right", "D-pad right"),
    ("Back", "Touchpad"),
    ("Start", "Options"),
    ("LS Y+", "Left stick up"),
    ("LS Y-", "Left stick down"),
    ("LS X-", "Left stick left"),
    ("LS X+", "Left stick right"),
    ("RS Y+", "Right stick up"),
    ("RS Y-", "Right stick down"),
    ("RS X-", "Right stick left"),
    ("RS X+", "Right stick right"),
];

pub struct ShadPs4;

impl super::EmulatorBackend for ShadPs4 {
    fn console(&self) -> Console {
        Console::Ps4
    }

    fn name(&self) -> &'static str {
        "shadPS4"
    }

    fn newest_version(&self) -> futures_util::future::BoxFuture<'static, Result<String, String>> {
        Box::pin(newest_version())
    }

    fn features(&self) -> Features {
        // Starting games and the Skylanders portal, both through shadPS4's
        // launcher line. The same line pauses the game under Omoio's menus,
        // so it hears nothing from the pad there.
        Features {
            portal: true,
            quiet_behind: true,
            ..Features::default()
        }
    }

    fn recognises(&self, path: &Path) -> bool {
        find_root(path).is_some()
    }

    fn identify(&self, path: &Path) -> Result<Game, String> {
        identify(path)
    }

    fn icon(&self, game: &Game) -> Option<PathBuf> {
        Some(game.path.join("sce_sys").join("icon0.png")).filter(|icon| icon.is_file())
    }

    fn prepare(&self, app: &AppHandle, game: &Game) {
        let Ok(dir) = install_dir(app) else {
            return;
        };
        if crate::core::figures::is_skylanders(&game.title) {
            let _ = plug_in_portal(&dir.join(USER_DIR), &game.title_id);
        }
    }

    fn hush(&self, pid: u32, hushed: bool) -> Result<(), String> {
        ipc::pause(pid, hushed)
    }

    fn portal_figures(&self, pid: u32) -> Result<Vec<String>, String> {
        ipc::figures(pid)
    }

    fn portal_load(&self, pid: u32, slot: usize, figure: &Path) -> Result<Vec<String>, String> {
        ipc::load(pid, slot, figure)
    }

    fn portal_clear(&self, pid: u32, slot: usize) -> Result<Vec<String>, String> {
        ipc::clear(pid, slot)
    }

    fn portal_characters(&self, _pid: u32) -> Result<Vec<crate::core::figures::Character>, String> {
        // shadPS4 has no figure maker of its own, so there are no characters
        // to make. The figures already saved, such as those Cemu made, go on
        // its portal from the Saved tab.
        Ok(Vec::new())
    }

    fn button_names(&self) -> &'static [(&'static str, &'static str)] {
        &PS4_BUTTONS
    }

    fn write_layout(
        &self,
        _app: &AppHandle,
        _title_id: &str,
        _players: &[crate::core::pad_layout::Player],
    ) -> Result<(), String> {
        // shadPS4 reads the first pad with its own layout, which already puts
        // every button where Omoio's standard layout does. A layout changed
        // on the Controller screen doesn't reach it yet.
        Ok(())
    }

    fn tune_picture(&self, _app: &AppHandle, _display_height: u32, _graphics_memory: u64) -> Result<Option<u32>, String> {
        // An error rather than "nothing to do": the app-wide "tuned" flag is
        // only set on success, and setting it from here would stop RPCS3 from
        // ever being sized for the machine.
        Err("Omoio does not size shadPS4's picture.".to_string())
    }

    fn launch(&self, app: &AppHandle, game: &Game) -> Result<u32, String> {
        let dir = install_dir(app)?;
        let exe = dir.join(EXE);
        if !exe.is_file() {
            return Err("Install shadPS4 from the Emulators screen first, then you can play.".to_string());
        }
        if !game.path.exists() {
            return Err("This game isn't where it was. Reconnect the drive it's on.".to_string());
        }
        // Started from its own folder, so its `user` folder is the one beside
        // it. Not fullscreen: Omoio places the picture itself, in its window
        // or across the screen, the same as for the other emulators.
        let mut cmd = command(&exe);
        cmd.current_dir(&dir).arg("-g").arg(&game.path).arg("-f").arg("false");
        ipc::start(cmd)
    }

    fn detect_version(&self, app: &AppHandle) -> Option<String> {
        detect_version(app)
    }

    fn log_file(&self, app: &AppHandle) -> Option<PathBuf> {
        install_dir(app).ok().map(|dir| dir.join(USER_DIR).join("log").join("shad_log.txt"))
    }

    fn catalogue(&self, _app: &AppHandle) -> Option<Vec<crate::core::catalogue::Entry>> {
        // No list of how PS4 games run is read yet.
        Some(Vec::new())
    }

    fn refresh_catalogue<'a>(
        &'a self,
        _app: &'a AppHandle,
        _cancel: &'a AtomicBool,
    ) -> futures_util::future::BoxFuture<'a, Result<usize, String>> {
        Box::pin(async { Ok(0) })
    }

    fn catalogue_source(&self) -> (&'static str, &'static str) {
        ("", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sfo::tests::{build, text};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("omoio-shadps4-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn game_folder(at: &Path, category: &str) {
        std::fs::create_dir_all(at.join("sce_sys")).unwrap();
        std::fs::write(at.join("eboot.bin"), b"x").unwrap();
        let sfo = build(&[
            text("APP_VER", "01.05"),
            text("CATEGORY", category),
            text("TITLE", "Skylanders SuperChargers"),
            text("TITLE_ID", "CUSA02180"),
        ]);
        std::fs::write(at.join("sce_sys").join("param.sfo"), sfo).unwrap();
    }

    #[test]
    fn a_game_is_identified_by_its_param_sfo() {
        let dir = scratch("identify");
        game_folder(&dir.join("CUSA02180"), "gd");
        let game = identify(&dir).unwrap();
        assert_eq!(game.console, Console::Ps4);
        assert_eq!(game.title_id, "CUSA02180");
        assert_eq!(game.title, "Skylanders SuperChargers");
        assert_eq!(game.version.as_deref(), Some("01.05"));
        assert!(game.path.ends_with("CUSA02180"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn updates_and_add_ons_are_not_games() {
        let dir = scratch("update");
        game_folder(&dir, "gp");
        assert!(identify(&dir).unwrap_err().contains("update"));
        game_folder(&dir, "ac");
        assert!(identify(&dir).unwrap_err().contains("add-on"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_without_its_program_is_not_a_game() {
        let dir = scratch("partial");
        game_folder(&dir, "gd");
        std::fs::remove_file(dir.join("eboot.bin")).unwrap();
        assert!(find_root(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_portal_is_plugged_in_without_touching_other_settings() {
        let dir = scratch("portal");
        plug_in_portal(&dir, "CUSA02180").unwrap();
        let path = dir.join("custom_configs").join("CUSA02180.json");
        let settings: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(settings["Input"]["usb_device_backend"], 1);

        std::fs::write(&path, r#"{"GPU": {"fsr": true}, "Input": {"use_special_pad": false}}"#).unwrap();
        plug_in_portal(&dir, "CUSA02180").unwrap();
        let settings: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(settings["Input"]["usb_device_backend"], 1);
        assert_eq!(settings["Input"]["use_special_pad"], false);
        assert_eq!(settings["GPU"]["fsr"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn versions_and_builds_are_read_from_the_release() {
        assert_eq!(version_of("v.0.18.0"), "0.18.0");
        assert_eq!(version_of("v0.19.1"), "0.19.1");
        assert!(is_windows_build("shadps4-win64-sdl-0.18.0.zip"));
        assert!(!is_windows_build("shadps4-linux-sdl-0.18.0.zip"));
        assert!(is_title_id("CUSA02180"));
        assert!(!is_title_id("CUSA0218"));
        assert!(!is_title_id("../../x"));
    }
}
