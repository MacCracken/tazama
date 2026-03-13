use std::path::Path;

use bytes::Bytes;
use tazama_core::ThumbnailSpec;

use crate::decode::video::VideoDecoder;
use crate::error::MediaPipelineError;
use crate::probe;

/// Generate thumbnails from a media file at regular intervals.
///
/// Returns a vector of `(timestamp_ms, rgba_bytes)` pairs.
pub async fn generate_thumbnails(
    path: &Path,
    spec: ThumbnailSpec,
) -> Result<Vec<(u64, Bytes)>, MediaPipelineError> {
    let info = probe::probe(path).await?;

    let Some(video) = info.video_streams.first() else {
        return Err(MediaPipelineError::Decode(
            "no video stream found".into(),
        ));
    };

    let frame_rate = video.frame_rate;
    let duration_ms = info.duration_ms;
    let mut thumbnails = Vec::new();

    let mut timestamp_ms = 0u64;
    while timestamp_ms < duration_ms {
        let frame_index = if frame_rate.1 > 0 {
            (timestamp_ms as f64 * frame_rate.0 as f64 / frame_rate.1 as f64 / 1000.0) as u64
        } else {
            0
        };

        let frame = VideoDecoder::decode_frame(path, frame_index, frame_rate).await?;

        // If the requested size differs from decoded, we return as-is.
        // Scaling would require an additional videoscale pipeline element;
        // for now we trust the caller to handle sizing or we add scaling later.
        let _ = (spec.width, spec.height);

        thumbnails.push((timestamp_ms, frame.data));
        timestamp_ms += spec.interval_ms;
    }

    Ok(thumbnails)
}
