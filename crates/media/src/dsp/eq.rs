//! 3-band parametric EQ using biquad filters.
//!
//! Low shelf at 200 Hz, mid peaking at 1 kHz, high shelf at 5 kHz.
//! Filter coefficients follow Robert Bristow-Johnson's Audio EQ Cookbook.

use std::f64::consts::PI;

/// Biquad filter state for one channel.
#[derive(Clone, Debug)]
struct BiquadState {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadState {
    fn new() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, coeffs: &BiquadCoeffs, x0: f64) -> f64 {
        let y0 = coeffs.b0 * x0 + coeffs.b1 * self.x1 + coeffs.b2 * self.x2
            - coeffs.a1 * self.y1
            - coeffs.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;
        y0
    }
}

/// Normalized biquad coefficients (a0 divided out).
#[derive(Clone, Debug)]
struct BiquadCoeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl BiquadCoeffs {
    /// Returns true if all coefficients are finite (not NaN or Inf).
    fn is_valid(&self) -> bool {
        self.b0.is_finite()
            && self.b1.is_finite()
            && self.b2.is_finite()
            && self.a1.is_finite()
            && self.a2.is_finite()
    }
}

/// Compute low-shelf biquad coefficients (Bristow-Johnson cookbook).
fn low_shelf(sample_rate: u32, freq: f64, gain_db: f64) -> BiquadCoeffs {
    let a = 10.0_f64.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * freq / sample_rate as f64;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (std::f64::consts::SQRT_2 - 1.0) + 2.0).sqrt();
    let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

    let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
    if a0.abs() < 1e-10 {
        return BiquadCoeffs {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
    }
    BiquadCoeffs {
        b0: (a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha)) / a0,
        b1: (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0,
        b2: (a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha)) / a0,
        a1: (-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0,
        a2: ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha) / a0,
    }
}

/// Compute peaking EQ biquad coefficients (Bristow-Johnson cookbook).
fn peaking_eq(sample_rate: u32, freq: f64, gain_db: f64, q: f64) -> BiquadCoeffs {
    let a = 10.0_f64.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * freq / sample_rate as f64;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let a0 = 1.0 + alpha / a;
    if a0.abs() < 1e-10 {
        return BiquadCoeffs {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
    }
    BiquadCoeffs {
        b0: (1.0 + alpha * a) / a0,
        b1: (-2.0 * cos_w0) / a0,
        b2: (1.0 - alpha * a) / a0,
        a1: (-2.0 * cos_w0) / a0,
        a2: (1.0 - alpha / a) / a0,
    }
}

/// Compute high-shelf biquad coefficients (Bristow-Johnson cookbook).
fn high_shelf(sample_rate: u32, freq: f64, gain_db: f64) -> BiquadCoeffs {
    let a = 10.0_f64.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * freq / sample_rate as f64;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (std::f64::consts::SQRT_2 - 1.0) + 2.0).sqrt();
    let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

    let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
    if a0.abs() < 1e-10 {
        return BiquadCoeffs {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
    }
    BiquadCoeffs {
        b0: (a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha)) / a0,
        b1: (-2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0,
        b2: (a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha)) / a0,
        a1: (2.0 * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0,
        a2: ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha) / a0,
    }
}

/// Apply a 3-band parametric EQ to interleaved stereo (or multi-channel) samples.
///
/// - Low shelf at 200 Hz
/// - Mid peaking EQ at 1 kHz (Q = 1.0)
/// - High shelf at 5 kHz
///
/// Gains are in dB. 0 dB = no change.
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

    // Skip processing if all gains are effectively zero
    if low_gain_db.abs() < 1e-6 && mid_gain_db.abs() < 1e-6 && high_gain_db.abs() < 1e-6 {
        return;
    }

    let ch = channels as usize;
    let low_coeffs = low_shelf(sample_rate, 200.0, low_gain_db as f64);
    let mid_coeffs = peaking_eq(sample_rate, 1000.0, mid_gain_db as f64, 1.0);
    let high_coeffs = high_shelf(sample_rate, 5000.0, high_gain_db as f64);

    // Skip bands with invalid (NaN/Inf) coefficients
    let low_valid = low_gain_db.abs() > 1e-6 && low_coeffs.is_valid();
    let mid_valid = mid_gain_db.abs() > 1e-6 && mid_coeffs.is_valid();
    let high_valid = high_gain_db.abs() > 1e-6 && high_coeffs.is_valid();

    if !low_valid && !mid_valid && !high_valid {
        return;
    }

    // Per-channel filter states for each band
    let mut low_states: Vec<BiquadState> = (0..ch).map(|_| BiquadState::new()).collect();
    let mut mid_states: Vec<BiquadState> = (0..ch).map(|_| BiquadState::new()).collect();
    let mut high_states: Vec<BiquadState> = (0..ch).map(|_| BiquadState::new()).collect();

    for frame in samples.chunks_mut(ch) {
        for (c, sample) in frame.iter_mut().enumerate() {
            let mut s = *sample as f64;
            if low_valid {
                s = low_states[c].process(&low_coeffs, s);
            }
            if mid_valid {
                s = mid_states[c].process(&mid_coeffs, s);
            }
            if high_valid {
                s = high_states[c].process(&high_coeffs, s);
            }
            let out = s as f32;
            *sample = if out.is_finite() { out } else { 0.0 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_stays_silent() {
        let mut samples = vec![0.0f32; 4096];
        apply_eq(&mut samples, 48000, 2, 6.0, 3.0, -3.0);
        for s in &samples {
            assert!(s.abs() < 1e-10, "silence should remain silence");
        }
    }

    #[test]
    fn zero_gain_is_passthrough() {
        let original: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut processed = original.clone();
        apply_eq(&mut processed, 48000, 2, 0.0, 0.0, 0.0);
        assert_eq!(original, processed);
    }

    #[test]
    fn low_boost_increases_low_frequency_energy() {
        // Generate a 100 Hz tone at 48 kHz, stereo
        let sample_rate = 48000u32;
        let freq = 100.0;
        let num_frames = 4096;
        let mut samples: Vec<f32> = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let v = (2.0 * std::f64::consts::PI * freq * i as f64 / sample_rate as f64).sin()
                as f32
                * 0.5;
            samples.push(v); // L
            samples.push(v); // R
        }

        let original_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        apply_eq(&mut samples, sample_rate, 2, 12.0, 0.0, 0.0);
        let boosted_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();

        assert!(
            boosted_energy > original_energy * 1.5,
            "low boost should increase energy of 100 Hz tone: original={original_energy}, boosted={boosted_energy}"
        );
    }

    #[test]
    fn high_boost_increases_high_frequency_energy() {
        // Generate a 10 kHz tone at 48 kHz, stereo
        let sample_rate = 48000u32;
        let freq = 10000.0;
        let num_frames = 4096;
        let mut samples: Vec<f32> = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let v = (2.0 * std::f64::consts::PI * freq * i as f64 / sample_rate as f64).sin()
                as f32
                * 0.5;
            samples.push(v);
            samples.push(v);
        }

        let original_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        apply_eq(&mut samples, sample_rate, 2, 0.0, 0.0, 12.0);
        let boosted_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();

        assert!(
            boosted_energy > original_energy * 1.5,
            "high boost should increase energy of 10 kHz tone"
        );
    }

    #[test]
    fn empty_samples_no_panic() {
        let mut samples: Vec<f32> = Vec::new();
        apply_eq(&mut samples, 48000, 2, 6.0, 3.0, -3.0);
    }

    #[test]
    fn mono_processing_works() {
        let sample_rate = 48000u32;
        let freq = 100.0;
        let num_frames = 2048;
        let mut samples: Vec<f32> = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            let v = (2.0 * std::f64::consts::PI * freq * i as f64 / sample_rate as f64).sin()
                as f32
                * 0.5;
            samples.push(v);
        }

        let original_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        apply_eq(&mut samples, sample_rate, 1, 12.0, 0.0, 0.0);
        let boosted_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();

        assert!(boosted_energy > original_energy * 1.5);
    }

    #[test]
    fn mono_mid_boost() {
        // Generate a 1 kHz tone (center of mid band) at 48 kHz, mono
        let sample_rate = 48000u32;
        let freq = 1000.0;
        let num_frames = 4096;
        let mut samples: Vec<f32> = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            let v = (2.0 * std::f64::consts::PI * freq * i as f64 / sample_rate as f64).sin()
                as f32
                * 0.5;
            samples.push(v);
        }

        let original_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        apply_eq(&mut samples, sample_rate, 1, 0.0, 12.0, 0.0);
        let boosted_energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();

        assert!(
            boosted_energy > original_energy * 1.5,
            "mid boost should increase energy of 1 kHz tone: orig={original_energy}, boosted={boosted_energy}"
        );
    }

    #[test]
    fn zero_gain_all_bands_is_exact_passthrough() {
        // Verify that zero gain returns exactly the same samples (early return path)
        let original: Vec<f32> = (0..2048).map(|i| (i as f32 * 0.007).sin()).collect();
        let mut processed = original.clone();
        apply_eq(&mut processed, 44100, 1, 0.0, 0.0, 0.0);
        // Should be bitwise identical due to early return
        for (o, p) in original.iter().zip(processed.iter()) {
            assert_eq!(o.to_bits(), p.to_bits());
        }
    }

    #[test]
    fn zero_channels_no_panic() {
        let mut samples = vec![0.5f32; 100];
        let original = samples.clone();
        apply_eq(&mut samples, 48000, 0, 6.0, 3.0, -3.0);
        // Should return early without modifying
        assert_eq!(samples, original);
    }

    #[test]
    fn zero_sample_rate_no_panic() {
        let mut samples = vec![0.5f32; 100];
        let original = samples.clone();
        apply_eq(&mut samples, 0, 2, 6.0, 3.0, -3.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn high_shelf_boost_10khz_tone() {
        // A 10kHz tone should be boosted by a high shelf at 5kHz
        let sample_rate = 48000u32;
        let freq = 10000.0;
        let num_frames = 4096;
        let mut samples: Vec<f32> = (0..num_frames)
            .map(|i| {
                (2.0 * std::f64::consts::PI * freq * i as f64 / sample_rate as f64).sin() as f32
                    * 0.5
            })
            .collect();

        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_eq(&mut samples, sample_rate, 1, 0.0, 0.0, 12.0);
        let boosted_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            boosted_energy > original_energy * 1.5,
            "high shelf should boost 10kHz: orig={original_energy}, boosted={boosted_energy}"
        );
    }

    #[test]
    fn negative_gain_cuts_frequency() {
        // Negative gain on low shelf should reduce 100Hz energy
        let sample_rate = 48000u32;
        let freq = 100.0;
        let num_frames = 4096;
        let mut samples: Vec<f32> = (0..num_frames)
            .map(|i| {
                (2.0 * std::f64::consts::PI * freq * i as f64 / sample_rate as f64).sin() as f32
                    * 0.5
            })
            .collect();

        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_eq(&mut samples, sample_rate, 1, -12.0, 0.0, 0.0);
        let cut_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            cut_energy < original_energy * 0.7,
            "negative gain should cut: orig={original_energy}, cut={cut_energy}"
        );
    }

    #[test]
    fn very_short_input_one_sample() {
        // Single sample should not panic
        let mut samples = vec![0.5f32];
        apply_eq(&mut samples, 48000, 1, 6.0, 3.0, -3.0);
        // Just verify no panic and output is finite
        assert!(samples[0].is_finite());
    }

    #[test]
    fn nan_input_produces_finite_output() {
        let mut samples = vec![f32::NAN; 1024];
        apply_eq(&mut samples, 48000, 1, 6.0, 3.0, -3.0);
        for s in &samples {
            assert!(s.is_finite(), "NaN input should not propagate through EQ");
        }
    }

    #[test]
    fn nan_gain_skips_band() {
        let original: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut processed = original.clone();
        apply_eq(&mut processed, 48000, 1, f32::NAN, 0.0, 0.0);
        // NaN gain produces NaN coefficients which are invalid, so band is skipped
        assert_eq!(original, processed, "NaN gain should skip the band");
    }

    #[test]
    fn all_bands_boosted_simultaneously() {
        // Boost all three bands at once on a broadband signal
        let sample_rate = 48000u32;
        let num_frames = 4096;
        // Mix of low, mid, and high frequencies
        let mut samples: Vec<f32> = (0..num_frames)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                let low = (2.0 * std::f64::consts::PI * 100.0 * t).sin();
                let mid = (2.0 * std::f64::consts::PI * 1000.0 * t).sin();
                let high = (2.0 * std::f64::consts::PI * 10000.0 * t).sin();
                ((low + mid + high) / 3.0 * 0.3) as f32
            })
            .collect();

        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_eq(&mut samples, sample_rate, 1, 6.0, 6.0, 6.0);
        let boosted_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            boosted_energy > original_energy * 1.3,
            "all bands boosted should increase total energy"
        );
    }
}
