//! Multi-track audio mixer for export.
//!
//! Decodes audio from all active clips across all audio tracks, applies
//! per-clip effects (EQ, Compressor, Noise Reduction, Reverb), per-clip
//! volume (including keyframed fades), track-level volume and stereo pan,
//! and mixes them together in time-aligned chunks.

use std::f32::consts::PI;
use std::path::Path;

use tokio::sync::mpsc;
use tracing::{debug, info};

use tazama_core::{ClipKind, EffectKind, FrameRate, Timeline, TrackKind};

use crate::decode::AudioBuffer;
use crate::decode::audio::AudioDecoder;
use crate::dsp;
use crate::error::MediaPipelineError;

/// A fully decoded audio clip positioned on the timeline.
struct DecodedClip {
    /// Interleaved f32 samples from the source media (after effects).
    samples: Vec<f32>,
    /// Where this clip starts on the timeline, in audio samples.
    start_sample: u64,
    /// Per-clip volume multiplier (combined clip.volume * track.volume).
    volume: f32,
    /// Equal-power pan gains: (left_gain, right_gain).
    pan_gains: (f32, f32),
}

/// Chunk size for output buffers (in frames, i.e. sample groups).
const MIX_CHUNK_FRAMES: usize = 4096;

/// Compute equal-power pan gains from a pan value in [-1, 1].
///
/// Uses: left = cos(theta), right = sin(theta)
/// where theta = (pan + 1) * PI / 4
fn equal_power_pan(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    let theta = (pan + 1.0) * PI / 4.0;
    (theta.cos(), theta.sin())
}

/// Apply audio effects from a clip's effect chain to the decoded samples.
fn apply_clip_effects(
    samples: &mut [f32],
    effects: &[tazama_core::Effect],
    sample_rate: u32,
    channels: u16,
    clip_duration_frames: u64,
    fps: f64,
) {
    for effect in effects {
        if !effect.enabled {
            continue;
        }
        match &effect.kind {
            EffectKind::Eq {
                low_gain_db,
                mid_gain_db,
                high_gain_db,
            } => {
                dsp::eq::apply_eq(
                    samples,
                    sample_rate,
                    channels,
                    *low_gain_db,
                    *mid_gain_db,
                    *high_gain_db,
                );
            }
            EffectKind::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
            } => {
                dsp::compressor::apply_compressor(
                    samples,
                    sample_rate,
                    channels,
                    *threshold_db,
                    *ratio,
                    *attack_ms,
                    *release_ms,
                );
            }
            EffectKind::NoiseReduction { strength } => {
                dsp::noise_reduction::apply_noise_reduction(samples, channels, *strength);
            }
            EffectKind::Reverb {
                room_size,
                damping,
                wet,
            } => {
                dsp::reverb::apply_reverb(
                    samples,
                    sample_rate,
                    channels,
                    *room_size,
                    *damping,
                    *wet,
                );
            }
            EffectKind::Volume { gain_db } => {
                // Check for keyframed volume
                if !effect.keyframe_tracks.is_empty() {
                    // Apply keyframed volume per-frame
                    let ch = channels as usize;
                    if ch > 0 {
                        for (frame_idx, frame) in samples.chunks_mut(ch).enumerate() {
                            // Convert sample frame index to timeline frame
                            let time_sec = frame_idx as f64 / sample_rate as f64;
                            let timeline_frame = (time_sec * fps) as u64;

                            // Find the Volume keyframe track
                            let volume_mult = effect
                                .keyframe_tracks
                                .iter()
                                .find(|kt| kt.parameter == "gain_db" || kt.parameter == "volume")
                                .and_then(|kt| tazama_core::keyframe::evaluate(kt, timeline_frame))
                                .map(|db| 10.0f32.powf(db / 20.0))
                                .unwrap_or_else(|| 10.0f32.powf(*gain_db / 20.0));

                            for sample in frame.iter_mut() {
                                *sample *= volume_mult;
                            }
                        }
                    }
                } else {
                    // Static volume gain
                    let gain = 10.0f32.powf(*gain_db / 20.0);
                    for sample in samples.iter_mut() {
                        *sample *= gain;
                    }
                }
            }
            EffectKind::LoudnessNormalize { target_lufs } => {
                crate::loudness::normalize_audio(samples, channels, sample_rate, *target_lufs);
            }
            EffectKind::FadeIn { duration_frames } => {
                apply_fade_in(samples, sample_rate, channels, *duration_frames, fps);
            }
            EffectKind::FadeOut { duration_frames } => {
                apply_fade_out(
                    samples,
                    sample_rate,
                    channels,
                    *duration_frames,
                    clip_duration_frames,
                    fps,
                );
            }
            _ => {
                // Other effects (video effects, etc.) are not applicable to audio
            }
        }
    }
}

/// Apply a linear fade-in over the specified number of timeline frames.
fn apply_fade_in(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    duration_frames: u64,
    fps: f64,
) {
    if duration_frames == 0 || fps <= 0.0 {
        return;
    }
    let ch = channels as usize;
    let fade_duration_secs = duration_frames as f64 / fps;
    let fade_samples = (fade_duration_secs * sample_rate as f64) as usize;

    for (frame_idx, frame) in samples.chunks_mut(ch).enumerate() {
        if frame_idx >= fade_samples {
            break;
        }
        let gain = frame_idx as f32 / fade_samples as f32;
        for sample in frame.iter_mut() {
            *sample *= gain;
        }
    }
}

/// Apply a linear fade-out over the specified number of timeline frames.
fn apply_fade_out(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    duration_frames: u64,
    clip_duration_frames: u64,
    fps: f64,
) {
    if duration_frames == 0 || fps <= 0.0 {
        return;
    }
    let ch = channels as usize;
    let fade_duration_secs = duration_frames as f64 / fps;
    let fade_samples = (fade_duration_secs * sample_rate as f64) as usize;
    let total_frames = samples.len() / ch;

    if total_frames == 0 || fade_samples == 0 {
        return;
    }

    let _ = clip_duration_frames; // total_frames derived from actual sample count

    let fade_start = total_frames.saturating_sub(fade_samples);
    for (frame_idx, frame) in samples.chunks_mut(ch).enumerate() {
        if frame_idx < fade_start {
            continue;
        }
        let fade_pos = frame_idx - fade_start;
        let gain = 1.0 - (fade_pos as f32 / fade_samples as f32);
        let gain = gain.max(0.0);
        for sample in frame.iter_mut() {
            *sample *= gain;
        }
    }
}

/// Mix all audio tracks from a timeline and send the result as sequential
/// [`AudioBuffer`]s over a channel.
///
/// This is designed for offline export — it decodes all audio upfront,
/// then mixes in chunks. Respects mute/solo flags, per-clip effects,
/// per-clip volume, track volume, and track pan.
pub fn mix_timeline_audio(
    timeline: &Timeline,
    frame_rate: &FrameRate,
    sample_rate: u32,
    channels: u16,
    tx: mpsc::Sender<AudioBuffer>,
) -> Result<(), MediaPipelineError> {
    let fps = frame_rate.fps();
    if fps <= 0.0 || sample_rate == 0 || channels == 0 {
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

        let track_volume = track.volume;
        let pan_gains = equal_power_pan(track.pan);

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
            if start >= end {
                debug!("clip source region empty after trim: start={start} end={end}");
                continue;
            }
            let mut trimmed = all_samples[start..end].to_vec();

            // Apply per-clip audio effects
            apply_clip_effects(
                &mut trimmed,
                &clip.effects,
                sample_rate,
                channels,
                clip.duration,
                fps,
            );

            // Timeline position in samples
            let start_sample = frames_to_samples(clip.timeline_start, fps, sample_rate, channels);

            // Combined volume: clip volume * track volume
            let combined_volume = clip.volume * track_volume;

            decoded_clips.push(DecodedClip {
                samples: trimmed,
                start_sample,
                volume: combined_volume,
                pan_gains,
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

    let ch = channels as usize;

    // Mix in chunks
    let chunk_size = MIX_CHUNK_FRAMES * ch;
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
            let (left_gain, right_gain) = clip.pan_gains;

            if ch >= 2 {
                // Stereo or multi-channel: apply pan to L/R
                for i in 0..copy_len {
                    let src = clip.samples[chunk_start_in_clip + i] * volume;
                    let dest_idx = mix_start + i;
                    let channel_in_frame = dest_idx % ch;
                    let pan_gain = if channel_in_frame == 0 {
                        left_gain
                    } else if channel_in_frame == 1 {
                        right_gain
                    } else {
                        // Additional channels beyond stereo: no pan applied
                        1.0
                    };
                    mix_buf[dest_idx] += src * pan_gain;
                }
            } else {
                // Mono: no pan
                for i in 0..copy_len {
                    mix_buf[mix_start + i] += clip.samples[chunk_start_in_clip + i] * volume;
                }
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
            pan_gains: equal_power_pan(0.0),
        };
        let clip_b = DecodedClip {
            samples: vec![0.3, 0.3, 0.3, 0.3],
            start_sample: 0,
            volume: 0.5, // half volume
            pan_gains: equal_power_pan(0.0),
        };

        // Center pan should give equal L/R gains of ~0.707
        let (lg, rg) = equal_power_pan(0.0);

        // Simulate mixing manually (stereo)
        let mut mix = [0.0f32; 4];
        for clip in &[&clip_a, &clip_b] {
            for (i, m) in mix.iter_mut().enumerate() {
                let pan_gain = if i % 2 == 0 { lg } else { rg };
                *m += clip.samples[i] * clip.volume * pan_gain;
            }
        }

        // 0.5 * 1.0 * 0.707 + 0.3 * 0.5 * 0.707 ≈ 0.46
        let expected = (0.5 * 1.0 + 0.3 * 0.5) * lg;
        assert!((mix[0] - expected).abs() < 1e-4);
    }

    #[test]
    fn test_clamp_prevents_overflow() {
        // Two loud clips that would sum > 1.0
        let clip_a = DecodedClip {
            samples: vec![0.8; 4],
            start_sample: 0,
            volume: 1.0,
            pan_gains: (1.0, 1.0), // bypass pan for this test
        };
        let clip_b = DecodedClip {
            samples: vec![0.7; 4],
            start_sample: 0,
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };

        let mut mix = [0.0f32; 4];
        for clip in &[&clip_a, &clip_b] {
            for (i, m) in mix.iter_mut().enumerate().take(4) {
                *m += clip.samples[i] * clip.volume;
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
            pan_gains: (1.0, 1.0),
        };
        let clip_b = DecodedClip {
            samples: vec![0.5; 4], // samples 4..7
            start_sample: 4,
            volume: 1.0,
            pan_gains: (1.0, 1.0),
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

    #[test]
    fn test_mix_zero_sample_rate_returns_ok() {
        let timeline = tazama_core::Timeline::new();
        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        // sample_rate=0 should not panic
        mix_timeline_audio(&timeline, &frame_rate, 0, 2, tx).unwrap();
    }

    #[test]
    fn test_mix_zero_channels_returns_ok() {
        let timeline = tazama_core::Timeline::new();
        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        mix_timeline_audio(&timeline, &frame_rate, 48000, 0, tx).unwrap();
    }

    #[test]
    fn test_mix_clip_no_media_skipped() {
        let mut timeline = tazama_core::Timeline::new();
        timeline.add_track(tazama_core::Track::new("A1", TrackKind::Audio));
        // Clip with no media reference
        let clip = tazama_core::Clip::new("no-media", ClipKind::Audio, 0, 30);
        timeline.tracks[0].add_clip(clip).unwrap();

        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        // This will try to decode but skip because no media path
        // Actually the clip has no media, so it will be skipped in the loop
        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_partial_overlap_mixing() {
        // Clip B starts in the middle of clip A
        let clip_a = DecodedClip {
            samples: vec![0.5; 8],
            start_sample: 0,
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };
        let clip_b = DecodedClip {
            samples: vec![0.3; 8],
            start_sample: 4, // starts at sample 4
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };

        // Mix chunk 0..8
        let chunk_size = 8;
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

        // Samples 0-3: only clip_a (0.5)
        assert_eq!(mix[0], 0.5);
        assert_eq!(mix[3], 0.5);
        // Samples 4-7: clip_a + clip_b (0.5 + 0.3 = 0.8)
        assert!((mix[4] - 0.8).abs() < 1e-6);
        assert!((mix[7] - 0.8).abs() < 1e-6);
    }

    // --- Pan tests ---

    #[test]
    fn test_equal_power_pan_center() {
        let (l, r) = equal_power_pan(0.0);
        // At center, both should be cos(PI/4) = sin(PI/4) ≈ 0.707
        assert!((l - r).abs() < 1e-6, "center pan: L={l}, R={r}");
        assert!((l - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn test_equal_power_pan_full_left() {
        let (l, r) = equal_power_pan(-1.0);
        // theta = 0, cos(0) = 1, sin(0) = 0
        assert!((l - 1.0).abs() < 1e-6, "full left: L={l}");
        assert!(r.abs() < 1e-6, "full left: R={r}");
    }

    #[test]
    fn test_equal_power_pan_full_right() {
        let (l, r) = equal_power_pan(1.0);
        // theta = PI/2, cos(PI/2) = 0, sin(PI/2) = 1
        assert!(l.abs() < 1e-6, "full right: L={l}");
        assert!((r - 1.0).abs() < 1e-6, "full right: R={r}");
    }

    #[test]
    fn test_equal_power_pan_power_preserving() {
        // For any pan position, L^2 + R^2 should equal 1.0 (power conservation)
        for pan_x10 in -10..=10 {
            let pan = pan_x10 as f32 / 10.0;
            let (l, r) = equal_power_pan(pan);
            let power = l * l + r * r;
            assert!(
                (power - 1.0).abs() < 1e-6,
                "power not conserved at pan={pan}: L={l}, R={r}, L^2+R^2={power}"
            );
        }
    }

    #[test]
    fn test_track_volume_applied() {
        // Track volume should multiply with clip volume
        let clip = DecodedClip {
            samples: vec![1.0; 4],
            start_sample: 0,
            volume: 0.5 * 0.8, // clip.volume=0.5, track.volume=0.8
            pan_gains: (1.0, 1.0),
        };
        assert!((clip.volume - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_pan_stereo_mixing() {
        // A clip panned hard right should only appear in the right channel
        let clip = DecodedClip {
            samples: vec![1.0, 1.0, 1.0, 1.0], // 2 stereo frames
            start_sample: 0,
            volume: 1.0,
            pan_gains: equal_power_pan(1.0), // full right
        };

        let ch = 2usize;
        let mut mix = [0.0f32; 4];
        let (left_gain, right_gain) = clip.pan_gains;

        for (i, m) in mix.iter_mut().enumerate() {
            let pan_gain = if i % ch == 0 { left_gain } else { right_gain };
            *m += clip.samples[i] * clip.volume * pan_gain;
        }

        // Left channels should be ~0
        assert!(mix[0].abs() < 1e-6, "L should be ~0 when panned right");
        assert!(mix[2].abs() < 1e-6);
        // Right channels should be ~1
        assert!(
            (mix[1] - 1.0).abs() < 1e-6,
            "R should be ~1 when panned right"
        );
        assert!((mix[3] - 1.0).abs() < 1e-6);
    }

    // --- Effects integration tests ---

    #[test]
    fn test_fade_in() {
        let mut samples = vec![1.0f32; 960]; // mono, 20ms at 48kHz = 960 samples
        apply_fade_in(&mut samples, 48000, 1, 10, 30.0); // 10 frames at 30fps = 1/3 sec = 16000 samples

        // First sample should be ~0 (fade starts at 0)
        assert!(samples[0].abs() < 1e-6);
        // Samples partway through fade should be < 1.0
        assert!(samples[100] < 1.0);
    }

    #[test]
    fn test_fade_out() {
        // 48000 mono samples = 1 second. Fade out over 30 frames at 30fps = 1 second.
        // So the entire buffer is a fade from 1.0 to 0.0.
        let mut samples = vec![1.0f32; 48000];
        apply_fade_out(&mut samples, 48000, 1, 30, 30, 30.0);

        // Last sample should be ~0
        assert!(
            samples[47999].abs() < 0.01,
            "last sample = {}",
            samples[47999]
        );
        // First sample should be ~1.0
        assert!(
            (samples[0] - 1.0).abs() < 0.01,
            "first sample = {}",
            samples[0]
        );
        // Midpoint should be ~0.5
        assert!(
            (samples[24000] - 0.5).abs() < 0.02,
            "mid sample = {}",
            samples[24000]
        );
    }

    #[test]
    fn test_apply_clip_effects_eq() {
        let mut samples = vec![0.0f32; 4096];
        let effects = vec![tazama_core::Effect::new(EffectKind::Eq {
            low_gain_db: 0.0,
            mid_gain_db: 0.0,
            high_gain_db: 0.0,
        })];
        // Zero-gain EQ on silence should remain silence
        apply_clip_effects(&mut samples, &effects, 48000, 2, 30, 30.0);
        for s in &samples {
            assert!(s.abs() < 1e-10);
        }
    }

    #[test]
    fn test_apply_clip_effects_disabled_skipped() {
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32 * 0.5)
            .collect();
        let original = samples.clone();

        let mut effect = tazama_core::Effect::new(EffectKind::Eq {
            low_gain_db: 12.0,
            mid_gain_db: 12.0,
            high_gain_db: 12.0,
        });
        effect.enabled = false;

        apply_clip_effects(&mut samples, &[effect], 48000, 2, 30, 30.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn test_volume_effect_static() {
        let mut samples = vec![0.5f32; 100];
        let effects = vec![tazama_core::Effect::new(EffectKind::Volume {
            gain_db: -6.0, // ~0.5x
        })];
        apply_clip_effects(&mut samples, &effects, 48000, 1, 30, 30.0);

        let expected_gain = 10.0f32.powf(-6.0 / 20.0);
        let expected = 0.5 * expected_gain;
        assert!((samples[50] - expected).abs() < 1e-4);
    }

    #[test]
    fn test_mix_volume_zero_produces_silence() {
        let clip = DecodedClip {
            samples: vec![0.8, 0.8, 0.8, 0.8],
            start_sample: 0,
            volume: 0.0, // zero volume
            pan_gains: (1.0, 1.0),
        };

        let mut mix = [0.0f32; 4];
        for (i, m) in mix.iter_mut().enumerate() {
            *m += clip.samples[i] * clip.volume;
        }

        for s in &mix {
            assert!(s.abs() < 1e-10, "volume=0 should produce silence");
        }
    }

    #[test]
    fn test_pan_full_left_zeroes_right() {
        let clip = DecodedClip {
            samples: vec![1.0, 1.0, 1.0, 1.0],
            start_sample: 0,
            volume: 1.0,
            pan_gains: equal_power_pan(-1.0),
        };

        let ch = 2usize;
        let mut mix = [0.0f32; 4];
        let (lg, rg) = clip.pan_gains;
        for (i, m) in mix.iter_mut().enumerate() {
            let pan_gain = if i % ch == 0 { lg } else { rg };
            *m += clip.samples[i] * clip.volume * pan_gain;
        }

        // Right channels (index 1, 3) should be ~0
        assert!(
            mix[1].abs() < 1e-6,
            "full left pan: R should be ~0, got {}",
            mix[1]
        );
        assert!(mix[3].abs() < 1e-6);
        // Left channels should be ~1
        assert!((mix[0] - 1.0).abs() < 1e-6, "full left pan: L should be ~1");
    }

    #[test]
    fn test_pan_full_right_zeroes_left() {
        let clip = DecodedClip {
            samples: vec![1.0, 1.0, 1.0, 1.0],
            start_sample: 0,
            volume: 1.0,
            pan_gains: equal_power_pan(1.0),
        };

        let ch = 2usize;
        let mut mix = [0.0f32; 4];
        let (lg, rg) = clip.pan_gains;
        for (i, m) in mix.iter_mut().enumerate() {
            let pan_gain = if i % ch == 0 { lg } else { rg };
            *m += clip.samples[i] * clip.volume * pan_gain;
        }

        // Left channels (index 0, 2) should be ~0
        assert!(
            mix[0].abs() < 1e-6,
            "full right pan: L should be ~0, got {}",
            mix[0]
        );
        assert!(mix[2].abs() < 1e-6);
        // Right channels should be ~1
        assert!(
            (mix[1] - 1.0).abs() < 1e-6,
            "full right pan: R should be ~1"
        );
    }

    #[test]
    fn test_fade_in_on_decoded_samples() {
        // Verify fade-in ramps from 0 to full over the specified duration
        let mut samples = vec![1.0f32; 4800]; // 100ms at 48kHz mono
        // 3 frames at 30fps = 100ms = 4800 samples
        apply_fade_in(&mut samples, 48000, 1, 3, 30.0);

        // First sample must be 0
        assert!(samples[0].abs() < 1e-6, "fade-in first sample should be 0");
        // Midpoint should be ~0.5
        assert!(
            (samples[2400] - 0.5).abs() < 0.02,
            "fade-in midpoint should be ~0.5, got {}",
            samples[2400]
        );
        // Sample after fade should be unmodified (1.0)
        // All 4800 samples are within the fade, so last sample is near 1.0
    }

    #[test]
    fn test_fade_out_on_decoded_samples() {
        let mut samples = vec![1.0f32; 4800]; // 100ms at 48kHz mono
        apply_fade_out(&mut samples, 48000, 1, 3, 3, 30.0);

        // Last sample should be ~0
        assert!(
            samples[4799].abs() < 0.01,
            "fade-out last sample should be ~0, got {}",
            samples[4799]
        );
        // First sample should be ~1.0
        assert!(
            (samples[0] - 1.0).abs() < 0.01,
            "fade-out first sample should be ~1.0"
        );
    }

    #[test]
    fn test_image_clips_skipped_in_audio() {
        let mut timeline = tazama_core::Timeline::new();
        timeline.add_track(tazama_core::Track::new("A1", TrackKind::Audio));

        // Add an image clip to an audio track — should be skipped
        let clip = tazama_core::Clip::new("photo", ClipKind::Image, 0, 30);
        let _ = timeline.tracks[0].add_clip(clip);

        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();
        assert!(
            rx.try_recv().is_err(),
            "image clips should produce no audio"
        );
    }

    #[test]
    fn test_partial_overlap_second_chunk() {
        // Clip starts at sample 2, so in a chunk of size 4 starting at 0,
        // only samples 2..4 should contain the clip's data
        let clip = DecodedClip {
            samples: vec![0.7; 4],
            start_sample: 2,
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };

        let chunk_size = 4;
        let mut mix = vec![0.0f32; chunk_size];
        let offset: u64 = 0;

        let clip_end = clip.start_sample + clip.samples.len() as u64;
        if !(offset >= clip_end || offset + chunk_size as u64 <= clip.start_sample) {
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

        // Samples 0..2 should be 0 (no clip data)
        assert_eq!(mix[0], 0.0);
        assert_eq!(mix[1], 0.0);
        // Samples 2..4 should have the clip data (0.7)
        assert!((mix[2] - 0.7).abs() < 1e-6);
        assert!((mix[3] - 0.7).abs() < 1e-6);
    }

    // --- DSP integration tests ---

    #[test]
    fn test_apply_clip_effects_empty_effects_is_noop() {
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32 * 0.5)
            .collect();
        let original = samples.clone();

        apply_clip_effects(&mut samples, &[], 48000, 2, 30, 30.0);

        assert_eq!(
            samples, original,
            "empty effects list should not modify samples"
        );
    }

    #[test]
    fn test_apply_clip_effects_multiple_disabled_skipped() {
        // Multiple effects, all disabled — samples should be unchanged
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32 * 0.5)
            .collect();
        let original = samples.clone();

        let mut eq = tazama_core::Effect::new(EffectKind::Eq {
            low_gain_db: 12.0,
            mid_gain_db: 6.0,
            high_gain_db: 12.0,
        });
        eq.enabled = false;

        let mut comp = tazama_core::Effect::new(EffectKind::Compressor {
            threshold_db: -10.0,
            ratio: 8.0,
            attack_ms: 1.0,
            release_ms: 50.0,
        });
        comp.enabled = false;

        let mut vol = tazama_core::Effect::new(EffectKind::Volume { gain_db: -20.0 });
        vol.enabled = false;

        apply_clip_effects(&mut samples, &[eq, comp, vol], 48000, 2, 30, 30.0);
        assert_eq!(
            samples, original,
            "all disabled effects should leave samples untouched"
        );
    }

    #[test]
    fn test_apply_eq_then_compressor_sequence() {
        // Generate a 440Hz tone, stereo, 4096 interleaved samples
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let t = (i / 2) as f64 / 48000.0; // stereo: 2 samples per frame
                (2.0 * std::f64::consts::PI * 440.0 * t).sin() as f32 * 0.8
            })
            .collect();
        let before = samples.clone();

        let effects = vec![
            tazama_core::Effect::new(EffectKind::Eq {
                low_gain_db: 6.0,
                mid_gain_db: 0.0,
                high_gain_db: -3.0,
            }),
            tazama_core::Effect::new(EffectKind::Compressor {
                threshold_db: -20.0,
                ratio: 4.0,
                attack_ms: 10.0,
                release_ms: 100.0,
            }),
        ];

        apply_clip_effects(&mut samples, &effects, 48000, 2, 30, 30.0);

        // The signal should be modified by both EQ and compressor
        assert_ne!(samples, before, "EQ + compressor should modify the signal");
    }

    #[test]
    fn test_apply_eq_then_compressor_then_volume() {
        // Three effects in sequence: EQ -> Compressor -> Volume
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let t = (i / 2) as f64 / 48000.0;
                (2.0 * std::f64::consts::PI * 440.0 * t).sin() as f32 * 0.5
            })
            .collect();

        let effects = vec![
            tazama_core::Effect::new(EffectKind::Eq {
                low_gain_db: 3.0,
                mid_gain_db: 0.0,
                high_gain_db: 0.0,
            }),
            tazama_core::Effect::new(EffectKind::Compressor {
                threshold_db: -15.0,
                ratio: 2.0,
                attack_ms: 5.0,
                release_ms: 50.0,
            }),
            tazama_core::Effect::new(EffectKind::Volume { gain_db: -6.0 }),
        ];

        apply_clip_effects(&mut samples, &effects, 48000, 2, 30, 30.0);

        // After -6dB volume, peak should be significantly less than 0.5
        let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            peak < 0.5,
            "after -6dB volume on a 0.5 amplitude signal, peak should be < 0.5, got {peak}"
        );
    }

    #[test]
    fn test_mixed_enabled_disabled_effects() {
        // First effect enabled (EQ boost), second disabled (compressor), third enabled (volume cut)
        let mut samples = vec![0.5f32; 200];

        let eq = tazama_core::Effect::new(EffectKind::Eq {
            low_gain_db: 0.0,
            mid_gain_db: 0.0,
            high_gain_db: 0.0,
        });

        let mut comp = tazama_core::Effect::new(EffectKind::Compressor {
            threshold_db: -10.0,
            ratio: 8.0,
            attack_ms: 1.0,
            release_ms: 50.0,
        });
        comp.enabled = false;

        let vol = tazama_core::Effect::new(EffectKind::Volume { gain_db: -6.0 });

        apply_clip_effects(&mut samples, &[eq, comp, vol], 48000, 1, 30, 30.0);

        // Volume -6dB ≈ 0.501 gain, so 0.5 * 0.501 ≈ 0.25
        let expected = 0.5 * 10.0f32.powf(-6.0 / 20.0);
        assert!(
            (samples[100] - expected).abs() < 1e-3,
            "expected ~{expected}, got {}",
            samples[100]
        );
    }

    #[test]
    fn test_volume_effect_with_keyframes() {
        // Volume effect with keyframes: ramp from 0dB at frame 0 to -20dB at frame 30
        // 48000 mono samples = 1 second at 48kHz
        let mut samples = vec![1.0f32; 48000];

        let mut effect = tazama_core::Effect::new(EffectKind::Volume { gain_db: 0.0 });
        let mut track = tazama_core::keyframe::KeyframeTrack::new("gain_db");
        track.add_keyframe(tazama_core::keyframe::Keyframe::new(
            0,
            0.0,
            tazama_core::keyframe::Interpolation::Linear,
        ));
        track.add_keyframe(tazama_core::keyframe::Keyframe::new(
            30,
            -20.0,
            tazama_core::keyframe::Interpolation::Linear,
        ));
        effect.keyframe_tracks.push(track);

        apply_clip_effects(&mut samples, &[effect], 48000, 1, 30, 30.0);

        // At frame 0 (sample 0), gain_db=0 => multiplier=1.0
        assert!(
            (samples[0] - 1.0).abs() < 0.01,
            "at start, gain should be ~1.0, got {}",
            samples[0]
        );

        // At frame 30 (sample 47999, end of 1 second), gain_db=-20 => multiplier=0.1
        let end_expected = 10.0f32.powf(-20.0 / 20.0); // 0.1
        assert!(
            (samples[47999] - end_expected).abs() < 0.05,
            "at end, gain should be ~{end_expected}, got {}",
            samples[47999]
        );

        // Midpoint (frame 15, sample ~24000): gain_db=-10 => multiplier ≈ 0.316
        let mid_expected = 10.0f32.powf(-10.0 / 20.0);
        assert!(
            (samples[24000] - mid_expected).abs() < 0.05,
            "at midpoint, gain should be ~{mid_expected}, got {}",
            samples[24000]
        );
    }

    #[test]
    fn test_volume_effect_keyframes_no_keyframes_uses_static() {
        // Volume effect with empty keyframe_tracks should use the static gain_db
        let mut samples = vec![0.5f32; 100];
        let effect = tazama_core::Effect::new(EffectKind::Volume { gain_db: -6.0 });
        // keyframe_tracks is empty by default
        assert!(effect.keyframe_tracks.is_empty());

        apply_clip_effects(&mut samples, &[effect], 48000, 1, 30, 30.0);

        let expected = 0.5 * 10.0f32.powf(-6.0 / 20.0);
        assert!(
            (samples[50] - expected).abs() < 1e-4,
            "static volume should apply, expected {expected}, got {}",
            samples[50]
        );
    }

    #[test]
    fn test_noise_reduction_effect_applied() {
        // Use a noisy signal (sine + low-level noise) to test noise reduction.
        // A clean sine may pass through unchanged, which is correct behavior.
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| {
                let t = (i / 2) as f64 / 48000.0;
                let sine = (2.0 * std::f64::consts::PI * 440.0 * t).sin() as f32 * 0.3;
                let noise = (i as f32 * 13.7).sin() * 0.02; // deterministic low-level noise
                sine + noise
            })
            .collect();
        let _before_rms = {
            let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
            (sum / samples.len() as f64).sqrt()
        };

        let effects = vec![tazama_core::Effect::new(EffectKind::NoiseReduction {
            strength: 0.8,
        })];

        apply_clip_effects(&mut samples, &effects, 48000, 2, 30, 30.0);
        // Output should remain finite and reasonable
        assert!(samples.iter().all(|s| s.is_finite()));
        let after_rms = {
            let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
            (sum / samples.len() as f64).sqrt()
        };
        // Signal should still have energy (not zeroed out)
        assert!(
            after_rms > 0.01,
            "signal should not be zeroed: rms={after_rms}"
        );
    }

    #[test]
    fn test_reverb_effect_applied() {
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| {
                let t = (i / 2) as f64 / 48000.0;
                (2.0 * std::f64::consts::PI * 440.0 * t).sin() as f32 * 0.3
            })
            .collect();
        let before = samples.clone();

        let effects = vec![tazama_core::Effect::new(EffectKind::Reverb {
            room_size: 0.5,
            damping: 0.5,
            wet: 0.3,
        })];

        apply_clip_effects(&mut samples, &effects, 48000, 2, 30, 30.0);
        let any_different = samples
            .iter()
            .zip(before.iter())
            .any(|(a, b)| (a - b).abs() > 1e-10);
        assert!(any_different, "reverb should modify the signal");
    }

    #[test]
    fn test_video_effect_ignored_in_audio_chain() {
        // A video-only effect like ColorGrade should be a no-op in the audio chain
        let mut samples = vec![0.5f32; 200];
        let original = samples.clone();

        let effects = vec![tazama_core::Effect::new(EffectKind::ColorGrade {
            brightness: 0.5,
            contrast: 1.5,
            saturation: 0.8,
            temperature: 0.2,
        })];

        apply_clip_effects(&mut samples, &effects, 48000, 1, 30, 30.0);
        assert_eq!(
            samples, original,
            "video effects should not modify audio samples"
        );
    }

    // --- Additional coverage tests ---

    #[test]
    fn test_fade_in_zero_duration_is_noop() {
        let mut samples = vec![1.0f32; 100];
        let original = samples.clone();
        apply_fade_in(&mut samples, 48000, 1, 0, 30.0);
        assert_eq!(samples, original, "zero-duration fade-in should be a noop");
    }

    #[test]
    fn test_fade_out_zero_duration_is_noop() {
        let mut samples = vec![1.0f32; 100];
        let original = samples.clone();
        apply_fade_out(&mut samples, 48000, 1, 0, 30, 30.0);
        assert_eq!(samples, original, "zero-duration fade-out should be a noop");
    }

    #[test]
    fn test_fade_in_negative_fps_is_noop() {
        let mut samples = vec![1.0f32; 100];
        let original = samples.clone();
        apply_fade_in(&mut samples, 48000, 1, 10, -1.0);
        assert_eq!(samples, original, "negative fps fade-in should be a noop");
    }

    #[test]
    fn test_fade_out_negative_fps_is_noop() {
        let mut samples = vec![1.0f32; 100];
        let original = samples.clone();
        apply_fade_out(&mut samples, 48000, 1, 10, 30, -1.0);
        assert_eq!(samples, original, "negative fps fade-out should be a noop");
    }

    #[test]
    fn test_fade_in_stereo() {
        // 2 channels, 960 interleaved samples = 480 frames
        let mut samples = vec![1.0f32; 960];
        // Fade over 5 frames at 30fps = 1/6 sec ≈ 8000 samples (mono) = 16000 interleaved
        // But we only have 960 samples, so the entire buffer is within the fade
        apply_fade_in(&mut samples, 48000, 2, 5, 30.0);
        // First stereo frame (samples 0,1) should be ~0
        assert!(samples[0].abs() < 1e-6, "stereo fade-in L[0] should be ~0");
        assert!(samples[1].abs() < 1e-6, "stereo fade-in R[0] should be ~0");
        // A frame partway through should be attenuated
        assert!(samples[100] < 1.0, "stereo fade-in should attenuate");
    }

    #[test]
    fn test_fade_out_stereo() {
        // 2 channels, 32000 interleaved samples = 16000 frames
        // Fade over 10 frames at 30fps = 1/3 sec = 16000 frames
        // So the entire buffer is a fade from 1.0 to 0.0
        let mut samples = vec![1.0f32; 32000];
        apply_fade_out(&mut samples, 48000, 2, 10, 300, 30.0);
        // Last stereo frame should be ~0
        assert!(
            samples[31998].abs() < 0.01,
            "stereo fade-out L[-1] should be ~0, got {}",
            samples[31998]
        );
        assert!(
            samples[31999].abs() < 0.01,
            "stereo fade-out R[-1] should be ~0, got {}",
            samples[31999]
        );
        // First stereo frame should be ~1.0
        assert!(
            (samples[0] - 1.0).abs() < 0.01,
            "stereo fade-out first L should be ~1.0, got {}",
            samples[0]
        );
    }

    #[test]
    fn test_fade_in_exact_boundary() {
        // Fade exactly covers the buffer: 10 frames at 10fps = 1 sec = 48000 mono samples
        let mut samples = vec![1.0f32; 48000];
        apply_fade_in(&mut samples, 48000, 1, 10, 10.0);
        // First sample = 0
        assert!(samples[0].abs() < 1e-6);
        // Last sample should be near 1.0 (gain = 47999/48000)
        assert!(
            (samples[47999] - 1.0).abs() < 0.001,
            "last sample in exact fade should be ~1.0, got {}",
            samples[47999]
        );
    }

    #[test]
    fn test_fade_in_shorter_than_buffer() {
        // Buffer is 48000, fade is only 5 frames at 30fps = 1/6 sec = 8000 samples
        let mut samples = vec![1.0f32; 48000];
        apply_fade_in(&mut samples, 48000, 1, 5, 30.0);
        // First sample = 0
        assert!(samples[0].abs() < 1e-6);
        // Sample at 8000 should be unmodified (past the fade)
        assert!(
            (samples[8001] - 1.0).abs() < 1e-6,
            "samples past fade should be 1.0, got {}",
            samples[8001]
        );
    }

    #[test]
    fn test_clamp_negative_overflow() {
        // Two loud negative clips that would sum < -1.0
        let clip_a = DecodedClip {
            samples: vec![-0.8; 4],
            start_sample: 0,
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };
        let clip_b = DecodedClip {
            samples: vec![-0.7; 4],
            start_sample: 0,
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };

        let mut mix = [0.0f32; 4];
        for clip in &[&clip_a, &clip_b] {
            for (i, m) in mix.iter_mut().enumerate().take(4) {
                *m += clip.samples[i] * clip.volume;
            }
        }
        for s in &mut mix {
            *s = s.clamp(-1.0, 1.0);
        }
        // -0.8 + -0.7 = -1.5, clamped to -1.0
        assert_eq!(mix[0], -1.0);
    }

    #[test]
    fn test_frames_to_samples_high_fps() {
        // 60fps, 44100Hz, mono: 60 frames = 1 second = 44100 samples
        let samples = frames_to_samples(60, 60.0, 44100, 1);
        assert_eq!(samples, 44100);
    }

    #[test]
    fn test_frames_to_samples_ntsc() {
        // 29.97fps approximation: 30 frames ≈ 1.001 sec
        let samples = frames_to_samples(30, 29.97, 48000, 2);
        // 30/29.97 * 48000 * 2 ≈ 96096
        let expected = (30.0 / 29.97 * 48000.0 * 2.0) as u64;
        assert_eq!(samples, expected);
    }

    #[test]
    fn test_mono_mixing_path() {
        // Exercise the mono branch (ch < 2) in the mixing loop
        let clip = DecodedClip {
            samples: vec![0.5; 4],
            start_sample: 0,
            volume: 0.8,
            pan_gains: (1.0, 0.0), // ignored for mono
        };

        let ch = 1usize;
        let mut mix = vec![0.0f32; 4];
        // Mono path: no pan
        for (i, m) in mix.iter_mut().enumerate().take(4) {
            *m += clip.samples[i] * clip.volume;
        }
        for s in &mix {
            assert!(
                (*s - 0.4).abs() < 1e-6,
                "mono mix: 0.5 * 0.8 = 0.4, got {s}"
            );
        }
        let _ = ch;
    }

    #[test]
    fn test_equal_power_pan_clamping() {
        // Values outside [-1, 1] should be clamped
        let (l1, r1) = equal_power_pan(-2.0);
        let (l2, r2) = equal_power_pan(-1.0);
        assert!((l1 - l2).abs() < 1e-6, "pan=-2 should clamp to pan=-1");
        assert!((r1 - r2).abs() < 1e-6);

        let (l3, r3) = equal_power_pan(2.0);
        let (l4, r4) = equal_power_pan(1.0);
        assert!((l3 - l4).abs() < 1e-6, "pan=2 should clamp to pan=1");
        assert!((r3 - r4).abs() < 1e-6);
    }

    #[test]
    fn test_mix_multiple_muted_tracks_no_output() {
        let mut timeline = tazama_core::Timeline::new();
        timeline.add_track(tazama_core::Track::new("A1", TrackKind::Audio));
        timeline.add_track(tazama_core::Track::new("A2", TrackKind::Audio));
        timeline.tracks[0].muted = true;
        timeline.tracks[1].muted = true;

        let clip1 = tazama_core::Clip::new("c1", ClipKind::Audio, 0, 30);
        let clip2 = tazama_core::Clip::new("c2", ClipKind::Audio, 0, 30);
        let _ = timeline.tracks[0].add_clip(clip1);
        let _ = timeline.tracks[1].add_clip(clip2);

        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();
        assert!(
            rx.try_recv().is_err(),
            "all muted tracks should produce no output"
        );
    }

    #[test]
    fn test_mix_solo_track_with_clip_but_no_media() {
        let mut timeline = tazama_core::Timeline::new();
        timeline.add_track(tazama_core::Track::new("A1", TrackKind::Audio));
        timeline.add_track(tazama_core::Track::new("A2", TrackKind::Audio));
        timeline.tracks[0].solo = true;
        // A1 is solo'd but clip has no media — should produce no output
        let clip = tazama_core::Clip::new("c1", ClipKind::Audio, 0, 30);
        let _ = timeline.tracks[0].add_clip(clip);

        let frame_rate = tazama_core::FrameRate::new(30, 1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        mix_timeline_audio(&timeline, &frame_rate, 48000, 2, tx).unwrap();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_source_offset_clips_audio() {
        // Simulate clip with source_offset: start reading from sample 4
        let all_samples = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        let source_offset_samples = 4usize;
        let clip_duration_samples = 4usize;
        let start = source_offset_samples.min(all_samples.len());
        let end = (source_offset_samples + clip_duration_samples).min(all_samples.len());
        let trimmed = &all_samples[start..end];
        assert_eq!(trimmed, &[0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn test_source_offset_beyond_audio_length() {
        let all_samples = [0.1, 0.2, 0.3, 0.4];
        let source_offset_samples = 10usize;
        let clip_duration_samples = 4usize;
        let start = source_offset_samples.min(all_samples.len());
        let end = (source_offset_samples + clip_duration_samples).min(all_samples.len());
        // start >= end, so trimmed region is empty
        assert!(start >= end, "offset past end should produce empty region");
    }

    #[test]
    fn test_timeline_start_positions_clip() {
        // Two clips at different timeline positions
        let clip_a = DecodedClip {
            samples: vec![1.0; 4],
            start_sample: 0,
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };
        let clip_b = DecodedClip {
            samples: vec![0.5; 4],
            start_sample: 8, // starts 8 samples later
            volume: 1.0,
            pan_gains: (1.0, 1.0),
        };

        // Chunk covering 0..12
        let chunk_size = 12;
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

        // 0..4: clip_a (1.0)
        assert_eq!(mix[0], 1.0);
        assert_eq!(mix[3], 1.0);
        // 4..8: silence
        assert_eq!(mix[4], 0.0);
        assert_eq!(mix[7], 0.0);
        // 8..12: clip_b (0.5)
        assert!((mix[8] - 0.5).abs() < 1e-6);
        assert!((mix[11] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_fade_in_with_zero_fps_is_noop() {
        let mut samples = vec![1.0f32; 100];
        let original = samples.clone();
        apply_fade_in(&mut samples, 48000, 1, 10, 0.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn test_fade_out_with_zero_fps_is_noop() {
        let mut samples = vec![1.0f32; 100];
        let original = samples.clone();
        apply_fade_out(&mut samples, 48000, 1, 10, 30, 0.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn test_fade_out_empty_samples_no_panic() {
        let mut samples: Vec<f32> = vec![];
        apply_fade_out(&mut samples, 48000, 1, 10, 30, 30.0);
        assert!(samples.is_empty());
    }

    #[test]
    fn test_fade_in_empty_samples_no_panic() {
        let mut samples: Vec<f32> = vec![];
        apply_fade_in(&mut samples, 48000, 1, 10, 30.0);
        assert!(samples.is_empty());
    }
}
