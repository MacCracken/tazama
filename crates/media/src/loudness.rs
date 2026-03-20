use bytes::Bytes;
use std::time::Duration;

use crate::decode::AudioBuffer;

/// Measure the integrated loudness (LUFS) of an audio buffer.
pub fn measure_loudness(buf: &AudioBuffer) -> f64 {
    let tarang_buf = to_tarang_buffer(buf);
    let metrics = tarang::audio::loudness::measure_loudness(&tarang_buf);
    metrics.integrated_lufs
}

/// Normalize audio samples in-place to a target loudness in LUFS.
///
/// Converts the samples to a tarang `AudioBuffer`, runs loudness normalization,
/// and copies the result back into the original slice.
pub fn normalize_audio(
    samples: &mut [f32],
    channels: u16,
    sample_rate: u32,
    target_lufs: f32,
) {
    if samples.is_empty() {
        return;
    }

    let byte_data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let num_frames = samples.len() / channels.max(1) as usize;

    let tarang_buf = tarang::core::AudioBuffer {
        data: Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels,
        sample_rate,
        num_frames,
        timestamp: Duration::ZERO,
    };

    if let Ok(normalized) =
        tarang::audio::loudness::normalize_loudness(&tarang_buf, target_lufs as f64)
    {
        // Copy normalized samples back
        for (i, chunk) in normalized.data.chunks_exact(4).enumerate() {
            if i < samples.len() {
                samples[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
        }
    }
}

fn to_tarang_buffer(buf: &AudioBuffer) -> tarang::core::AudioBuffer {
    let byte_data: Vec<u8> = buf.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let num_frames = buf.samples.len() / buf.channels.max(1) as usize;
    tarang::core::AudioBuffer {
        data: Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels: buf.channels,
        sample_rate: buf.sample_rate,
        num_frames,
        timestamp: Duration::from_nanos(buf.timestamp_ns),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_loudness_silence() {
        let buf = AudioBuffer {
            sample_rate: 48000,
            channels: 2,
            samples: vec![0.0; 96000],
            timestamp_ns: 0,
        };
        let lufs = measure_loudness(&buf);
        // Silence should be very quiet (large negative LUFS)
        assert!(lufs < -50.0, "silence should measure very low LUFS: {lufs}");
    }

    #[test]
    fn normalize_audio_noop_on_empty() {
        let mut samples = vec![];
        normalize_audio(&mut samples, 2, 48000, -14.0);
        assert!(samples.is_empty());
    }

    #[test]
    fn normalize_audio_modifies_samples() {
        // Loud signal
        let mut samples: Vec<f32> = (0..48000).map(|i| (i as f32 * 0.01).sin() * 0.9).collect();
        let original = samples.clone();
        normalize_audio(&mut samples, 1, 48000, -23.0);
        // After normalizing to -23 LUFS, the samples should differ from the original
        let changed = samples.iter().zip(&original).any(|(a, b)| (a - b).abs() > 1e-6);
        assert!(changed, "normalization should modify samples");
    }
}
