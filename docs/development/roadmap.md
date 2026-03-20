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
- [x] 6 agnoshi intents ("edit video", "add clip", "apply effect", "export project", "add marker", "show timeline")
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

- [x] Per-file error feedback in batch import — accumulates results and shows summary toast with per-file detail (2026.3.15)
- [x] GPU-accelerated preview — `render_preview_frame` now runs the full GPU Renderer pipeline with effects, compositing, and transitions (2026.3.15)

---

## Post-v1 Completed

- **Non-AI Features** — Keyframe animation engine, audio DSP (EQ/compressor/noise reduction/reverb), mixer with per-track volume/pan, voiceover recording, LUT import, text overlay, PiP transform, speed ramping, proxy workflow, multi-cam editing, project autosave, WASM plugin system, tarang export migration, format expansion (ProRes/DNxHR/MKV/GIF), hardware encode detection (VAAPI/NVENC fallback) (2026.3.18)
- **Dependency Migration** — Tarang to crates.io (single crate v0.19.3), ai-hwaccel added as non-optional dep (2026.3.19)

---

## P0 — Code Audit & Refactoring

High priority. The post-v1 feature push added significant surface area across all crates. Before building on top of it, harden the foundation.

### Round 1: Structural Audit
- [ ] Review all new DSP modules (`crates/media/src/dsp/`) — validate filter coefficients, edge cases (silence, DC offset, NaN), benchmark performance
- [ ] Audit GPU render path — verify keyframe resolution doesn't regress render perf, check LUT/text/transform shader correctness with real media
- [ ] Review command.rs — ensure all new EditCommand variants have symmetric apply/undo, add integration tests for SetKeyframes and SwitchAngle
- [ ] Audit autosave — stress test with rapid mutations and simulated crashes, verify recovery across all project shapes
- [ ] Review export pipeline — test all 6 export formats with real media, verify hardware encode fallback on machines without VAAPI/NVENC
- [ ] Audit WASM plugin runtime — sandboxing, memory limits, malicious plugin handling
- [ ] TypeScript/Rust type parity — automated check that serde output matches TS interfaces

### Round 2: Refactoring
- [ ] Extract common GPU dispatch patterns (2-buffer, 3-buffer) into a typed helper to reduce boilerplate in render.rs
- [ ] Consolidate effect parameter resolution — the `resolve_param` closure is repeated per-clip; lift to a shared utility
- [ ] Unify audio effect application — mixer applies effects inline; consider a trait-based `AudioProcessor` pipeline
- [ ] Reduce render.rs size (~1000+ lines) — split into `render/effects.rs`, `render/transitions.rs`, `render/collect.rs`
- [ ] Review serde defaults — ensure all new `#[serde(default)]` fields are backward-compatible with existing project files
- [ ] Clean up proxy.rs GStreamer pipeline — handle audio-only and image inputs gracefully
- [ ] Evaluate cosmic-text performance — cache FontSystem/SwashCache across frames instead of per-call

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
