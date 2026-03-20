use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use tauri::State;
use tazama_core::{MediaInfo, Project, ProjectSettings};
use tazama_media::ExportConfig;
use tazama_storage::AutosaveManager;
use tokio::sync::Mutex;

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

    if let Err(e) = app.emit(
        "export-progress",
        ExportProgress {
            frames_written: 0,
            total_frames,
            done: false,
        },
    ) {
        tracing::warn!("failed to emit event: {e}");
    }

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
            if let Err(e) = app_handle.emit(
                "export-progress",
                ExportProgress {
                    frames_written: frame_index + 1,
                    total_frames,
                    done: false,
                },
            ) {
                tracing::warn!("failed to emit event: {e}");
            }
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
        if let Err(e) = app.emit("export-progress", &progress) {
            tracing::warn!("failed to emit event: {e}");
        }
        if progress.done {
            break;
        }
    }

    if let Err(e) = app.emit(
        "export-progress",
        ExportProgress {
            frames_written: total_frames,
            total_frames,
            done: true,
        },
    ) {
        tracing::warn!("failed to emit event: {e}");
    }

    Ok(())
}

// --- Autosave commands ---

#[tauri::command]
pub async fn start_autosave(
    autosave: State<'_, Arc<Mutex<AutosaveManager>>>,
) -> Result<(), String> {
    let mut mgr = autosave.lock().await;
    mgr.start();
    Ok(())
}

#[tauri::command]
pub async fn stop_autosave(autosave: State<'_, Arc<Mutex<AutosaveManager>>>) -> Result<(), String> {
    let mut mgr = autosave.lock().await;
    mgr.stop();
    Ok(())
}

#[tauri::command]
pub async fn check_autosave_recovery(path: String) -> Result<Option<Project>, String> {
    let recovered = tazama_storage::autosave::recover(std::path::Path::new(&path)).await;
    Ok(recovered)
}

#[tauri::command]
pub async fn cleanup_autosave(path: String) -> Result<(), String> {
    tazama_storage::autosave::cleanup(std::path::Path::new(&path)).await;
    Ok(())
}

#[tauri::command]
pub async fn notify_autosave(
    autosave: State<'_, Arc<Mutex<AutosaveManager>>>,
    project: Project,
    path: String,
) -> Result<(), String> {
    let mgr = autosave.lock().await;
    mgr.update_project(project, PathBuf::from(path)).await;
    Ok(())
}

// --- Recording commands ---

#[tauri::command]
pub async fn start_recording(sample_rate: u32, channels: u16) -> Result<(), String> {
    tazama_media::record::start(sample_rate, channels).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_recording() -> Result<String, String> {
    let path = tazama_media::record::stop().map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// --- Proxy commands ---

#[tauri::command]
pub async fn generate_proxies(
    project: Project,
    proxy_dir: String,
    target_width: u32,
) -> Result<Vec<String>, String> {
    tazama_media::init().map_err(|e| e.to_string())?;
    let mut proxy_paths = Vec::new();

    for track in &project.timeline.tracks {
        for clip in &track.clips {
            if let Some(media) = &clip.media {
                let proxy = tazama_media::proxy::generate_proxy(
                    std::path::Path::new(&media.path),
                    std::path::Path::new(&proxy_dir),
                    target_width,
                )
                .await
                .map_err(|e| e.to_string())?;
                proxy_paths.push(proxy.to_string_lossy().to_string());
            }
        }
    }

    Ok(proxy_paths)
}

#[tauri::command]
pub async fn set_proxy_mode(_enabled: bool) -> Result<(), String> {
    // Proxy mode toggle - the frontend uses this to decide whether
    // to use proxy_path or original path for preview
    Ok(())
}

#[tauri::command]
pub async fn measure_loudness(path: String) -> Result<f64, String> {
    tazama_media::init().map_err(|e| e.to_string())?;

    let path = std::path::PathBuf::from(path);
    let mut rx = tazama_media::decode::audio::AudioDecoder::decode(&path)
        .map_err(|e| e.to_string())?;

    // Collect all decoded audio into one buffer
    let mut all_samples = Vec::new();
    let mut sample_rate = 48000;
    let mut channels = 2u16;
    while let Some(buf) = rx.recv().await {
        sample_rate = buf.sample_rate;
        channels = buf.channels;
        all_samples.extend_from_slice(&buf.samples);
    }

    if all_samples.is_empty() {
        return Err("no audio data found".into());
    }

    let combined = tazama_media::AudioBuffer {
        sample_rate,
        channels,
        samples: all_samples,
        timestamp_ns: 0,
    };
    Ok(tazama_media::loudness::measure_loudness(&combined))
}

#[tauri::command]
pub async fn extract_waveform(
    path: String,
    peaks_per_second: u32,
) -> Result<tazama_core::WaveformData, String> {
    tazama_media::init().map_err(|e| e.to_string())?;
    tazama_media::waveform::extract_waveform(std::path::Path::new(&path), peaks_per_second)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_thumbnails(
    path: String,
    spec: tazama_core::ThumbnailSpec,
) -> Result<Vec<ThumbnailResult>, String> {
    tazama_media::init().map_err(|e| e.to_string())?;

    let thumbs = tazama_media::thumbnail::generate_thumbnails(
        std::path::Path::new(&path),
        spec,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(thumbs
        .into_iter()
        .map(|(timestamp_ms, data)| ThumbnailResult {
            timestamp_ms,
            data: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &data,
            ),
        })
        .collect())
}

#[derive(Serialize)]
pub struct ThumbnailResult {
    pub timestamp_ms: u64,
    pub data: String,
}

// --- AI features ---

#[tauri::command]
pub async fn detect_highlights(
    path: String,
    max_highlights: u32,
) -> Result<Vec<tazama_media::ai::Highlight>, String> {
    tazama_media::init().map_err(|e| e.to_string())?;
    tazama_media::ai::detect_highlights(
        std::path::Path::new(&path),
        max_highlights as usize,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn transcribe_audio(
    path: String,
    language_hint: Option<String>,
) -> Result<Vec<tazama_media::ai::SubtitleCue>, String> {
    tazama_media::init().map_err(|e| e.to_string())?;

    let path = std::path::PathBuf::from(path);
    let mut rx = tazama_media::decode::audio::AudioDecoder::decode(&path)
        .map_err(|e| e.to_string())?;

    let mut all_samples = Vec::new();
    let mut sample_rate = 48000u32;
    let mut channels = 2u16;
    while let Some(buf) = rx.recv().await {
        sample_rate = buf.sample_rate;
        channels = buf.channels;
        all_samples.extend_from_slice(&buf.samples);
    }

    if all_samples.is_empty() {
        return Err("no audio data found".into());
    }

    let byte_data: Vec<u8> = all_samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let num_frames = all_samples.len() / channels.max(1) as usize;
    let tarang_buf = tarang::core::AudioBuffer {
        data: bytes::Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels,
        sample_rate,
        num_frames,
        timestamp: std::time::Duration::ZERO,
    };

    // Prepare for transcription
    let prepared = tarang::ai::prepare_audio_for_transcription(&tarang_buf);
    let duration_secs = prepared.num_frames as f64 / prepared.sample_rate as f64;

    let request = tarang::ai::TranscriptionRequest {
        audio_codec: "pcm_f32le".to_string(),
        sample_rate: prepared.sample_rate,
        channels: prepared.channels,
        duration_secs,
        language_hint,
    };

    let hoosh_config = tarang::ai::HooshConfig {
        endpoint: std::env::var("HOOSH_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:8088".to_string()),
        api_key: std::env::var("HOOSH_API_KEY").ok(),
        model: tarang::ai::WhisperModel::Base,
        timeout: std::time::Duration::from_secs(120),
        max_wav_bytes: 50 * 1024 * 1024,
        chunk_duration_secs: 30.0,
    };

    let client = tarang::ai::HooshClient::new(hoosh_config)
        .map_err(|e| e.to_string())?;
    let result = client
        .transcribe(&request, &prepared)
        .await
        .map_err(|e| e.to_string())?;

    let cues: Vec<tazama_media::ai::SubtitleCue> = result
        .segments
        .iter()
        .enumerate()
        .map(|(i, seg)| tazama_media::ai::SubtitleCue {
            index: i + 1,
            start_ms: (seg.start * 1000.0) as u64,
            end_ms: (seg.end * 1000.0) as u64,
            text: seg.text.clone(),
        })
        .collect();

    Ok(cues)
}

#[tauri::command]
pub async fn auto_color_correct(
    path: String,
    timestamp_ms: u64,
) -> Result<tazama_media::ai::ColorCorrection, String> {
    tazama_media::init().map_err(|e| e.to_string())?;

    let path = std::path::PathBuf::from(path);
    tokio::task::spawn_blocking(move || {
        let mut demuxer = tazama_media::thumbnail::create_demuxer(&path)
            .map_err(|e| e.to_string())?;
        let info = demuxer.probe().map_err(|e| e.to_string())?;
        let (video_stream_idx, codec) = tazama_media::thumbnail::find_video_stream(&info)
            .ok_or("no video stream")?;

        let config = tarang::video::DecoderConfig::for_codec(codec)
            .map_err(|e| e.to_string())?;
        let mut decoder = tarang::video::VideoDecoder::new(config)
            .map_err(|e| e.to_string())?;
        if let Some(tarang::core::StreamInfo::Video(vs)) = info.streams.get(video_stream_idx) {
            decoder.init(vs);
        }

        let target_ns = timestamp_ms * 1_000_000;

        loop {
            let packet = demuxer.next_packet().map_err(|e| e.to_string())?;
            if packet.stream_index != video_stream_idx {
                continue;
            }
            decoder.send_packet(&packet.data, packet.timestamp)
                .map_err(|e| e.to_string())?;

            while let Ok(frame) = decoder.receive_frame() {
                if frame.timestamp.as_nanos() as u64 >= target_ns {
                    return Ok(tazama_media::ai::auto_color_correct(&frame));
                }
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn suggest_transitions(
    path: String,
    fps: f64,
) -> Result<Vec<(u64, tazama_media::ai::TransitionSuggestion)>, String> {
    tazama_media::init().map_err(|e| e.to_string())?;
    tazama_media::ai::suggest_transitions(std::path::Path::new(&path), fps)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn describe_clip(
    path: String,
    language_hint: Option<String>,
) -> Result<tazama_media::ai::ClipDescription, String> {
    tazama_media::init().map_err(|e| e.to_string())?;

    // First transcribe
    let path_buf = std::path::PathBuf::from(&path);
    let mut rx = tazama_media::decode::audio::AudioDecoder::decode(&path_buf)
        .map_err(|e| e.to_string())?;

    let mut all_samples = Vec::new();
    let mut sample_rate = 48000u32;
    let mut channels = 2u16;
    while let Some(buf) = rx.recv().await {
        sample_rate = buf.sample_rate;
        channels = buf.channels;
        all_samples.extend_from_slice(&buf.samples);
    }

    let duration_ms = if sample_rate > 0 && channels > 0 {
        (all_samples.len() as u64 * 1000) / (sample_rate as u64 * channels as u64)
    } else {
        0
    };

    // Try to get transcription for context
    let cues = if !all_samples.is_empty() {
        let byte_data: Vec<u8> = all_samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let num_frames = all_samples.len() / channels.max(1) as usize;
        let tarang_buf = tarang::core::AudioBuffer {
            data: bytes::Bytes::from(byte_data),
            sample_format: tarang::core::SampleFormat::F32,
            channels,
            sample_rate,
            num_frames,
            timestamp: std::time::Duration::ZERO,
        };
        let prepared = tarang::ai::prepare_audio_for_transcription(&tarang_buf);
        let dur_secs = prepared.num_frames as f64 / prepared.sample_rate as f64;
        let request = tarang::ai::TranscriptionRequest {
            audio_codec: "pcm_f32le".to_string(),
            sample_rate: prepared.sample_rate,
            channels: prepared.channels,
            duration_secs: dur_secs,
            language_hint,
        };
        let hoosh_config = tarang::ai::HooshConfig {
            endpoint: std::env::var("HOOSH_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8088".to_string()),
            api_key: std::env::var("HOOSH_API_KEY").ok(),
            model: tarang::ai::WhisperModel::Base,
            timeout: std::time::Duration::from_secs(120),
            max_wav_bytes: 50 * 1024 * 1024,
            chunk_duration_secs: 30.0,
        };
        let client = tarang::ai::HooshClient::new(hoosh_config)
            .map_err(|e| e.to_string())?;
        match client.transcribe(&request, &prepared).await {
            Ok(result) => result
                .segments
                .iter()
                .enumerate()
                .map(|(i, seg)| tazama_media::ai::SubtitleCue {
                    index: i + 1,
                    start_ms: (seg.start * 1000.0) as u64,
                    end_ms: (seg.end * 1000.0) as u64,
                    text: seg.text.clone(),
                })
                .collect(),
            Err(_) => vec![],
        }
    } else {
        vec![]
    };

    let llm_config = tazama_media::ai::LlmConfig::default();
    tazama_media::ai::describe_clip(&llm_config, &cues, duration_ms, true)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refine_subtitles(
    cues: Vec<tazama_media::ai::SubtitleCue>,
) -> Result<Vec<tazama_media::ai::SubtitleCue>, String> {
    let config = tazama_media::ai::LlmConfig::default();
    tazama_media::ai::refine_subtitles(&config, &cues)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn translate_subtitles(
    cues: Vec<tazama_media::ai::SubtitleCue>,
    target_language: String,
) -> Result<Vec<tazama_media::ai::SubtitleCue>, String> {
    let config = tazama_media::ai::LlmConfig::default();
    tazama_media::ai::translate_subtitles(&config, &cues, &target_language)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn detect_hardware() -> Result<serde_json::Value, String> {
    let hardware = tazama_media::hwaccel::hardware_summary();
    let encoders = tazama_media::available_encoders();

    serde_json::to_value(serde_json::json!({
        "accelerators": hardware,
        "available_encoders": encoders,
    }))
    .map_err(|e| e.to_string())
}

/// Response for a rendered preview frame.
#[derive(Serialize)]
pub struct PreviewFrame {
    /// Base64-encoded RGBA pixel data.
    pub data: String,
    pub width: u32,
    pub height: u32,
}

/// Render a single preview frame at the given timeline position.
///
/// Runs the full GPU render pipeline (effects, compositing, transitions) so the
/// preview matches what export produces.
#[tauri::command]
pub async fn render_preview_frame(
    project: Project,
    frame_index: u64,
) -> Result<PreviewFrame, String> {
    tazama_media::init().map_err(|e| e.to_string())?;

    let width = project.settings.width;
    let height = project.settings.height;

    // Fast path: no clips at this frame → return black without touching the GPU
    if project
        .timeline
        .topmost_video_clip_at(frame_index)
        .is_none()
    {
        let black = vec![0u8; (width * height * 4) as usize];
        return Ok(PreviewFrame {
            data: base64::engine::general_purpose::STANDARD.encode(&black),
            width,
            height,
        });
    }

    let settings = project.settings.clone();
    let timeline = project.timeline.clone();
    let frame_rate = (
        settings.frame_rate.numerator,
        settings.frame_rate.denominator,
    );

    let gpu_frame = tokio::task::spawn_blocking(move || -> Result<tazama_gpu::GpuFrame, String> {
        let gpu_ctx =
            Arc::new(tazama_gpu::GpuContext::new().map_err(|e| format!("GPU init failed: {e}"))?);
        let renderer = tazama_gpu::Renderer::new(Arc::clone(&gpu_ctx))
            .map_err(|e| format!("renderer init failed: {e}"))?;
        let frame_source = Arc::new(crate::frame_source::MediaFrameSource::new(frame_rate));

        renderer
            .render_frame(&timeline, frame_index, frame_source.as_ref(), &settings)
            .map_err(|e| format!("render frame: {e}"))
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(PreviewFrame {
        data: base64::engine::general_purpose::STANDARD.encode(&gpu_frame.data),
        width: gpu_frame.width,
        height: gpu_frame.height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_project_creates_with_defaults() {
        let project = new_project("Test".into(), 1920, 1080).await.unwrap();
        assert_eq!(project.name, "Test");
        assert_eq!(project.settings.width, 1920);
        assert_eq!(project.settings.height, 1080);
        assert!(project.timeline.tracks.is_empty()); // starts with no tracks
    }

    #[tokio::test]
    async fn new_project_4k() {
        let project = new_project("4K".into(), 3840, 2160).await.unwrap();
        assert_eq!(project.settings.width, 3840);
        assert_eq!(project.settings.height, 2160);
    }

    #[tokio::test]
    async fn save_and_open_project_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.tazama");
        let project = new_project("Round".into(), 1280, 720).await.unwrap();
        save_project(project.clone(), path.display().to_string())
            .await
            .unwrap();
        let loaded = open_project(path.display().to_string()).await.unwrap();
        assert_eq!(loaded.name, "Round");
        assert_eq!(loaded.settings.width, 1280);
    }

    #[tokio::test]
    async fn open_project_nonexistent_returns_error() {
        let result = open_project("/tmp/nonexistent_tazama_test.tazama".into()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn import_media_nonexistent_source_errors() {
        let dir = tempfile::tempdir().unwrap();
        let result = import_media(
            dir.path().display().to_string(),
            "/tmp/nonexistent_media.mp4".into(),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn import_media_copies_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        std::fs::write(&source, b"test data").unwrap();
        let result = import_media(
            dir.path().display().to_string(),
            source.display().to_string(),
        )
        .await;
        assert!(result.is_ok());
        let dest = result.unwrap();
        assert!(std::path::Path::new(&dest).exists());
    }

    #[tokio::test]
    async fn probe_media_nonexistent_errors() {
        tazama_media::init().ok();
        let result = probe_media("/tmp/nonexistent_media.mp4".into()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn check_autosave_no_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.tazama");
        let result = check_autosave_recovery(path.display().to_string())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn cleanup_autosave_nonexistent_is_ok() {
        let result = cleanup_autosave("/tmp/nonexistent_project.tazama".into()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stop_recording_without_start_errors() {
        let result = stop_recording().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn set_proxy_mode_is_noop() {
        let result = set_proxy_mode(true).await;
        assert!(result.is_ok());
        let result = set_proxy_mode(false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn detect_hardware_returns_json() {
        tazama_media::init().ok();
        let result = detect_hardware().await.unwrap();
        assert!(result.get("accelerators").is_some());
        assert!(result.get("available_encoders").is_some());
    }

    #[tokio::test]
    async fn generate_proxies_empty_project() {
        tazama_media::init().ok();
        let project = new_project("Empty".into(), 1920, 1080).await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let result = generate_proxies(project, dir.path().display().to_string(), 640).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
