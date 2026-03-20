use std::path::Path;

use bytes::Bytes;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use tokio::sync::mpsc;
use tokio::task;
use tracing::{debug, error};

use super::{DecoderConfig, FrameRange, VideoFrame};
use crate::error::MediaPipelineError;

/// Video file extensions handled by tarang when the feature is enabled.
#[cfg(feature = "tarang")]
const TARANG_VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mkv", "webm"];

#[cfg(feature = "tarang")]
fn is_tarang_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| TARANG_VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// RAII guard that sets a GStreamer pipeline to Null on drop.
struct PipelineGuard(gstreamer::Pipeline);

impl Drop for PipelineGuard {
    fn drop(&mut self) {
        let _ = self.0.set_state(gstreamer::State::Null);
    }
}

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
            #[cfg(feature = "tarang")]
            if is_tarang_video(&path) {
                if let Err(e) = decode_tarang_video(&path, range, tx.clone()) {
                    error!("tarang video decode error: {e}");
                }
                return;
            }

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
        task::spawn_blocking(move || {
            #[cfg(feature = "tarang")]
            if is_tarang_video(&path) {
                return decode_tarang_single_frame(&path, frame_index, frame_rate);
            }

            decode_single_frame(&path, frame_index, frame_rate)
        })
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

        let caps = src_pad
            .current_caps()
            .unwrap_or_else(|| src_pad.query_caps(None));
        let structure = caps.structure(0);
        if let Some(s) = structure
            && s.name().starts_with("video/")
        {
            let Some(sink_pad) = videoconvert.static_pad("sink") else {
                return;
            };
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
    let _guard = PipelineGuard(pipeline.clone());

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

    // Guard handles pipeline cleanup on all exit paths
    Ok(())
}

fn decode_single_frame(
    path: &Path,
    frame_index: u64,
    frame_rate: (u32, u32),
) -> Result<VideoFrame, MediaPipelineError> {
    let (pipeline, appsink) = build_video_pipeline(path)?;
    let _guard = PipelineGuard(pipeline.clone());

    // Seek to the target timestamp — use checked arithmetic to avoid overflow
    let (num, den) = frame_rate;
    let timestamp_ns = if num > 0 {
        frame_index
            .checked_mul(den as u64)
            .and_then(|v| v.checked_mul(1_000_000_000))
            .map(|v| v / num as u64)
            .unwrap_or(u64::MAX)
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

    // Guard handles pipeline cleanup
    Ok(frame)
}

#[cfg(feature = "tarang")]
fn create_tarang_demuxer(
    path: &Path,
) -> Result<Box<dyn tarang::demux::Demuxer>, MediaPipelineError> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 32];
    let n = file.read(&mut header)?;
    drop(file);

    let format = tarang::demux::detect_format(&header[..n])
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let demuxer: Box<dyn tarang::demux::Demuxer> = match format {
        tarang::core::ContainerFormat::Mp4 => Box::new(tarang::demux::Mp4Demuxer::new(reader)),
        tarang::core::ContainerFormat::Mkv | tarang::core::ContainerFormat::WebM => {
            Box::new(tarang::demux::MkvDemuxer::new(reader))
        }
        other => {
            return Err(MediaPipelineError::UnsupportedFormat(format!("{other:?}")));
        }
    };
    Ok(demuxer)
}

#[cfg(feature = "tarang")]
fn create_tarang_decoder(
    codec: tarang::core::VideoCodec,
) -> Result<tarang::video::VideoDecoder, MediaPipelineError> {
    let config = tarang::video::DecoderConfig::for_codec(codec)?;
    let decoder = tarang::video::VideoDecoder::new(config)?;
    Ok(decoder)
}

/// Find the first video stream index and its codec from tarang MediaInfo.
#[cfg(feature = "tarang")]
fn find_video_stream(info: &tarang::core::MediaInfo) -> Option<(usize, tarang::core::VideoCodec)> {
    for (idx, stream) in info.streams.iter().enumerate() {
        if let tarang::core::StreamInfo::Video(vs) = stream {
            return Some((idx, vs.codec));
        }
    }
    None
}

#[cfg(feature = "tarang")]
fn decode_tarang_video(
    path: &Path,
    range: FrameRange,
    tx: mpsc::Sender<VideoFrame>,
) -> Result<(), MediaPipelineError> {
    let mut demuxer = create_tarang_demuxer(path)?;
    let info = demuxer.probe()?;

    let (video_stream_idx, codec) = find_video_stream(&info)
        .ok_or_else(|| MediaPipelineError::Decode("no video stream found".into()))?;

    let mut decoder = create_tarang_decoder(codec)?;

    // Initialize decoder with stream info
    if let Some(tarang::core::StreamInfo::Video(vs)) = info.streams.get(video_stream_idx) {
        decoder.init(vs);
    }

    let mut frame_index = 0u64;

    loop {
        if frame_index > range.end {
            break;
        }

        let packet = match demuxer.next_packet() {
            Ok(p) => p,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("EndOfStream") || msg.contains("end of stream") {
                    break;
                }
                // Try to continue on transient errors
                continue;
            }
        };

        if packet.stream_index != video_stream_idx {
            continue;
        }

        decoder.send_packet(&packet.data, packet.timestamp)?;

        // Drain all available frames from the decoder
        loop {
            match decoder.receive_frame() {
                Ok(tarang_frame) => {
                    if frame_index >= range.start {
                        let frame =
                            crate::convert::tarang_frame_to_tazama(&tarang_frame, frame_index)?;
                        if tx.blocking_send(frame).is_err() {
                            debug!("tarang video decode receiver dropped");
                            return Ok(());
                        }
                    }
                    frame_index += 1;
                    if frame_index > range.end {
                        return Ok(());
                    }
                }
                Err(_) => break, // No more frames available, need more packets
            }
        }
    }

    // Flush remaining frames
    if let Ok(()) = decoder.flush() {
        loop {
            match decoder.receive_frame() {
                Ok(tarang_frame) => {
                    if frame_index >= range.start && frame_index <= range.end {
                        let frame =
                            crate::convert::tarang_frame_to_tazama(&tarang_frame, frame_index)?;
                        if tx.blocking_send(frame).is_err() {
                            break;
                        }
                    }
                    frame_index += 1;
                }
                Err(_) => break,
            }
        }
    }

    Ok(())
}

#[cfg(feature = "tarang")]
fn decode_tarang_single_frame(
    path: &Path,
    frame_index: u64,
    frame_rate: (u32, u32),
) -> Result<VideoFrame, MediaPipelineError> {
    let mut demuxer = create_tarang_demuxer(path)?;
    let info = demuxer.probe()?;

    let (video_stream_idx, codec) = find_video_stream(&info)
        .ok_or_else(|| MediaPipelineError::Decode("no video stream found".into()))?;

    let mut decoder = create_tarang_decoder(codec)?;

    if let Some(tarang::core::StreamInfo::Video(vs)) = info.streams.get(video_stream_idx) {
        decoder.init(vs);
    }

    // Seek to the target timestamp
    let (num, den) = frame_rate;
    let timestamp = if num > 0 {
        std::time::Duration::from_nanos(
            frame_index
                .checked_mul(den as u64)
                .and_then(|v| v.checked_mul(1_000_000_000))
                .map(|v| v / num as u64)
                .unwrap_or(u64::MAX),
        )
    } else {
        std::time::Duration::ZERO
    };

    let _ = demuxer.seek(timestamp);

    // Decode until we get a frame
    loop {
        let packet = demuxer
            .next_packet()
            .map_err(|e| MediaPipelineError::Decode(e.to_string()))?;

        if packet.stream_index != video_stream_idx {
            continue;
        }

        decoder.send_packet(&packet.data, packet.timestamp)?;

        if let Ok(tarang_frame) = decoder.receive_frame() {
            return crate::convert::tarang_frame_to_tazama(&tarang_frame, frame_index);
        }
    }
}

fn sample_to_frame(sample: &gstreamer::Sample, frame_index: u64) -> Option<VideoFrame> {
    let buffer = sample.buffer()?;
    let caps = sample.caps()?;
    let structure = caps.structure(0)?;

    let width = structure.get::<i32>("width").ok()? as u32;
    let height = structure.get::<i32>("height").ok()? as u32;

    let map = buffer.map_readable().ok()?;
    let data = Bytes::copy_from_slice(map.as_slice());

    let timestamp_ns = buffer.pts().map(|pts| pts.nseconds()).unwrap_or(0);

    Some(VideoFrame {
        frame_index,
        width,
        height,
        data,
        timestamp_ns,
    })
}
