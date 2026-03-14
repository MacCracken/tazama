# Tazama

*Swahili: to watch, to observe*

AI-native non-linear video editor built with Rust, Vulkan, and GStreamer.

## Features

- Multi-track video and audio timeline with clip splitting, trimming, and overlap detection
- GPU-accelerated rendering via Vulkan compute shaders (color grading, crop, transitions)
- Real-time preview with source frame decoding
- Export to MP4 (H.264/AAC) and WebM (VP9/Opus)
- Multi-track audio mixing with per-clip volume control
- Undo/redo history for all editing operations
- MCP server for AI agent integration (6 tools + 6 agnoshi intents)
- Keyboard-driven NLE workflow (J/K/L shuttle, I/O loop points, razor/slip tools)

## Architecture

| Crate | Purpose |
|-------|---------|
| `tazama-core` | Timeline model, clips, effects, undo/redo — pure logic, no I/O |
| `tazama-media` | GStreamer media pipeline (probe, decode, encode, thumbnails, waveforms, audio mixing) |
| `tazama-storage` | Project persistence (SQLite + JSON), media asset management |
| `tazama-gpu` | Vulkan compute pipelines for rendering and effects (6 shaders) |
| `tazama` (app) | Tauri v2 desktop shell (7 IPC commands) |
| `tazama-mcp` | MCP server (6 tools for Claude / AI agent integration) |

**Frontend:** React 19 + TypeScript + Vite 6 + Tailwind CSS v4 + Zustand 5

```
┌──────────────────────────────────────────────┐
│  Toolbar (file, edit tools, transport, time)  │
├──────────┬──────────────────┬────────────────┤
│  Media   │     Preview      │   Inspector    │
│  Browser │     Monitor      │   (clip/track) │
├──────────┴──────────────────┴────────────────┤
│  Timeline (multi-track, clips, playhead)      │
└──────────────────────────────────────────────┘
```

## Prerequisites

- Rust 1.85+ (edition 2024)
- Node.js 20+ and npm
- GStreamer 1.24+ with plugins: base, good, bad, ugly, libav
- Vulkan SDK or lavapipe (software fallback)
- System packages: `libgstreamer1.0-dev`, `libvulkan-dev`, `libasound2-dev` (or equivalents)

## Quick Start

```bash
# Install system dependencies and Rust tools
./scripts/setup-dev.sh

# Build everything (backend + frontend)
make build

# Run the Tauri dev server (hot-reload)
make run
```

## Testing

```bash
make test              # All tests (143 unit + integration)
make test-unit         # Unit tests only
make test-mcp          # MCP integration tests
make test-coverage     # Coverage report (tarpaulin)
make check             # Full quality check (fmt + clippy + test)
```

**Coverage:** Core crate averages 97%. GPU/media/app crates require runtime services (Vulkan, GStreamer, Tauri) and are covered by integration tests.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Space` | Play/pause |
| `J` / `K` / `L` | Shuttle reverse / pause / forward |
| `I` / `O` | Set loop in/out points |
| `Arrow Left/Right` | Step frame back/forward |
| `V` / `B` / `S` | Select / razor / slip tool |
| `Delete` | Remove selected clip |
| `+` / `-` | Zoom timeline |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Undo / redo |
| `Ctrl+S` / `Ctrl+N` / `Ctrl+O` | Save / new / open project |
| `Ctrl+E` | Export |

## MCP Tools

Tazama exposes an MCP server for AI agent integration via stdio JSON-RPC:

| Tool | Description |
|------|-------------|
| `tazama_create_project` | Create a new video project with default tracks |
| `tazama_add_clip` | Add a clip to the timeline (auto-probes media) |
| `tazama_apply_effect` | Apply an effect to a clip (color grade, crop, speed, etc.) |
| `tazama_get_timeline` | Get current timeline state as JSON |
| `tazama_export` | Export project to MP4/WebM |
| `tazama_add_marker` | Add a named marker at a frame position |

AGNOS marketplace integration via `.agnos-agent/manifest.toml` with 6 intents.

## Effects

| Effect | Type | Parameters |
|--------|------|------------|
| Color Grade | Video | brightness, contrast, saturation, temperature |
| Crop | Video | left, top, right, bottom (0.0–1.0) |
| Speed | Video | factor (e.g., 2.0 = 2x speed) |
| Dissolve | Transition | duration_frames |
| Wipe | Transition | duration_frames |
| Fade | Transition | duration_frames |
| Fade In/Out | Audio | duration_frames |
| Volume | Audio | gain_db |

## Documentation

- [Development Roadmap](docs/development/roadmap.md)
- [GPU Development Guide](docs/development/gpu-guide.md)
- [ADR-001: Rust + Vulkan + GStreamer](docs/adr/001-rust-vulkan-gstreamer.md)
- [ADR-002: GPU Compute Pipeline](docs/adr/002-gpu-compute-pipeline.md)
- [ADR-003: Export & Audio Mixing](docs/adr/003-export-audio-mixing.md)
- [Contributing](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## License

AGPL-3.0 — see [LICENSE](LICENSE)
