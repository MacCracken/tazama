//! Schroeder reverb — backed by dhvani.
//!
//! Classic design: 4 parallel comb filters + 2 series allpass filters.
//! Parameters control room size, damping, and wet/dry mix.

use dhvani::buffer::AudioBuffer;
use dhvani::dsp::{Reverb, ReverbParams};

/// Apply reverb in-place on interleaved f32 samples.
pub fn apply_reverb(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    room_size: f32,
    damping: f32,
    wet: f32,
) {
    let room_size = room_size.clamp(0.0, 1.0);
    let damping = damping.clamp(0.0, 1.0);
    let wet = wet.clamp(0.0, 1.0);

    if samples.is_empty() || channels == 0 || sample_rate == 0 || wet < 0.001 {
        return;
    }

    let params = ReverbParams {
        room_size,
        damping,
        mix: wet,
    };
    let mut reverb = Reverb::new(params, sample_rate);

    if let Ok(mut buf) =
        AudioBuffer::from_interleaved(samples.to_vec(), channels as u32, sample_rate)
    {
        reverb.process(&mut buf);
        samples.copy_from_slice(&buf.samples);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input_stays_silent() {
        let mut samples = vec![0.0f32; 4800];
        apply_reverb(&mut samples, 48000, 1, 0.5, 0.5, 0.3);
        assert!(samples.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn zero_wet_passthrough() {
        let original: Vec<f32> = (0..4800)
            .map(|i| (i as f64 / 48000.0 * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32)
            .collect();
        let mut samples = original.clone();
        apply_reverb(&mut samples, 48000, 1, 0.5, 0.5, 0.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn impulse_produces_tail() {
        let mut samples = vec![0.0f32; 48000]; // 1 second
        samples[0] = 1.0; // impulse
        apply_reverb(&mut samples, 48000, 1, 0.8, 0.3, 1.0);
        // Reverb should produce non-zero samples after the impulse
        let tail_energy: f64 = samples[1000..].iter().map(|s| (*s as f64).powi(2)).sum();
        assert!(tail_energy > 0.0, "reverb should produce a tail");
    }

    #[test]
    fn output_finite() {
        let mut samples: Vec<f32> = (0..4800)
            .map(|i| (i as f64 / 48000.0 * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32)
            .collect();
        apply_reverb(&mut samples, 48000, 1, 0.9, 0.1, 0.5);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn stereo() {
        let mut samples: Vec<f32> = (0..9600)
            .map(|i| (i as f64 / 48000.0 * 440.0 * 2.0 * std::f64::consts::PI).sin() as f32 * 0.5)
            .collect();
        apply_reverb(&mut samples, 48000, 2, 0.5, 0.5, 0.3);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn empty_input() {
        let mut samples: Vec<f32> = vec![];
        apply_reverb(&mut samples, 48000, 1, 0.5, 0.5, 0.3);
    }
}
