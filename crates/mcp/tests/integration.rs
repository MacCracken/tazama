use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// Helper: spawn the tazama-mcp binary, send a JSON-RPC request, read back the response.
struct McpProcess {
    child: std::process::Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl McpProcess {
    fn start() -> Self {
        let bin = env!("CARGO_BIN_EXE_tazama-mcp");
        let child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start tazama-mcp");

        // Verify stdout is available, then take ownership
        assert!(child.stdout.is_some(), "stdout not captured");
        let mut child = child;
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        Self { child, reader }
    }

    fn send(&mut self, request: &Value) -> Value {
        let stdin = self.child.stdin.as_mut().expect("stdin not available");
        let mut line = serde_json::to_string(request).unwrap();
        line.push('\n');
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.flush().unwrap();

        let mut response_line = String::new();
        self.reader.read_line(&mut response_line).unwrap();
        serde_json::from_str(response_line.trim()).expect("invalid JSON response")
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn test_initialize() {
    let mut mcp = McpProcess::start();
    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(response["result"]["serverInfo"]["name"], "tazama-mcp");
}

#[test]
fn test_tools_list() {
    let mut mcp = McpProcess::start();

    // Initialize first
    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));

    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 6);

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"tazama_create_project"));
    assert!(tool_names.contains(&"tazama_add_clip"));
    assert!(tool_names.contains(&"tazama_apply_effect"));
    assert!(tool_names.contains(&"tazama_get_timeline"));
    assert!(tool_names.contains(&"tazama_export"));
    assert!(tool_names.contains(&"tazama_add_marker"));
}

#[test]
fn test_create_project() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test Project" }
        }
    }));

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Created project 'Test Project'"));
    assert!(text.contains("1920x1080"));
}

#[test]
fn test_get_timeline_empty() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    // Create project first
    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Empty Project" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_get_timeline",
            "arguments": {}
        }
    }));

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let timeline: Value = serde_json::from_str(text).unwrap();
    let tracks = timeline["tracks"].as_array().unwrap();
    assert_eq!(tracks.len(), 2); // default video + audio tracks
    assert_eq!(tracks[0]["clips"].as_array().unwrap().len(), 0);
    assert_eq!(tracks[1]["clips"].as_array().unwrap().len(), 0);
}

#[test]
fn test_add_marker() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Marker Test" }
        }
    }));

    // Add a marker
    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_marker",
            "arguments": {
                "name": "Chapter 1",
                "frame": 150,
                "color": "red"
            }
        }
    }));

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Added marker 'Chapter 1'"));
    assert!(text.contains("frame 150"));

    // Verify marker appears in timeline
    let timeline_response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "tazama_get_timeline",
            "arguments": {}
        }
    }));

    let timeline_text = timeline_response["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let timeline: Value = serde_json::from_str(timeline_text).unwrap();
    let markers = timeline["markers"].as_array().unwrap();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0]["name"], "Chapter 1");
    assert_eq!(markers[0]["frame"], 150);
    assert_eq!(markers[0]["color"], "Red");
}

#[test]
fn test_apply_effect_no_project() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_apply_effect",
            "arguments": {
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "effect": "color_grade"
            }
        }
    }));

    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn test_unknown_method() {
    let mut mcp = McpProcess::start();

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "nonexistent/method",
        "params": {}
    }));

    assert_eq!(response["error"]["code"], -32601);
}

#[test]
fn test_create_project_custom_resolution() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "4K Project", "width": 3840, "height": 2160 }
        }
    }));

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("3840x2160"));
}

#[test]
fn test_create_project_missing_name() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": {}
        }
    }));

    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("name"));
}

#[test]
fn test_get_timeline_no_project() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_get_timeline",
            "arguments": {}
        }
    }));

    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("No project loaded"));
}

#[test]
fn test_unknown_tool() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",
            "arguments": {}
        }
    }));

    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Unknown tool"));
}

#[test]
fn test_add_marker_missing_params() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    // Missing name
    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_marker",
            "arguments": { "frame": 100 }
        }
    }));
    assert_eq!(response["result"]["isError"], true);

    // Missing frame
    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_marker",
            "arguments": { "name": "test" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn test_add_marker_unknown_color() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_marker",
            "arguments": { "name": "M1", "frame": 10, "color": "pink" }
        }
    }));

    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Unknown marker color"));
}

#[test]
fn test_add_marker_all_colors() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Colors" }
        }
    }));

    for (i, color) in ["orange", "yellow", "green", "blue", "purple", "white"].iter().enumerate() {
        let response = mcp.send(&json!({
            "jsonrpc": "2.0",
            "id": i + 3,
            "method": "tools/call",
            "params": {
                "name": "tazama_add_marker",
                "arguments": { "name": format!("M{}", i), "frame": i * 10, "color": color }
            }
        }));
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Added marker"));
    }
}

#[test]
fn test_apply_effect_missing_clip_id() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_apply_effect",
            "arguments": { "effect": "color_grade" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("clip_id"));
}

#[test]
fn test_apply_effect_invalid_uuid() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_apply_effect",
            "arguments": { "clip_id": "not-a-uuid", "effect": "crop" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Invalid clip_id"));
}

#[test]
fn test_apply_effect_unknown_effect() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_apply_effect",
            "arguments": {
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "effect": "nonexistent_effect"
            }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Unknown effect"));
}

#[test]
fn test_apply_effect_clip_not_found() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_apply_effect",
            "arguments": {
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "effect": "crop"
            }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Clip not found"));
}

#[test]
fn test_apply_effect_missing_effect_param() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_apply_effect",
            "arguments": { "clip_id": "00000000-0000-0000-0000-000000000000" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("effect"));
}

#[test]
fn test_add_clip_no_project() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_clip",
            "arguments": { "track": "V1", "source": "/tmp/test.mp4" }
        }
    }));

    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn test_add_clip_missing_params() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    // Missing track
    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_clip",
            "arguments": { "source": "/tmp/test.mp4" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);

    // Missing source
    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_clip",
            "arguments": { "track": "Video 1" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn test_add_clip_track_not_found() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_clip",
            "arguments": { "track": "NonexistentTrack", "source": "/tmp/test.mp4" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Track not found"));
}

#[test]
fn test_export_no_project() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_export",
            "arguments": { "output_path": "/tmp/out.mp4" }
        }
    }));

    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn test_export_missing_output_path() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_export",
            "arguments": {}
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("output_path"));
}

#[test]
fn test_export_unsupported_format() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_export",
            "arguments": { "output_path": "/tmp/out.avi", "format": "avi" }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Unsupported format"));
}

#[test]
fn test_add_marker_no_project() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_add_marker",
            "arguments": { "name": "M1", "frame": 10 }
        }
    }));

    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn test_speed_effect_negative_factor() {
    let mut mcp = McpProcess::start();

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }));

    mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "tazama_create_project",
            "arguments": { "name": "Test" }
        }
    }));

    let response = mcp.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tazama_apply_effect",
            "arguments": {
                "clip_id": "00000000-0000-0000-0000-000000000000",
                "effect": "speed",
                "params": { "factor": -1.0 }
            }
        }
    }));
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Speed factor must be positive"));
}
