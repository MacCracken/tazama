use std::path::Path;

use bytes::Bytes;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use tokio::sync::mpsc;
use tokio::task;
use tracing::{debug, error};

use super::{DecoderConfig, FrameRange, VideoFrame};
use crate::error::MediaPipelineError;

/// Decodes video frames from a media file.
pub struct VideoDecoder {
    pub config: DecoderConfig,
}

impl VideoDecoder {
    pub fn new(config: DecoderConfig) -> Self {
        Self { config }
    }

    /// Decode a range of frames, sending them over a channel.
    pub fn decode(
        &self,
        range: FrameRange,
    ) -> Result<mpsc::Receiver<VideoFrame>, MediaPipelineError> {
        let path = self.config.path.clone();
        let (tx, rx) = mpsc::channel(16);

        task::spawn_blocking(move || {
            if let Err(e) = decode_video_range(&path, range, tx.clone()) {
                error!("video decode error: {e}");
            }
        });

        Ok(rx)
    }

    /// Decode a single frame at a specific index.
    pub async fn decode_frame(
        path: &Path,
        frame_index: u64,
        frame_rate: (u32, u32),
    ) -> Result<VideoFrame, MediaPipelineError> {
        let path = path.to_path_buf();
        task::spawn_blocking(move || decode_single_frame(&path, frame_index, frame_rate))
            .await
            .map_err(|e| MediaPipelineError::Decode(e.to_string()))?
    }
}

fn build_video_pipeline(path: &Path) -> Result<(gstreamer::Pipeline, AppSink), MediaPipelineError> {
    let pipeline = gstreamer::Pipeline::new();

    let filesrc = gstreamer::ElementFactory::make("filesrc")
        .property("location", path.to_str().unwrap_or_default())
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let decodebin = gstreamer::ElementFactory::make("decodebin")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let videoconvert = gstreamer::ElementFactory::make("videoconvert")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let appsink = gstreamer::ElementFactory::make("appsink")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?
        .dynamic_cast::<AppSink>()
        .map_err(|_| MediaPipelineError::Gstreamer("failed to cast to AppSink".into()))?;

    let caps = gstreamer_video::VideoCapsBuilder::new()
        .format(gstreamer_video::VideoFormat::Rgba)
        .build();
    appsink.set_caps(Some(&caps));
    appsink.set_drop(false);
    appsink.set_sync(false);

    pipeline
        .add_many([
            &filesrc,
            &decodebin,
            &videoconvert,
            appsink.upcast_ref::<gstreamer::Element>(),
        ])
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    filesrc
        .link(&decodebin)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // decodebin uses dynamic pads — connect on pad-added
    let videoconvert_weak = videoconvert.downgrade();
    decodebin.connect_pad_added(move |_, src_pad| {
        let Some(videoconvert) = videoconvert_weak.upgrade() else {
            return;
        };

        let caps = src_pad.current_caps().unwrap_or_else(|| src_pad.query_caps(None));
        let structure = caps.structure(0);
        if let Some(s) = structure
            && s.name().starts_with("video/")
        {
            let sink_pad = videoconvert.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                let _ = src_pad.link(&sink_pad);
            }
        }
    });

    videoconvert
        .link(&appsink)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    Ok((pipeline, appsink))
}

fn decode_video_range(
    path: &Path,
    range: FrameRange,
    tx: mpsc::Sender<VideoFrame>,
) -> Result<(), MediaPipelineError> {
    let (pipeline, appsink) = build_video_pipeline(path)?;

    pipeline
        .set_state(gstreamer::State::Playing)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    let mut frame_index = 0u64;

    loop {
        if frame_index > range.end {
            break;
        }

        let sample = match appsink.pull_sample() {
            Ok(sample) => sample,
            Err(_) => break, // EOS
        };

        if frame_index >= range.start
            && let Some(frame) = sample_to_frame(&sample, frame_index)
            && tx.blocking_send(frame).is_err()
        {
            debug!("video decode receiver dropped");
            break;
        }

        frame_index += 1;
    }

    pipeline
        .set_state(gstreamer::State::Null)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    Ok(())
}

fn decode_single_frame(
    path: &Path,
    frame_index: u64,
    frame_rate: (u32, u32),
) -> Result<VideoFrame, MediaPipelineError> {
    let (pipeline, appsink) = build_video_pipeline(path)?;

    // Seek to the target timestamp
    let (num, den) = frame_rate;
    let timestamp_ns = if num > 0 {
        (frame_index * den as u64 * 1_000_000_000) / num as u64
    } else {
        0
    };

    pipeline
        .set_state(gstreamer::State::Paused)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    // Wait for state change
    let _ = pipeline.state(gstreamer::ClockTime::from_seconds(5));

    let seek_pos = gstreamer::ClockTime::from_nseconds(timestamp_ns);
    pipeline
        .seek_simple(
            gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
            seek_pos,
        )
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    pipeline
        .set_state(gstreamer::State::Playing)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    let sample = appsink
        .pull_sample()
        .map_err(|_| MediaPipelineError::Decode("failed to pull frame after seek".into()))?;

    let frame = sample_to_frame(&sample, frame_index)
        .ok_or_else(|| MediaPipelineError::Decode("failed to extract frame data".into()))?;

    pipeline
        .set_state(gstreamer::State::Null)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    Ok(frame)
}

fn sample_to_frame(sample: &gstreamer::Sample, frame_index: u64) -> Option<VideoFrame> {
    let buffer = sample.buffer()?;
    let caps = sample.caps()?;
    let structure = caps.structure(0)?;

    let width = structure.get::<i32>("width").ok()? as u32;
    let height = structure.get::<i32>("height").ok()? as u32;

    let map = buffer.map_readable().ok()?;
    let data = Bytes::copy_from_slice(map.as_slice());

    let timestamp_ns = buffer
        .pts()
        .map(|pts| pts.nseconds())
        .unwrap_or(0);

    Some(VideoFrame {
        frame_index,
        width,
        height,
        data,
        timestamp_ns,
    })
}
