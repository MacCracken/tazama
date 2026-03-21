//! 3-band parametric EQ — backed by dhvani.
//!
//! Low shelf at 200 Hz, mid peaking at 1 kHz, high shelf at 5 kHz.

use dhvani::buffer::AudioBuffer;
use dhvani::dsp::{BandType, EqBandConfig, ParametricEq};

/// Apply 3-band EQ in-place on interleaved f32 samples.
pub fn apply_eq(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    low_gain_db: f32,
    mid_gain_db: f32,
    high_gain_db: f32,
) {
    if samples.is_empty() || channels == 0 || sample_rate == 0 {
        return;
    }

    // Skip if all gains are near zero
    if low_gain_db.abs() < 0.01 && mid_gain_db.abs() < 0.01 && high_gain_db.abs() < 0.01 {
        return;
    }

    let bands = vec![
        EqBandConfig {
            band_type: BandType::LowShelf,
            freq_hz: 200.0,
            gain_db: low_gain_db,
            q: 0.707,
            enabled: low_gain_db.abs() >= 0.01,
        },
        EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 1000.0,
            gain_db: mid_gain_db,
            q: 1.0,
            enabled: mid_gain_db.abs() >= 0.01,
        },
        EqBandConfig {
            band_type: BandType::HighShelf,
            freq_hz: 5000.0,
            gain_db: high_gain_db,
            q: 0.707,
            enabled: high_gain_db.abs() >= 0.01,
        },
    ];

    let mut eq = ParametricEq::new(bands, sample_rate, channels as u32);

    if let Ok(mut buf) =
        AudioBuffer::from_interleaved(samples.to_vec(), channels as u32, sample_rate)
    {
        eq.process(&mut buf);
        samples.copy_from_slice(&buf.samples);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(freq: f64, sr: u32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (i as f64 / sr as f64 * freq * 2.0 * std::f64::consts::PI).sin() as f32)
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        (sum / samples.len() as f64).sqrt() as f32
    }

    #[test]
    fn silent_input_stays_silent() {
        let mut samples = vec![0.0f32; 4800];
        apply_eq(&mut samples, 48000, 1, 6.0, 6.0, 6.0);
        assert!(samples.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn zero_gain_is_passthrough() {
        let original = make_sine(1000.0, 48000, 4800);
        let mut samples = original.clone();
        apply_eq(&mut samples, 48000, 1, 0.0, 0.0, 0.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn mid_boost_increases_energy() {
        let mut samples = make_sine(1000.0, 48000, 4800);
        let before = rms(&samples);
        apply_eq(&mut samples, 48000, 1, 0.0, 12.0, 0.0);
        let after = rms(&samples);
        assert!(
            after > before,
            "mid boost should increase RMS: {before} -> {after}"
        );
    }

    #[test]
    fn mid_cut_decreases_energy() {
        let mut samples = make_sine(1000.0, 48000, 4800);
        let before = rms(&samples);
        apply_eq(&mut samples, 48000, 1, 0.0, -12.0, 0.0);
        let after = rms(&samples);
        assert!(
            after < before,
            "mid cut should decrease RMS: {before} -> {after}"
        );
    }

    #[test]
    fn stereo_processing() {
        let mut samples: Vec<f32> = (0..9600)
            .map(|i| (i as f64 / 48000.0 * 1000.0 * 2.0 * std::f64::consts::PI).sin() as f32)
            .collect();
        apply_eq(&mut samples, 48000, 2, 0.0, 6.0, 0.0);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn empty_input() {
        let mut samples: Vec<f32> = vec![];
        apply_eq(&mut samples, 48000, 1, 6.0, 6.0, 6.0);
    }

    #[test]
    fn negative_gains() {
        let mut samples = make_sine(200.0, 48000, 4800);
        let before = rms(&samples);
        apply_eq(&mut samples, 48000, 1, -12.0, 0.0, 0.0);
        let after = rms(&samples);
        assert!(
            after < before,
            "low cut should decrease RMS for 200Hz signal"
        );
    }
}
