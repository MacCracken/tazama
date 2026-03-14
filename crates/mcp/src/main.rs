use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Value, json};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

use tazama_core::{
    Clip, ClipId, ClipKind, EditCommand, EditHistory, Effect, EffectKind, Marker, MarkerColor,
    MediaRef, Project, ProjectSettings, Timeline, Track, TrackKind,
};

/// MCP tools exposed by Tazama:
///
/// 1. tazama_create_project  — Create a new video project
/// 2. tazama_add_clip        — Add a clip to the timeline
/// 3. tazama_apply_effect    — Apply an effect to a clip
/// 4. tazama_get_timeline    — Get the current timeline state
/// 5. tazama_export          — Export the project to a video file
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
    tracing_subscriber::fmt()
        .with_env_filter("tazama_mcp=debug")
        .with_writer(std::io::stderr)
        .init();

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

    let format_str = args.get("format").and_then(|f| f.as_str()).unwrap_or("mp4");

    let format = match format_str {
        "mp4" => tazama_media::ExportFormat::Mp4,
        "webm" => tazama_media::ExportFormat::WebM,
        _ => {
            return mcp_error(
                id,
                format!("Unsupported format: {format_str}. Use mp4 or webm."),
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
