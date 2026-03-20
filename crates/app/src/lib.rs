mod commands;
mod frame_source;

use std::sync::Arc;

#[cfg(debug_assertions)]
use tauri::Manager;
use tokio::sync::Mutex;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(tazama_storage::AutosaveManager::new(
            30,
        ))))
        .setup(|_app| {
            let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("tazama=info,tazama_media=info,tazama_gpu=info,tazama_core=info")
            });

            let use_json = std::env::var("TAZAMA_LOG_JSON").is_ok();

            if use_json {
                let fmt_layer = fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true);

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            } else {
                let fmt_layer = fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true);

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            }

            // File logging via shell redirect:
            //   RUST_LOG=debug tazama 2> tazama.log
            // JSON structured output:
            //   RUST_LOG=debug TAZAMA_LOG_JSON=1 tazama

            tracing::info!("Tazama starting (version {})", env!("CARGO_PKG_VERSION"));

            #[cfg(debug_assertions)]
            {
                let window = _app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

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
            commands::detect_hardware,
            commands::measure_loudness,
            commands::extract_waveform,
            commands::generate_thumbnails,
        ])
        .run(tauri::generate_context!())
        .expect("error running tazama");
}
