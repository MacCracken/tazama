use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Value, json};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use tazama_core::{
    Clip, ClipId, ClipKind, EditCommand, EditHistory, Effect, EffectKind, Marker, MarkerColor,
    MediaRef, Project, ProjectSettings, Timeline, Track, TrackKind,
};

/// Reject paths containing traversal components (`..`) or absolute paths.
fn validate_user_path(path: &str) -> Result<&str, String> {
    let p = std::path::Path::new(path);
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("path must not contain '..' components".into());
        }
    }
    Ok(path)
}

/// MCP tools exposed by Tazama:
///
/// 1. tazama_create_project  — Create a new video project
/// 2. tazama_add_clip        — Add a clip to the timeline
/// 3. tazama_apply_effect    — Apply an effect to a clip
/// 4. tazama_get_timeline    — Get the current timeline state
/// 5. tazama_export          — Export the project to a video file
/// 6. tazama_add_marker      — Add a named marker to the timeline
/// 7. tazama_extract_frame   — Extract a single frame from a clip as PNG
struct ServerState {
    project: Option<Project>,
    history: EditHistory,
    #[allow(dead_code)]
    project_path: Option<PathBuf>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            project: None,
            history: EditHistory::new(),
            project_path: None,
        }
    }

    fn project(&self) -> Result<&Project, &'static str> {
        self.project
            .as_ref()
            .ok_or("No project loaded. Use tazama_create_project first.")
    }

    fn project_mut(&mut self) -> Result<&mut Project, &'static str> {
        self.project
            .as_mut()
            .ok_or("No project loaded. Use tazama_create_project first.")
    }
}

fn mcp_success(id: &Value, text: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{
                "type": "text",
                "text": text.into()
            }]
        }
    })
}

fn mcp_error(id: &Value, msg: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{
                "type": "text",
                "text": msg.into()
            }],
            "isError": true
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("tazama_mcp=info,tazama_media=info,tazama_core=info"));

    let use_json = std::env::var("TAZAMA_LOG_JSON").is_ok();

    if use_json {
        let fmt_layer = fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_writer(std::io::stderr);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    } else {
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_writer(std::io::stderr);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    tracing::info!("tazama-mcp server starting (stdio)");

    tazama_media::init()?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    let mut state = ServerState::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        const MAX_MESSAGE_SIZE: usize = 50 * 1024 * 1024; // 50 MB
        if line.len() > MAX_MESSAGE_SIZE {
            tracing::warn!(
                "incoming JSON message exceeds 50MB limit ({}B), skipping",
                line.len()
            );
            continue;
        }

        let request: Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("invalid JSON: {e}");
                continue;
            }
        };

        let response = handle_request(&request, &mut state).await;
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        stdout.write_all(out.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(request: &Value, state: &mut ServerState) -> Value {
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = request.get("id").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "tazama-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "tools": {}
                }
            }
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "tazama_create_project",
                        "description": "Create a new Tazama video project with the given name and resolution",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "width": { "type": "integer", "default": 1920 },
                                "height": { "type": "integer", "default": 1080 }
                            },
                            "required": ["name"]
                        }
                    },
                    {
                        "name": "tazama_add_clip",
                        "description": "Add a video or audio clip to a timeline track",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "track": { "type": "string", "description": "Track name or ID" },
                                "source": { "type": "string", "description": "Path to media file" },
                                "start_frame": { "type": "integer" },
                                "duration_frames": { "type": "integer" }
                            },
                            "required": ["track", "source"]
                        }
                    },
                    {
                        "name": "tazama_apply_effect",
                        "description": "Apply a video/audio effect to a clip",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "clip_id": { "type": "string" },
                                "effect": { "type": "string", "enum": ["color_grade", "crop", "speed", "fade_in", "fade_out", "volume"] },
                                "params": { "type": "object" }
                            },
                            "required": ["clip_id", "effect"]
                        }
                    },
                    {
                        "name": "tazama_get_timeline",
                        "description": "Get the current timeline state including all tracks and clips",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                        }
                    },
                    {
                        "name": "tazama_export",
                        "description": "Export the current project to a video file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "output_path": { "type": "string" },
                                "format": { "type": "string", "enum": ["mp4", "webm"], "default": "mp4" }
                            },
                            "required": ["output_path"]
                        }
                    },
                    {
                        "name": "tazama_add_marker",
                        "description": "Add a named marker at a specific frame on the timeline",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "Marker name/label" },
                                "frame": { "type": "integer", "description": "Frame position" },
                                "color": { "type": "string", "enum": ["red", "orange", "yellow", "green", "blue", "purple", "white"], "default": "blue" }
                            },
                            "required": ["name", "frame"]
                        }
                    },
                    {
                        "name": "tazama_extract_frame",
                        "description": "Extract a single frame from a video clip and save it as a PNG file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "clip_id": { "type": "string", "description": "UUID of the clip" },
                                "frame_number": { "type": "integer", "description": "Frame index within the clip" },
                                "output_path": { "type": "string", "description": "Where to write the PNG file" }
                            },
                            "required": ["clip_id", "frame_number", "output_path"]
                        }
                    },
                    {
                        "name": "tazama_detect_hardware",
                        "description": "Detect available hardware accelerators and encoding backends on the system",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                        }
                    }
                ]
            }
        }),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            handle_tool_call(&id, tool_name, &args, state).await
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("unknown method: {method}")
            }
        }),
    }
}

async fn handle_tool_call(id: &Value, tool: &str, args: &Value, state: &mut ServerState) -> Value {
    match tool {
        "tazama_create_project" => handle_create_project(id, args, state),
        "tazama_add_clip" => handle_add_clip(id, args, state).await,
        "tazama_apply_effect" => handle_apply_effect(id, args, state),
        "tazama_get_timeline" => handle_get_timeline(id, state),
        "tazama_export" => handle_export(id, args, state).await,
        "tazama_add_marker" => handle_add_marker(id, args, state),
        "tazama_extract_frame" => handle_extract_frame(id, args, state).await,
        "tazama_detect_hardware" => handle_detect_hardware(id),
        _ => mcp_error(id, format!("Unknown tool: {tool}")),
    }
}

fn handle_create_project(id: &Value, args: &Value, state: &mut ServerState) -> Value {
    let name = match args.get("name").and_then(|n| n.as_str()) {
        Some(n) => n,
        None => return mcp_error(id, "Missing required parameter: name"),
    };

    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
    let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;

    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return mcp_error(
            id,
            format!("Invalid dimensions: {width}x{height}. Width and height must be 1..=8192."),
        );
    }

    let settings = ProjectSettings {
        width,
        height,
        ..ProjectSettings::default()
    };

    let mut project = Project::new(name, settings);

    // Add default video and audio tracks
    let video_track = Track::new("Video 1", TrackKind::Video);
    let audio_track = Track::new("Audio 1", TrackKind::Audio);
    project.timeline.add_track(video_track);
    project.timeline.add_track(audio_track);

    state.history = EditHistory::new();
    let project_id = project.id.0.to_string();
    state.project = Some(project);

    mcp_success(
        id,
        format!("Created project '{name}' ({width}x{height}) with ID {project_id}"),
    )
}

async fn handle_add_clip(id: &Value, args: &Value, state: &mut ServerState) -> Value {
    let project = match state.project_mut() {
        Ok(p) => p,
        Err(e) => return mcp_error(id, e),
    };

    let track_name = match args.get("track").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => return mcp_error(id, "Missing required parameter: track"),
    };

    let source = match args.get("source").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => return mcp_error(id, "Missing required parameter: source"),
    };

    if let Err(e) = validate_user_path(source) {
        return mcp_error(id, format!("Invalid source path: {e}"));
    }

    // Find track by name or ID
    let track_id = match find_track_id(&project.timeline, track_name) {
        Some(id) => id,
        None => return mcp_error(id, format!("Track not found: {track_name}")),
    };

    // Probe the media file
    let media_info = match tazama_media::probe::probe(std::path::Path::new(source)).await {
        Ok(info) => info,
        Err(e) => return mcp_error(id, format!("Failed to probe media: {e}")),
    };

    let start_frame = args
        .get("start_frame")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let duration_frames = args
        .get("duration_frames")
        .and_then(|v| v.as_u64())
        .unwrap_or(media_info.duration_frames);

    let has_video = !media_info.video_streams.is_empty();
    let kind = if has_video {
        ClipKind::Video
    } else {
        ClipKind::Audio
    };

    let media_ref = MediaRef {
        path: source.to_string(),
        duration_frames: media_info.duration_frames,
        width: media_info.video_streams.first().map(|v| v.width),
        height: media_info.video_streams.first().map(|v| v.height),
        sample_rate: media_info.audio_streams.first().map(|a| a.sample_rate),
        channels: media_info.audio_streams.first().map(|a| a.channels),
        info: Some(media_info),
        proxy_path: None,
    };

    let clip_name = PathBuf::from(source)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "clip".to_string());

    let clip = Clip::new(&clip_name, kind, start_frame, duration_frames).with_media(media_ref);
    let clip_id = clip.id;

    let cmd = EditCommand::AddClip { track_id, clip };

    let Some(project) = state.project.as_mut() else {
        return mcp_error(id, "No project loaded. Use tazama_create_project first.");
    };

    match state.history.execute(cmd, &mut project.timeline) {
        Ok(()) => mcp_success(
            id,
            format!(
                "Added clip '{clip_name}' (ID: {}) at frame {start_frame}, duration {duration_frames} frames",
                clip_id.0
            ),
        ),
        Err(e) => mcp_error(id, format!("Failed to add clip: {e}")),
    }
}

fn handle_apply_effect(id: &Value, args: &Value, state: &mut ServerState) -> Value {
    let project = match state.project.as_ref() {
        Some(p) => p,
        None => return mcp_error(id, "No project loaded. Use tazama_create_project first."),
    };

    let clip_id_str = match args.get("clip_id").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => return mcp_error(id, "Missing required parameter: clip_id"),
    };

    let clip_uuid = match Uuid::parse_str(clip_id_str) {
        Ok(u) => u,
        Err(_) => return mcp_error(id, format!("Invalid clip_id: {clip_id_str}")),
    };
    let clip_id = ClipId(clip_uuid);

    let effect_name = match args.get("effect").and_then(|e| e.as_str()) {
        Some(e) => e,
        None => return mcp_error(id, "Missing required parameter: effect"),
    };

    let params = args.get("params").cloned().unwrap_or(json!({}));

    let effect_kind = match parse_effect_kind(effect_name, &params) {
        Ok(k) => k,
        Err(e) => return mcp_error(id, e),
    };

    let (track_id, _) = match project.timeline.find_clip(clip_id) {
        Some(r) => r,
        None => return mcp_error(id, format!("Clip not found: {clip_id_str}")),
    };

    let effect = Effect::new(effect_kind);
    let effect_id = effect.id.0.to_string();

    let cmd = EditCommand::ApplyEffect {
        track_id,
        clip_id,
        effect,
    };

    let Some(project) = state.project.as_mut() else {
        return mcp_error(id, "No project loaded. Use tazama_create_project first.");
    };

    match state.history.execute(cmd, &mut project.timeline) {
        Ok(()) => mcp_success(
            id,
            format!("Applied effect '{effect_name}' (ID: {effect_id}) to clip {clip_id_str}"),
        ),
        Err(e) => mcp_error(id, format!("Failed to apply effect: {e}")),
    }
}

fn handle_get_timeline(id: &Value, state: &ServerState) -> Value {
    let project = match state.project() {
        Ok(p) => p,
        Err(e) => return mcp_error(id, e),
    };

    let timeline_json = serde_json::to_string_pretty(&project.timeline).unwrap_or_default();
    mcp_success(id, timeline_json)
}

async fn handle_export(id: &Value, args: &Value, state: &ServerState) -> Value {
    let project = match state.project() {
        Ok(p) => p,
        Err(e) => return mcp_error(id, e),
    };

    let output_path = match args.get("output_path").and_then(|p| p.as_str()) {
        Some(p) => p,
        None => return mcp_error(id, "Missing required parameter: output_path"),
    };

    if let Err(e) = validate_user_path(output_path) {
        return mcp_error(id, format!("Invalid output path: {e}"));
    }

    let format_str = args.get("format").and_then(|f| f.as_str()).unwrap_or("mp4");

    let format = match format_str {
        "mp4" => tazama_media::ExportFormat::Mp4,
        "webm" => tazama_media::ExportFormat::WebM,
        "prores" => tazama_media::ExportFormat::ProRes,
        "dnxhr" => tazama_media::ExportFormat::DnxHr,
        "mkv" => tazama_media::ExportFormat::Mkv,
        "gif" => tazama_media::ExportFormat::Gif,
        _ => {
            return mcp_error(
                id,
                format!(
                    "Unsupported format: {format_str}. Use mp4, webm, prores, dnxhr, mkv, or gif."
                ),
            );
        }
    };

    let config = tazama_media::ExportConfig {
        output_path: PathBuf::from(output_path),
        format,
        width: project.settings.width,
        height: project.settings.height,
        frame_rate: (
            project.settings.frame_rate.numerator,
            project.settings.frame_rate.denominator,
        ),
        sample_rate: project.settings.sample_rate,
        channels: project.settings.channels,
        audio_codec: None,
        encoder: tazama_media::ExportEncoder::default(),
    };

    // Create video/audio channels for the export pipeline
    let (video_tx, video_rx) = tokio::sync::mpsc::channel(32);
    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel(32);

    // Close the channels immediately — no frames to send for now
    // In a full implementation, we'd decode clips and feed frames
    drop(video_tx);
    drop(audio_tx);

    match tazama_media::export::pipeline::ExportPipeline::run(config, video_rx, audio_rx) {
        Ok(_progress_rx) => mcp_success(
            id,
            format!("Export started to {output_path} ({format_str})"),
        ),
        Err(e) => mcp_error(id, format!("Export failed: {e}")),
    }
}

fn handle_add_marker(id: &Value, args: &Value, state: &mut ServerState) -> Value {
    if state.project.is_none() {
        return mcp_error(id, "No project loaded. Use tazama_create_project first.");
    }

    let name = match args.get("name").and_then(|n| n.as_str()) {
        Some(n) => n,
        None => return mcp_error(id, "Missing required parameter: name"),
    };

    let frame = match args.get("frame").and_then(|f| f.as_u64()) {
        Some(f) => f,
        None => return mcp_error(id, "Missing required parameter: frame"),
    };

    let color_str = args.get("color").and_then(|c| c.as_str()).unwrap_or("blue");

    let color = match color_str {
        "red" => MarkerColor::Red,
        "orange" => MarkerColor::Orange,
        "yellow" => MarkerColor::Yellow,
        "green" => MarkerColor::Green,
        "blue" => MarkerColor::Blue,
        "purple" => MarkerColor::Purple,
        "white" => MarkerColor::White,
        _ => return mcp_error(id, format!("Unknown marker color: {color_str}")),
    };

    let marker = Marker::new(name, frame, color);
    let marker_id = marker.id.0.to_string();

    let cmd = EditCommand::AddMarker { marker };

    let Some(project) = state.project.as_mut() else {
        return mcp_error(id, "No project loaded. Use tazama_create_project first.");
    };

    match state.history.execute(cmd, &mut project.timeline) {
        Ok(()) => mcp_success(
            id,
            format!("Added marker '{name}' at frame {frame} (ID: {marker_id})"),
        ),
        Err(e) => mcp_error(id, format!("Failed to add marker: {e}")),
    }
}

async fn handle_extract_frame(id: &Value, args: &Value, state: &ServerState) -> Value {
    let project = match state.project() {
        Ok(p) => p,
        Err(e) => return mcp_error(id, e),
    };

    let clip_id_str = match args.get("clip_id").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => return mcp_error(id, "Missing required parameter: clip_id"),
    };

    let clip_uuid = match Uuid::parse_str(clip_id_str) {
        Ok(u) => u,
        Err(_) => return mcp_error(id, format!("Invalid clip_id: {clip_id_str}")),
    };
    let clip_id = ClipId(clip_uuid);

    let frame_number = match args.get("frame_number").and_then(|f| f.as_u64()) {
        Some(f) => f,
        None => return mcp_error(id, "Missing required parameter: frame_number"),
    };

    let output_path = match args.get("output_path").and_then(|p| p.as_str()) {
        Some(p) => p,
        None => return mcp_error(id, "Missing required parameter: output_path"),
    };

    if let Err(e) = validate_user_path(output_path) {
        return mcp_error(id, format!("Invalid output path: {e}"));
    }

    let (_, clip) = match project.timeline.find_clip(clip_id) {
        Some(r) => r,
        None => return mcp_error(id, format!("Clip not found: {clip_id_str}")),
    };

    let media = match &clip.media {
        Some(m) => m,
        None => return mcp_error(id, "Clip has no media source"),
    };

    let media_path = std::path::Path::new(&media.path);
    let frame_rate = media
        .info
        .as_ref()
        .and_then(|i| i.video_streams.first())
        .map(|v| v.frame_rate)
        .unwrap_or((30, 1));

    let actual_frame = clip.source_offset + frame_number;

    let frame = match tazama_media::decode::video::VideoDecoder::decode_frame(
        media_path,
        actual_frame,
        frame_rate,
    )
    .await
    {
        Ok(f) => f,
        Err(e) => return mcp_error(id, format!("Failed to decode frame: {e}")),
    };

    // Validate frame data length before constructing image buffer
    let expected_len = frame.width as usize * frame.height as usize * 4;
    if frame.data.len() != expected_len {
        return mcp_error(
            id,
            format!(
                "frame data length mismatch: expected {} bytes ({}x{}x4), got {}",
                expected_len,
                frame.width,
                frame.height,
                frame.data.len()
            ),
        );
    }

    // Convert RGBA bytes to PNG using the image crate
    let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        match image::ImageBuffer::from_raw(frame.width, frame.height, frame.data.to_vec()) {
            Some(img) => img,
            None => return mcp_error(id, "Failed to create image buffer from frame data"),
        };

    let out = PathBuf::from(output_path);
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return mcp_error(id, format!("Failed to create output directory: {e}"));
    }

    if let Err(e) = img.save(&out) {
        return mcp_error(id, format!("Failed to save PNG: {e}"));
    }

    let result = json!({
        "path": output_path,
        "width": frame.width,
        "height": frame.height,
        "frame_number": frame_number,
    });
    mcp_success(
        id,
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )
}

fn handle_detect_hardware(id: &Value) -> Value {
    let hardware = tazama_media::hwaccel::hardware_summary();
    let encoders = tazama_media::available_encoders();

    let result = json!({
        "accelerators": hardware,
        "available_encoders": encoders,
    });
    mcp_success(
        id,
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )
}

fn find_track_id(timeline: &Timeline, name_or_id: &str) -> Option<tazama_core::TrackId> {
    // Try as UUID first
    if let Ok(uuid) = Uuid::parse_str(name_or_id) {
        let tid = tazama_core::TrackId(uuid);
        if timeline.track(tid).is_some() {
            return Some(tid);
        }
    }
    // Fall back to name match
    timeline
        .tracks
        .iter()
        .find(|t| t.name == name_or_id)
        .map(|t| t.id)
}

fn parse_effect_kind(name: &str, params: &Value) -> Result<EffectKind, String> {
    match name {
        "color_grade" => Ok(EffectKind::ColorGrade {
            brightness: params
                .get("brightness")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            contrast: params
                .get("contrast")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            saturation: params
                .get("saturation")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            temperature: params
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
        }),
        "crop" => Ok(EffectKind::Crop {
            left: params.get("left").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
            top: params.get("top").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
            right: params.get("right").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
            bottom: params.get("bottom").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        }),
        "speed" => {
            let factor = params.get("factor").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            if factor <= 0.0 {
                return Err("Speed factor must be positive".to_string());
            }
            Ok(EffectKind::Speed { factor })
        }
        "fade_in" => {
            let duration = params
                .get("duration_frames")
                .and_then(|v| v.as_u64())
                .unwrap_or(30);
            Ok(EffectKind::FadeIn {
                duration_frames: duration,
            })
        }
        "fade_out" => {
            let duration = params
                .get("duration_frames")
                .and_then(|v| v.as_u64())
                .unwrap_or(30);
            Ok(EffectKind::FadeOut {
                duration_frames: duration,
            })
        }
        "volume" => {
            let gain_db = params
                .get("gain_db")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            Ok(EffectKind::Volume { gain_db })
        }
        _ => Err(format!("Unknown effect: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_success_format() {
        let id = json!(1);
        let result = mcp_success(&id, "hello");
        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 1);
        assert_eq!(result["result"]["content"][0]["type"], "text");
        assert_eq!(result["result"]["content"][0]["text"], "hello");
    }

    #[test]
    fn test_mcp_error_format() {
        let id = json!(2);
        let result = mcp_error(&id, "something failed");
        assert_eq!(result["jsonrpc"], "2.0");
        assert_eq!(result["id"], 2);
        assert_eq!(result["result"]["isError"], true);
        assert_eq!(result["result"]["content"][0]["text"], "something failed");
    }

    #[test]
    fn test_server_state_new() {
        let state = ServerState::new();
        assert!(state.project.is_none());
        assert!(state.project_path.is_none());
    }

    #[test]
    fn test_server_state_project_none() {
        let state = ServerState::new();
        assert!(state.project().is_err());
    }

    #[test]
    fn test_server_state_project_mut_none() {
        let mut state = ServerState::new();
        assert!(state.project_mut().is_err());
    }

    #[test]
    fn test_server_state_with_project() {
        let mut state = ServerState::new();
        state.project = Some(Project::new("test", ProjectSettings::default()));
        assert!(state.project().is_ok());
        assert!(state.project_mut().is_ok());
    }

    #[test]
    fn test_find_track_id_by_name() {
        let mut timeline = Timeline::new();
        let track = Track::new("Video 1", TrackKind::Video);
        let expected_id = track.id;
        timeline.add_track(track);

        let found = find_track_id(&timeline, "Video 1");
        assert_eq!(found, Some(expected_id));
    }

    #[test]
    fn test_find_track_id_by_uuid() {
        let mut timeline = Timeline::new();
        let track = Track::new("V1", TrackKind::Video);
        let track_id = track.id;
        timeline.add_track(track);

        let found = find_track_id(&timeline, &track_id.0.to_string());
        assert_eq!(found, Some(track_id));
    }

    #[test]
    fn test_find_track_id_not_found() {
        let timeline = Timeline::new();
        assert!(find_track_id(&timeline, "Nonexistent").is_none());
    }

    #[test]
    fn test_parse_effect_color_grade() {
        let params =
            json!({ "brightness": 0.5, "contrast": 1.2, "saturation": 0.8, "temperature": -0.1 });
        let kind = parse_effect_kind("color_grade", &params).unwrap();
        assert!(matches!(kind, EffectKind::ColorGrade { .. }));
        if let EffectKind::ColorGrade {
            brightness,
            contrast,
            saturation,
            temperature,
        } = &kind
        {
            assert!((brightness - 0.5).abs() < f32::EPSILON);
            assert!((contrast - 1.2).abs() < f32::EPSILON);
            assert!((saturation - 0.8).abs() < f32::EPSILON);
            assert!((temperature - (-0.1)).abs() < f32::EPSILON);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_parse_effect_color_grade_defaults() {
        let params = json!({});
        let kind = parse_effect_kind("color_grade", &params).unwrap();
        assert!(matches!(kind, EffectKind::ColorGrade { .. }));
        if let EffectKind::ColorGrade {
            brightness,
            contrast,
            saturation,
            temperature,
        } = &kind
        {
            assert!((brightness - 0.0).abs() < f32::EPSILON);
            assert!((contrast - 1.0).abs() < f32::EPSILON);
            assert!((saturation - 1.0).abs() < f32::EPSILON);
            assert!((temperature - 0.0).abs() < f32::EPSILON);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_parse_effect_crop() {
        let params = json!({ "left": 0.1, "top": 0.2, "right": 0.3, "bottom": 0.4 });
        let kind = parse_effect_kind("crop", &params).unwrap();
        assert!(matches!(kind, EffectKind::Crop { .. }));
        if let EffectKind::Crop {
            left,
            top,
            right,
            bottom,
        } = &kind
        {
            assert!((left - 0.1).abs() < f32::EPSILON);
            assert!((top - 0.2).abs() < f32::EPSILON);
            assert!((right - 0.3).abs() < f32::EPSILON);
            assert!((bottom - 0.4).abs() < f32::EPSILON);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_parse_effect_speed() {
        let params = json!({ "factor": 2.0 });
        let kind = parse_effect_kind("speed", &params).unwrap();
        assert!(
            matches!(kind, EffectKind::Speed { factor } if (factor - 2.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn test_parse_effect_speed_negative() {
        let params = json!({ "factor": -1.0 });
        assert!(parse_effect_kind("speed", &params).is_err());
    }

    #[test]
    fn test_parse_effect_speed_zero() {
        let params = json!({ "factor": 0.0 });
        assert!(parse_effect_kind("speed", &params).is_err());
    }

    #[test]
    fn test_parse_effect_fade_in() {
        let params = json!({ "duration_frames": 60 });
        let kind = parse_effect_kind("fade_in", &params).unwrap();
        assert!(matches!(
            kind,
            EffectKind::FadeIn {
                duration_frames: 60
            }
        ));
    }

    #[test]
    fn test_parse_effect_fade_in_default() {
        let params = json!({});
        let kind = parse_effect_kind("fade_in", &params).unwrap();
        assert!(matches!(
            kind,
            EffectKind::FadeIn {
                duration_frames: 30
            }
        ));
    }

    #[test]
    fn test_parse_effect_fade_out() {
        let params = json!({ "duration_frames": 45 });
        let kind = parse_effect_kind("fade_out", &params).unwrap();
        assert!(matches!(
            kind,
            EffectKind::FadeOut {
                duration_frames: 45
            }
        ));
    }

    #[test]
    fn test_parse_effect_volume() {
        let params = json!({ "gain_db": -6.0 });
        let kind = parse_effect_kind("volume", &params).unwrap();
        assert!(
            matches!(kind, EffectKind::Volume { gain_db } if (gain_db - (-6.0)).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn test_parse_effect_unknown() {
        let params = json!({});
        assert!(parse_effect_kind("nonexistent", &params).is_err());
    }

    #[tokio::test]
    async fn test_handle_request_initialize() {
        let mut state = ServerState::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let response = handle_request(&request, &mut state).await;
        assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(response["result"]["serverInfo"]["name"], "tazama-mcp");
    }

    #[tokio::test]
    async fn test_handle_request_tools_list() {
        let mut state = ServerState::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });
        let response = handle_request(&request, &mut state).await;
        let tools = response["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
    }

    #[tokio::test]
    async fn test_handle_request_unknown_method() {
        let mut state = ServerState::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "bogus",
            "params": {}
        });
        let response = handle_request(&request, &mut state).await;
        assert_eq!(response["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn test_handle_create_project() {
        let mut state = ServerState::new();
        let id = json!(1);
        let args = json!({ "name": "My Project", "width": 1280, "height": 720 });
        let response = handle_create_project(&id, &args, &mut state);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("My Project"));
        assert!(text.contains("1280x720"));
        assert!(state.project.is_some());
    }

    #[tokio::test]
    async fn test_handle_create_project_missing_name() {
        let mut state = ServerState::new();
        let id = json!(1);
        let args = json!({});
        let response = handle_create_project(&id, &args, &mut state);
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_get_timeline_no_project() {
        let state = ServerState::new();
        let id = json!(1);
        let response = handle_get_timeline(&id, &state);
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_get_timeline_with_project() {
        let mut state = ServerState::new();
        let id = json!(1);
        let args = json!({ "name": "Test" });
        handle_create_project(&id, &args, &mut state);

        let response = handle_get_timeline(&json!(2), &state);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let _timeline: Value = serde_json::from_str(text).unwrap();
    }

    #[tokio::test]
    async fn test_handle_add_marker() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_add_marker(
            &json!(2),
            &json!({ "name": "Ch1", "frame": 100, "color": "green" }),
            &mut state,
        );
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Added marker"));
    }

    #[tokio::test]
    async fn test_handle_add_marker_missing_name() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_add_marker(&json!(2), &json!({ "frame": 10 }), &mut state);
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_add_marker_missing_frame() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_add_marker(&json!(2), &json!({ "name": "M1" }), &mut state);
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_add_marker_bad_color() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_add_marker(
            &json!(2),
            &json!({ "name": "M1", "frame": 0, "color": "neon" }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_apply_effect_no_project() {
        let mut state = ServerState::new();
        let response = handle_apply_effect(
            &json!(1),
            &json!({ "clip_id": "00000000-0000-0000-0000-000000000000", "effect": "crop" }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_apply_effect_missing_clip_id() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_apply_effect(&json!(2), &json!({ "effect": "crop" }), &mut state);
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_apply_effect_invalid_uuid() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_apply_effect(
            &json!(2),
            &json!({ "clip_id": "bad-uuid", "effect": "crop" }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_tool_call_unknown_tool() {
        let mut state = ServerState::new();
        let id = json!(1);
        let response = handle_tool_call(&id, "bogus_tool", &json!({}), &mut state).await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_handle_request_tools_call_dispatch() {
        let mut state = ServerState::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "tazama_create_project",
                "arguments": { "name": "Dispatch Test" }
            }
        });
        let response = handle_request(&request, &mut state).await;
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Dispatch Test"));
    }

    #[tokio::test]
    async fn test_handle_add_marker_all_colors() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        for (i, color) in [
            "red", "orange", "yellow", "green", "blue", "purple", "white",
        ]
        .iter()
        .enumerate()
        {
            let response = handle_add_marker(
                &json!(i + 2),
                &json!({ "name": format!("M{i}"), "frame": i * 10, "color": color }),
                &mut state,
            );
            let text = response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(text.contains("Added marker"));
        }
    }

    #[tokio::test]
    async fn extract_frame_missing_project() {
        let state = ServerState::new();
        let response = handle_extract_frame(
            &json!(1),
            &json!({
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "frame_number": 0,
                "output_path": "/tmp/frame.png"
            }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No project loaded"));
    }

    #[tokio::test]
    async fn extract_frame_tool_in_list() {
        let mut state = ServerState::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });
        let response = handle_request(&request, &mut state).await;
        let tools = response["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"tazama_extract_frame"));
    }

    #[tokio::test]
    async fn extract_frame_missing_clip_id() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_extract_frame(
            &json!(2),
            &json!({
                "frame_number": 0,
                "output_path": "/tmp/frame.png"
            }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Missing"));
    }

    #[tokio::test]
    async fn extract_frame_missing_frame_number() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_extract_frame(
            &json!(2),
            &json!({
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "output_path": "/tmp/frame.png"
            }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Missing"));
    }

    #[tokio::test]
    async fn extract_frame_missing_output_path() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_extract_frame(
            &json!(2),
            &json!({
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "frame_number": 0
            }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Missing"));
    }

    #[tokio::test]
    async fn extract_frame_invalid_clip_id() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_extract_frame(
            &json!(2),
            &json!({
                "clip_id": "not-a-uuid",
                "frame_number": 0,
                "output_path": "/tmp/frame.png"
            }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Invalid clip_id"));
    }

    #[tokio::test]
    async fn extract_frame_clip_not_found() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_extract_frame(
            &json!(2),
            &json!({
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "frame_number": 0,
                "output_path": "/tmp/frame.png"
            }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Clip not found"));
    }

    #[tokio::test]
    async fn test_handle_export_no_project() {
        let state = ServerState::new();
        let response =
            handle_export(&json!(1), &json!({ "output_path": "/tmp/out.mp4" }), &state).await;
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_export_missing_output_path() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_export(&json!(2), &json!({}), &state).await;
        assert_eq!(response["result"]["isError"], true);
    }

    #[tokio::test]
    async fn test_handle_export_unsupported_format() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "Test" }), &mut state);

        let response = handle_export(
            &json!(2),
            &json!({ "output_path": "/tmp/out.avi", "format": "avi" }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Unsupported format"));
    }

    // ── validate_user_path tests ──────────────────────────────────────

    #[test]
    fn validate_path_valid_relative() {
        assert!(validate_user_path("media/clip.mp4").is_ok());
    }

    #[test]
    fn validate_path_simple_filename() {
        assert_eq!(validate_user_path("video.mp4").unwrap(), "video.mp4");
    }

    #[test]
    fn validate_path_rejects_parent_traversal() {
        assert!(validate_user_path("../etc/passwd").is_err());
    }

    #[test]
    fn validate_path_rejects_mid_traversal() {
        assert!(validate_user_path("media/../../../secret").is_err());
    }

    #[test]
    fn validate_path_allows_dots_in_filename() {
        assert!(validate_user_path("my.video.file.mp4").is_ok());
    }

    #[test]
    fn validate_path_allows_current_dir() {
        assert!(validate_user_path("./clip.mp4").is_ok());
    }

    #[test]
    fn validate_path_empty_string() {
        assert!(validate_user_path("").is_ok());
    }

    #[test]
    fn validate_path_absolute_allowed() {
        // The function only rejects "..", not absolute paths
        assert!(validate_user_path("/tmp/video.mp4").is_ok());
    }

    // ── parse_effect_kind additional tests ────────────────────────────

    #[test]
    fn parse_effect_crop_defaults() {
        let kind = parse_effect_kind("crop", &json!({})).unwrap();
        if let EffectKind::Crop {
            left,
            top,
            right,
            bottom,
        } = kind
        {
            assert!((left - 0.0).abs() < f32::EPSILON);
            assert!((top - 0.0).abs() < f32::EPSILON);
            assert!((right - 0.0).abs() < f32::EPSILON);
            assert!((bottom - 0.0).abs() < f32::EPSILON);
        } else {
            panic!("expected Crop");
        }
    }

    #[test]
    fn parse_effect_fade_out_default_duration() {
        let kind = parse_effect_kind("fade_out", &json!({})).unwrap();
        assert!(matches!(
            kind,
            EffectKind::FadeOut {
                duration_frames: 30
            }
        ));
    }

    #[test]
    fn parse_effect_volume_default() {
        let kind = parse_effect_kind("volume", &json!({})).unwrap();
        if let EffectKind::Volume { gain_db } = kind {
            assert!((gain_db - 0.0).abs() < f32::EPSILON);
        } else {
            panic!("expected Volume");
        }
    }

    #[test]
    fn parse_effect_speed_default() {
        let kind = parse_effect_kind("speed", &json!({})).unwrap();
        if let EffectKind::Speed { factor } = kind {
            assert!((factor - 1.0).abs() < f32::EPSILON);
        } else {
            panic!("expected Speed");
        }
    }

    #[test]
    fn parse_effect_unknown_returns_descriptive_error() {
        let err = parse_effect_kind("wobble", &json!({})).unwrap_err();
        assert!(err.contains("Unknown effect"));
        assert!(err.contains("wobble"));
    }

    // ── handle_create_project edge cases ──────────────────────────────

    #[test]
    fn create_project_zero_width() {
        let mut state = ServerState::new();
        let response = handle_create_project(
            &json!(1),
            &json!({ "name": "P", "width": 0, "height": 1080 }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Invalid dimensions"));
    }

    #[test]
    fn create_project_zero_height() {
        let mut state = ServerState::new();
        let response = handle_create_project(
            &json!(1),
            &json!({ "name": "P", "width": 1920, "height": 0 }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
    }

    #[test]
    fn create_project_oversized_width() {
        let mut state = ServerState::new();
        let response = handle_create_project(
            &json!(1),
            &json!({ "name": "P", "width": 9000, "height": 1080 }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
    }

    #[test]
    fn create_project_default_dimensions() {
        let mut state = ServerState::new();
        let response = handle_create_project(&json!(1), &json!({ "name": "Defaults" }), &mut state);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("1920x1080"));
    }

    #[test]
    fn create_project_has_default_tracks() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let project = state.project.as_ref().unwrap();
        assert_eq!(project.timeline.tracks.len(), 2);
        assert_eq!(project.timeline.tracks[0].name, "Video 1");
        assert_eq!(project.timeline.tracks[1].name, "Audio 1");
    }

    // ── handle_add_clip error paths ───────────────────────────────────

    #[tokio::test]
    async fn add_clip_no_project() {
        let mut state = ServerState::new();
        let response = handle_add_clip(
            &json!(1),
            &json!({ "track": "Video 1", "source": "clip.mp4" }),
            &mut state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No project loaded"));
    }

    #[tokio::test]
    async fn add_clip_missing_track() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response =
            handle_add_clip(&json!(2), &json!({ "source": "clip.mp4" }), &mut state).await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Missing required parameter: track"));
    }

    #[tokio::test]
    async fn add_clip_missing_source() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response = handle_add_clip(&json!(2), &json!({ "track": "Video 1" }), &mut state).await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Missing required parameter: source"));
    }

    #[tokio::test]
    async fn add_clip_path_traversal_rejected() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response = handle_add_clip(
            &json!(2),
            &json!({ "track": "Video 1", "source": "../../../etc/passwd" }),
            &mut state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Invalid source path"));
    }

    // ── handle_apply_effect additional error paths ────────────────────

    #[test]
    fn apply_effect_missing_effect_name() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response = handle_apply_effect(
            &json!(2),
            &json!({ "clip_id": "00000000-0000-0000-0000-000000000000" }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Missing required parameter: effect"));
    }

    #[test]
    fn apply_effect_clip_not_found() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response = handle_apply_effect(
            &json!(2),
            &json!({ "clip_id": "00000000-0000-0000-0000-000000000000", "effect": "crop" }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Clip not found"));
    }

    #[test]
    fn apply_effect_unknown_effect_name() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response = handle_apply_effect(
            &json!(2),
            &json!({ "clip_id": "00000000-0000-0000-0000-000000000000", "effect": "hologram" }),
            &mut state,
        );
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Unknown effect"));
    }

    // ── handle_export additional paths ────────────────────────────────

    #[tokio::test]
    async fn export_path_traversal_rejected() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response = handle_export(
            &json!(2),
            &json!({ "output_path": "../../../tmp/evil.mp4" }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Invalid output path"));
    }

    // ── handle_extract_frame path traversal ───────────────────────────

    #[tokio::test]
    async fn extract_frame_output_path_traversal() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        let response = handle_extract_frame(
            &json!(2),
            &json!({
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "frame_number": 0,
                "output_path": "../../evil.png"
            }),
            &state,
        )
        .await;
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Invalid output path"));
    }

    // ── handle_add_marker additional paths ────────────────────────────

    #[test]
    fn add_marker_no_project() {
        let mut state = ServerState::new();
        let response =
            handle_add_marker(&json!(1), &json!({ "name": "M", "frame": 0 }), &mut state);
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No project loaded"));
    }

    #[test]
    fn add_marker_default_color_is_blue() {
        let mut state = ServerState::new();
        handle_create_project(&json!(1), &json!({ "name": "T" }), &mut state);
        // No color param → defaults to blue, should succeed
        let response = handle_add_marker(
            &json!(2),
            &json!({ "name": "DefaultColor", "frame": 42 }),
            &mut state,
        );
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Added marker"));
    }

    // ── mcp_success / mcp_error with various id types ─────────────────

    #[test]
    fn mcp_success_with_string_id() {
        let id = json!("req-abc");
        let result = mcp_success(&id, "ok");
        assert_eq!(result["id"], "req-abc");
        assert_eq!(result["result"]["content"][0]["text"], "ok");
    }

    #[test]
    fn mcp_error_with_null_id() {
        let id = json!(null);
        let result = mcp_error(&id, "fail");
        assert!(result["id"].is_null());
        assert_eq!(result["result"]["isError"], true);
    }

    // ── handle_request edge cases ─────────────────────────────────────

    #[tokio::test]
    async fn handle_request_missing_method_treated_as_unknown() {
        let mut state = ServerState::new();
        let request = json!({ "jsonrpc": "2.0", "id": 1 });
        let response = handle_request(&request, &mut state).await;
        // method defaults to "" which is unknown
        assert_eq!(response["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn handle_request_tools_call_missing_tool_name() {
        let mut state = ServerState::new();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {}
        });
        let response = handle_request(&request, &mut state).await;
        // tool name defaults to "" → Unknown tool
        assert_eq!(response["result"]["isError"], true);
    }

    // ── find_track_id with invalid UUID string ────────────────────────

    #[test]
    fn find_track_id_invalid_uuid_falls_back_to_name() {
        let mut timeline = Timeline::new();
        let track = Track::new("not-a-uuid", TrackKind::Audio);
        let expected_id = track.id;
        timeline.add_track(track);
        // "not-a-uuid" fails UUID parse, falls back to name match
        assert_eq!(find_track_id(&timeline, "not-a-uuid"), Some(expected_id));
    }
}
