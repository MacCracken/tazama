use bytes::Bytes;

use crate::decode::VideoFrame;
use crate::error::MediaPipelineError;

/// Convert a tarang `VideoFrame` (YUV420p or RGB24) to RGBA bytes.
///
/// Uses `tarang_ai::yuv420p_to_rgb24` for the YUV->RGB conversion,
/// then expands RGB24 to RGBA32 with alpha=255.
pub fn yuv420p_to_rgba(frame: &tarang_core::VideoFrame) -> Result<Vec<u8>, MediaPipelineError> {
    let rgb = tarang_ai::yuv420p_to_rgb24(frame)
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?;

    let pixel_count = rgb.len() / 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for chunk in rgb.chunks_exact(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(255);
    }
    Ok(rgba)
}

/// Convert a tarang `VideoFrame` to a tazama `VideoFrame`.
pub fn tarang_frame_to_tazama(
    frame: &tarang_core::VideoFrame,
    frame_index: u64,
) -> Result<VideoFrame, MediaPipelineError> {
    let rgba = yuv420p_to_rgba(frame)?;
    Ok(VideoFrame {
        frame_index,
        width: frame.width,
        height: frame.height,
        data: Bytes::from(rgba),
        timestamp_ns: frame.timestamp.as_nanos() as u64,
    })
}
