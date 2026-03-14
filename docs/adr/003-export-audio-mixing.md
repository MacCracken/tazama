# ADR-003: Export Pipeline & Multi-Track Audio Mixing

## Status

Accepted

## Context

Tazama needs to produce final video files by combining GPU-rendered video frames with mixed audio from multiple timeline tracks. The export pipeline must bridge three separate systems:
1. **GPU renderer** (Vulkan compute → RGBA frames)
2. **Audio mixer** (GStreamer decode → per-clip volume → additive mix)
3. **GStreamer encoder** (RGBA + PCM → MP4/WebM container)

Each system lives in a different crate with no circular dependencies allowed.

## Decision

### FrameSource trait for media→GPU decoupling

The `tazama-gpu` crate defines a `FrameSource` trait. The `tazama` app crate provides `MediaFrameSource`, which implements this trait by calling GStreamer's video decoder. This avoids `tazama-gpu` depending on `tazama-media`.

```
tazama-gpu: defines FrameSource trait
tazama-media: provides VideoDecoder
tazama (app): MediaFrameSource implements FrameSource using VideoDecoder
```

### AudioOutput trait for preview decoupling

Similarly, `tazama-gpu`'s `PreviewLoop` accepts an `Option<Arc<dyn AudioOutput>>` trait object instead of directly referencing `tazama-media::AudioPreview`. The app layer implements the bridge.

### Offline audio mixing (Shruti pattern)

Audio is mixed offline for export using an additive mixing algorithm inspired by Shruti's mixer:

1. **Decode phase** — GStreamer decodes all audio from each active clip into `Vec<f32>`
2. **Trim phase** — Source samples are trimmed to `[source_offset..source_offset+duration]`
3. **Mix phase** — Walk through time in 4096-frame chunks, summing overlapping clip contributions with per-clip `volume` applied
4. **Clamp phase** — Output samples clamped to [-1.0, 1.0]
5. **Output** — Mixed `AudioBuffer`s sent via channel to the GStreamer encoder

Mute/solo flags are respected. Video tracks are excluded from audio mixing.

### Channel-based pipeline architecture

Export uses three parallel tasks connected by `tokio::sync::mpsc` channels:

```
[GPU render task] → video_tx → [GStreamer export pipeline]
[Audio mix task]  → audio_tx → [GStreamer export pipeline]
```

The export pipeline runs in a `spawn_blocking` task (GStreamer is synchronous). Progress is reported via `tokio::sync::watch` and forwarded to the frontend via Tauri events.

### Base64 preview frames

For the preview canvas, individual frames are decoded on-demand via a Tauri IPC command (`render_preview_frame`). The RGBA data is returned as base64 to avoid JSON serialization overhead of raw byte arrays. The frontend decodes to `ImageData` and draws on a `<canvas>`.

## Consequences

- Export is fully functional end-to-end: GPU rendering + multi-track audio → MP4/WebM
- Audio mixing is offline (decode-all-then-mix), which uses more memory for large projects but is simple and correct
- No circular crate dependencies — trait objects bridge crate boundaries
- Preview shows source frames only (no GPU effects) — GPU-accelerated preview is a post-v1 enhancement
- Base64 encoding adds ~33% overhead to preview frames; acceptable for scrubbing, but streaming playback should use a more efficient transport
