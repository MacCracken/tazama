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

### Tarang Media Backend Migration (in progress)
- Remaining: MP4 video muxing (tarang Mp4Muxer getting video support now)
- Remaining: WebM export (VP9 encode pending vpx-sys fix)
- Remaining: drop GStreamer as required dependency (optional fallback only)

### AI Features (Tier 1)
- Scene detection — tarang-ai has scene boundary detection, needs UI wiring
- Auto-cut / highlights — AI selects best segments from long footage
- Subtitle generation — tarang-ai has transcription routing (hoosh/Whisper), needs SRT/VTT output + burn-in
- AI color grading — match color between clips, auto color correct
- Smart transitions — AI suggests transition type/duration based on content

### AI Features (Tier 2)
- AI voiceover / TTS (generate voiceover from text, multiple voices)
- B-roll suggestions (AI suggests stock/generated footage for gaps)
- Style transfer (apply visual style from reference image/video)
- Background removal (AI-powered chroma key without green screen)
- Audio cleanup (AI noise removal, voice isolation)

### Platform
- [ ] Windows CI test matrix
- [ ] Windows MSI/NSIS installer
