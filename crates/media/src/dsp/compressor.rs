//! Dynamic range compressor with envelope follower.
//!
//! Reduces gain for signals above the threshold using a configurable ratio,
//! attack time, and release time.

/// Apply dynamic range compression to interleaved audio samples.
///
/// - `threshold_db`: level above which compression kicks in (e.g. -20.0)
/// - `ratio`: compression ratio (e.g. 4.0 means 4:1)
/// - `attack_ms`: how quickly the compressor responds to loud signals
/// - `release_ms`: how quickly the compressor releases after signal drops
pub fn apply_compressor(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
) {
    if samples.is_empty() || channels == 0 || sample_rate == 0 || ratio <= 1.0 {
        return;
    }

    let ch = channels as usize;
    let threshold_lin = db_to_linear(threshold_db);

    // Time constants for envelope follower
    let attack_coeff = if attack_ms > 0.0 {
        (-1.0 / (attack_ms * 0.001 * sample_rate as f32)).exp()
    } else {
        0.0
    };
    let release_coeff = if release_ms > 0.0 {
        (-1.0 / (release_ms * 0.001 * sample_rate as f32)).exp()
    } else {
        0.0
    };

    let mut envelope = 0.0f32;

    for frame in samples.chunks_mut(ch) {
        // Compute peak level across all channels in this frame
        let peak = frame.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));

        // Envelope follower
        if peak > envelope {
            envelope = attack_coeff * envelope + (1.0 - attack_coeff) * peak;
        } else {
            envelope = release_coeff * envelope + (1.0 - release_coeff) * peak;
        }

        // Compute gain reduction
        let gain = if envelope > threshold_lin && envelope > 1e-10 {
            let env_db = linear_to_db(envelope);
            let over_db = env_db - threshold_db;
            let compressed_over = over_db / ratio;
            let target_db = threshold_db + compressed_over;
            db_to_linear(target_db - env_db)
        } else {
            1.0
        };

        for sample in frame.iter_mut() {
            *sample *= gain;
        }
    }
}

#[inline]
fn db_to_linear(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

#[inline]
fn linear_to_db(lin: f32) -> f32 {
    20.0 * lin.log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_stays_silent() {
        let mut samples = vec![0.0f32; 4096];
        apply_compressor(&mut samples, 48000, 2, -20.0, 4.0, 10.0, 100.0);
        for s in &samples {
            assert!(s.abs() < 1e-10);
        }
    }

    #[test]
    fn quiet_signal_unaffected() {
        // Signal well below threshold (-20 dB ≈ 0.1 linear)
        let amplitude = 0.05f32; // about -26 dB
        let mut samples: Vec<f32> = (0..2048)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original = samples.clone();
        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 10.0, 100.0);

        // Should be mostly unchanged (envelope follower may cause tiny differences at start)
        let orig_energy: f64 = original.iter().map(|s| (*s as f64).powi(2)).sum();
        let comp_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        let ratio = comp_energy / orig_energy;
        assert!(
            ratio > 0.9,
            "quiet signal should be mostly unaffected, ratio={ratio}"
        );
    }

    #[test]
    fn loud_signal_gets_reduced() {
        // Signal above threshold
        let amplitude = 0.9f32; // about -1 dB, well above -20 dB threshold
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 1.0, 50.0);
        let compressed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            compressed_energy < original_energy * 0.8,
            "loud signal should be reduced: orig={original_energy}, comp={compressed_energy}"
        );
    }

    #[test]
    fn higher_ratio_compresses_more() {
        let amplitude = 0.9f32;
        let base: Vec<f32> = (0..4096)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();

        let mut low_ratio = base.clone();
        apply_compressor(&mut low_ratio, 48000, 1, -20.0, 2.0, 1.0, 50.0);
        let low_energy: f64 = low_ratio.iter().map(|s| (*s as f64).powi(2)).sum();

        let mut high_ratio = base.clone();
        apply_compressor(&mut high_ratio, 48000, 1, -20.0, 10.0, 1.0, 50.0);
        let high_energy: f64 = high_ratio.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            high_energy < low_energy,
            "higher ratio should compress more: 2:1={low_energy}, 10:1={high_energy}"
        );
    }

    #[test]
    fn stereo_processing() {
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let v =
                    (2.0 * std::f64::consts::PI * 440.0 * (i / 2) as f64 / 96000.0).sin() as f32;
                v * 0.9
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_compressor(&mut samples, 48000, 2, -20.0, 4.0, 1.0, 50.0);
        let compressed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(compressed_energy < original_energy * 0.8);
    }

    #[test]
    fn ratio_one_or_below_is_noop() {
        let mut samples = vec![0.9f32; 1024];
        let original = samples.clone();
        apply_compressor(&mut samples, 48000, 1, -20.0, 1.0, 10.0, 100.0);
        assert_eq!(samples, original);

        apply_compressor(&mut samples, 48000, 1, -20.0, 0.5, 10.0, 100.0);
        assert_eq!(samples, original);
    }

    #[test]
    fn empty_samples_no_panic() {
        let mut samples: Vec<f32> = Vec::new();
        apply_compressor(&mut samples, 48000, 2, -20.0, 4.0, 10.0, 100.0);
    }

    #[test]
    fn signal_below_threshold_no_compression() {
        // Signal at -40 dB (0.01 linear), threshold at -10 dB (0.316 linear)
        let amplitude = 0.01f32;
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original = samples.clone();
        apply_compressor(&mut samples, 48000, 1, -10.0, 4.0, 10.0, 100.0);

        // Energy should be essentially the same since signal is well below threshold
        let orig_energy: f64 = original.iter().map(|s| (*s as f64).powi(2)).sum();
        let comp_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        let ratio = comp_energy / orig_energy;
        assert!(
            ratio > 0.95,
            "signal below threshold should be unaffected, ratio={ratio}"
        );
    }

    #[test]
    fn extreme_ratio_compresses_heavily() {
        let amplitude = 0.9f32;
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        // Ratio of 100:1 is effectively a limiter
        apply_compressor(&mut samples, 48000, 1, -20.0, 100.0, 1.0, 50.0);
        let compressed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        assert!(
            compressed_energy < original_energy * 0.5,
            "extreme ratio should heavily compress: orig={original_energy}, comp={compressed_energy}"
        );
    }

    #[test]
    fn zero_attack_instant_response() {
        let amplitude = 0.9f32;
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 0.0, 50.0);
        let compressed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        // Should still compress with zero attack
        assert!(compressed_energy < original_energy * 0.8);
    }

    #[test]
    fn very_quiet_near_silence() {
        // Signal at ~-80 dB (0.0001 linear), well below any reasonable threshold
        let amplitude = 0.0001f32;
        let mut samples: Vec<f32> = (0..2048)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original = samples.clone();
        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 10.0, 100.0);

        // Energy ratio should be very close to 1.0 (no compression applied)
        let orig_energy: f64 = original.iter().map(|s| (*s as f64).powi(2)).sum();
        let comp_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        let ratio = comp_energy / orig_energy;
        assert!(
            ratio > 0.99,
            "near-silence should be unaffected, ratio={ratio}"
        );
    }

    #[test]
    fn alternating_loud_quiet() {
        // Alternating loud and quiet samples to exercise attack/release
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                if (i / 256) % 2 == 0 {
                    0.9 // loud
                } else {
                    0.01 // quiet
                }
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 5.0, 50.0);
        let compressed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        // Overall energy should be reduced due to loud portions being compressed
        assert!(
            compressed_energy < original_energy,
            "alternating signal should have reduced energy"
        );
    }

    #[test]
    fn negative_threshold() {
        // Negative threshold (e.g. -40 dB) is valid and means compression starts
        // at a very low level
        let amplitude = 0.1f32; // about -20 dB, above -40 dB threshold
        let mut samples: Vec<f32> = (0..4096)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_compressor(&mut samples, 48000, 1, -40.0, 4.0, 5.0, 50.0);
        let compressed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        // With such a low threshold, even moderate signals get compressed
        assert!(
            compressed_energy < original_energy * 0.95,
            "negative threshold should compress moderate signal"
        );
    }

    #[test]
    fn ratio_one_exact_noop() {
        // ratio=1.0 means 1:1 compression (no compression), triggers early return
        let mut samples = vec![0.9f32; 1024];
        let original = samples.clone();
        apply_compressor(&mut samples, 48000, 1, -20.0, 1.0, 10.0, 100.0);
        assert_eq!(samples, original, "ratio=1.0 should be exact passthrough");
    }

    #[test]
    fn mono_channel_processing() {
        let amplitude = 0.9f32;
        let mut samples: Vec<f32> = (0..2048)
            .map(|i| {
                let v = (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 96000.0).sin() as f32;
                v * amplitude
            })
            .collect();
        let original_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        apply_compressor(&mut samples, 48000, 1, -20.0, 4.0, 1.0, 50.0);
        let compressed_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        assert!(
            compressed_energy < original_energy * 0.8,
            "mono loud signal should be compressed"
        );
    }
}
