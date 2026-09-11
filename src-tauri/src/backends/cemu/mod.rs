//! Cemu as the Wii U's emulator.
//!
//! Every name here was read out of Cemu's own source at tag v2.6 or measured
//! on its release. What was found, and how, is in docs/what-we-verified.md.

pub mod compat;

use crate::core::console::{Console, Features};
use crate::core::library::Game;
use crate::core::types::Progress;
use futures_util::StreamExt;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

const RELEASES_API: &str = "https://api.github.com/repos/cemu-project/Cemu/releases/latest";
const USER_AGENT: &str = "Omoio";

/// Cemu prints nothing for `--version` when started without a console, which
/// is how Omoio starts it (tested), so the release tag is kept here instead.
const VERSION_FILE: &str = "omoio-version.txt";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub fn install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let local_data = app.path().local_data_dir().map_err(|e| e.to_string())?;
    Ok(local_data.join("Omoio").join("cemu"))
}

fn exe_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(install_dir(app)?.join("Cemu.exe"))
}

pub fn detect_version(app: &AppHandle) -> Option<String> {
    let dir = install_dir(app).ok()?;
    if !dir.join("Cemu.exe").is_file() {
        return None;
    }
    let tag = std::fs::read_to_string(dir.join(VERSION_FILE)).ok()?;
    let tag = tag.trim();
    Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

fn emit(app: &AppHandle, stage: &str, bytes: u64, total: u64) {
    let _ = app.emit(
        "cemu-install-progress",
        Progress {
            stage: stage.to_string(),
            bytes,
            total,
        },
    );
}

/// Downloads the newest release from Cemu's own GitHub page and unpacks it
/// into Omoio's folder. Returns the version installed.
pub async fn install(app: AppHandle, cancel: Arc<AtomicBool>) -> Result<String, String> {
    let client = reqwest::Client::new();

    emit(&app, "checking", 0, 0);
    let response = client
        .get(RELEASES_API)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't reach GitHub. Check your internet connection and try again.".to_string())?;
    if !response.status().is_success() {
        return Err("GitHub isn't answering right now. Try again in a while.".to_string());
    }
    let release: Release = response
        .json()
        .await
        .map_err(|_| "GitHub answered in a form Omoio doesn't understand.".to_string())?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name.ends_with("-windows-x64.zip"))
        .ok_or("Cemu's newest release has no Windows build.")?;

    let dir = install_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let zip_path = dir.join(&asset.name);

    // Cemu publishes no checksum beside its builds, unlike RPCS3, so there is
    // nothing to check the download against beyond HTTPS from GitHub itself.
    let download = client
        .get(&asset.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|_| "Couldn't download Cemu. Check your internet connection and try again.".to_string())?;
    let total = download.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&zip_path).await.map_err(|e| e.to_string())?;
    let mut stream = download.bytes_stream();
    let mut done: u64 = 0;
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&zip_path);
            return Err("cancelled".to_string());
        }
        let chunk = chunk.map_err(|_| "The download stopped part way. Try again.".to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        emit(&app, "downloading", done, total);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    emit(&app, "extracting", 0, 1);
    let (from, into) = (zip_path.clone(), dir.clone());
    tauri::async_runtime::spawn_blocking(move || unpack(&from, &into))
        .await
        .map_err(|e| e.to_string())??;
    let _ = std::fs::remove_file(&zip_path);

    // A folder named portable beside Cemu.exe keeps its settings, saves and
    // cache inside Omoio's folder instead of %APPDATA%\Cemu, where the user
    // may already have a Cemu of their own.
    std::fs::create_dir_all(dir.join("portable")).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(VERSION_FILE), &release.tag_name).map_err(|e| e.to_string())?;

    emit(&app, "done", 1, 1);
    detect_version(&app).ok_or_else(|| "Cemu unpacked, but Cemu.exe isn't where it should be.".to_string())
}

/// The release zip holds one folder, such as `Cemu_2.6/`, around everything.
/// It is dropped, so Cemu.exe sits in Omoio's folder whatever the version.
fn unpack(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| "The Cemu download is damaged. Try again.".to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name already refuses anything that climbs out of the folder.
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let Some(inner) = without_top_folder(&name) else {
            continue;
        };
        let target = dest.join(inner);
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

fn without_top_folder(path: &Path) -> Option<PathBuf> {
    let mut parts = path.components();
    parts.next()?;
    let rest: PathBuf = parts.collect();
    (!rest.as_os_str().is_empty()).then_some(rest)
}

/// An unpacked Wii U title: the three folders Cemu itself looks for.
fn is_game_folder(dir: &Path) -> bool {
    ["code", "content", "meta"].iter().all(|part| dir.join(part).is_dir())
}

/// Archives usually unpack into a folder of their own name, so the game may
/// sit one level below what was picked. Only when there is exactly one.
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

#[derive(Debug, Default, PartialEq)]
struct Meta {
    title_id: String,
    version: String,
    long_name: String,
    short_name: String,
}

/// One element's text out of meta.xml. The file is a flat list of elements
/// written by Nintendo's tools, the same shape Cemu's ParsedMetaXml.h reads,
/// so each value is found by its element name rather than walked as a tree.
fn element(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}");
    let mut from = 0;
    while let Some(at) = xml[from..].find(&open) {
        let after = from + at + open.len();
        // "<title_id" must not match "<title_idx".
        if xml[after..].starts_with(|c: char| c == '>' || c.is_whitespace()) {
            let start = after + xml[after..].find('>')? + 1;
            let end = start + xml[start..].find(&format!("</{name}>"))?;
            let raw = &xml[start..end];
            return Some(
                quick_xml::escape::unescape(raw)
                    .map(|text| text.into_owned())
                    .unwrap_or_else(|_| raw.to_string()),
            );
        }
        from = after;
    }
    None
}

fn parse_meta(xml: &str) -> Meta {
    let get = |name: &str| element(xml, name).unwrap_or_default();
    Meta {
        title_id: get("title_id").trim().to_string(),
        version: get("title_version").trim().to_string(),
        long_name: get("longname_en"),
        short_name: get("shortname_en"),
    }
}

/// A title's type is the low byte of the high half of its id, per Cemu's
/// TitleId.h: 00 a game, 02 a demo, 0C add-on content, 0E an update.
fn title_type(title_id: &str) -> Option<u8> {
    u8::from_str_radix(title_id.get(6..8)?, 16).ok()
}

fn identify(picked: &Path) -> Result<Game, String> {
    let root = find_root(picked).ok_or("This doesn't look like an unpacked Wii U game.")?;
    let xml = std::fs::read_to_string(root.join("meta").join("meta.xml"))
        .map_err(|_| "Couldn't read meta.xml. This folder doesn't look like a Wii U game.".to_string())?;
    let meta = parse_meta(&xml);

    let title_id = meta.title_id.to_ascii_uppercase();
    if title_id.len() != 16 || !title_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("This game's meta.xml has no title ID, so it can't be identified.".to_string());
    }
    // An update or add-on has the same three folders as a game. Letting one
    // into the library would put a second copy of the name beside the game.
    match title_type(&title_id) {
        Some(0x0E) => return Err("This is a game update, not a game. Import the game itself first.".to_string()),
        Some(0x0C) => return Err("This is add-on content, not a game. Import the game itself first.".to_string()),
        _ => {}
    }

    // Nintendo breaks long names over lines for the console's menu.
    let title = [meta.long_name, meta.short_name]
        .into_iter()
        .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
        .find(|name| !name.is_empty())
        .unwrap_or_else(|| title_id.clone());

    Ok(Game {
        console: Console::WiiU,
        title_id,
        title,
        version: (!meta.version.is_empty()).then_some(meta.version),
        update_version: None,
        size_bytes: crate::import::directory_size(&root),
        path: root,
    })
}

/// Cemu is a GUI program; without this every start of it from Omoio would
/// flash a console window over whatever the user is looking at.
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

pub struct Cemu;

impl super::EmulatorBackend for Cemu {
    fn console(&self) -> Console {
        Console::WiiU
    }

    fn features(&self) -> Features {
        // Starting games is what Omoio does for Cemu. Nothing beyond that is
        // offered until it has been checked against Cemu the way RPCS3's was.
        Features::default()
    }

    fn recognises(&self, path: &Path) -> bool {
        find_root(path).is_some()
    }

    fn identify(&self, path: &Path) -> Result<Game, String> {
        identify(path)
    }

    fn icon(&self, _game: &Game) -> Option<PathBuf> {
        // The dump's own icon is a TGA, which the interface cannot show. A
        // RAWG cover or the drawn tile stands in for it.
        None
    }

    fn prepare(&self, _app: &AppHandle) {
        // Cemu keeps its own controller settings. Omoio writes none for it.
    }

    fn tune_picture(&self, _app: &AppHandle, _display_height: u32, _graphics_memory: u64) -> Result<Option<u32>, String> {
        // An error rather than "nothing to do": the app-wide "tuned" flag is
        // only set on success, and setting it from here would stop RPCS3 from
        // ever being sized for the machine.
        Err("Omoio does not size Cemu's picture.".to_string())
    }

    fn launch(&self, app: &AppHandle, game: &Game) -> Result<u32, String> {
        let exe = exe_path(app)?;
        if !exe.is_file() {
            return Err("Install Cemu from the Emulators screen first, then you can play.".to_string());
        }
        if !game.path.is_dir() {
            return Err("This game's folder isn't there. Reconnect the drive it's on.".to_string());
        }
        // The game's own folder, so Cemu reads its meta and starts it as a
        // proper title rather than in the standalone mode it keeps for loose
        // programs.
        let child = command(&exe)
            .arg("-g")
            .arg(&game.path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(child.id())
    }

    fn detect_version(&self, app: &AppHandle) -> Option<String> {
        detect_version(app)
    }

    fn log_file(&self, app: &AppHandle) -> Option<PathBuf> {
        install_dir(app).ok().map(|dir| dir.join("portable").join("log.txt"))
    }

    fn catalogue(&self, app: &AppHandle) -> Option<Vec<crate::core::catalogue::Entry>> {
        compat::entries(app)
    }

    fn refresh_catalogue<'a>(
        &'a self,
        app: &'a AppHandle,
        cancel: &'a AtomicBool,
    ) -> futures_util::future::BoxFuture<'a, Result<usize, String>> {
        Box::pin(compat::refresh(app, cancel))
    }

    fn catalogue_source(&self) -> (&'static str, &'static str) {
        ("Wii U results from the Cemu wiki", "https://wiki.cemu.info/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Made up, in the shape Cemu's parser reads: a menu element holding each
    /// value as text, with the escaping and line breaks real names carry.
    const META: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<menu type="complex" access="777">
  <version type="unsignedInt" length="4">33</version>
  <product_code type="string" length="32">WUP-P-ABCD</product_code>
  <title_version type="unsignedInt" length="4">16</title_version>
  <title_id type="hexBinary" length="8">0005000010abcd00</title_id>
  <longname_en type="string" length="512">Example Game &amp; Friends
Deluxe</longname_en>
  <shortname_en type="string" length="256">Example Game</shortname_en>
</menu>"#;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("omoio-cemu-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn game_folder(at: &Path, meta: &str) {
        for part in ["code", "content", "meta"] {
            std::fs::create_dir_all(at.join(part)).unwrap();
        }
        std::fs::write(at.join("meta").join("meta.xml"), meta).unwrap();
    }

    #[test]
    fn reads_the_fields_omoio_shows() {
        let meta = parse_meta(META);
        assert_eq!(meta.title_id, "0005000010abcd00");
        assert_eq!(meta.version, "16");
        assert_eq!(meta.long_name, "Example Game & Friends
Deluxe");
        assert_eq!(meta.short_name, "Example Game");
    }

    #[test]
    fn an_element_is_not_matched_by_a_longer_one_starting_the_same_way() {
        let xml = "<menu><title_idx>no</title_idx><title_id>yes</title_id></menu>";
        assert_eq!(element(xml, "title_id").as_deref(), Some("yes"));
        assert_eq!(element(xml, "version"), None);
    }

    #[test]
    fn a_game_is_identified_by_its_meta() {
        let dir = scratch("identify");
        game_folder(&dir, META);
        let game = identify(&dir).unwrap();
        assert_eq!(game.console, Console::WiiU);
        assert_eq!(game.title_id, "0005000010ABCD00");
        assert_eq!(game.title, "Example Game & Friends Deluxe");
        assert_eq!(game.version.as_deref(), Some("16"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn updates_and_add_ons_are_not_games() {
        assert_eq!(title_type("0005000010ABCD00"), Some(0x00));
        assert_eq!(title_type("0005000E10ABCD00"), Some(0x0E));
        assert_eq!(title_type("0005000C10ABCD00"), Some(0x0C));

        let dir = scratch("update");
        game_folder(&dir, &META.replace("0005000010abcd00", "0005000e10abcd00"));
        assert!(identify(&dir).unwrap_err().contains("update"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_game_can_sit_one_level_down_but_not_among_several() {
        let dir = scratch("nested");
        game_folder(&dir.join("Example Game"), META);
        assert_eq!(find_root(&dir), Some(dir.join("Example Game")));

        game_folder(&dir.join("Another Game"), META);
        assert_eq!(find_root(&dir), None, "two games inside is not clear");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_without_all_three_parts_is_not_a_game() {
        let dir = scratch("partial");
        std::fs::create_dir_all(dir.join("code")).unwrap();
        std::fs::create_dir_all(dir.join("meta")).unwrap();
        assert!(find_root(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_release_folder_is_dropped_when_unpacking() {
        assert_eq!(without_top_folder(Path::new("Cemu_2.6/Cemu.exe")), Some(PathBuf::from("Cemu.exe")));
        assert_eq!(
            without_top_folder(Path::new("Cemu_2.6/resources/de/cemu.mo")),
            Some(PathBuf::from("resources/de/cemu.mo"))
        );
        assert_eq!(without_top_folder(Path::new("Cemu_2.6")), None);
    }
}
