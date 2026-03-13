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
