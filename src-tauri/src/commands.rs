use crate::backends::rpcs3;
use crate::core::types::HardwareInfo;
use crate::hardware;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_hardware_info() -> HardwareInfo {
    hardware::detect()
}

pub struct InstallState {
    cancel: Arc<AtomicBool>,
}

impl Default for InstallState {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
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
