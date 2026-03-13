use tazama_core::Timeline;

use crate::context::{GpuContext, GpuError};

/// Renders timeline frames using the GPU compute pipeline.
pub struct Renderer {
    _ctx: (),
}

impl Renderer {
    pub fn new(_ctx: &GpuContext) -> Self {
        Self { _ctx: () }
    }

    /// Render a single frame of the timeline at the given frame index.
    pub fn render_frame(&self, _timeline: &Timeline, _frame: u64) -> Result<Vec<u8>, GpuError> {
        // TODO: composite all visible clips at this frame
        // 1. Determine which clips are active at `frame`
        // 2. Decode source frames (via GStreamer)
        // 3. Apply effects via compute shaders
        // 4. Composite layers
        // 5. Return RGBA buffer
        Ok(Vec::new())
    }
}
