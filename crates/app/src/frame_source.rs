use std::path::Path;
use std::sync::Mutex;

use tazama_gpu::GpuError;
use tazama_gpu::frame_source::{FrameSource, GpuFrame};
use tazama_media::decode::video::VideoDecoder;

/// Bridges the media decoder to the GPU renderer's FrameSource trait.
///
/// Decodes individual video frames on demand using GStreamer, producing
/// RGBA data the GPU renderer can upload and process.
pub struct MediaFrameSource {
    frame_rate: (u32, u32),
    /// Cache the last decoded frame to avoid redundant decodes when the same
    /// frame is requested multiple times (e.g. transitions read two clips).
    cache: Mutex<Option<(String, u64, GpuFrame)>>,
}

impl MediaFrameSource {
    pub fn new(frame_rate: (u32, u32)) -> Self {
        Self {
            frame_rate,
            cache: Mutex::new(None),
        }
    }
}

impl FrameSource for MediaFrameSource {
    fn get_frame(&self, media_path: &str, frame_index: u64) -> Result<GpuFrame, GpuError> {
        // Check cache
        {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((ref path, idx, ref frame)) = *cache
                && path == media_path
                && idx == frame_index
            {
                return Ok(GpuFrame {
                    frame_index: frame.frame_index,
                    width: frame.width,
                    height: frame.height,
                    data: frame.data.clone(),
                    timestamp_ns: frame.timestamp_ns,
                });
            }
        }

        let path = Path::new(media_path);
        let frame_rate = self.frame_rate;

        // Decode on a blocking thread (GStreamer is synchronous internally)
        let video_frame = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { VideoDecoder::decode_frame(path, frame_index, frame_rate).await })
        })
        .map_err(|e| GpuError::FrameSource(e.to_string()))?;

        let gpu_frame = GpuFrame {
            frame_index: video_frame.frame_index,
            width: video_frame.width,
            height: video_frame.height,
            data: video_frame.data.clone(),
            timestamp_ns: video_frame.timestamp_ns,
        };

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            *cache = Some((
                media_path.to_string(),
                frame_index,
                GpuFrame {
                    frame_index: video_frame.frame_index,
                    width: video_frame.width,
                    height: video_frame.height,
                    data: video_frame.data,
                    timestamp_ns: video_frame.timestamp_ns,
                },
            ));
        }

        Ok(gpu_frame)
    }
}
