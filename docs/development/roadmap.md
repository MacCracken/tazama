# Tazama Development Roadmap

Tazama is an AI-native non-linear video editor.

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

### Tarang Media Backend Migration
- [x] MKV muxing via tarang MkvMuxer (replaced custom EBML muxer)
- [x] MP4 video muxing via tarang Mp4Muxer (native, no GStreamer fallback)
- [x] Pixel format conversion via tarang::video::convert
- [x] Video scaling via tarang::video::scale
- [x] Loudness measurement & normalization via tarang::audio::loudness
- [x] Content-based thumbnail generation via tarang::ai::ThumbnailGenerator
- Remaining: WebM export (VP9 encode pending vpx-sys fix)
- Remaining: drop GStreamer as required dependency (optional fallback only)

### UI Completeness
- [x] Interactive effect parameter editors (sliders for all 15 effect types)
- [x] Effect preset menu (12 presets)
- [x] Keyframe animation UI (per-parameter K toggle, add/remove keyframes)
- [x] Mixer panel (volume faders, pan, mute/solo per track)
- [x] Audio waveform visualization on timeline clips
- [x] Thumbnail previews in media browser
- [x] Proxy workflow UI (generate + toggle)
- [x] Undo debouncing for continuous slider operations
- [x] MKV export format option
- [x] Hardware info in export dialog
- [x] Drag-and-drop from media browser to timeline track (drop target with position-aware placement)
- [x] Clip context menu (right-click: split at playhead, duplicate, delete)
- [x] Timeline snapping (clip edges, playhead, markers — 5-frame threshold)

### AI Features (Tier 1)
- [x] Scene detection — tarang-ai SceneDetector wired into thumbnail generation
- [x] Content-based frame scoring — tarang-ai ThumbnailGenerator with saliency heuristics
- [ ] Auto-cut / highlights — AI selects best segments from long footage
- [ ] Subtitle generation — tarang-ai has transcription routing (hoosh/Whisper), needs SRT/VTT output + burn-in
- [ ] AI color grading — match color between clips, auto color correct
- [ ] Smart transitions — AI suggests transition type/duration based on content

### AI Features (Tier 2)
- AI voiceover / TTS (generate voiceover from text, multiple voices)
- B-roll suggestions (AI suggests stock/generated footage for gaps)
- Style transfer (apply visual style from reference image/video)
- Background removal (AI-powered chroma key without green screen)
- Audio cleanup (AI noise removal, voice isolation)

### Platform
- [ ] Windows CI test matrix
- [ ] Windows MSI/NSIS installer
