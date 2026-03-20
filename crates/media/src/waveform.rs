use std::path::Path;

use tazama_core::WaveformData;

use crate::decode::audio::AudioDecoder;
use crate::error::MediaPipelineError;
use crate::probe;

/// Extract waveform peak data from a media file's audio.
pub async fn extract_waveform(
    path: &Path,
    peaks_per_second: u32,
) -> Result<WaveformData, MediaPipelineError> {
    let info = probe::probe(path).await?;

    let Some(audio_info) = info.audio_streams.first() else {
        return Err(MediaPipelineError::Decode("no audio stream found".into()));
    };

    let sample_rate = audio_info.sample_rate;
    let channels = audio_info.channels;
    let samples_per_peak = sample_rate / peaks_per_second;

    let mut rx = AudioDecoder::decode(path)?;

    // Accumulate all samples
    let mut all_samples: Vec<Vec<f32>> = (0..channels).map(|_| Vec::new()).collect();

    while let Some(buffer) = rx.recv().await {
        // Deinterleave
        for (i, sample) in buffer.samples.iter().enumerate() {
            let channel = i % channels as usize;
            all_samples[channel].push(*sample);
        }
    }

    // Compute peaks
    let mut peaks: Vec<Vec<(f32, f32)>> = Vec::new();
    for channel_samples in &all_samples {
        let mut channel_peaks = Vec::new();
        for chunk in channel_samples.chunks(samples_per_peak as usize) {
            let min = chunk.iter().copied().fold(f32::INFINITY, f32::min);
            let max = chunk.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            channel_peaks.push((min, max));
        }
        peaks.push(channel_peaks);
    }

    Ok(WaveformData {
        sample_rate,
        channels,
        peaks_per_second,
        peaks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn extract_waveform_nonexistent_file_returns_error() {
        crate::init().ok();
        let path = PathBuf::from("/tmp/nonexistent_audio_file_tazama_test.wav");
        let result = extract_waveform(&path, 10).await;
        assert!(result.is_err(), "expected error for nonexistent file");
    }

    #[test]
    fn waveform_data_construction_and_field_access() {
        let data = WaveformData {
            sample_rate: 44100,
            channels: 2,
            peaks_per_second: 10,
            peaks: vec![
                vec![(-0.5, 0.5), (-0.3, 0.8)],
                vec![(-0.1, 0.2), (-0.9, 0.9)],
            ],
        };
        assert_eq!(data.sample_rate, 44100);
        assert_eq!(data.channels, 2);
        assert_eq!(data.peaks_per_second, 10);
        assert_eq!(data.peaks.len(), 2);
        assert_eq!(data.peaks[0].len(), 2);
        assert_eq!(data.peaks[1][1], (-0.9, 0.9));
    }

    #[test]
    fn waveform_data_empty_peaks() {
        let data = WaveformData {
            sample_rate: 48000,
            channels: 1,
            peaks_per_second: 20,
            peaks: vec![vec![]],
        };
        assert_eq!(data.channels, 1);
        assert!(data.peaks[0].is_empty());
    }

    #[test]
    fn waveform_data_zero_channels() {
        let data = WaveformData {
            sample_rate: 44100,
            channels: 0,
            peaks_per_second: 10,
            peaks: vec![],
        };
        assert_eq!(data.channels, 0);
        assert!(data.peaks.is_empty());
    }

    #[test]
    fn waveform_data_single_channel_single_peak() {
        let data = WaveformData {
            sample_rate: 16000,
            channels: 1,
            peaks_per_second: 1,
            peaks: vec![vec![(-1.0, 1.0)]],
        };
        assert_eq!(data.peaks.len(), 1);
        assert_eq!(data.peaks[0][0], (-1.0, 1.0));
    }

    // --- Peak computation logic tests ---
    // These test the exact algorithm used in extract_waveform for computing peaks.

    /// Helper that mirrors the peak computation from extract_waveform.
    fn compute_peaks(all_samples: &[Vec<f32>], samples_per_peak: usize) -> Vec<Vec<(f32, f32)>> {
        let mut peaks: Vec<Vec<(f32, f32)>> = Vec::new();
        for channel_samples in all_samples {
            let mut channel_peaks = Vec::new();
            for chunk in channel_samples.chunks(samples_per_peak) {
                let min = chunk.iter().copied().fold(f32::INFINITY, f32::min);
                let max = chunk.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                channel_peaks.push((min, max));
            }
            peaks.push(channel_peaks);
        }
        peaks
    }

    #[test]
    fn peak_computation_silence() {
        let samples = vec![vec![0.0f32; 100]];
        let peaks = compute_peaks(&samples, 50);
        assert_eq!(peaks.len(), 1);
        assert_eq!(peaks[0].len(), 2);
        assert_eq!(peaks[0][0], (0.0, 0.0));
        assert_eq!(peaks[0][1], (0.0, 0.0));
    }

    #[test]
    fn peak_computation_sine_wave() {
        // A full cycle sine wave from 0 to 2*PI sampled at 100 points
        let samples: Vec<f32> = (0..100)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / 100.0).sin())
            .collect();
        let peaks = compute_peaks(&[samples], 100);
        assert_eq!(peaks.len(), 1);
        assert_eq!(peaks[0].len(), 1);
        let (min, max) = peaks[0][0];
        assert!(min < -0.9, "sine min should be < -0.9, got {min}");
        assert!(max > 0.9, "sine max should be > 0.9, got {max}");
    }

    #[test]
    fn peak_computation_multiple_chunks() {
        // 200 samples split into 4 chunks of 50
        let mut samples = vec![0.0f32; 200];
        // Chunk 0: all 0.5
        for s in &mut samples[0..50] {
            *s = 0.5;
        }
        // Chunk 1: range [-0.3, 0.8]
        samples[50] = -0.3;
        samples[99] = 0.8;
        // Chunk 2: all -1.0
        for s in &mut samples[100..150] {
            *s = -1.0;
        }
        // Chunk 3: all 1.0
        for s in &mut samples[150..200] {
            *s = 1.0;
        }

        let peaks = compute_peaks(&[samples], 50);
        assert_eq!(peaks[0].len(), 4);
        assert_eq!(peaks[0][0], (0.5, 0.5));
        assert_eq!(peaks[0][1].0, -0.3);
        assert_eq!(peaks[0][1].1, 0.8);
        assert_eq!(peaks[0][2], (-1.0, -1.0));
        assert_eq!(peaks[0][3], (1.0, 1.0));
    }

    #[test]
    fn peak_computation_stereo() {
        let ch0 = vec![0.5f32, 0.5, 0.5, 0.5];
        let ch1 = vec![-0.5f32, -0.5, -0.5, -0.5];
        let peaks = compute_peaks(&[ch0, ch1], 4);
        assert_eq!(peaks.len(), 2);
        assert_eq!(peaks[0][0], (0.5, 0.5));
        assert_eq!(peaks[1][0], (-0.5, -0.5));
    }

    #[test]
    fn peak_computation_partial_last_chunk() {
        // 7 samples with chunk size 4: two chunks (4 + 3)
        let samples = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7];
        let peaks = compute_peaks(&[samples], 4);
        assert_eq!(peaks[0].len(), 2);
        assert_eq!(peaks[0][0], (0.1, 0.4));
        assert_eq!(peaks[0][1], (0.5, 0.7));
    }

    #[test]
    fn peak_computation_single_sample_chunks() {
        let samples = vec![0.1, -0.2, 0.3];
        let peaks = compute_peaks(&[samples], 1);
        assert_eq!(peaks[0].len(), 3);
        assert_eq!(peaks[0][0], (0.1, 0.1));
        assert_eq!(peaks[0][1], (-0.2, -0.2));
        assert_eq!(peaks[0][2], (0.3, 0.3));
    }

    #[test]
    fn peak_computation_empty_channel() {
        let samples: Vec<f32> = vec![];
        let peaks = compute_peaks(&[samples], 10);
        assert_eq!(peaks[0].len(), 0);
    }

    #[test]
    fn samples_per_peak_calculation() {
        // Mirrors sample_rate / peaks_per_second
        let sample_rate = 44100u32;
        let peaks_per_second = 10u32;
        let samples_per_peak = sample_rate / peaks_per_second;
        assert_eq!(samples_per_peak, 4410);

        let sample_rate = 48000u32;
        let peaks_per_second = 100u32;
        let samples_per_peak = sample_rate / peaks_per_second;
        assert_eq!(samples_per_peak, 480);
    }

    #[test]
    fn deinterleave_logic() {
        // Test the deinterleave logic used in extract_waveform
        let channels = 2u16;
        let interleaved = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6];
        let mut all_samples: Vec<Vec<f32>> = (0..channels).map(|_| Vec::new()).collect();
        for (i, sample) in interleaved.iter().enumerate() {
            let channel = i % channels as usize;
            all_samples[channel].push(*sample);
        }
        assert_eq!(all_samples[0], vec![0.1, 0.3, 0.5]); // L
        assert_eq!(all_samples[1], vec![0.2, 0.4, 0.6]); // R
    }

    #[test]
    fn deinterleave_mono() {
        let channels = 1u16;
        let interleaved = [0.1f32, 0.2, 0.3];
        let mut all_samples: Vec<Vec<f32>> = (0..channels).map(|_| Vec::new()).collect();
        for (i, sample) in interleaved.iter().enumerate() {
            let channel = i % channels as usize;
            all_samples[channel].push(*sample);
        }
        assert_eq!(all_samples[0], vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn waveform_data_many_channels() {
        let data = WaveformData {
            sample_rate: 48000,
            channels: 6, // 5.1 surround
            peaks_per_second: 10,
            peaks: (0..6).map(|_| vec![(0.0, 0.0); 10]).collect(),
        };
        assert_eq!(data.channels, 6);
        assert_eq!(data.peaks.len(), 6);
        assert_eq!(data.peaks[5].len(), 10);
    }

    #[test]
    fn waveform_data_high_peaks_per_second() {
        let data = WaveformData {
            sample_rate: 44100,
            channels: 1,
            peaks_per_second: 1000,
            peaks: vec![vec![(0.0, 0.0); 1000]],
        };
        assert_eq!(data.peaks_per_second, 1000);
        assert_eq!(data.peaks[0].len(), 1000);
    }
}
