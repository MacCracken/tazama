use tokio::sync::mpsc;
use tazama_core::{ProjectSettings, Timeline};

use crate::context::GpuError;
use crate::frame_source::{FrameSource, GpuFrame};
use crate::render::Renderer;

/// Render all frames of a timeline for export.
///
/// Iterates over every frame in the timeline duration and sends each rendered
/// frame to `frame_tx`, which feeds into the encode pipeline.
pub async fn render_all_frames(
    renderer: &Renderer,
    timeline: &Timeline,
    settings: &ProjectSettings,
    frame_source: &dyn FrameSource,
    frame_tx: &mpsc::Sender<GpuFrame>,
) -> Result<(), GpuError> {
    let total_frames = timeline.duration_frames();

    for frame_index in 0..total_frames {
        let frame = renderer.render_frame(timeline, frame_index, frame_source, settings)?;
        frame_tx
            .send(frame)
            .await
            .map_err(|_| GpuError::FrameSource("export frame channel closed".to_string()))?;
    }

    Ok(())
}
