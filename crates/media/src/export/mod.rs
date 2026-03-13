pub mod pipeline;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Mp4,
    WebM,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub output_path: PathBuf,
    pub format: ExportFormat,
    pub width: u32,
    pub height: u32,
    pub frame_rate: (u32, u32),
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    pub frames_written: u64,
    pub total_frames: u64,
    pub done: bool,
}

impl ExportProgress {
    pub fn fraction(&self) -> f64 {
        if self.total_frames == 0 {
            0.0
        } else {
            self.frames_written as f64 / self.total_frames as f64
        }
    }
}
