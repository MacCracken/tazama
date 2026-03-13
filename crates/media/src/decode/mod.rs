pub mod audio;
pub mod video;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a decode session.
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    pub path: PathBuf,
}

/// A decoded video frame in RGBA format.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub data: Bytes,
    pub timestamp_ns: u64,
}

/// A decoded audio buffer in interleaved f32 format.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
    pub timestamp_ns: u64,
}

/// Range of frames to decode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FrameRange {
    pub start: u64,
    pub end: u64,
}
