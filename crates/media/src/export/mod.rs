pub mod pipeline;
#[cfg(feature = "tarang")]
pub mod tarang_pipeline;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::{mpsc, watch};

use crate::decode::{AudioBuffer, VideoFrame};
use crate::error::MediaPipelineError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Mp4,
    WebM,
    ProRes,
    DnxHr,
    Mkv,
    Gif,
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
    #[serde(default)]
    pub hardware_accel: bool,
}

/// Creates an export pipeline, using the best available backend.
///
/// When the `tarang` feature is enabled this will attempt the tarang pipeline
/// first and fall back to GStreamer on failure.  Without the feature flag the
/// GStreamer pipeline is used directly.
pub fn create_export_pipeline(
    config: ExportConfig,
    video_rx: mpsc::Receiver<VideoFrame>,
    audio_rx: mpsc::Receiver<AudioBuffer>,
    total_frames: u64,
) -> Result<watch::Receiver<ExportProgress>, MediaPipelineError> {
    // For now, always use GStreamer. When tarang is fully implemented,
    // add runtime detection.
    pipeline::ExportPipeline::run_with_total(config, video_rx, audio_rx, total_frames)
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

    #[test]
    fn export_format_serde_new_variants() {
        for (variant, expected) in [
            (ExportFormat::ProRes, "\"ProRes\""),
            (ExportFormat::DnxHr, "\"DnxHr\""),
            (ExportFormat::Mkv, "\"Mkv\""),
            (ExportFormat::Gif, "\"Gif\""),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected);
            let back: ExportFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn hardware_accel_defaults_to_false() {
        let json = r#"{
            "output_path": "/tmp/out.mp4",
            "format": "Mp4",
            "width": 1920,
            "height": 1080,
            "frame_rate": [30, 1],
            "sample_rate": 48000,
            "channels": 2
        }"#;
        let config: ExportConfig = serde_json::from_str(json).unwrap();
        assert!(!config.hardware_accel);
    }

    #[test]
    fn hardware_accel_serde_roundtrip() {
        let config = ExportConfig {
            output_path: "/tmp/out.mp4".into(),
            format: ExportFormat::Mp4,
            width: 1920,
            height: 1080,
            frame_rate: (30, 1),
            sample_rate: 48000,
            channels: 2,
            hardware_accel: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: ExportConfig = serde_json::from_str(&json).unwrap();
        assert!(back.hardware_accel);
    }
}
