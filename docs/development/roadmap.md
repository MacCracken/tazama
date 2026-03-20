# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor.

## Completed

- **Phase 0–5** — Scaffold, media pipeline, editing backend, GPU rendering, desktop UI, MCP & AGNOS integration
- **Post-v1 non-AI features** — Keyframe animation, audio DSP, mixer, voiceover, LUT, text, PiP, speed ramping, proxy, multi-cam, autosave, WASM plugins, format expansion, hardware encode (2026.3.18)
- **Dependency migration** — Tarang to crates.io v0.19.3, ai-hwaccel non-optional (2026.3.19)
- **Code audit & security** — DSP hardening, GPU render split, autosave race fix, export encoder selection, WASM sandboxing, path traversal fix, integer overflow protection, TS/Rust type parity (2026.3.19)
- **P1 code quality** — test assertions, magic constants, EffectContext refactor, JSON size limits, GStreamer state validation, TOCTOU fix, emit logging (2026.3.19)
- **Test coverage** — 724 tests, 51.6% coverage. Benchmarks: DSP, keyframe, timeline serde, tarang vs GStreamer (2026.3.19)
- **Tarang default pipeline** — feature flag removed, always-on. Audio probe 15.4× faster, decode 3.96× faster than GStreamer (2026.3.19)
- **ai-hwaccel integration** — cached registry, encoder detection, GPU info logging, IPC command, MCP tool (2026.3.20)

---

## Engineering Backlog

### Test Coverage (remaining gaps need hardware/media fixtures)
- [ ] Export pipeline integration — end-to-end encoding tests (13% coverage)
- [x] GPU render/dispatch/transitions — 8 integration tests on real Vulkan (AMD RADV) (2026.3.20)
- [ ] App command integration tests — all Tauri IPC handlers
- [ ] Audio/video decode — requires real media files or mocks (0%)
- [ ] Playback module — requires CPAL audio device mock (0%)

### Benchmarks
- [ ] GPU render — frame render time at 1080p/4K, effect chain overhead
- [ ] Export pipeline — encode throughput per format
- [ ] Tarang vs GStreamer video — probe/decode with real MP4/MKV/WebM fixtures

---

## Post-v1 Features

### Tarang Media Backend Migration (in progress)
- Remaining: full tarang video export (currently stub, falls back to GStreamer)
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
