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
- [ ] WebM export (VP9 encode pending vpx-sys fix)
- [ ] Drop GStreamer as required dependency (optional fallback only)

### AI Features (Tier 1)
- [x] Auto-cut / highlights — scene detection + content scoring, top N segments ranked by visual interest
- [x] Subtitle generation — hoosh/Whisper transcription → timed cues, SRT/VTT formatters
- [x] AI color grading — luminance histogram analysis → auto brightness/contrast/saturation correction
- [x] Smart transitions — scene boundary type + change score → Cut/Dissolve/Fade suggestions
- [x] LLM clip description — transcribe + summarize via hoosh chat completions
- [x] Subtitle refinement — LLM removes fillers, fixes grammar
- [x] Subtitle translation — LLM translates to target language

### AI Features (Tier 2)
- [ ] AI voiceover / TTS (generate voiceover from text, multiple voices)
- [ ] B-roll suggestions (AI suggests stock/generated footage for gaps)
- [ ] Style transfer (apply visual style from reference image/video)
- [ ] Background removal (AI-powered chroma key without green screen)
- [ ] Audio cleanup (AI noise removal, voice isolation)

### Platform
- [ ] Windows CI test matrix
- [ ] Windows MSI/NSIS installer
