use thiserror::Error;

use crate::context::GpuContext;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("pipeline creation failed: {0}")]
    Creation(String),
}

/// A Vulkan compute pipeline for video effects processing.
pub struct ComputePipeline {
    _ctx: (),
}

impl ComputePipeline {
    pub fn new(_ctx: &GpuContext) -> Result<Self, PipelineError> {
        // TODO: create compute pipeline for color grading, transitions, etc.
        Ok(Self { _ctx: () })
    }
}
