mod commands;
mod frame_source;

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(tazama_storage::AutosaveManager::new(
            30,
        ))))
        .setup(|app| {
            let _main_window = app.get_webview_window("main").unwrap();
            tracing_subscriber::fmt()
                .with_env_filter("tazama=debug")
                .init();
            tracing::info!("Tazama starting");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::new_project,
            commands::open_project,
            commands::save_project,
            commands::import_media,
            commands::probe_media,
            commands::export_project,
            commands::render_preview_frame,
            commands::start_autosave,
            commands::stop_autosave,
            commands::check_autosave_recovery,
            commands::cleanup_autosave,
            commands::notify_autosave,
            commands::start_recording,
            commands::stop_recording,
            commands::generate_proxies,
            commands::set_proxy_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error running tazama");
}
