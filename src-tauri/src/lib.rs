pub mod archive;
mod backends;
mod commands;
pub mod core;
mod hardware;
pub mod import;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::InstallState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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
            commands::get_firmware_version,
            commands::install_firmware,
            commands::list_games,
            commands::import_game,
            commands::launch_game,
            commands::remove_game,
            commands::get_games_folder,
            commands::set_games_folder,
            commands::import_archive,
            commands::cancel_import,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
