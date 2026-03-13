# Tazama

*Swahili: to watch, to observe*

AI-native non-linear video editor built with Rust + Vulkan.

## Architecture

| Crate | Purpose |
|-------|---------|
| `tazama-core` | Timeline model, clips, effects — pure logic, no I/O |
| `tazama-storage` | Project persistence, media asset management (SQLite) |
| `tazama-gpu` | Vulkan compute pipelines for rendering and effects |
| `tazama` (app) | Tauri v2 desktop shell |
| `tazama-mcp` | MCP server (5 tools for Claude integration) |

## Tech Stack

- **Language**: Rust (edition 2024)
- **GUI**: Tauri v2 + React/TypeScript
- **Media**: GStreamer (decode/encode/pipeline)
- **GPU**: Vulkan via ash (compute shaders for effects, color grading, compositing)
- **Audio**: PipeWire
- **Storage**: SQLite (sqlx) + JSON project files

## Quick Start

```bash
./scripts/setup-dev.sh
make build
make run
```

## AI Features (planned)

- Auto-cut / scene detection
- AI voiceover / TTS
- Subtitle generation
- B-roll suggestions
- Style transfer
- AI color grading
- Smart transitions

## MCP Tools

| Tool | Description |
|------|-------------|
| `tazama_create_project` | Create a new video project |
| `tazama_add_clip` | Add a clip to the timeline |
| `tazama_apply_effect` | Apply an effect to a clip |
| `tazama_get_timeline` | Get current timeline state |
| `tazama_export` | Export project to video file |
