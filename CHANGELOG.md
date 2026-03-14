# Changelog

## 2026.3.13

### Phase 3 — GPU Rendering

#### Vulkan Compute Pipelines (`tazama-gpu`)
- Vulkan context initialization via `ash::Entry::load()` with runtime device detection
- `gpu-allocator` integration for buffer memory management (CpuToGpu, GpuOnly, GpuToCpu)
- 6 pre-compiled GLSL compute shaders: color_grade, composite, crop, dissolve, wipe, fade
- `PipelineCache` with per-effect compute pipelines and descriptor pool
- `GpuBuffer` abstraction for staging uploads, compute intermediates, and readback
- `ShaderModule` loader with `include_bytes!()` for embedded SPIR-V

#### Renderer (`tazama-gpu`)
- Multi-track timeline compositing with alpha-over blending and per-clip opacity
- Sequential effect chain: ColorGrade → Crop (skips audio effects and Speed)
- Transition support: dissolve, wipe, fade between adjacent clips
- Speed factor extraction from clip effects for variable playback rate
- Transparent black frame for empty timeline regions

#### Preview & Export (`tazama-gpu`)
- `PreviewLoop` — tokio task rendering at project frame rate with frame dropping
- `render_all_frames()` — sequential frame rendering for export pipeline
- `FrameSource` trait for decoupling from media decoder
- `GpuFrame` type for decoded RGBA frames

#### Infrastructure
- `scripts/compile_shaders.sh` — GLSL → SPIR-V compilation via `glslangValidator`
- `make compile-shaders` Makefile target
- Software fallback via lavapipe (`VK_ICD_FILENAMES` env var)
- ADR-002: GPU compute pipeline architecture decisions
- GPU development guide (shader workflow, testing, debugging)

#### Tests
- 7 unit tests (clip collection, frame indexing, speed factor, muted tracks, buffer sizing)

### Phase 2 — Functional Editing Backend

#### Clip Operations (`tazama-core`)
- Clip trim with source media bounds validation
- Clip split at timeline frame (correct source offset math, new ClipId)
- Clip duplicate (deep clone with new ID)
- Overlap detection in `Track::add_clip` (was TODO)
- Track-level mutations: `move_clip`, `split_clip`, `trim_clip`, `duplicate_clip`
- Locked track enforcement on all mutations
- `Timeline::find_clip` / `find_clip_mut` for cross-track clip lookup
- New error variants: `InvalidSplitPoint`, `InvalidTrim`, `TrackLocked`

#### Undo/Redo System (`tazama-core`)
- `EditCommand` enum with 9 variants (AddClip, RemoveClip, MoveClip, TrimClip, SplitClip, AddTrack, RemoveTrack, ApplyEffect, RemoveEffect)
- Each command stores enough state for both `apply()` and `undo()`
- `EditHistory` with undo/redo stacks; new actions clear redo stack

#### Playback Position (`tazama-core`)
- `PlaybackState` enum (Stopped, Playing, Paused)
- `PlaybackPosition` with frame tracking, seek, and advance with loop region wrapping

#### SQLite Persistence (`tazama-storage`)
- Initial migration: `media_cache` and `projects` tables
- `Database::get_cached_media_info` / `cache_media_info` — invalidates on file size/mtime change
- `Database::save_project` / `load_project` / `list_projects` — full project JSON round-trip
- All queries use runtime `sqlx::query()` (no compile-time DATABASE_URL needed)

#### MCP Tool Dispatch (`tazama-mcp`)
- Stateful `ServerState` holding project + edit history
- `tazama_create_project` — creates project with default video/audio tracks
- `tazama_add_clip` — probes media via GStreamer, creates clip + MediaRef, applies via EditHistory
- `tazama_apply_effect` — parses effect kind/params, applies via EditHistory
- `tazama_get_timeline` — serializes timeline to JSON
- `tazama_export` — builds ExportConfig from project settings, runs GStreamer export pipeline
- GStreamer initialized once at startup

#### Tests
- 20 tests in `tazama-core` (clip ops, overlap, split math, move rejection, trim bounds, locked tracks, undo/redo cycles, playback)
- 4 tests in `tazama-storage` (in-memory SQLite: cache round-trip, project round-trip, list, missing project error)

### Phase 1 — Media Pipeline

- GStreamer probe/inspection (duration, resolution, codec, frame rate)
- Video decode pipeline (H.264, H.265, VP9, AV1 → raw RGBA frames)
- Audio decode pipeline (AAC, Opus, FLAC, MP3 → raw PCM F32)
- Thumbnail generation (keyframe extraction at intervals)
- Audio waveform extraction (min/max peaks per channel)
- Export pipeline (raw frames → encode → mux → MP4/WebM)
- Core type serde round-trip tests

### Phase 0 — Scaffold

- Initial project scaffold
- Core types: Project, Timeline, Track, Clip, Effect
- Storage layer with SQLite and media import
- GPU crate stubs (Vulkan via ash)
- Tauri v2 app shell with basic commands
- MCP server with 5 tool definitions
