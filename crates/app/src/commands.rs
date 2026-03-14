use tazama_core::{MediaInfo, Project, ProjectSettings};
use tazama_media::ExportConfig;

#[tauri::command]
pub async fn new_project(name: String, width: u32, height: u32) -> Result<Project, String> {
    let settings = ProjectSettings {
        width,
        height,
        ..Default::default()
    };
    Ok(Project::new(name, settings))
}

#[tauri::command]
pub async fn open_project(path: String) -> Result<Project, String> {
    tazama_storage::ProjectStore::load(std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_project(project: Project, path: String) -> Result<(), String> {
    tazama_storage::ProjectStore::save(&project, std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_media(project_root: String, source: String) -> Result<String, String> {
    let store = tazama_storage::MediaStore::new(&project_root);
    let dest = store
        .import(std::path::Path::new(&source))
        .await
        .map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn probe_media(path: String) -> Result<MediaInfo, String> {
    tazama_media::init().map_err(|e| e.to_string())?;
    tazama_media::probe::probe(std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_project(
    app: tauri::AppHandle,
    project: Project,
    config: ExportConfig,
) -> Result<(), String> {
    use tauri::Emitter;
    use tazama_media::ExportProgress;

    tazama_media::init().map_err(|e| e.to_string())?;

    let total_frames = project
        .timeline
        .tracks
        .iter()
        .flat_map(|t| &t.clips)
        .map(|c| c.timeline_start + c.duration)
        .max()
        .unwrap_or(0);

    let _ = app.emit(
        "export-progress",
        ExportProgress {
            frames_written: 0,
            total_frames,
            done: false,
        },
    );

    // Create channels for the export pipeline
    let (video_tx, video_rx) = tokio::sync::mpsc::channel(16);
    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel(16);

    // Start export pipeline in background
    let _progress_rx = tazama_media::export::pipeline::ExportPipeline::run(
        config,
        video_rx,
        audio_rx,
    )
    .map_err(|e| e.to_string())?;

    // Close channels immediately — no frames to feed yet (GPU render integration in Phase 5)
    drop(video_tx);
    drop(audio_tx);

    let _ = app.emit(
        "export-progress",
        ExportProgress {
            frames_written: total_frames,
            total_frames,
            done: true,
        },
    );

    Ok(())
}
