//! Spectral gating noise reduction using FFT.
//!
//! Performs STFT with a 2048-sample window and 50% overlap,
//! zeros frequency bins below a threshold, and reconstructs via IFFT.

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

const WINDOW_SIZE: usize = 2048;
const HOP_SIZE: usize = WINDOW_SIZE / 2; // 50% overlap

/// Apply spectral noise reduction to interleaved audio samples.
///
/// `strength` controls the gate threshold (0.0 = no reduction, 1.0 = aggressive).
/// Processes each channel independently via STFT → spectral gate → IFFT with
/// overlap-add reconstruction.
pub fn apply_noise_reduction(samples: &mut [f32], channels: u16, strength: f32) {
    if samples.is_empty() || channels == 0 || strength <= 0.0 {
        return;
    }

    let ch = channels as usize;
    let num_frames = samples.len() / ch;
    if num_frames == 0 {
        return;
    }

    // Process each channel independently
    for c in 0..ch {
        // De-interleave this channel
        let mut channel_data: Vec<f32> = (0..num_frames).map(|i| samples[i * ch + c]).collect();

        process_channel(&mut channel_data, strength);

        // Re-interleave
        for (i, &val) in channel_data.iter().enumerate() {
            samples[i * ch + c] = val;
        }
    }
}

fn process_channel(data: &mut [f32], strength: f32) {
    let len = data.len();
    if len < WINDOW_SIZE {
        // Too short for FFT processing; apply simple amplitude gate
        let threshold = strength * 0.01;
        for s in data.iter_mut() {
            if s.abs() < threshold {
                *s = 0.0;
            }
        }
        return;
    }

    let mut planner = FftPlanner::new();
    let fft_forward = planner.plan_fft_forward(WINDOW_SIZE);
    let fft_inverse = planner.plan_fft_inverse(WINDOW_SIZE);

    // Hann window
    let window: Vec<f32> = (0..WINDOW_SIZE)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / WINDOW_SIZE as f32).cos()))
        .collect();

    // First pass: estimate noise floor from the average magnitude spectrum
    let mut avg_magnitude = vec![0.0f64; WINDOW_SIZE];
    let mut num_windows = 0usize;

    let mut pos = 0;
    while pos + WINDOW_SIZE <= len {
        let mut buffer: Vec<Complex<f32>> = (0..WINDOW_SIZE)
            .map(|i| Complex::new(data[pos + i] * window[i], 0.0))
            .collect();

        fft_forward.process(&mut buffer);

        for (j, bin) in buffer.iter().enumerate() {
            avg_magnitude[j] += bin.norm() as f64;
        }
        num_windows += 1;
        pos += HOP_SIZE;
    }

    if num_windows == 0 {
        return;
    }

    for m in &mut avg_magnitude {
        *m /= num_windows as f64;
    }

    // Threshold: bins below (strength * average_magnitude) get attenuated
    let threshold_mult = strength * 1.5;

    // Second pass: apply spectral gate with overlap-add reconstruction.
    let mut output = vec![0.0f32; len];

    pos = 0;
    while pos + WINDOW_SIZE <= len {
        let mut buffer: Vec<Complex<f32>> = (0..WINDOW_SIZE)
            .map(|i| Complex::new(data[pos + i] * window[i], 0.0))
            .collect();

        fft_forward.process(&mut buffer);

        // Spectral gate: attenuate bins below threshold
        for (j, bin) in buffer.iter_mut().enumerate() {
            let threshold = avg_magnitude[j] as f32 * threshold_mult;
            let mag = bin.norm();
            if mag < threshold && mag > 1e-10 {
                // Soft gate: scale down rather than hard zero to reduce artifacts
                let gain = mag / threshold;
                *bin *= gain;
            }
        }

        fft_inverse.process(&mut buffer);

        // Overlap-add with synthesis window and IFFT normalization
        let scale = 1.0 / WINDOW_SIZE as f32;
        for i in 0..WINDOW_SIZE {
            if pos + i < len {
                output[pos + i] += buffer[i].re * scale * window[i];
            }
        }

        pos += HOP_SIZE;
    }

    // Apply constant normalization and copy to output.
    // For regions covered by the STFT, use the reconstructed signal.
    // For edge regions (only covered by one window), the normalization
    // factor should be adjusted, but we handle this by blending with
    // the original signal.
    // Compute the actual window overlap sum at each position for proper normalization
    let mut win_sum = vec![0.0f32; len];
    pos = 0;
    while pos + WINDOW_SIZE <= len {
        for i in 0..WINDOW_SIZE {
            if pos + i < len {
                win_sum[pos + i] += window[i] * window[i];
            }
        }
        pos += HOP_SIZE;
    }

    for i in 0..len {
        if win_sum[i] > 1e-10 {
            let reconstructed = output[i] / win_sum[i];
            // Noise reduction should never increase amplitude — clamp to original magnitude
            let orig_abs = data[i].abs();
            data[i] = reconstructed.clamp(-orig_abs, orig_abs);
        }
        // Samples not covered by any window keep their original value.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_stays_silent() {
        let mut samples = vec![0.0f32; 4096];
        apply_noise_reduction(&mut samples, 1, 0.5);
        for s in &samples {
            assert!(s.abs() < 1e-10);
        }
    }

    #[test]
    fn zero_strength_is_noop() {
        let original: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48000.0).sin() as f32)
            .collect();
        let mut processed = original.clone();
        apply_noise_reduction(&mut processed, 1, 0.0);
        assert_eq!(original, processed);
    }

    #[test]
    fn low_level_noise_gets_attenuated() {
        // Create low-level bipolar noise (centered around zero)
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| {
                // Simple pseudo-random bipolar noise
                let noise = ((i as f64 * 17.0 + 3.7).sin() * 43758.5453).fract() as f32;
                (noise * 2.0 - 1.0) * 0.01 // bipolar, very quiet noise
            })
            .collect();

        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_noise_reduction(&mut samples, 1, 0.8);
        let reduced_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            reduced_energy < original_energy,
            "noise should be attenuated: orig={original_energy}, reduced={reduced_energy}"
        );
    }

    #[test]
    fn strong_signal_mostly_preserved() {
        // Create a strong 440 Hz tone
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48000.0).sin() as f32 * 0.8)
            .collect();

        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_noise_reduction(&mut samples, 1, 0.3);
        let processed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        // Strong signal should retain most of its energy
        let ratio = processed_energy / original_energy;
        assert!(
            ratio > 0.5,
            "strong signal should be mostly preserved, ratio={ratio}"
        );
    }

    #[test]
    fn stereo_processing() {
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| {
                let frame = i / 2;
                (2.0 * std::f64::consts::PI * 440.0 * frame as f64 / 48000.0).sin() as f32 * 0.01
            })
            .collect();

        // Should not panic with stereo input
        apply_noise_reduction(&mut samples, 2, 0.5);
    }

    #[test]
    fn empty_samples_no_panic() {
        let mut samples: Vec<f32> = Vec::new();
        apply_noise_reduction(&mut samples, 2, 0.5);
    }

    #[test]
    fn short_buffer_no_panic() {
        let mut samples = vec![0.01f32; 100];
        apply_noise_reduction(&mut samples, 1, 0.5);
    }

    #[test]
    fn all_zero_input() {
        let mut samples = vec![0.0f32; 8192];
        apply_noise_reduction(&mut samples, 1, 0.8);
        for s in &samples {
            assert!(s.abs() < 1e-10, "all-zero input should stay zero");
        }
    }

    #[test]
    fn pure_sine_mostly_preserved() {
        // A strong pure sine wave should mostly pass through noise reduction
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin() as f32 * 0.9)
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_noise_reduction(&mut samples, 1, 0.3);
        let processed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        let ratio = processed_energy / original_energy;
        assert!(
            ratio > 0.4,
            "pure sine should be mostly preserved, ratio={ratio}"
        );
    }

    #[test]
    fn strength_zero_exact_passthrough() {
        // strength=0.0 triggers early return
        let original: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48000.0).sin() as f32 * 0.5)
            .collect();
        let mut processed = original.clone();
        apply_noise_reduction(&mut processed, 1, 0.0);
        for (o, p) in original.iter().zip(processed.iter()) {
            assert_eq!(
                o.to_bits(),
                p.to_bits(),
                "strength=0 should be bit-identical"
            );
        }
    }

    #[test]
    fn strength_one_maximum_reduction() {
        // Maximum strength should aggressively reduce noise
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| {
                let noise = ((i as f64 * 17.0 + 3.7).sin() * 43758.5453).fract() as f32;
                (noise * 2.0 - 1.0) * 0.01
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_noise_reduction(&mut samples, 1, 1.0);
        let reduced_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            reduced_energy < original_energy,
            "max strength should reduce noise energy"
        );
    }

    #[test]
    fn very_short_input_below_fft_window() {
        // Input shorter than WINDOW_SIZE (2048) triggers the simple amplitude gate
        let mut samples = vec![0.005f32; 500];
        apply_noise_reduction(&mut samples, 1, 0.8);
        // With strength=0.8, threshold=0.008; samples at 0.005 < 0.008 → zeroed
        for s in &samples {
            assert!(
                s.abs() < 1e-10,
                "short input below threshold should be zeroed"
            );
        }
    }

    #[test]
    fn mono_channel_noise_reduction() {
        let mut samples: Vec<f32> = (0..8192)
            .map(|i| {
                let noise = ((i as f64 * 7.3 + 1.1).sin() * 12345.6789).fract() as f32;
                (noise * 2.0 - 1.0) * 0.005
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_noise_reduction(&mut samples, 1, 0.7);
        let reduced_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            reduced_energy < original_energy,
            "mono noise should be reduced"
        );
    }
}
