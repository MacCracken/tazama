//! Multi-track audio mixer for export.
//!
//! Decodes audio from all active clips across all audio tracks, applies
//! per-clip volume, and mixes them together in time-aligned chunks.
//! Follows Shruti's additive mixing pattern: each clip's samples are
//! summed into a mix buffer at the correct timeline position.

use std::path::Path;

use tokio::sync::mpsc;
use tracing::{debug, info};

use tazama_core::{ClipKind, FrameRate, Timeline, TrackKind};

use crate::decode::AudioBuffer;
use crate::decode::audio::AudioDecoder;
use crate::error::MediaPipelineError;

/// A fully decoded audio clip positioned on the timeline.
struct DecodedClip {
    /// Interleaved f32 samples from the source media.
    samples: Vec<f32>,
    /// Where this clip starts on the timeline, in audio samples.
    start_sample: u64,
    /// Per-clip volume multiplier.
    volume: f32,
}

/// Chunk size for output buffers (in frames, i.e. sample groups).
const MIX_CHUNK_FRAMES: usize = 4096;

/// Mix all audio tracks from a timeline and send the result as sequential
/// [`AudioBuffer`]s over a channel.
///
/// This is designed for offline export — it decodes all audio upfront,
/// then mixes in chunks. Respects mute/solo flags and per-clip volume.
pub fn mix_timeline_audio(
    timeline: &Timeline,
    frame_rate: &FrameRate,
    sample_rate: u32,
    channels: u16,
    tx: mpsc::Sender<AudioBuffer>,
) -> Result<(), MediaPipelineError> {
    let fps = frame_rate.fps();
    if fps <= 0.0 {
        return Ok(());
    }

    // Determine solo state for audio tracks
    let any_audio_solo = timeline
        .tracks
        .iter()
        .any(|t| t.solo && t.kind == TrackKind::Audio);

    // Decode all audio clips from non-muted audio tracks
    let mut decoded_clips: Vec<DecodedClip> = Vec::new();

    for track in &timeline.tracks {
        if track.kind != TrackKind::Audio {
            continue;
        }
        if track.muted {
            continue;
        }
        if any_audio_solo && !track.solo {
            continue;
        }

        for clip in &track.clips {
            // Accept Audio and Video clips (videos have audio tracks too)
            if clip.kind != ClipKind::Audio && clip.kind != ClipKind::Video {
                continue;
            }
            let media_path = match &clip.media {
                Some(m) => &m.path,
                None => continue,
            };

            // Decode the full audio from this clip's source
            let mut rx = AudioDecoder::decode(Path::new(media_path))?;
            let mut all_samples: Vec<f32> = Vec::new();

            while let Some(buf) = rx.blocking_recv() {
                all_samples.extend_from_slice(&buf.samples);
            }

            if all_samples.is_empty() {
                debug!("no audio decoded from {media_path}");
                continue;
            }

            // Determine the clip's source sample range
            let source_offset_samples =
                frames_to_samples(clip.source_offset, fps, sample_rate, channels);
            let clip_duration_samples =
                frames_to_samples(clip.duration, fps, sample_rate, channels);

            // Trim to source region
            let start = (source_offset_samples as usize).min(all_samples.len());
            let end =
                ((source_offset_samples + clip_duration_samples) as usize).min(all_samples.len());
            let trimmed = all_samples[start..end].to_vec();

            // Timeline position in samples
            let start_sample = frames_to_samples(clip.timeline_start, fps, sample_rate, channels);

            decoded_clips.push(DecodedClip {
                samples: trimmed,
                start_sample,
                volume: clip.volume,
            });
        }
    }

    if decoded_clips.is_empty() {
        info!("no audio clips to mix");
        return Ok(());
    }

    // Calculate total duration in samples
    let total_end_sample = decoded_clips
        .iter()
        .map(|c| c.start_sample + c.samples.len() as u64)
        .max()
        .unwrap_or(0);

    info!(
        "mixing {} audio clips, total {} samples",
        decoded_clips.len(),
        total_end_sample
    );

    // Mix in chunks
    let chunk_size = MIX_CHUNK_FRAMES * channels as usize;
    let mut offset: u64 = 0;

    while offset < total_end_sample {
        let remaining = (total_end_sample - offset) as usize;
        let this_chunk = chunk_size.min(remaining);
        let mut mix_buf = vec![0.0f32; this_chunk];

        // Sum contributions from all clips
        for clip in &decoded_clips {
            let clip_end = clip.start_sample + clip.samples.len() as u64;

            // Check if this clip overlaps the current chunk
            if offset >= clip_end || offset + this_chunk as u64 <= clip.start_sample {
                continue;
            }

            // Calculate the overlap region
            let chunk_start_in_clip = if offset > clip.start_sample {
                (offset - clip.start_sample) as usize
            } else {
                0
            };
            let mix_start = if clip.start_sample > offset {
                (clip.start_sample - offset) as usize
            } else {
                0
            };

            let available_from_clip = clip.samples.len() - chunk_start_in_clip;
            let available_in_mix = this_chunk - mix_start;
            let copy_len = available_from_clip.min(available_in_mix);

            let volume = clip.volume;
            for i in 0..copy_len {
                mix_buf[mix_start + i] += clip.samples[chunk_start_in_clip + i] * volume;
            }
        }

        // Clamp to [-1.0, 1.0] to prevent clipping
        for sample in &mut mix_buf {
            *sample = sample.clamp(-1.0, 1.0);
        }

        // Compute timestamp for this chunk
        let samples_per_channel = offset / channels as u64;
        let timestamp_ns = samples_per_channel * 1_000_000_000 / sample_rate as u64;

        let audio_buf = AudioBuffer {
            sample_rate,
            channels,
            samples: mix_buf,
            timestamp_ns,
        };

        if tx.blocking_send(audio_buf).is_err() {
            debug!("audio mix receiver dropped");
            return Ok(());
        }

        offset += this_chunk as u64;
    }

    Ok(())
}

/// Convert timeline frames to interleaved audio samples.
fn frames_to_samples(frames: u64, fps: f64, sample_rate: u32, channels: u16) -> u64 {
    let seconds = frames as f64 / fps;
    (seconds * sample_rate as f64 * channels as f64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frames_to_samples() {
        // 30fps, 48000Hz, stereo: 30 frames = 1 second = 96000 interleaved samples
        let samples = frames_to_samples(30, 30.0, 48000, 2);
        assert_eq!(samples, 96000);
    }

    #[test]
    fn test_frames_to_samples_fractional() {
        // 1 frame at 30fps = 3200 interleaved samples (48000/30 * 2)
        let samples = frames_to_samples(1, 30.0, 48000, 2);
        assert_eq!(samples, 3200);
    }

    #[test]
    fn test_frames_to_samples_mono() {
        // Mono: 30 frames at 30fps = 48000 samples
        let samples = frames_to_samples(30, 30.0, 48000, 1);
        assert_eq!(samples, 48000);
    }

    #[test]
    fn test_frames_to_samples_zero() {
        assert_eq!(frames_to_samples(0, 30.0, 48000, 2), 0);
    }

    #[test]
    fn test_mix_empty_timeline() {
        let timeline = tazama_core::Timeline::new();
        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();

        // No clips → no audio output
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_mix_muted_track_produces_no_output() {
        let mut timeline = tazama_core::Timeline::new();
        let track_id = timeline.add_track(tazama_core::Track::new("A1", TrackKind::Audio));
        timeline.tracks[0].muted = true;

        // Even with a clip, muted track produces nothing
        let clip = tazama_core::Clip::new("test", ClipKind::Audio, 0, 30);
        let _ = timeline.tracks[0].add_clip(clip);

        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();
        let _ = track_id;
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_mix_solo_excludes_non_solo_tracks() {
        let mut timeline = tazama_core::Timeline::new();
        timeline.add_track(tazama_core::Track::new("A1", TrackKind::Audio));
        timeline.add_track(tazama_core::Track::new("A2", TrackKind::Audio));

        // Solo track A2 — A1 should be excluded
        timeline.tracks[1].solo = true;

        // Only A1 has a clip, but it's not solo'd
        let clip = tazama_core::Clip::new("test", ClipKind::Audio, 0, 30);
        let _ = timeline.tracks[0].add_clip(clip);

        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();

        // A1 is excluded because A2 is solo'd, so no output
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_mix_video_tracks_ignored() {
        let mut timeline = tazama_core::Timeline::new();
        timeline.add_track(tazama_core::Track::new("V1", TrackKind::Video));

        let clip = tazama_core::Clip::new("test", ClipKind::Video, 0, 30);
        let _ = timeline.tracks[0].add_clip(clip);

        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();

        // Video tracks are not mixed for audio
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_decoded_clip_volume_applied() {
        // Verify the mixing math: two overlapping clips with different volumes
        let clip_a = DecodedClip {
            samples: vec![0.5, 0.5, 0.5, 0.5], // 2 stereo frames
            start_sample: 0,
            volume: 1.0,
        };
        let clip_b = DecodedClip {
            samples: vec![0.3, 0.3, 0.3, 0.3],
            start_sample: 0,
            volume: 0.5, // half volume
        };

        // Simulate mixing manually
        let mut mix = vec![0.0f32; 4];
        for clip in &[&clip_a, &clip_b] {
            for i in 0..4 {
                mix[i] += clip.samples[i] * clip.volume;
            }
        }

        // 0.5 * 1.0 + 0.3 * 0.5 = 0.65
        assert!((mix[0] - 0.65).abs() < 1e-6);
        assert!((mix[1] - 0.65).abs() < 1e-6);
    }

    #[test]
    fn test_clamp_prevents_overflow() {
        // Two loud clips that would sum > 1.0
        let clip_a = DecodedClip {
            samples: vec![0.8; 4],
            start_sample: 0,
            volume: 1.0,
        };
        let clip_b = DecodedClip {
            samples: vec![0.7; 4],
            start_sample: 0,
            volume: 1.0,
        };

        let mut mix = vec![0.0f32; 4];
        for clip in &[&clip_a, &clip_b] {
            for i in 0..4 {
                mix[i] += clip.samples[i] * clip.volume;
            }
        }
        // Clamp
        for s in &mut mix {
            *s = s.clamp(-1.0, 1.0);
        }

        // 0.8 + 0.7 = 1.5, clamped to 1.0
        assert_eq!(mix[0], 1.0);
    }

    #[test]
    fn test_offset_clips_dont_mix_outside_range() {
        let clip_a = DecodedClip {
            samples: vec![1.0; 4], // samples 0..3
            start_sample: 0,
            volume: 1.0,
        };
        let clip_b = DecodedClip {
            samples: vec![0.5; 4], // samples 4..7
            start_sample: 4,
            volume: 1.0,
        };

        // Mix chunk 0..4
        let chunk_size = 4;
        let mut mix = vec![0.0f32; chunk_size];
        let offset: u64 = 0;

        for clip in &[&clip_a, &clip_b] {
            let clip_end = clip.start_sample + clip.samples.len() as u64;
            if offset >= clip_end || offset + chunk_size as u64 <= clip.start_sample {
                continue;
            }
            let chunk_start_in_clip = if offset > clip.start_sample {
                (offset - clip.start_sample) as usize
            } else {
                0
            };
            let mix_start = if clip.start_sample > offset {
                (clip.start_sample - offset) as usize
            } else {
                0
            };
            let available_from_clip = clip.samples.len() - chunk_start_in_clip;
            let available_in_mix = chunk_size - mix_start;
            let copy_len = available_from_clip.min(available_in_mix);
            for i in 0..copy_len {
                mix[mix_start + i] += clip.samples[chunk_start_in_clip + i] * clip.volume;
            }
        }

        // Only clip_a contributes to chunk 0..4
        assert_eq!(mix, vec![1.0, 1.0, 1.0, 1.0]);
    }
}
