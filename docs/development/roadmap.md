# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor.

## Completed

- **Phase 0–5** — Scaffold, media pipeline, editing backend, GPU rendering, desktop UI, MCP & AGNOS integration
- **Post-v1 non-AI features** — Keyframe animation, audio DSP, mixer, voiceover, LUT, text, PiP, speed ramping, proxy, multi-cam, autosave, WASM plugins, format expansion, hardware encode (2026.3.18)
- **Dependencies** — Tarang to crates.io v0.19.3 (always-on, 15× faster probe), ai-hwaccel integrated (cached registry, encoder detection, IPC/MCP) (2026.3.19–20)
- **Code audit & security** — DSP hardening, GPU render split, autosave race fix, export encoder selection, WASM sandboxing, path traversal, integer overflow, TS/Rust type parity, EffectContext refactor, JSON limits, TOCTOU fix (2026.3.19)
- **Test & benchmark suite** — 745 tests, GPU integration (AMD RADV), app command tests, criterion benchmarks (DSP, keyframe, serde, tarang vs GStreamer) (2026.3.19–20)

---

## Engineering Backlog

### Test Coverage
- [ ] GPU integration tests — NVIDIA (NVENC/CUDA), Intel (oneAPI), lavapipe (CI headless)
- [ ] Export pipeline integration — end-to-end encoding tests (13% coverage)
- [ ] Audio/video decode — requires real media files or mocks (0%)
- [ ] Playback module — requires CPAL audio device mock (0%)

### Benchmarks
- [ ] GPU render — frame render time at 1080p/4K, effect chain overhead
- [ ] GPU render — cross-vendor comparison (AMD RADV vs NVIDIA vs Intel)
- [ ] Export pipeline — encode throughput per format
- [x] Tarang vs GStreamer video — probe 18-20× faster, decode 32.6× faster (2026.3.20)

### Tarang vs GStreamer (benchmarked 2026.3.19–20)
- Audio probe: **15.4× faster** (80 µs vs 1.24 ms)
- Audio decode: **3.96× faster** (336 µs vs 1.33 ms)
- Video probe: **18-20× faster** (158–179 µs vs 3.1–3.4 ms) across MP4/WebM/MKV
- Video decode (10 frames H.264): **32.6× faster** (175 µs vs 5.7 ms)

---

## Post-v1 Features

### Tarang Media Backend Migration (in progress)
- MKV export fully native via tarang (H.264 + audio + EBML muxer) (2026.3.20)
- Remaining: MP4 video muxing (tarang Mp4Muxer is audio-only)
- Remaining: WebM export (VP9 encode pending vpx-sys compatibility fix)
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
