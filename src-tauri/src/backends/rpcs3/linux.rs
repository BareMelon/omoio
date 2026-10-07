//! The official Linux x86_64 AppImage, checked against GitHub's asset digest.

use super::*;

pub(super) async fn install(app: AppHandle, cancel: Arc<AtomicBool>) -> Result<String, String> {
    if std::env::consts::ARCH != "x86_64" {
        return Err("Automatic RPCS3 installation currently supports x86_64 Linux.".into());
    }
    emit(&app, "checking", 0, 0);
    let client = reqwest::Client::new();
    let release = latest_release(&client).await?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with("_linux64.AppImage"))
        .ok_or("No Linux x86_64 build found in the latest RPCS3 release")?;
    let expected = asset
        .digest
        .as_deref()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("GitHub did not provide a SHA-256 digest for the RPCS3 AppImage")?;
    let dir = install_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    remove_old_archives(&dir);
    // A fixed local name avoids using a release's filename as a filesystem path.
    let image = dir.join("rpcs3-download.AppImage");
    download(&client, &asset.browser_download_url, &image, &app, &cancel).await?;
    if cancel.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(&image);
        return Err("cancelled".into());
    }
    emit(&app, "verifying", 0, 1);
    if sha256_hex(&image)? != expected.to_ascii_lowercase() {
        let _ = std::fs::remove_file(&image);
        return Err("Downloaded RPCS3 AppImage failed checksum verification".into());
    }
    emit(&app, "extracting", 0, 1);
    INSTALLING.store(true, Ordering::Relaxed);
    let _installing = Installing;
    let from = image.clone();
    let into = dir.clone();
    let unpacked = tauri::async_runtime::spawn_blocking(move || {
        crate::platform::install_appimage(&from, &into)
    })
    .await
    .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&image);
    unpacked?;
    std::fs::create_dir_all(data_dir(&app)?).map_err(|e| e.to_string())?;
    let version =
        read_version(&exe_path(&app)?).ok_or("RPCS3 installed but did not report a version")?;
    emit(&app, "done", 1, 1);
    Ok(version)
}
