use bytes::Bytes;

use crate::context::GpuError;

/// A decoded RGBA frame for GPU processing.
pub struct GpuFrame {
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub data: Bytes,
    pub timestamp_ns: u64,
}

/// Trait for providing decoded video frames to the renderer.
///
/// Implemented by the media layer to decouple GPU rendering from media decoding.
pub trait FrameSource: Send + Sync {
    fn get_frame(&self, media_path: &str, frame_index: u64) -> Result<GpuFrame, GpuError>;
}
