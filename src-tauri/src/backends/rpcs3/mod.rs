pub mod account;
pub mod compat;
pub mod controllers;
pub mod firmware;
pub mod fixes;
pub mod game_config;
pub mod graphics;
pub mod launch;
#[cfg(windows)]
pub mod overlay;
pub mod packages;
pub mod patches;
pub mod saves;
pub mod updates;

use crate::core::types::Progress;
use futures_util::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

const RELEASES_API: &str = "https://api.github.com/repos/RPCS3/rpcs3-binaries-win/releases/latest";
const USER_AGENT: &str = "Omoio";

#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

// RPCS3 ships as a portable folder, not an installer - extracting the
// archive here is the whole install.
pub fn install_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let local_data = app.path().local_data_dir().map_err(|e| e.to_string())?;
    Ok(local_data.join("Omoio").join("rpcs3"))
}

fn exe_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(install_dir(app)?.join("rpcs3.exe"))
}

pub fn detect_version(app: &AppHandle) -> Option<String> {
    let exe = exe_path(app).ok()?;
    if !exe.exists() {
        return None;
    }
    read_version(&exe)
}

pub fn open_in_explorer(folder: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(folder)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = folder;
        Err("Only supported on Windows.".to_string())
    }
}

// RPCS3 is a console-less GUI binary; without this flag every call to it
// flashes a console window over whatever the user is looking at.
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

// Asking the binary itself rather than trusting whatever we last installed:
// the installed build's actual behaviour is what counts.
fn read_version(exe: &Path) -> Option<String> {
    let output = command(exe).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // "RPCS3 0.0.42-19884-3ef20ebb Alpha" -> "0.0.42-19884-3ef20ebb"
    text.split_whitespace().nth(1).map(|s| s.to_string())
}

async fn latest_release(client: &reqwest::Client) -> Result<Release, String> {
    let response = client
        .get(RELEASES_API)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub returned {}", response.status()));
    }
    response.json::<Release>().await.map_err(|e| e.to_string())
}

/// The version a build's archive carries in its name, in the form
/// `--version` prints: `rpcs3-v0.0.42-19985-6ba56a52_win64_msvc.7z` is
/// `0.0.42-19985-6ba56a52`.
fn version_from_archive(name: &str) -> Option<&str> {
    name.strip_prefix("rpcs3-v")?.strip_suffix("_win64_msvc.7z")
}

pub async fn newest_version() -> Result<String, String> {
    let release = latest_release(&reqwest::Client::new()).await?;
    release
        .assets
        .iter()
        .find_map(|asset| version_from_archive(&asset.name))
        .map(str::to_string)
        .ok_or_else(|| "No Windows build found in the latest RPCS3 release".to_string())
}

pub async fn install(app: AppHandle, cancel: Arc<AtomicBool>) -> Result<String, String> {
    let client = reqwest::Client::new();

    emit(&app, "checking", 0, 0);
    let release = latest_release(&client).await?;

    let archive = release
        .assets
        .iter()
        .find(|a| a.name.ends_with("_win64_msvc.7z"))
        .ok_or("No Windows build found in the latest RPCS3 release")?;
    let checksum_name = format!("{}.sha256", archive.name);
    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name == checksum_name)
        .ok_or("No checksum file found for the RPCS3 build")?;

    let dest_dir = install_dir(&app)?;
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let archive_path = dest_dir.join(&archive.name);

    download(&client, &archive.browser_download_url, &archive_path, &app, &cancel).await?;
    if cancel.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(&archive_path);
        return Err("cancelled".to_string());
    }

    emit(&app, "verifying", 0, 1);
    let expected = client
        .get(&checksum_asset.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?
        .trim()
        .to_lowercase();
    let actual = sha256_hex(&archive_path)?;
    if actual != expected {
        let _ = std::fs::remove_file(&archive_path);
        return Err("Downloaded file failed checksum verification".to_string());
    }

    emit(&app, "extracting", 0, 1);
    let extract_dir = dest_dir.clone();
    let archive_path_clone = archive_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        sevenz_rust2::decompress_file(&archive_path_clone, &extract_dir)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let _ = std::fs::remove_file(&archive_path);

    let exe = exe_path(&app)?;
    let version = read_version(&exe).ok_or("RPCS3 installed but did not report a version")?;
    emit(&app, "done", 1, 1);
    Ok(version)
}

async fn download(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    app: &AppHandle,
    cancel: &Arc<AtomicBool>,
) -> Result<(), String> {
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let total = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(dest).await.map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        emit(app, "downloading", downloaded, total);
    }

    Ok(())
}

fn emit(app: &AppHandle, stage: &str, bytes: u64, total: u64) {
    let _ = app.emit(
        "rpcs3-install-progress",
        Progress {
            stage: stage.to_string(),
            bytes,
            total,
        },
    );
}

fn sha256_hex(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// RPCS3 as the PS3's emulator, for the parts of Omoio that work across
/// consoles. Each method hands on to the modules above.
pub struct Rpcs3;

impl super::EmulatorBackend for Rpcs3 {
    fn console(&self) -> crate::core::console::Console {
        crate::core::console::Console::Ps3
    }

    fn name(&self) -> &'static str {
        "RPCS3"
    }

    fn newest_version(&self) -> futures_util::future::BoxFuture<'static, Result<String, String>> {
        Box::pin(newest_version())
    }

    fn features(&self) -> crate::core::console::Features {
        crate::core::console::Features {
            updates: true,
            patches: true,
            settings: true,
            saves: true,
            compatibility: true,
        }
    }

    fn recognises(&self, path: &Path) -> bool {
        crate::import::recognises(path)
    }

    fn identify(&self, path: &Path) -> Result<crate::core::library::Game, String> {
        crate::import::identify(path).map_err(|e| e.to_string())
    }

    fn icon(&self, game: &crate::core::library::Game) -> Option<PathBuf> {
        crate::import::icon_path(&game.path)
    }

    fn prepare(&self, app: &AppHandle) {
        // RPCS3 starts bound to the keyboard, so a pad plugged in for the
        // first time would do nothing. Set it up unless someone already has.
        controllers::set_up_if_needed(app);
    }

    fn tune_picture(
        &self,
        app: &AppHandle,
        display_height: u32,
        graphics_memory: u64,
    ) -> Result<Option<u32>, String> {
        graphics::apply(app, display_height, graphics_memory)
    }

    fn launch(&self, app: &AppHandle, game: &crate::core::library::Game) -> Result<u32, String> {
        launch::launch(app, game)
    }

    fn detect_version(&self, app: &AppHandle) -> Option<String> {
        detect_version(app)
    }

    fn log_file(&self, app: &AppHandle) -> Option<PathBuf> {
        install_dir(app).ok().map(|dir| dir.join("log").join("RPCS3.log"))
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
        ("PS3 results from RPCS3", "https://rpcs3.net/compatibility")
    }

    fn apply_fixes(
        &self,
        app: &AppHandle,
        game: &crate::core::library::Game,
        applied: &[String],
    ) -> Vec<&'static str> {
        let version = game.running_version().unwrap_or_default();
        fixes::apply(app, &game.title_id, version, applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_builds_version_is_read_from_its_archive_name() {
        assert_eq!(
            version_from_archive("rpcs3-v0.0.42-19985-6ba56a52_win64_msvc.7z"),
            Some("0.0.42-19985-6ba56a52")
        );
        assert_eq!(version_from_archive("rpcs3-v0.0.42-19985-6ba56a52_win64_msvc.7z.sha256"), None);
    }
}
