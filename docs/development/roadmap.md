# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor.

## Completed

- **Phase 0–5** — Scaffold, media pipeline, editing backend, GPU rendering, desktop UI, MCP & AGNOS integration
- **Post-v1 non-AI features** — Keyframe animation, audio DSP, mixer, voiceover, LUT, text, PiP, speed ramping, proxy, multi-cam, autosave, WASM plugins, format expansion, hardware encode (2026.3.18)
- **Dependencies** — Tarang always-on (15-33× faster than GStreamer), ai-hwaccel integrated (2026.3.19)
- **Code audit & security** — DSP hardening, GPU render split, autosave race fix, WASM sandboxing, path traversal, integer overflow, EffectContext refactor (2026.3.19)
- **Test & benchmark suite** — 760 tests, GPU integration, app command tests, criterion benchmarks (2026.3.19)
- **Tarang export pipeline** — MKV native (H.264 + audio + EBML muxer) (2026.3.19)
- **Windows release builds** — added to release matrix with GStreamer MSVC + vcpkg codec deps (2026.3.19)

---

## Engineering Backlog

### Test Coverage
- [ ] GPU integration — NVIDIA, Intel, lavapipe (CI)
- [ ] Export pipeline — end-to-end encoding tests
- [ ] Audio/video decode — real media files or mocks
- [ ] Playback module — CPAL audio device mock

### Benchmarks
- [ ] GPU render — 1080p/4K, cross-vendor comparison
- [ ] Export pipeline — encode throughput per format

---

## Post-v1 Features

### Tarang Media Backend Migration (in progress)
- Remaining: MP4 video muxing (tarang Mp4Muxer getting video support now)
- Remaining: WebM export (VP9 encode pending vpx-sys fix)
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
- [ ] Windows CI test matrix
- [ ] Windows MSI/NSIS installer
