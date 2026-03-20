# Changelog

## 2026.3.20

### Tarang 0.20.3 & ai-hwaccel 0.20.3 Integration

#### Phase 1: Native MKV/MP4 Muxing
- **Replaced custom EBML muxer** (~200 lines) with tarang's `MkvMuxer` and `Mp4Muxer`
- **MP4 export now native** via tarang — no longer falls back to GStreamer
- `ExportMuxer` trait abstraction unifies MKV and MP4 write paths
- Tests updated from EBML byte-level to muxer integration tests

#### Phase 2: Pixel Format Conversion
- `convert.rs` — switched from `tarang::ai::yuv420p_to_rgb24` to `tarang::video::convert::yuv420p_to_rgb24` (proper module in 0.20.3)
- `tarang_pipeline.rs` — replaced 35 lines of manual BT.601 `rgba_to_yuv420p()` with tarang's `rgb24_to_yuv420p`

#### Phase 3: Video Scaling for Thumbnails
- Thumbnails now scale to `spec.width`/`spec.height` via `tarang::video::scale::scale_frame` (Lanczos3 filter)
- Previously returned native resolution regardless of spec

#### Phase 4: Loudness Measurement & Normalization
- New `EffectKind::LoudnessNormalize { target_lufs }` effect variant
- `crates/media/src/loudness.rs` — `measure_loudness()` and `normalize_audio()` wrapping tarang's loudness API
- Wired into `apply_clip_effects()` audio effect chain
- `measure_loudness` Tauri IPC command for per-clip LUFS measurement
- LoudnessMeter UI component in ClipInspector (click to measure, color-coded LUFS display)

#### Phase 5: GPU Monitoring Data
- `HardwareInfo` extended with `memory_used_bytes`, `memory_free_bytes`, `temperature_c`, `gpu_utilization_percent`
- Populated from `ai_hwaccel::AcceleratorProfile` fields
- Hardware panel in ExportDialog shows GPU temp, utilization, and free memory

#### Phase 6: Content-Based Thumbnails
- `ThumbnailStrategy` enum (`SceneBased` | `ContentBased`) with `#[serde(default)]` backward compat
- Scene-based uses existing `SceneDetector`; content-based uses tarang's `ThumbnailGenerator`
- Strategy toggle in MediaBrowser UI

### UI Enhancements

#### Interactive Effect Editors
- All 15 effect types now have slider/input controls (was read-only text display)
- `Param` component with label, range slider, numeric readout, and suffix
- `updateEffect` store action for live parameter updates
- Undo debouncing: `_mutateSilent` + `_pushUndo` — slider drags produce exactly one undo entry instead of hundreds

#### Effect Preset Menu
- "Add Effect" button now opens a dropdown with 12 presets: ColorGrade, Volume, FadeIn/Out, Speed, EQ, Compressor, NoiseReduction, Reverb, LoudnessNormalize, Crop, Transform

#### Keyframe Animation UI
- `KeyframeEditor` wired into EffectList below each effect's parameter controls
- "K" toggle per parameter to enable/disable animation
- "+" button adds keyframe at current playback position
- Expandable keyframe list with frame/value display and delete
- Supports all animatable params including LoudnessNormalize and FadeIn/Out

#### Mixer Panel
- Restyled with vertical volume faders, horizontal pan sliders, M/S buttons per track
- Integrated into AppShell as collapsible 280px panel alongside timeline
- "Mixer" toggle button in Toolbar

#### Audio Waveform Visualization
- `extract_waveform` Tauri IPC command wrapping existing Rust backend
- ClipBlock auto-loads waveform on mount (cached per media path)
- Canvas-based waveform overlay with semi-transparent white peaks
- ResizeObserver for proper height tracking on zoom changes

#### Thumbnail Generation Pipeline
- `generate_thumbnails` Tauri IPC command — returns base64-encoded thumbnail data
- MediaItem auto-generates thumbnail preview on first render
- Module-level cache persists across re-renders; regenerates on strategy change

#### Proxy Workflow UI
- "Generate Proxies" button in MediaBrowser (creates 640px-wide proxy files)
- "Proxy" mode toggle calling `setProxyMode` IPC

#### Export Dialog
- MKV added as native export format option
- Hardware info panel showing GPU family, free memory, temperature, utilization

#### Drag-and-Drop to Timeline
- TrackRow now accepts media drops from MediaBrowser with `onDragOver`/`onDrop` handlers
- Drop position calculated from mouse coordinates → frame number at correct zoom/scroll
- Clip kind auto-matched to track kind (Audio track → Audio clip)
- Visual drop highlight with dashed outline during dragover

#### Clip Context Menu
- Right-click on any clip opens a floating context menu
- Split at Playhead (disabled when playhead is outside clip range)
- Duplicate (places copy immediately after original)
- Delete
- ESC or click-outside dismissal

#### Timeline Snapping
- `useSnap` hook collects snap targets: all clip start/end edges, playhead position, markers, frame 0
- 5-frame snap threshold
- Integrated into `useDragClip` (clip move) and `useTrimClip` (left/right edge trim)
- Both hooks now use `_pushUndo` once on drag start + `_mutateSilent` during drag — one undo entry per gesture

### AI Features (Tier 1)

#### Auto-cut / Highlights
- `crates/media/src/ai.rs` — `detect_highlights()` uses `SceneDetector` + `content_score()` to rank video segments
- Scores every 5th frame for performance, groups by scene boundary, returns top N by average score
- `detect_highlights` Tauri IPC command + TS binding

#### Subtitle Generation
- `transcribe_audio` Tauri command — decodes audio, prepares for Whisper, routes through hoosh
- `segments_to_srt()` and `segments_to_vtt()` formatters for subtitle export
- Uses `tarang::ai::HooshClient::transcribe()` with configurable endpoint via `HOOSH_ENDPOINT` env var
- UI shows timed cue list in AITools panel

#### AI Color Grading
- `auto_color_correct()` — analyzes frame luminance histogram (256 bins), computes correction gains
- Brightness offset: shift mean toward neutral 128; Contrast: normalize std dev to 50; Saturation: boost flat images, tame over-contrasty
- `auto_color_correct` Tauri command + "Auto Color" button applies ColorGrade effect with computed values

#### Smart Transitions
- `suggest_transition()` maps scene boundary characteristics to transition recommendations
- HardCut + high score → Cut; moderate → Dissolve; GradualTransition → Dissolve or Fade
- `suggest_transitions` Tauri command analyzes entire video, returns per-boundary suggestions with reasoning
- UI displays suggestions with timestamps, types, durations, and explanations

#### AI Tools UI
- `AITools` component in ClipInspector — four buttons: Auto Color, Highlights, Transcribe, Transitions
- Results displayed inline with formatted timestamps and scores
- Loading states prevent concurrent operations

### Backend
- `extract_waveform` Tauri command registered
- `generate_thumbnails` Tauri command with base64-encoded thumbnail results
- `measure_loudness` Tauri command (decodes all audio, returns integrated LUFS)
- 555 tests passing (was 529)

## 2026.3.19

### Dependency Migration
- **Tarang** migrated from local path dependencies (5 sub-crates) to single published crate `tarang = "0.19.3"` on crates.io
  - `tarang-core`, `tarang-audio`, `tarang-demux`, `tarang-video`, `tarang-ai` → single `tarang` crate with module imports (`tarang::core::`, `tarang::audio::`, etc.)
  - Feature flag simplified: `tarang = ["dep:tarang", "dep:symphonia"]`
  - Tarang switched from CalVer to SemVer for crates.io compatibility
- **ai-hwaccel** `0.19.3` added as non-optional workspace dependency
  - Universal AI hardware accelerator detection (CUDA, ROCm, Metal, Vulkan, Intel NPU, AMD XDNA, TPU, Gaudi, AWS Neuron, oneAPI, Qualcomm)
  - Added to both `tazama-media` and `tazama-gpu` — always-on, OS-agnostic, best-effort detection
  - Zero vendor SDK compile-time dependencies

### Tarang Video Export Pipeline
- **MKV export fully native** via tarang — H.264 encode (openh264) + audio encode + custom dual-track EBML muxer
- RGBA→YUV420p conversion with BT.601 coefficients
- Audio codec selection: FLAC, Opus, AAC per config, with format-specific defaults
- Progress reporting via watch channel, same interface as GStreamer pipeline
- MP4/WebM/ProRes/DnxHr/GIF fall back to GStreamer (tarang Mp4Muxer is audio-only, VP9 encoder pending vpx-sys fix)
- 15 new tests covering conversion, codec selection, EBML muxing

### ai-hwaccel Integration
- **Hardware detection wired in** — `ai_hwaccel::CachedRegistry` with 5-minute TTL in `tazama-media::hwaccel`
- `available_encoders()` now uses ai-hwaccel to detect VAAPI (AMD/Intel GPU) and NVENC (NVIDIA GPU) instead of GStreamer `ElementFactory::find`
- Removed stale `hardware_accel: bool` from `ExportConfig` — replaced by `ExportEncoder` enum + ai-hwaccel detection
- GPU crate logs detected hardware on context init via `log_detected_hardware()`
- `detect_gpu_hardware()` public function exposes GPU info (name, VRAM, compute capability, driver)
- New `detect_hardware` Tauri IPC command returns accelerators + available encoders
- New `tazama_detect_hardware` MCP tool (8 tools total)
- TypeScript `HardwareInfo` type and `detectHardware()` IPC wrapper added

### Tarang Default Pipeline
- **Tarang is now always-on** — removed `tarang` feature flag, tarang + symphonia are non-optional dependencies
- Removed 43 `#[cfg(feature = "tarang")]` gates across 8 source files
- Tarang handles probe, decode, and thumbnails as primary path; GStreamer falls back for unsupported formats
- Benchmarks confirmed: audio probe 15.4× faster, audio decode 3.96× faster than GStreamer

### Test Coverage (529 tests, 48.3% line coverage)
- **Waveform** — struct construction, error paths (5 tests)
- **Thumbnail** — spec tests, tarang extension helpers, nonexistent file error (13 tests)
- **Record** — WAV header validation, overflow protection, state defaults (5 tests)
- **MediaStore** — import error paths, directory creation, content preservation, overwrite (4 tests)
- **Probe** — FileNotFound, empty file, all container formats, tarang codec mapping, frame rate rationals (20 tests)
- **DSP integration** — chained effects, disabled skipping, volume keyframes, video effect ignored (10 tests)
- **Keyframe** — bezier extreme tangents, overshoot, same-frame div-by-zero, integrated speed, boundary evaluation (9 tests)
- **Clip overlap** — overlapping/adjacent/boundary clips, zero-duration, move overlap/resolve (8 tests)
- **Empty timeline** — duration_frames, audible/visible tracks on empty timeline (4 tests)
- High-coverage modules: DSP (97%+), text.rs (100%), storage (90%+), core types (85%+)

### P1 Code Quality (7 items)
- **Test assertions** — replaced 11 `panic!()` assertions with `assert!(matches!(...))` in effect.rs and MCP tests
- **Magic constants** — extracted named constants for WASM timeout (5s), export bus timeout (120s), WASM memory (16MB/256 pages), compute workgroup size (256)
- **apply_effect refactor** — 10 individual parameters replaced with `EffectContext` struct, returns `Option<GpuBuffer>` for cleaner ownership
- **JSON size limits** — 50MB cap on deserialization in database cache, project loading, and MCP message parsing
- **GStreamer state validation** — export pipeline now verifies Playing state with 10-second timeout after set
- **Proxy TOCTOU fix** — replaced `exists()` + `metadata()` race with single `metadata()` match on NotFound
- **Emit error logging** — all 4 `let _ = app.emit(...)` instances now log warnings on failure

### Security Audit Fixes
- **MCP path traversal** — added `validate_user_path()` rejecting `..` components in add_clip source, export output, and extract_frame output paths
- **MCP input validation** — project width/height clamped to 1–8192 range
- **Integer overflow protection** — checked arithmetic in frame buffer size (gpu), WAV header (record), crop parameters clamped to [0.0, 1.0] with saturating_add
- **Float-to-int safety** — speed/frame calculations clamped to non-negative before u64 cast
- **Keyframe div-by-zero** — returns left.value when two keyframes share the same frame
- **ImageBuffer validation** — frame data length checked before `from_raw()` in MCP extract_frame
- **WASM memory bounds** — plugin params checked against 16MB limit before write, `buf_size * 2` overflow guard
- **Autosave stop signal** — `tx.send()` failure now logged instead of silently dropped

### P0 Code Audit & Refactoring (14 items)

#### Round 1: Structural Audit
- **DSP hardening** — NaN/Inf guards on all 4 modules (compressor, EQ, noise reduction, reverb), threshold clamping [-120, 120] dB, biquad coefficient validation (a0 near-zero skip), minimum reverb delay length 4 samples, 8 new edge-case tests
- **GPU render audit** — All 8 compute shaders verified correct (trilinear LUT, alpha-over blending, color transforms). Keyframe resolution O(P×K) cost documented
- **Command.rs audit** — All 16 EditCommand variants confirmed symmetric apply/undo. Added 10 redo tests + 1 complex integration test (34 total, up from 23)
- **Autosave race fix** — Dirty flag now reset atomically with project snapshot (prevents lost updates). Autosave writes to `.tmp` then renames (atomic). Corrupt JSON recovery now logs parse error details
- **Export pipeline** — GIF format fixed (was x264enc+mp4mux, now gifenc). Added `ExportEncoder` enum (Auto/Software/Vaapi/Nvenc/Tarang) with `available_encoders()` system probe. Hardware encoder failures now logged with details
- **WASM plugin sandboxing** — Epoch interruption enabled (5-second timeout kills runaway plugins). Memory capped at 16MB fixed (cannot grow). Wasmtime traps caught and returned as descriptive errors
- **TS/Rust type parity** — Added `PlaybackPosition`, `WaveformData`, `ThumbnailSpec`, `ExportEncoder` to TypeScript types. Fixed `MultiCamGroup` signed offset documentation

#### Round 2: Refactoring
- **GPU render.rs split** — 1252-line monolith split into `render/{mod,effects,transitions,dispatch,collect}.rs`. `resolve_param` extracted as module-level function replacing per-clip closure
- **Serde defaults verified** — All `#[serde(default)]` fields (keyframe_tracks, proxy_path, volume, pan, multicam_groups) confirmed backward-compatible
- **Proxy input validation** — Early rejection for audio-only (wav/mp3/flac/ogg/m4a/aac) and image (png/jpg/gif/bmp/tiff/svg) files with clear error
- **cosmic-text caching** — `FontSystem` cached via `LazyLock<Mutex<>>` (was creating new instance per frame render)

### GPU Integration Tests
- 8 integration tests on real Vulkan hardware (context, pipeline, renderer, buffer roundtrip, empty timeline, color grade, crop, frame size)
- `require_gpu!()` macro gracefully skips tests when no Vulkan device available (CI)
- CI installs `mesa-vulkan-drivers` (lavapipe) and sets `VK_ICD_FILENAMES` for software Vulkan

### App Command Tests
- 13 integration tests covering 13/17 Tauri IPC handlers (project CRUD, media import/probe, autosave recovery, recording, proxy, hardware detection)

### Tarang vs GStreamer Video Benchmarks
- Video probe: **18-20× faster** (158–179 µs vs 3.1–3.4 ms) across MP4/WebM/MKV
- Video decode (10 frames H.264): **32.6× faster** (175 µs vs 5.7 ms)
- Test fixtures generated via `scripts/generate-test-fixtures.sh` (ffmpeg)

### Windows Release Builds
- Added `windows-latest` to release build matrix
- GStreamer MSVC runtime + dev installed via MSI
- Tarang codec deps (dav1d, libvpx, openh264, opus, fdk-aac) installed via vcpkg
- Windows packaging: `.exe` binaries in `.zip` archive with SHA256 checksum
- Release page updated with Windows x64 download row

### CI & Build Fixes
- Added tarang codec dependencies (dav1d, libvpx, openh264, opus, fdk-aac) to CI action, Dockerfile, and testing docs
- Removed stale `create-tarang-stubs.sh` step from CI (tarang is now on crates.io)
- Lavapipe software Vulkan configured for CI GPU tests

### Version Sync
- All version references bumped to `2026.3.19` (VERSION, Cargo.toml, tauri.conf.json, package.json, marketplace recipe, agent manifest)
- Marketplace recipe and agent manifest synced (were stale at `2026.3.15`)

## 2026.3.18-2

### Export Audio Codec Selection
- `ExportAudioCodec` enum (Aac, Opus, Flac) — users can now choose audio codec for export
- `ExportConfig.audio_codec` field wired through tarang export pipeline
- FLAC lossless export available via tarang-audio's pure Rust FLAC encoder (LPC + fixed prediction, Rice coding)

### Dependency Updates
- Tarang bumped to 2026.3.18 (FLAC compression with Levinson-Durbin LPC, backlog error fixes)
- OS recipe (`tazama.toml`) now declares `tarang` as runtime + build dependency

## 2026.3.18-1

### Post-v1 Non-AI Features
- Keyframe animation engine with linear, hold, and bezier cubic interpolation
- Audio DSP: 3-band EQ, compressor, spectral noise reduction (rustfft), Schroeder reverb
- Audio mixer with per-track volume/pan (equal-power pan law), clip effect chain
- Voiceover recording via CPAL with WAV export
- LUT import (.cube 3D LUT parser + trilinear interpolation compute shader)
- Text/title overlay via cosmic-text rasterization
- Picture-in-picture transform (scale + translate compute shader)
- Speed ramping with keyframed variable speed (trapezoidal integration)
- Proxy workflow (GStreamer transcode to lower-res preview files)
- Multi-cam editing (angle groups with sync offsets, switch commands)
- Project autosave (tokio interval task, crash recovery, cleanup on manual save)
- WASM plugin system via wasmtime (optional `plugins` feature)
- Tarang export migration stub with GStreamer fallback
- Export format expansion: ProRes, DNxHR, MKV, GIF
- Hardware encode detection: VAAPI -> NVENC -> x264enc fallback
- 453 tests, 50%+ coverage (threshold raised to 40%)

### Infrastructure
- CI tarang stub script for builds without the tarang repo
- Version bump script (`scripts/bump-version.sh`) with patch/today modes
- VERSION file as single source of truth for all version references

## 2026.3.15

### GPU-Accelerated Preview

#### Preview Rendering (`tazama`)
- `render_preview_frame` now runs the full GPU render pipeline (Vulkan compute effects, multi-track compositing, transitions) instead of raw source frame decode
- Preview output matches export: ColorGrade, Crop, Speed, dissolve/wipe/fade transitions all visible in real-time scrubbing
- Fast path: skips GPU init when no clips are active at the requested frame (returns black)
- GPU context and renderer created per-request on a blocking task to avoid blocking the async runtime

#### Batch Import Error Feedback (`ui/`)
- Import now accumulates per-file success/failure results and shows a summary toast after the batch completes
- On partial failure: "Imported N of M files. Failed: filename: reason" with per-file detail
- On total failure: "All N imports failed" with per-file detail
- On full success of multi-file import: "Imported N files" success toast
- Toast component updated to render multi-line messages

#### Infrastructure
- Dockerfile for `tazama-mcp` server (multi-stage build, AGNOS runtime base, GStreamer + ALSA runtime libs)
- `.dockerignore` excluding target/, node_modules/, dist/, ui/, docs/, .git/

## 2026.3.13

### Export Integration & Preview

#### End-to-End Export Pipeline (`tazama`)
- `MediaFrameSource` — bridges media decoder to GPU `FrameSource` trait with single-frame cache
- `export_project` — fully wired: GPU Renderer → VideoFrame → GStreamer encode pipeline
- `AudioOutput` trait in gpu crate decouples preview audio from media crate (no circular deps)
- `render_preview_frame` command — decodes source video frame at timeline position, returns base64 RGBA
- `Timeline::topmost_video_clip_at()` — finds the highest-priority visible clip at a frame (respects mute/solo/visible)

#### Multi-Track Audio Mixer (`tazama-media`)
- `mix.rs` — offline mixer following Shruti's additive pattern
- Decodes all audio from active clips, applies per-clip volume, sums overlapping regions in 4096-frame chunks
- Respects track mute/solo flags, clips to [-1.0, 1.0] to prevent clipping
- Proper timeline positioning via source_offset → duration range and timeline_start → sample offset
- 11 unit tests (frame conversion, empty timeline, mute/solo/volume/clamp/offset logic)

#### Preview Canvas (`ui/`)
- `PreviewCanvas` — renders decoded video frames on HTML canvas via base64 RGBA `ImageData`
- Calls `render_preview_frame` IPC command on playback position change
- Frame skipping when decode is slower than position changes (pending ref guard)
- Auto-clears on project close

#### Export Pipeline Improvements (`tazama-media`)
- `ExportPipeline::run_with_total()` — accepts total frame count for accurate progress tracking
- Pipeline progress events now report real `total_frames` instead of 0
- Replaced all `unwrap()` calls in GStreamer buffer operations with proper error propagation
- `pipeline.bus().unwrap()` → safe `.ok_or_else()` pattern

### Code Audit Fixes

#### Safety & Correctness
- GStreamer RAII `PipelineGuard` — decode pipelines now always set to Null on exit (video.rs, audio.rs)
- `static_pad("sink").unwrap()` → safe `let Some(pad) = ... else { return }` in both decoders
- `GpuContext::Drop` — replaced unsafe `drop_in_place` with `Option<Allocator>` + `.take()` for safe ordered destruction
- SPIR-V alignment validation — `shader.rs` rejects non-4-byte-aligned bytecode before `chunks_exact`
- `FrameRate::new()` — asserts denominator > 0; `fps()` returns 0.0 defensively
- Crop dimension underflow — `saturating_sub().max(1)` prevents zero-size GPU buffers
- Integer overflow in frame timestamp — `checked_mul` chain with `u64::MAX` fallback
- Audio buffer alignment — truncates to 4-byte boundary before `chunks_exact(4)`
- Mutex poisoning resilience — standardized on `unwrap_or_else(|e| e.into_inner())` across all crates
- GPU buffer allocator — handles `Option<Allocator>` after context destruction

#### MCP Server (`tazama-mcp`)
- Removed unsupported `"mov"` format from export tool schema (only mp4/webm supported)
- Fixed 3 `.as_mut().unwrap()` panics → safe `let Some(...) else { return mcp_error() }` pattern
- 6 agnoshi intents (was 5) — added "add marker" intent

#### Frontend (`ui/`)
- `Ctrl+O` keyboard shortcut for opening projects
- Export button disabled during export (prevents double-click)
- `ImportButton` now calls `importMedia()` to copy files into project directory
- `NewProjectDialog` — min bounds validation (100x100), disabled Create button when invalid
- `MediaItem` double-click — shows toast when no video track exists
- `ExportProgress` — safer listener cleanup with `unlistenFn` variable pattern
- `FileActions` — loading state with "Loading..." indicator and disabled buttons during open/save

#### Build & Dependencies
- Removed unused `tempfile` dev-dependency from storage crate
- Added `base64` workspace dependency for preview frame encoding

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
