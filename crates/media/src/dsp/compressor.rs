//! Dynamic range compressor — backed by dhvani.

use dhvani::buffer::AudioBuffer;
use dhvani::dsp::{Compressor, CompressorParams};

/// Apply dynamic range compression in-place on interleaved f32 samples.
pub fn apply_compressor(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
) {
    if samples.is_empty() || channels == 0 || sample_rate == 0 {
        return;
    }

    let params = CompressorParams {
        threshold_db,
        ratio: ratio.max(1.0),
        attack_ms: attack_ms.max(0.0),
        release_ms: release_ms.max(0.0),
        makeup_gain_db: 0.0,
        knee_db: 0.0,
    };
    let mut comp = Compressor::new(params, sample_rate);

    if let Ok(mut buf) =
        AudioBuffer::from_interleaved(samples.to_vec(), channels as u32, sample_rate)
    {
        comp.process(&mut buf);
        samples.copy_from_slice(&buf.samples);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(freq: f64, sr: u32, n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| (i as f64 / sr as f64 * freq * 2.0 * std::f64::consts::PI).sin() as f32 * amp)
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        (sum / samples.len() as f64).sqrt() as f32
    }

    #[test]
    fn silent_input() {
        let mut samples = vec![0.0f32; 4800];
        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 10.0, 100.0);
        assert!(samples.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn quiet_signal_unchanged() {
        let mut samples = make_sine(440.0, 48000, 4800, 0.01);
        let before = rms(&samples);
        apply_compressor(&mut samples, 48000, 1, -10.0, 4.0, 10.0, 100.0);
        let after = rms(&samples);
        assert!((after - before).abs() / before < 0.1);
    }

    #[test]
    fn loud_signal_compressed() {
        let mut samples = make_sine(440.0, 48000, 48000, 0.9);
        let before = rms(&samples);
        apply_compressor(&mut samples, 48000, 1, -20.0, 8.0, 1.0, 10.0);
        let after = rms(&samples);
        assert!(
            after < before,
            "loud signal should be compressed: {before} -> {after}"
        );
    }

    #[test]
    fn stereo() {
        let mut samples = make_sine(440.0, 48000, 9600, 0.8);
        apply_compressor(&mut samples, 48000, 2, -20.0, 4.0, 10.0, 100.0);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn empty_input() {
        let mut samples: Vec<f32> = vec![];
        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 10.0, 100.0);
    }

    #[test]
    fn output_finite() {
        let mut samples = make_sine(440.0, 48000, 4800, 1.0);
        apply_compressor(&mut samples, 48000, 1, -40.0, 20.0, 0.1, 0.1);
        assert!(samples.iter().all(|s| s.is_finite()));
    }
}
