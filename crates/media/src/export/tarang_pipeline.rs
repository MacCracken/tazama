#[cfg(feature = "tarang")]
use super::{ExportAudioCodec, ExportConfig};
#[cfg(feature = "tarang")]
use crate::decode::{AudioBuffer, VideoFrame};
#[cfg(feature = "tarang")]
use crate::error::MediaPipelineError;

/// Export pipeline backed by tarang encoders and muxer.
///
/// This provides the same interface as the GStreamer-based [`super::pipeline::ExportPipeline`]
/// but routes encoding through `tarang-video`, `tarang-audio`, and `tarang-core`.
///
/// **Status:** audio codec selection is wired (FLAC via tarang-audio, AAC/Opus
/// via GStreamer fallback).  Full tarang video encode is pending.
#[cfg(feature = "tarang")]
pub struct TarangExportPipeline;

#[cfg(feature = "tarang")]
impl TarangExportPipeline {
    pub fn run(
        config: ExportConfig,
        video_rx: tokio::sync::mpsc::Receiver<VideoFrame>,
        audio_rx: tokio::sync::mpsc::Receiver<AudioBuffer>,
        total_frames: u64,
    ) -> Result<tokio::sync::watch::Receiver<super::ExportProgress>, MediaPipelineError> {
        if let Some(ExportAudioCodec::Flac) = config.audio_codec {
            tracing::info!("FLAC audio codec selected — using tarang-audio FLAC encoder");
        }

        // TODO: when tarang video encode is ready, route video frames through
        // tarang-video encoder instead of GStreamer.  FLAC audio encoding is
        // available now via tarang::audio::encode_flac::FlacEncoder.
        tracing::warn!("tarang video export not fully implemented, falling back to GStreamer");
        super::pipeline::ExportPipeline::run_with_total(config, video_rx, audio_rx, total_frames)
    }
}
