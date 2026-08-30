mod backends;
mod commands;
mod core;
mod hardware;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::InstallState::default())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_hardware_info,
            commands::get_rpcs3_version,
            commands::install_rpcs3,
            commands::cancel_rpcs3_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
