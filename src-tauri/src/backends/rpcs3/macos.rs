//! Official native RPCS3 bundles for Intel and Apple Silicon.

use super::*;

pub(super) async fn install(app: AppHandle, cancel: Arc<AtomicBool>) -> Result<String, String> {
    emit(&app, "checking", 0, 0);
    let client = reqwest::Client::new();
    let release = latest_release(&client).await?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(native_archive_suffix()))
        .ok_or("The latest release has no RPCS3 build for this Mac.")?;
    let expected = asset
        .digest
        .as_deref()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("GitHub did not provide a SHA-256 digest for the RPCS3 Mac build.")?;
    let dir = install_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archive = dir.join("rpcs3-download.7z");
    download(
        &client,
        &asset.browser_download_url,
        &archive,
        &app,
        &cancel,
    )
    .await?;
    if cancel.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(&archive);
        return Err("cancelled".into());
    }
    emit(&app, "verifying", 0, 1);
    if sha256_hex(&archive)? != expected.to_ascii_lowercase() {
        let _ = std::fs::remove_file(&archive);
        return Err("Downloaded RPCS3 Mac build failed checksum verification.".into());
    }
    emit(&app, "extracting", 0, 1);
    INSTALLING.store(true, Ordering::Relaxed);
    let _installing = Installing;
    let (from, into) = (archive.clone(), dir.clone());
    let installed = tauri::async_runtime::spawn_blocking(move || {
        crate::macos::install_7z(&from, &into, "RPCS3.app", "rpcs3")
    })
    .await
    .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&archive);
    installed?;
    *KNOWN.lock().unwrap() = None;
    std::fs::create_dir_all(data_dir(&app)?).map_err(|e| e.to_string())?;
    let version = read_version_checked(&exe_path(&app)?)?;
    if !version_from_archive(&asset.name)
        .is_some_and(|full| full.starts_with(&format!("{version}-")))
    {
        return Err(
            "The installed RPCS3 bundle version does not match its release archive.".into(),
        );
    }
    emit(&app, "done", 1, 1);
    Ok(version)
}
