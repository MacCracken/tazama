mod commands;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error running tazama");
}
