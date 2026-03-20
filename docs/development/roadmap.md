# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor. MVP target: import media, arrange clips on a multi-track timeline, apply basic effects, preview in real-time, and export to MP4/WebM.

## Completed

- **Phase 0–5** — Scaffold, media pipeline, editing backend, GPU rendering, desktop UI, MCP & AGNOS integration
- **Post-v1 non-AI features** — Keyframe animation, audio DSP, mixer, voiceover recording, LUT import, text overlay, PiP, speed ramping, proxy workflow, multi-cam editing, autosave, WASM plugins, format expansion (ProRes/DNxHR/MKV/GIF), hardware encode detection (2026.3.18)
- **Dependency migration** — Tarang to crates.io (single crate v0.19.3), ai-hwaccel added as non-optional dep (2026.3.19)
- **P0 code audit & refactoring** — DSP hardening, GPU render split, command.rs redo tests, autosave race fix, export encoder selection, WASM sandboxing, TS/Rust type parity, proxy input validation, cosmic-text caching (2026.3.19)
- **Security audit** — MCP path traversal fix, input validation, integer overflow protection, float-to-int safety, keyframe div-by-zero, ImageBuffer validation, WASM memory bounds (2026.3.19)
- **P1 code quality** — test assertions, magic constants, EffectContext refactor, JSON size limits, GStreamer state validation, TOCTOU fix, emit logging (2026.3.19)
- **Test coverage push** — 686 tests, 51.6% coverage. Waveform, thumbnail, record, MediaStore, probe, DSP integration, keyframe bezier, clip overlap, empty timeline (2026.3.19)
- **Benchmarks** — criterion suite: DSP (4), keyframe (6), timeline serde (2). See docs/development/benchmarks.md (2026.3.19)

---

## Engineering Backlog

### Test Coverage (51.6% — remaining gaps need hardware/media fixtures)
- [ ] Export pipeline integration — end-to-end encoding tests (13% coverage)
- [ ] GPU render/dispatch/transitions — requires mock GPU context or lavapipe (0% coverage)
- [ ] App command integration tests — all Tauri IPC handlers
- [ ] Audio/video decode — requires real media files or mocks (0% coverage)
- [ ] Playback module — requires CPAL audio device mock (0% coverage)

### Benchmarks
- [ ] GPU render — frame render time at 1080p/4K, effect chain overhead
- [ ] Export pipeline — encode throughput per format
- [ ] Probe/decode — media file probe latency, video decode frame rate
- [x] Tarang vs GStreamer — audio probe 15.4× faster, decode 3.96× faster (2026.3.19)
- [ ] Tarang vs GStreamer — video probe/decode with real MP4/MKV/WebM fixtures

---

## Post-v1 Features

### Tarang Media Backend Migration (in progress)
- Tarang now always-on (feature flag removed), GStreamer as fallback (2026.3.19)
- Audio probe 15.4× faster, audio decode 3.96× faster than GStreamer
- Remaining: full tarang video export (currently stub, falls back to GStreamer)
- ai-hwaccel integrated — cached registry, encoder detection, GPU info, IPC command, MCP tool (2026.3.20)
- Remaining: drop GStreamer as required dependency (optional fallback only)

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
- Windows release builds (MSVC toolchain, Vulkan on Windows, GStreamer MSVC binaries, Tauri Windows target, MSI/NSIS installer, CI cross-compilation)
