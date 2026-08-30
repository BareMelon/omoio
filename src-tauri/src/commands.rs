use crate::core::types::HardwareInfo;
use crate::hardware;

#[tauri::command]
pub fn get_hardware_info() -> HardwareInfo {
    hardware::detect()
}
