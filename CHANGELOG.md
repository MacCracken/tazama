# Changelog

## 2026.3.13

### Phase 5 — MCP & AGNOS Integration

#### Core Types (`tazama-core`)
- `Marker` type with `MarkerId`, `MarkerColor` (Red/Orange/Yellow/Green/Blue/Purple/White)
- `Timeline::markers` field with `add_marker()`, `remove_marker()`, `markers_in_range()` methods
- `Track::solo` and `Track::visible` fields (default false/true)
- `Timeline::audible_tracks()` and `visible_video_tracks()` helpers (solo/mute logic)
- `EditCommand::AddMarker` and `RemoveMarker` variants with full undo/redo support

#### Audio Preview (`tazama-media`)
- CPAL-based `AudioPreview` for real-time audio playback via PipeWire/ALSA
- `VecDeque<f32>` ring buffer behind `Arc<Mutex<>>` for preview (non-RT-critical)
- `feed()`, `seek()`, `set_playing()` API for preview loop integration

#### GPU Renderer (`tazama-gpu`)
- `collect_active_clips()` respects `!track.visible` and solo logic
- `apply_transitions()` respects visible and solo flags
- `PreviewLoop::start()` accepts optional `Arc<AudioPreview>` for audio output

#### MCP Server (`tazama-mcp`)
- `tazama_add_marker` tool — add named markers at frame positions with color
- 6 tools total (was 5)
- MCP integration test suite (7 tests): initialize, tools/list, create_project, get_timeline, add_marker, apply_effect_no_project, unknown_method

#### AGNOS & Marketplace
- `.agnos-agent/manifest.toml` with 5 agnoshi intents for AI tool discovery
- `recipes/marketplace/tazama.toml` — ark package recipe with sandbox rules

#### Frontend (`ui/`)
- `Marker` and `MarkerColor` TypeScript types
- `Timeline.markers`, `Track.solo`, `Track.visible` fields
- `addMarker`, `removeMarker`, `toggleTrackSolo`, `toggleTrackVisible` store actions
- Solo (S) and visible (eye) buttons in TrackHeader
- Colored triangle marker indicators on TimelineRuler

### Phase 4 — Desktop UI

#### Frontend Scaffold
- React 19 + Vite 6 + TypeScript 5 + Tailwind CSS v4 frontend at `ui/`
- Tauri v2 config with 1440x900 window, dev server integration
- Dark theme with 25+ CSS custom properties (--bg-*, --text-*, --clip-*, etc.)
- Zustand 5 state management with Immer for immutable updates

#### TypeScript Types & IPC (`ui/src/types/`, `ui/src/ipc/`)
- Full TypeScript mirror of all Rust core types (Project, Timeline, Track, Clip, Effect, MediaInfo)
- EffectKind as externally-tagged discriminated union matching Rust serde output
- Typed `invoke()` wrappers for all 6 Tauri commands

#### State Management (`ui/src/stores/`)
- `projectStore` — project lifecycle, track/clip/effect CRUD with automatic undo history
- `historyStore` — snapshot-based undo/redo (max 100 entries, structuredClone)
- `playbackStore` — transport controls, shuttle speed, loop regions
- `uiStore` — selection, zoom/scroll, active tool, panel sizes, dialog/toast state

#### App Shell & Layout (`ui/src/components/layout/`)
- CSS Grid layout: toolbar (40px) + three-panel editor + timeline
- Media browser (240px) | preview monitor | inspector (280px)
- Draggable panel resizers

#### Toolbar (`ui/src/components/toolbar/`)
- File actions: new, open, save, export with Tauri dialog plugin
- Edit tools: select (V), razor (B), slip (S) with inline SVG icons
- Transport controls: play/pause, stop, step forward/back
- Timecode display: HH:MM:SS:FF from frame position + project frame rate

#### Timeline Panel (`ui/src/components/timeline/`)
- DOM-based timeline with absolutely-positioned clip blocks
- TimelineRuler with tick marks and click-to-seek
- TrackRow with header (name, kind badge, mute/lock buttons) + clip lane
- ClipBlock colored by kind (video=blue, audio=green, image=purple, title=amber)
- Playhead (red vertical line with triangle marker)
- Drag-to-move clips, edge-handle trimming, razor tool splitting
- Ctrl+scroll zoom (0.1–10 px/frame), horizontal scroll

#### Media Browser (`ui/src/components/media/`)
- Import button with file dialog (video, audio, image formats)
- Media asset list with drag-and-drop to timeline
- GStreamer probe for imported file metadata

#### Inspector Panel (`ui/src/components/inspector/`)
- Clip inspector: name, position, duration, kind, opacity/volume sliders
- Effect list with add/remove, per-EffectKind parameter display
- Track inspector: name, kind, muted/locked toggles, remove button
- Context-aware: shows clip, track, or empty state based on selection

#### Preview Monitor (`ui/src/components/preview/`)
- 16:9 aspect-ratio container with canvas element
- Timecode overlay (M:SS:FF format)

#### Project Dialogs (`ui/src/components/project/`)
- Welcome screen with new project / open project / recent projects
- New project dialog with resolution presets (1080p, 4K, 720p, square, vertical)

#### Export (`ui/src/components/export/`)
- Export dialog with format selection (MP4/WebM) and resolution display
- Progress bar with Tauri event listener for `export-progress` events

#### Shared Components (`ui/src/components/shared/`)
- Modal with ESC-to-close and backdrop click
- Toast notifications (error/success/info, auto-dismiss 4s)
- ErrorBoundary with recovery button
- Slider and NumberInput controls

#### Keyboard Shortcuts (`ui/src/hooks/`)
- Space: play/pause, J/K/L: shuttle control, I/O: loop points
- Arrow keys: step frame, Delete: remove selected clip
- Ctrl+Z/Ctrl+Shift+Z: undo/redo, Ctrl+S: save, Ctrl+N: new, Ctrl+E: export
- B: razor, V: select, S: slip, +/-: zoom timeline
- Input-field-aware (shortcuts disabled when typing)

#### Rust Backend (`tazama`)
- `probe_media` command — GStreamer media probe via IPC
- `export_project` command — export pipeline with progress events
- `tazama-media` dependency added to app crate

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
