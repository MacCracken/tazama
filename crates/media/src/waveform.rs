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
}
