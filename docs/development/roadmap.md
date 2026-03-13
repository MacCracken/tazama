# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor. MVP target: import media, arrange clips on a multi-track timeline, apply basic effects, preview in real-time, and export to MP4/WebM.

## Completed Phases

- **Phase 0** — Scaffold (workspace, core types, storage/gpu/mcp stubs, Makefile, ADR-001)

---

## Phase 1 — Media Pipeline

GStreamer integration for decoding, encoding, and media inspection.

- [ ] Media probe / inspection (`gstreamer-pbutils` discoverer — duration, resolution, codec, frame rate)
- [ ] Video decode pipeline (file → demux → decode → raw frames; H.264, H.265, VP9, AV1)
- [ ] Audio decode pipeline (decode to raw PCM; AAC, Opus, FLAC, MP3)
- [ ] Thumbnail generation (extract keyframes at intervals for timeline UI)
- [ ] Audio waveform extraction (generate waveform data for timeline audio tracks)
- [ ] Export pipeline (raw frames → encode → mux → file; MP4 H.264+AAC, WebM VP9+Opus)
- [ ] PipeWire audio monitoring (route preview audio through PipeWire for playback)
- [ ] Core tests (unit tests for all core types — timeline ops, clip manipulation, effect params)

## Phase 2 — Timeline Engine

Clip operations, playback sequencing, and the in-memory editing model.

- [ ] Clip trimming (adjust source_offset and duration, ripple vs. rolling trim)
- [ ] Clip splitting (split at frame position into two clips)
- [ ] Clip move / reorder (between positions, between tracks, snap-to-grid)
- [ ] Overlap detection (prevent/handle overlaps on same track)
- [ ] Undo/redo system (command pattern — every edit is reversible)
- [ ] Playback clock (frame-accurate position, play/pause/seek/scrub)
- [ ] Multi-track compositing order (track stacking, per-track visibility/mute/solo)
- [ ] Markers and regions (user-placed markers, in/out points)

## Phase 3 — GPU Rendering

Vulkan compute pipelines for real-time preview and final export rendering.

- [ ] Vulkan initialization (instance, device selection, compute queue via ash)
- [ ] Frame upload/download (CPU ↔ GPU buffer transfers for decoded frames)
- [ ] Compositing shader (alpha composite multiple tracks)
- [ ] Color grading shader (brightness, contrast, saturation, temperature, lift/gamma/gain)
- [ ] Transition shaders (dissolve, wipe, fade between adjacent clips)
- [ ] Crop / transform (position, scale, rotation, crop per clip)
- [ ] Preview render loop (real-time frame output at project frame rate)
- [ ] Export render loop (offline rendering — all frames → encode pipeline)
- [ ] Software fallback (CPU-based rendering via lavapipe for systems without GPU)

## Phase 4 — Desktop UI

Tauri v2 + React/TypeScript frontend.

- [ ] Frontend scaffold (Vite + React + TypeScript, Tauri integration)
- [ ] Timeline panel (multi-track with clip blocks, drag/drop, scrubber)
- [ ] Preview monitor (video preview receiving rendered frames)
- [ ] Media browser (import, browse, search project media assets)
- [ ] Inspector panel (clip properties, effect parameters)
- [ ] Toolbar (cut, trim, split, snap, magnet, zoom tools)
- [ ] Keyboard shortcuts (standard NLE keybindings — J/K/L, I/O, spacebar)
- [ ] Export dialog (format, resolution, bitrate, output path)
- [ ] Project management (new, open, save, recent projects)
- [ ] Theming (dark theme, AGNOS aethersafha integration)

## Phase 5 — MCP & AGNOS Integration

Wire up the 5 MCP tools, add agnoshi intents, package for marketplace.

- [ ] `tazama_create_project` implementation
- [ ] `tazama_add_clip` implementation
- [ ] `tazama_add_effect` implementation
- [ ] `tazama_get_timeline` implementation
- [ ] `tazama_export` implementation
- [ ] 5 agnoshi intents ("edit video", "add clip", "apply effect", "export project", "show timeline")
- [ ] `.agnos-agent` bundle (agent manifest for marketplace)
- [ ] Marketplace recipe (`recipes/marketplace/tazama.toml` for ark)
- [ ] MCP integration test suite

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
