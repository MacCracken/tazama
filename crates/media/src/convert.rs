use bytes::Bytes;

use crate::decode::VideoFrame;
use crate::error::MediaPipelineError;

/// Convert a tarang `VideoFrame` (YUV420p or RGB24) to RGBA bytes.
///
/// Uses `tarang::video::convert::yuv420p_to_rgb24` for YUV→RGB, then
/// delegates to ranga for RGB24→RGBA32 expansion.
pub fn yuv420p_to_rgba(frame: &tarang::core::VideoFrame) -> Result<Vec<u8>, MediaPipelineError> {
    let rgb = tarang::video::convert::yuv420p_to_rgb24(frame)
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?;

    // Use ranga's RGB8→RGBA8 conversion (adds alpha=255)
    let rgb_buf = ranga::pixel::PixelBuffer::new(
        rgb.data.to_vec(),
        rgb.width,
        rgb.height,
        ranga::pixel::PixelFormat::Rgb8,
    )
    .map_err(|e| MediaPipelineError::Decode(e.to_string()))?;

    let rgba_buf = ranga::convert::rgb8_to_rgba8(&rgb_buf)
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?;

    Ok(rgba_buf.data)
}

/// Convert a tarang `VideoFrame` to a tazama `VideoFrame`.
pub fn tarang_frame_to_tazama(
    frame: &tarang::core::VideoFrame,
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
