#[cfg(feature = "tarang")]
use super::ExportConfig;
#[cfg(feature = "tarang")]
use crate::decode::{AudioBuffer, VideoFrame};
#[cfg(feature = "tarang")]
use crate::error::MediaPipelineError;

/// Export pipeline backed by tarang encoders and muxer.
///
/// This provides the same interface as the GStreamer-based [`super::pipeline::ExportPipeline`]
/// but routes encoding through `tarang-video`, `tarang-audio`, and `tarang-core`.
///
/// **Status:** stub implementation that logs a warning and falls back to the
/// GStreamer pipeline.  Once tarang encoder support is complete this will
/// perform the full encode natively.
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
        // Similar to GStreamer pipeline but using tarang-video encoder + tarang-audio encoder + tarang-core muxer
        // Spawn blocking task that:
        // 1. Creates tarang muxer for the output format
        // 2. Feeds video frames through tarang-video encoder
        // 3. Feeds audio buffers through tarang-audio encoder
        // 4. Muxes into output file
        // For now, create a stub that logs a warning and falls back

        tracing::warn!("tarang export pipeline not fully implemented, falling back to GStreamer");
        super::pipeline::ExportPipeline::run_with_total(config, video_rx, audio_rx, total_frames)
    }
}
