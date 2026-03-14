use std::sync::Arc;

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
    use tazama_media::{ExportProgress, VideoFrame};

    tazama_media::init().map_err(|e| e.to_string())?;

    let total_frames = project.timeline.duration_frames();
    if total_frames == 0 {
        return Err("timeline is empty — nothing to export".into());
    }

    let _ = app.emit(
        "export-progress",
        ExportProgress {
            frames_written: 0,
            total_frames,
            done: false,
        },
    );

    // Initialize GPU context and renderer
    let gpu_ctx =
        Arc::new(tazama_gpu::GpuContext::new().map_err(|e| format!("GPU init failed: {e}"))?);
    let renderer = tazama_gpu::Renderer::new(Arc::clone(&gpu_ctx))
        .map_err(|e| format!("renderer init failed: {e}"))?;

    let frame_rate = (
        project.settings.frame_rate.numerator,
        project.settings.frame_rate.denominator,
    );
    let frame_source = Arc::new(crate::frame_source::MediaFrameSource::new(frame_rate));

    // Create channels for the export pipeline
    let (video_tx, video_rx) = tokio::sync::mpsc::channel(16);
    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel(64);

    // Start export pipeline in background
    let mut progress_rx = tazama_media::export::pipeline::ExportPipeline::run_with_total(
        config,
        video_rx,
        audio_rx,
        total_frames,
    )
    .map_err(|e| e.to_string())?;

    let settings = project.settings.clone();
    let timeline = project.timeline.clone();
    let app_handle = app.clone();

    // Render and feed frames on a blocking task (GPU work is synchronous)
    let video_handle = tokio::task::spawn_blocking(move || -> Result<(), String> {
        for frame_index in 0..total_frames {
            let gpu_frame = renderer
                .render_frame(&timeline, frame_index, frame_source.as_ref(), &settings)
                .map_err(|e| format!("render frame {frame_index}: {e}"))?;

            let video_frame = VideoFrame {
                frame_index: gpu_frame.frame_index,
                width: gpu_frame.width,
                height: gpu_frame.height,
                data: gpu_frame.data,
                timestamp_ns: gpu_frame.timestamp_ns,
            };

            video_tx
                .blocking_send(video_frame)
                .map_err(|_| "export pipeline closed unexpectedly".to_string())?;

            // Emit progress
            let _ = app_handle.emit(
                "export-progress",
                ExportProgress {
                    frames_written: frame_index + 1,
                    total_frames,
                    done: false,
                },
            );
        }
        // video_tx drops here, signaling EOS to the export pipeline
        Ok(())
    });

    // Mix all audio tracks together and feed to the export pipeline
    let audio_timeline = project.timeline.clone();
    let audio_frame_rate = project.settings.frame_rate;
    let audio_sample_rate = project.settings.sample_rate;
    let audio_channels = project.settings.channels;
    let audio_handle = tokio::task::spawn_blocking(move || -> Result<(), String> {
        tazama_media::mix::mix_timeline_audio(
            &audio_timeline,
            &audio_frame_rate,
            audio_sample_rate,
            audio_channels,
            audio_tx,
        )
        .map_err(|e| format!("audio mix: {e}"))
    });

    // Wait for both feed tasks
    let video_result = video_handle.await.map_err(|e| e.to_string())?;
    let audio_result = audio_handle.await.map_err(|e| e.to_string())?;
    video_result?;
    audio_result?;

    // Wait for export pipeline to signal completion
    while progress_rx.changed().await.is_ok() {
        let progress = progress_rx.borrow().clone();
        let _ = app.emit("export-progress", &progress);
        if progress.done {
            break;
        }
    }

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
