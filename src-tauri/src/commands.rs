use crate::archive;
use crate::backends::rpcs3;
use crate::core::library::Library;
use crate::core::settings::Settings;
use crate::core::types::{GameEntry, HardwareInfo, Progress};
use crate::hardware;
use crate::import;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn get_hardware_info() -> HardwareInfo {
    hardware::detect()
}

#[derive(Default)]
pub struct InstallState {
    cancel: Arc<AtomicBool>,
    cancel_import: Arc<AtomicBool>,
}

#[tauri::command]
pub fn get_rpcs3_version(app: AppHandle) -> Option<String> {
    rpcs3::detect_version(&app)
}

#[tauri::command]
pub async fn install_rpcs3(app: AppHandle, state: State<'_, InstallState>) -> Result<String, String> {
    state.cancel.store(false, Ordering::Relaxed);
    let cancel = state.cancel.clone();
    rpcs3::install(app, cancel).await
}

#[tauri::command]
pub fn cancel_rpcs3_install(state: State<'_, InstallState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_firmware_version(app: AppHandle) -> Option<String> {
    rpcs3::firmware::detect_version(&app)
}

fn omoio_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let data = app.path().data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("Omoio"))
}

fn library_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(omoio_data_dir(app)?.join("library.json"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(omoio_data_dir(app)?.join("settings.json"))
}

#[tauri::command]
pub fn get_games_folder(app: AppHandle) -> Result<Option<String>, String> {
    Ok(Settings::load(&settings_path(&app)?)
        .games_folder
        .map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn set_games_folder(app: AppHandle, path: String) -> Result<(), String> {
    let file = settings_path(&app)?;
    let mut settings = Settings::load(&file);
    settings.games_folder = Some(PathBuf::from(path));
    settings.save(&file)
}

#[tauri::command]
pub fn cancel_import(state: State<'_, InstallState>) {
    state.cancel_import.store(true, Ordering::Relaxed);
}

/// Unpacks a compressed dump into the games folder, then imports what came out.
#[tauri::command]
pub async fn import_archive(
    app: AppHandle,
    path: String,
    state: State<'_, InstallState>,
) -> Result<GameEntry, String> {
    state.cancel_import.store(false, Ordering::Relaxed);
    let cancel = state.cancel_import.clone();

    let games_folder = Settings::load(&settings_path(&app)?)
        .games_folder
        .ok_or("Choose a games folder first.")?;

    tauri::async_runtime::spawn_blocking(move || {
        let source = PathBuf::from(&path);
        let kind = archive::detect_kind(&source)
            .ok_or("That file isn't a .7z or .zip archive.")?;

        let needed = archive::unpacked_size(&source, kind)?;
        let free = free_space(&games_folder);
        if free.is_some_and(|free| free < needed) {
            return Err(format!(
                "This game needs {} GB unpacked and the drive has less than that free.",
                needed / 1024u64.pow(3)
            ));
        }

        let name = source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .ok_or("That file has no name.")?;
        let dest = games_folder.join(&name);
        if dest.exists() {
            return Err("There's already a folder with that name in your games folder.".to_string());
        }

        let mut last_sent = 0u64;
        let outcome = archive::extract(&source, kind, &dest, &cancel, &mut |done| {
            // One event per percent rather than per chunk: 3900 files would
            // otherwise flood the interface with redundant redraws.
            let step = (needed / 100).max(1);
            if done - last_sent >= step || done >= needed {
                last_sent = done;
                emit_import_progress(&app, "unpacking", done, needed);
            }
        })?;
        if outcome.is_err() {
            return Err("cancelled".to_string());
        }

        emit_import_progress(&app, "identifying", needed, needed);
        let game = match import::identify(&dest) {
            Ok(game) => game,
            Err(e) => {
                // Nothing importable came out, so don't leave it behind.
                let _ = std::fs::remove_dir_all(&dest);
                return Err(e.to_string());
            }
        };

        let library_file = library_path(&app)?;
        let mut library = Library::load(&library_file);
        library.upsert(game.clone());
        library.save(&library_file)?;
        Ok(entry(&app, game))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn emit_import_progress(app: &AppHandle, stage: &str, bytes: u64, total: u64) {
    let _ = app.emit(
        "import-progress",
        Progress {
            stage: stage.to_string(),
            bytes,
            total,
        },
    );
}

#[cfg(windows)]
fn free_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    // The folder may not exist yet, so ask about the nearest parent that does.
    let mut probe = path;
    while !probe.exists() {
        probe = probe.parent()?;
    }
    let wide: Vec<u16> = probe.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0u64;
    unsafe {
        GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut available), None, None).ok()?;
    }
    Some(available)
}

#[cfg(not(windows))]
fn free_space(_path: &Path) -> Option<u64> {
    None
}

/// Keeps a copy of the dump's tile art under our own folder. Done once per
/// game, and only while the drive is there, so the library keeps its art after
/// the drive goes away.
fn cached_cover(app: &AppHandle, game: &crate::core::library::Game) -> Option<String> {
    let covers = omoio_data_dir(app).ok()?.join("covers");
    let cached = covers.join(format!("{}.png", game.title_id));

    if !cached.is_file() {
        let source = import::icon_path(&game.path)?;
        std::fs::create_dir_all(&covers).ok()?;
        std::fs::copy(source, &cached).ok()?;
    }
    Some(cached.to_string_lossy().into_owned())
}

fn entry(app: &AppHandle, game: crate::core::library::Game) -> GameEntry {
    GameEntry {
        available: game.path.is_dir(),
        cover: cached_cover(app, &game),
        game,
    }
}

#[tauri::command]
pub fn list_games(app: AppHandle) -> Result<Vec<GameEntry>, String> {
    let library = Library::load(&library_path(&app)?);
    Ok(library.games().iter().cloned().map(|g| entry(&app, g)).collect())
}

#[tauri::command]
pub async fn import_game(app: AppHandle, path: String) -> Result<GameEntry, String> {
    // Measuring a dump means walking every file in it, so this stays off the
    // UI thread.
    tauri::async_runtime::spawn_blocking(move || {
        let game = import::identify(Path::new(&path)).map_err(|e| e.to_string())?;
        let library_file = library_path(&app)?;
        let mut library = Library::load(&library_file);
        library.upsert(game.clone());
        library.save(&library_file)?;
        Ok(entry(&app, game))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn launch_game(app: AppHandle, title_id: String) -> Result<(), String> {
    let library = Library::load(&library_path(&app)?);
    let game = library
        .games()
        .iter()
        .find(|g| g.title_id == title_id)
        .ok_or("That game isn't in your library any more.")?;
    rpcs3::launch::launch(&app, game)
}

#[tauri::command]
pub fn remove_game(app: AppHandle, title_id: String) -> Result<(), String> {
    let library_file = library_path(&app)?;
    let mut library = Library::load(&library_file);
    // Only the entry goes; the user's files are theirs and stay where they are.
    if library.remove(&title_id) {
        library.save(&library_file)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn install_firmware(app: AppHandle, path: String) -> Result<String, String> {
    // Unpacking firmware takes long enough to block the UI thread, so it runs
    // on the blocking pool.
    tauri::async_runtime::spawn_blocking(move || {
        rpcs3::firmware::install(&app, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}
