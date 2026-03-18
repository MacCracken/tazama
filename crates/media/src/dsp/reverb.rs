//! Schroeder reverb with allpass and comb filters.
//!
//! Classic design: 4 parallel comb filters → 2 series allpass filters.
//! Parameters control room size, damping, and wet/dry mix.

/// Apply Schroeder reverb to interleaved audio samples.
///
/// - `room_size`: 0.0 to 1.0, controls feedback amount (larger = longer tail)
/// - `damping`: 0.0 to 1.0, controls high-frequency damping
/// - `wet`: 0.0 to 1.0, wet/dry mix (0 = fully dry, 1 = fully wet)
pub fn apply_reverb(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    room_size: f32,
    damping: f32,
    wet: f32,
) {
    if samples.is_empty() || channels == 0 || sample_rate == 0 || wet <= 0.0 {
        return;
    }

    let ch = channels as usize;
    let num_frames = samples.len() / ch;
    if num_frames == 0 {
        return;
    }

    let wet = wet.clamp(0.0, 1.0);
    let dry = 1.0 - wet;
    let feedback = room_size.clamp(0.0, 1.0) * 0.9 + 0.05; // map to [0.05, 0.95]
    let damp = damping.clamp(0.0, 1.0);

    // Process each channel independently
    for c in 0..ch {
        // De-interleave
        let mut channel_data: Vec<f32> = (0..num_frames).map(|i| samples[i * ch + c]).collect();

        process_channel(&mut channel_data, sample_rate, feedback, damp, wet, dry);

        // Re-interleave
        for (i, &val) in channel_data.iter().enumerate() {
            samples[i * ch + c] = val;
        }
    }
}

fn process_channel(
    data: &mut [f32],
    sample_rate: u32,
    feedback: f32,
    damp: f32,
    wet: f32,
    dry: f32,
) {
    // Scale delay lengths to sample rate (reference: 44100 Hz)
    let scale = sample_rate as f32 / 44100.0;

    // Comb filter delay lengths (in samples), from Schroeder/Moorer design
    let comb_lengths: [usize; 4] = [
        (1557.0 * scale) as usize,
        (1617.0 * scale) as usize,
        (1491.0 * scale) as usize,
        (1422.0 * scale) as usize,
    ];

    // Allpass filter delay lengths
    let allpass_lengths: [usize; 2] = [(225.0 * scale) as usize, (556.0 * scale) as usize];

    let mut combs: Vec<CombFilter> = comb_lengths
        .iter()
        .map(|&len| CombFilter::new(len, feedback, damp))
        .collect();

    let mut allpasses: Vec<AllpassFilter> = allpass_lengths
        .iter()
        .map(|&len| AllpassFilter::new(len))
        .collect();

    let len = data.len();
    let mut reverb_out = vec![0.0f32; len];

    // Sum output of parallel comb filters
    for comb in &mut combs {
        for i in 0..len {
            reverb_out[i] += comb.process(data[i]);
        }
    }

    // Scale comb output
    let comb_scale = 1.0 / combs.len() as f32;
    for s in &mut reverb_out {
        *s *= comb_scale;
    }

    // Series allpass filters
    for allpass in &mut allpasses {
        for sample in reverb_out.iter_mut() {
            *sample = allpass.process(*sample);
        }
    }

    // Mix wet and dry
    for (d, r) in data.iter_mut().zip(reverb_out.iter()) {
        *d = *d * dry + *r * wet;
    }
}

/// Comb filter with low-pass feedback (for damping).
struct CombFilter {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    damp: f32,
    damp_prev: f32,
}

impl CombFilter {
    fn new(delay_len: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_len.max(1)],
            index: 0,
            feedback,
            damp,
            damp_prev: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];

        // Low-pass filter on the feedback path (damping)
        let filtered = output * (1.0 - self.damp) + self.damp_prev * self.damp;
        self.damp_prev = filtered;

        self.buffer[self.index] = input + filtered * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();

        output
    }
}

/// Allpass filter for diffusion.
struct AllpassFilter {
    buffer: Vec<f32>,
    index: usize,
}

impl AllpassFilter {
    fn new(delay_len: usize) -> Self {
        Self {
            buffer: vec![0.0; delay_len.max(1)],
            index: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.index];
        let output = -input + buffered;
        self.buffer[self.index] = input + buffered * 0.5;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_stays_silent() {
        let mut samples = vec![0.0f32; 4096];
        apply_reverb(&mut samples, 48000, 2, 0.5, 0.5, 0.5);
        for s in &samples {
            assert!(s.abs() < 1e-10, "silence input should produce silence");
        }
    }

    #[test]
    fn zero_wet_is_passthrough() {
        let original: Vec<f32> = (0..2048)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48000.0).sin() as f32)
            .collect();
        let mut processed = original.clone();
        apply_reverb(&mut processed, 48000, 1, 0.5, 0.5, 0.0);
        assert_eq!(original, processed);
    }

    #[test]
    fn impulse_produces_tail() {
        // Single impulse followed by silence
        let mut samples = vec![0.0f32; 48000]; // 1 second at 48kHz mono
        samples[0] = 1.0;

        apply_reverb(&mut samples, 48000, 1, 0.7, 0.3, 1.0);

        // There should be non-zero samples well after the impulse (reverb tail)
        let tail_energy: f64 = samples[4800..].iter().map(|s| (*s as f64).powi(2)).sum();
        assert!(
            tail_energy > 1e-6,
            "reverb should produce a tail after impulse: tail_energy={tail_energy}"
        );
    }

    #[test]
    fn larger_room_longer_tail() {
        let make_impulse = || {
            let mut s = vec![0.0f32; 48000];
            s[0] = 1.0;
            s
        };

        let mut small_room = make_impulse();
        apply_reverb(&mut small_room, 48000, 1, 0.2, 0.5, 1.0);
        let small_late_energy: f64 = small_room[24000..]
            .iter()
            .map(|s| (*s as f64).powi(2))
            .sum();

        let mut large_room = make_impulse();
        apply_reverb(&mut large_room, 48000, 1, 0.9, 0.5, 1.0);
        let large_late_energy: f64 = large_room[24000..]
            .iter()
            .map(|s| (*s as f64).powi(2))
            .sum();

        assert!(
            large_late_energy > small_late_energy,
            "larger room should have more late energy: small={small_late_energy}, large={large_late_energy}"
        );
    }

    #[test]
    fn stereo_processing() {
        let mut samples = vec![0.0f32; 8192];
        samples[0] = 1.0; // L impulse
        samples[1] = 0.5; // R impulse

        apply_reverb(&mut samples, 48000, 2, 0.5, 0.5, 0.5);

        // Both channels should have some reverb
        let left_energy: f64 = (0..4096).map(|i| (samples[i * 2] as f64).powi(2)).sum();
        let right_energy: f64 = (0..4096).map(|i| (samples[i * 2 + 1] as f64).powi(2)).sum();

        assert!(left_energy > 1e-6);
        assert!(right_energy > 1e-6);
    }

    #[test]
    fn empty_samples_no_panic() {
        let mut samples: Vec<f32> = Vec::new();
        apply_reverb(&mut samples, 48000, 2, 0.5, 0.5, 0.5);
    }

    #[test]
    fn short_input_fewer_than_delay_line() {
        // Input shorter than the smallest comb delay (~1422 samples at 44100)
        // Should not panic and should produce output
        let mut samples = vec![0.0f32; 100];
        samples[0] = 1.0;
        apply_reverb(&mut samples, 44100, 1, 0.5, 0.5, 0.5);
        // No panic is the main assertion; samples may or may not change
    }

    #[test]
    fn wet_zero_exact_passthrough() {
        // wet=0.0 triggers early return, so samples must be bit-identical
        let original: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48000.0).sin() as f32)
            .collect();
        let mut processed = original.clone();
        apply_reverb(&mut processed, 48000, 2, 0.9, 0.9, 0.0);
        for (o, p) in original.iter().zip(processed.iter()) {
            assert_eq!(o.to_bits(), p.to_bits());
        }
    }

    #[test]
    fn wet_one_fully_wet() {
        // With wet=1.0, dry=0.0 so the output is purely reverb (no dry pass-through)
        let mut samples = vec![0.0f32; 48000];
        samples[0] = 1.0;
        let original = samples.clone();

        apply_reverb(&mut samples, 48000, 1, 0.5, 0.5, 1.0);

        // The first sample should differ from original since dry component is zero
        // and reverb component replaces it
        assert_ne!(
            samples, original,
            "fully wet reverb should differ from original"
        );
    }

    #[test]
    fn high_room_size_near_one() {
        let mut samples = vec![0.0f32; 48000];
        samples[0] = 1.0;
        apply_reverb(&mut samples, 48000, 1, 0.99, 0.5, 1.0);

        // Very high room size should produce a long, sustained tail
        let late_energy: f64 = samples[40000..].iter().map(|s| (*s as f64).powi(2)).sum();
        assert!(
            late_energy > 1e-8,
            "room_size near 1.0 should have audible late tail: {late_energy}"
        );
    }

    #[test]
    fn damping_one_maximum() {
        // damping=1.0 should heavily damp high frequencies but not panic
        let mut samples = vec![0.0f32; 48000];
        samples[0] = 1.0;
        apply_reverb(&mut samples, 48000, 1, 0.5, 1.0, 1.0);

        // Should still produce some output (low frequencies pass through damping)
        let energy: f64 = samples[1000..].iter().map(|s| (*s as f64).powi(2)).sum();
        assert!(energy > 1e-10, "max damping should still produce a tail");
    }

    #[test]
    fn mono_input_processing() {
        let mut samples = vec![0.0f32; 8192];
        samples[0] = 1.0;
        apply_reverb(&mut samples, 44100, 1, 0.5, 0.5, 0.5);

        let energy: f64 = samples[2000..].iter().map(|s| (*s as f64).powi(2)).sum();
        assert!(energy > 1e-8, "mono reverb should produce a tail");
    }
}
