# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor. MVP target: import media, arrange clips on a multi-track timeline, apply basic effects, preview in real-time, and export to MP4/WebM.

## Completed Phases

- **Phase 0** — Scaffold (workspace, core types, storage/gpu/mcp stubs, Makefile, ADR-001)
- **Phase 1** — Media Pipeline (GStreamer probe, decode, thumbnails, waveforms, export)
- **Phase 2** — Functional Editing Backend (clip ops, undo/redo, SQLite persistence, MCP tool dispatch)
- **Phase 3** — GPU Rendering (Vulkan compute pipelines, 6 effect shaders, preview/export render loops)
- **Phase 4** — Desktop UI (React 19 + Vite + Tailwind v4, full NLE interface)
- **Phase 5** — MCP & AGNOS Integration (markers, audio preview, solo/visible, agnoshi intents, marketplace)

## Phase 5 — MCP & AGNOS Integration (complete)

Wire up remaining MCP features, add agnoshi intents, package for marketplace.

- [x] PipeWire audio monitoring (CPAL-based preview via ALSA/PipeWire plugin layer)
- [x] Markers as first-class timeline type with undo/redo support
- [x] Track solo/visible fields with GPU renderer and audio preview integration
- [x] `tazama_add_marker` MCP tool (6 tools total)
- [x] 5 agnoshi intents ("edit video", "add clip", "apply effect", "export project", "show timeline")
- [x] `.agnos-agent/manifest.toml` bundle for marketplace
- [x] Marketplace recipe (`recipes/marketplace/tazama.toml`)
- [x] MCP integration test suite (7 tests, spawns binary, tests JSON-RPC protocol)

## Phase 4 — Desktop UI (complete)

Tauri v2 + React 19 / TypeScript / Vite / Tailwind v4 / Zustand frontend.

- [x] Frontend scaffold (Vite + React + TypeScript, Tauri integration)
- [x] Timeline panel (multi-track with clip blocks, drag/drop, scrubber)
- [x] Preview monitor (video preview receiving rendered frames)
- [x] Media browser (import, browse, search project media assets)
- [x] Inspector panel (clip properties, effect parameters)
- [x] Toolbar (select/razor/slip tools, transport, timecode display)
- [x] Keyboard shortcuts (standard NLE keybindings — J/K/L, I/O, spacebar)
- [x] Export dialog (format, resolution, output path, progress bar)
- [x] Project management (new, open, save, recent projects, welcome screen)
- [x] Theming (dark theme with CSS custom properties)

---

## Engineering Backlog

Known issues and hardening work identified during code audit. Prioritized by severity.

### High Priority
- [x] GStreamer pipeline cleanup on early exit — RAII `PipelineGuard` added to video.rs and audio.rs
- [x] GStreamer `static_pad("sink").unwrap()` — replaced with `let Some(pad) = ... else { return }` in both decoders
- [x] Integer overflow in frame timestamp calculation — uses `checked_mul` chain with `unwrap_or(u64::MAX)` fallback
- [x] Multi-track audio mixing for export — `mix.rs` decodes all audio clips, applies per-clip volume, sums overlapping regions in time-aligned chunks (following Shruti's pattern)
- [ ] PreviewCanvas component — placeholder exists but no actual frame rendering from GPU to canvas. Needs Tauri event bridge or shared memory.
- [x] NewProjectDialog input validation — min bounds (100x100), Create button disabled when invalid
- [x] MediaItem double-click silent failure — shows toast when no video track exists

### Medium Priority
- [x] Mutex poisoning resilience — standardized on `unwrap_or_else(|e| e.into_inner())` in playback.rs, frame_source.rs, buffer.rs
- [x] Audio buffer alignment — truncates to 4-byte aligned length before `chunks_exact(4)` in audio decoder
- [x] ExportProgress listener cleanup — uses proper `unlistenFn` variable pattern instead of promise chain
- [x] Missing loading states — FileActions shows "Loading..." and disables buttons during open/save
- [x] `tazama_add_marker` missing from AGNOS manifest intents — added "add marker" intent with 4 examples
- [x] Unused `tempfile` dev-dependency in storage crate — removed from Cargo.toml
- [x] Export pipeline `total_frames` tracking — added `run_with_total()` method, app passes real total_frames

### Low Priority
- [ ] Per-file error feedback in batch import — currently shows toast per failure but doesn't indicate which files succeeded in batch.

---

## Post-v1 Features

### Audio Editing
- Audio mixer panel (per-track volume, pan, mute/solo)
- Audio effects (EQ, compressor, noise reduction, reverb)
- Waveform editing (visual trim, fade handles on audio clips)
- Voiceover recording (record via PipeWire directly into timeline)

### Advanced Effects
- Keyframe animation (animate any effect parameter over time, bezier curves)
- Speed ramping (variable speed with smooth transitions)
- LUT import (load .cube LUT files for color grading)
- Text / title editor (overlay text with fonts, animation, positioning)
- Picture-in-picture (resize and position clips within the frame)

### AI Features (Tier 1)
- Scene detection (auto-detect scene boundaries, suggest cuts)
- Auto-cut / highlights (AI selects best segments from long footage)
- Subtitle generation (speech-to-text → SRT/VTT, burn-in option)
- AI color grading (match color between clips, auto color correct)
- Smart transitions (AI suggests transition type/duration based on content)

### AI Features (Tier 2)
- AI voiceover / TTS (generate voiceover from text, multiple voices)
- B-roll suggestions (AI suggests stock/generated footage for gaps)
- Style transfer (apply visual style from reference image/video)
- Background removal (AI-powered chroma key without green screen)
- Audio cleanup (AI noise removal, voice isolation)

### Platform
- Plugin system (third-party effects/transitions as WASM or shared libs)
- Proxy workflow (low-res proxies for editing, swap on export)
- Multi-cam editing (sync and switch between multiple camera angles)
- Project autosave (periodic save with crash recovery)
- Hardware encode (VAAPI / NVENC for GPU-accelerated export)
- Format expansion (ProRes, DNxHR, MKV, GIF export)
