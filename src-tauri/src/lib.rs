pub mod archive;
mod backends;
mod commands;
pub mod core;
mod hardware;
pub mod import;
pub mod session;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::InstallState::default())
        .manage(session::Session::default())
        // The game picture is a separate window sitting over ours, so it has to
        // be moved whenever ours is, and taken down when ours closes rather
        // than left running with nothing to sit on.
        .on_window_event(|window, event| {
            use tauri::{Manager, WindowEvent};
            let app = window.app_handle();
            let session = app.state::<session::Session>();
            match event {
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    if let Some(main) = app.get_webview_window("main") {
                        session::place(&main, &session);
                    }
                }
                WindowEvent::Destroyed => {
                    session.stop();
                }
                _ => {}
            }
        })
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
            commands::game_settings,
            commands::set_game_settings,
            commands::stop_game,
            commands::playing_game,
            commands::set_game_fullscreen,
            commands::list_sessions,
            commands::read_session_log,
            commands::session_prompt,
            commands::game_compatibility,
            commands::refresh_compatibility,
            commands::game_patches,
            commands::set_patch_enabled,
            commands::refresh_patches,
            commands::scan_folder,
            commands::game_updates,
            commands::install_update,
            commands::cancel_update,
            commands::game_saves,
            commands::back_up_saves,
            commands::restore_saves,
            commands::forget_backup,
            commands::get_account,
            commands::list_regions,
            commands::set_username,
            commands::set_region,
            commands::needs_setup,
            commands::finish_setup,
            commands::get_settings,
            commands::set_start_fullscreen,
            commands::set_keep_sessions,
            commands::get_places,
            commands::reveal_folder,
            commands::forget_all_games,
            commands::clear_session_logs,
            commands::remove_game,
            commands::get_games_folder,
            commands::set_games_folder,
            commands::import_archive,
            commands::cancel_import,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
