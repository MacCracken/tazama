# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor. MVP target: import media, arrange clips on a multi-track timeline, apply basic effects, preview in real-time, and export to MP4/WebM.

## Completed

- **Phase 0–5** — Scaffold, media pipeline, editing backend, GPU rendering, desktop UI, MCP & AGNOS integration
- **Post-v1 non-AI features** — Keyframe animation, audio DSP, mixer, voiceover recording, LUT import, text overlay, PiP, speed ramping, proxy workflow, multi-cam editing, autosave, WASM plugins, format expansion (ProRes/DNxHR/MKV/GIF), hardware encode detection (2026.3.18)
- **Dependency migration** — Tarang to crates.io (single crate v0.19.3), ai-hwaccel added as non-optional dep (2026.3.19)
- **P0 code audit & refactoring** — DSP hardening, GPU render split, command.rs redo tests, autosave race fix, export encoder selection, WASM sandboxing, TS/Rust type parity, proxy input validation, cosmic-text caching (2026.3.19)
- **Security audit** — MCP path traversal fix, input validation, integer overflow protection, float-to-int safety, keyframe div-by-zero, ImageBuffer validation, WASM memory bounds (2026.3.19)

---

## P1 — Engineering Backlog

### Code Quality (complete)
- [x] Replace test `panic!()` assertions with `assert!(matches!(...))` — 11 instances in effect.rs and mcp tests (2026.3.19)
- [x] Extract magic number constants: WASM timeout, bus timeout, memory limits, workgroup size (2026.3.19)
- [x] Refactor `apply_effect()` signature — 10 params → `EffectContext` struct (2026.3.19)
- [x] JSON size limits — 50MB cap on deserialization in database and MCP (2026.3.19)
- [x] GStreamer pipeline state validation — verify Playing state with 10s timeout after set (2026.3.19)
- [x] TOCTOU fix in proxy.rs — replaced exists() + metadata() with single metadata() match (2026.3.19)
- [x] Emit error logging — all `let _ = app.emit(...)` now log on failure (2026.3.19)

### Test Coverage (48% → target 65%)
- [x] Waveform extraction — struct construction, error paths (2026.3.19)
- [x] Thumbnail generation — spec tests, tarang helpers, nonexistent file error (2026.3.19)
- [x] Record module — WAV header, overflow protection, state management (2026.3.19)
- [x] MediaStore::import() — nonexistent source, directory creation, content preservation, overwrite (2026.3.19)
- [x] Probe error paths — FileNotFound, empty file ProbeFailed, all container formats, tarang codec mapping (2026.3.19)
- [x] DSP integration — chained effects, disabled effects, volume keyframes, video effect ignored (2026.3.19)
- [x] Keyframe bezier edge cases — extreme/negative tangents, overshoot, same-frame, integrated speed (2026.3.19)
- [x] Clip overlap detection — overlapping/adjacent/boundary clips, zero-duration, move overlap (2026.3.19)
- [x] Empty timeline — duration_frames, audible_tracks, visible_video_tracks on empty (2026.3.19)
- [ ] Export pipeline integration — no end-to-end encoding tests (7% coverage)
- [ ] GPU render/dispatch/transitions — requires mock GPU context (0% coverage)
- [ ] App command integration tests (all Tauri IPC handlers)
- [ ] Audio/video decode — 0% coverage, requires real media files or mocks
- [ ] Playback module — 0% coverage

### Benchmarks
- [ ] DSP benchmarks (criterion) — EQ, compressor, noise reduction, reverb on realistic buffer sizes
- [ ] GPU render benchmarks — frame render time at 1080p/4K, effect chain overhead
- [ ] Export pipeline benchmarks — encode throughput per format (Mp4/WebM/ProRes/DnxHr/Mkv/Gif)
- [ ] Probe/decode benchmarks — media file probe latency, video decode frame rate
- [ ] Keyframe evaluation — interpolation throughput with many keyframe tracks
- [ ] Tarang vs GStreamer — comparative decode/probe benchmarks when tarang feature enabled

---

## Post-v1 Features

### Tarang Media Backend Migration (in progress)
- Tarang integrated behind `tarang` feature flag for probe, decode, thumbnails, and export fallback (2026.3.18)
- Migrated from local path deps to published crate `tarang = "0.19.3"` (2026.3.19)
- Remaining: replace GStreamer as primary pipeline (tarang currently used as first-try with GStreamer fallback)
- Remaining: full tarang video export (currently stub, falls back to GStreamer)

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
