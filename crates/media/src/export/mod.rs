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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_zero_total() {
        let p = ExportProgress {
            frames_written: 0,
            total_frames: 0,
            done: false,
        };
        assert_eq!(p.fraction(), 0.0);
    }

    #[test]
    fn fraction_half() {
        let p = ExportProgress {
            frames_written: 50,
            total_frames: 100,
            done: false,
        };
        assert!((p.fraction() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn fraction_complete() {
        let p = ExportProgress {
            frames_written: 100,
            total_frames: 100,
            done: true,
        };
        assert!((p.fraction() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn export_format_serde() {
        let json = serde_json::to_string(&ExportFormat::Mp4).unwrap();
        assert_eq!(json, "\"Mp4\"");
        let back: ExportFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ExportFormat::Mp4);

        let json = serde_json::to_string(&ExportFormat::WebM).unwrap();
        let back: ExportFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ExportFormat::WebM);
    }
}
