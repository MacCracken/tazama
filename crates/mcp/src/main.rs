use anyhow::Result;
use serde_json::{Value, json};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

/// MCP tools exposed by Tazama:
///
/// 1. tazama_create_project  — Create a new video project
/// 2. tazama_add_clip        — Add a clip to the timeline
/// 3. tazama_apply_effect    — Apply an effect to a clip
/// 4. tazama_get_timeline    — Get the current timeline state
/// 5. tazama_export          — Export the project to a video file

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tazama_mcp=debug")
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("tazama-mcp server starting (stdio)");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

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

        let response = handle_request(&request).await;
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        stdout.write_all(out.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(request: &Value) -> Value {
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
                                "format": { "type": "string", "enum": ["mp4", "webm", "mov"], "default": "mp4" }
                            },
                            "required": ["output_path"]
                        }
                    }
                ]
            }
        }),
        "tools/call" => {
            // TODO: dispatch to actual tool implementations
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{
                        "type": "text",
                        "text": "Tool execution not yet implemented"
                    }]
                }
            })
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
