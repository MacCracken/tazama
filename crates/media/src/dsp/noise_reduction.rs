//! Spectral gating noise reduction — backed by dhvani.
//!
//! Uses dhvani's STFT-based noise reduction with configurable strength.

use dhvani::buffer::AudioBuffer;

/// Apply spectral noise reduction in-place on interleaved f32 samples.
///
/// `strength` controls aggressiveness (0.0 = off, 1.0 = maximum reduction).
pub fn apply_noise_reduction(samples: &mut [f32], channels: u16, strength: f32) {
    if samples.is_empty() || channels == 0 || strength <= 0.0 {
        return;
    }

    // dhvani's noise_reduce operates on an AudioBuffer
    // Use 48000 as default sample rate (noise reduction is rate-independent for gating)
    let sample_rate = 48000;

    if let Ok(mut buf) =
        AudioBuffer::from_interleaved(samples.to_vec(), channels as u32, sample_rate)
    {
        dhvani::dsp::noise_reduce(&mut buf, strength);
        samples.copy_from_slice(&buf.samples);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input_stays_silent() {
        let mut samples = vec![0.0f32; 4096];
        apply_noise_reduction(&mut samples, 1, 0.5);
        assert!(samples.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn zero_strength_noop() {
        let original: Vec<f32> = (0..4096)
            .map(|i| (i as f64 / 48000.0 * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32)
            .collect();
        let mut samples = original.clone();
        apply_noise_reduction(&mut samples, 1, 0.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn strong_signal_preserved() {
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| (i as f64 / 48000.0 * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32 * 0.8)
            .collect();
        let before_rms = {
            let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
            (sum / samples.len() as f64).sqrt()
        };
        apply_noise_reduction(&mut samples, 1, 0.3);
        let after_rms = {
            let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
            (sum / samples.len() as f64).sqrt()
        };
        // Strong signal should retain most energy
        assert!(
            after_rms > before_rms * 0.5,
            "strong signal should be mostly preserved: {before_rms} -> {after_rms}"
        );
    }

    #[test]
    fn output_finite() {
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| (i as f64 / 48000.0 * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32)
            .collect();
        apply_noise_reduction(&mut samples, 1, 1.0);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn empty_input() {
        let mut samples: Vec<f32> = vec![];
        apply_noise_reduction(&mut samples, 1, 0.5);
    }

    #[test]
    fn stereo() {
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| (i as f64 / 48000.0 * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32 * 0.5)
            .collect();
        apply_noise_reduction(&mut samples, 2, 0.5);
        assert!(samples.iter().all(|s| s.is_finite()));
    }
}
