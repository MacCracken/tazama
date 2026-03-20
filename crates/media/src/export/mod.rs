pub mod pipeline;
pub mod tarang_pipeline;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::{mpsc, watch};
use tracing::info;

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

/// Audio codec selection for export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportAudioCodec {
    Aac,
    Opus,
    Flac,
}

/// Encoder selection for export.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ExportEncoder {
    /// Current behavior: try hw, fall back to sw
    #[default]
    Auto,
    /// Force software encoding
    Software,
    /// Force VAAPI hardware encoding
    Vaapi,
    /// Force NVENC hardware encoding
    Nvenc,
    /// Use tarang encoder when available
    Tarang,
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
    pub audio_codec: Option<ExportAudioCodec>,
    #[serde(default)]
    pub encoder: ExportEncoder,
}

/// Probe which encoder backends are available on this system.
///
/// Uses `ai-hwaccel` to detect hardware accelerators; returns a list of
/// `ExportEncoder` variants that can be used.  `Software` is always included.
pub fn available_encoders() -> Vec<ExportEncoder> {
    let mut encoders = vec![ExportEncoder::Software];

    if crate::hwaccel::has_vaapi() {
        info!("VAAPI encoder available (AMD/Intel GPU detected)");
        encoders.push(ExportEncoder::Vaapi);
    }
    if crate::hwaccel::has_nvenc() {
        info!("NVENC encoder available (NVIDIA GPU detected)");
        encoders.push(ExportEncoder::Nvenc);
    }

    // Tarang encoder is always available since tarang is always-on
    encoders.push(ExportEncoder::Tarang);
    encoders.push(ExportEncoder::Auto);
    encoders
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
    fn fraction_exceeds_total() {
        // frames_written > total_frames should produce fraction > 1.0
        let p = ExportProgress {
            frames_written: 150,
            total_frames: 100,
            done: false,
        };
        assert!(
            p.fraction() > 1.0,
            "fraction should exceed 1.0 when frames > total"
        );
        assert!((p.fraction() - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn export_config_serde_all_formats() {
        for format in [
            ExportFormat::Mp4,
            ExportFormat::WebM,
            ExportFormat::ProRes,
            ExportFormat::DnxHr,
            ExportFormat::Mkv,
            ExportFormat::Gif,
        ] {
            let config = ExportConfig {
                output_path: "/tmp/test.out".into(),
                format,
                width: 1280,
                height: 720,
                frame_rate: (24, 1),
                sample_rate: 44100,
                channels: 2,
                audio_codec: None,
                encoder: ExportEncoder::default(),
            };
            let json = serde_json::to_string(&config).unwrap();
            let back: ExportConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(back.format, format);
            assert_eq!(back.width, 1280);
            assert_eq!(back.height, 720);
        }
    }

    #[test]
    fn export_format_debug() {
        assert_eq!(format!("{:?}", ExportFormat::Mp4), "Mp4");
        assert_eq!(format!("{:?}", ExportFormat::WebM), "WebM");
        assert_eq!(format!("{:?}", ExportFormat::ProRes), "ProRes");
        assert_eq!(format!("{:?}", ExportFormat::DnxHr), "DnxHr");
        assert_eq!(format!("{:?}", ExportFormat::Mkv), "Mkv");
        assert_eq!(format!("{:?}", ExportFormat::Gif), "Gif");
    }

    #[test]
    fn export_encoder_default_is_auto() {
        let enc = ExportEncoder::default();
        assert_eq!(enc, ExportEncoder::Auto);
    }

    #[test]
    fn export_encoder_serde_roundtrip_all_variants() {
        for (variant, expected_json) in [
            (ExportEncoder::Auto, "\"Auto\""),
            (ExportEncoder::Software, "\"Software\""),
            (ExportEncoder::Vaapi, "\"Vaapi\""),
            (ExportEncoder::Nvenc, "\"Nvenc\""),
            (ExportEncoder::Tarang, "\"Tarang\""),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json);
            let back: ExportEncoder = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn export_encoder_debug() {
        assert_eq!(format!("{:?}", ExportEncoder::Auto), "Auto");
        assert_eq!(format!("{:?}", ExportEncoder::Software), "Software");
        assert_eq!(format!("{:?}", ExportEncoder::Vaapi), "Vaapi");
        assert_eq!(format!("{:?}", ExportEncoder::Nvenc), "Nvenc");
        assert_eq!(format!("{:?}", ExportEncoder::Tarang), "Tarang");
    }

    #[test]
    fn export_config_with_all_fields() {
        let config = ExportConfig {
            output_path: "/tmp/full_test.mp4".into(),
            format: ExportFormat::Mp4,
            width: 3840,
            height: 2160,
            frame_rate: (60, 1),
            sample_rate: 96000,
            channels: 6,
            audio_codec: Some(ExportAudioCodec::Aac),
            encoder: ExportEncoder::Nvenc,
        };
        assert_eq!(config.width, 3840);
        assert_eq!(config.height, 2160);
        assert_eq!(config.frame_rate, (60, 1));
        assert_eq!(config.sample_rate, 96000);
        assert_eq!(config.channels, 6);
        assert_eq!(config.audio_codec, Some(ExportAudioCodec::Aac));
        assert_eq!(config.encoder, ExportEncoder::Nvenc);
    }

    #[test]
    fn export_config_audio_codec_variants() {
        for codec in [
            ExportAudioCodec::Aac,
            ExportAudioCodec::Opus,
            ExportAudioCodec::Flac,
        ] {
            let config = ExportConfig {
                output_path: "/tmp/audio_test.mkv".into(),
                format: ExportFormat::Mkv,
                width: 1920,
                height: 1080,
                frame_rate: (30, 1),
                sample_rate: 48000,
                channels: 2,
                audio_codec: Some(codec),
                encoder: ExportEncoder::default(),
            };
            let json = serde_json::to_string(&config).unwrap();
            let back: ExportConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(back.audio_codec, Some(codec));
        }
    }

    #[test]
    fn export_audio_codec_serde_roundtrip() {
        for (variant, expected) in [
            (ExportAudioCodec::Aac, "\"Aac\""),
            (ExportAudioCodec::Opus, "\"Opus\""),
            (ExportAudioCodec::Flac, "\"Flac\""),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected);
            let back: ExportAudioCodec = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn export_config_encoder_defaults_to_auto() {
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
        assert_eq!(config.encoder, ExportEncoder::Auto);
        assert_eq!(config.audio_codec, None);
    }

    #[test]
    fn export_config_serde_with_encoder() {
        for encoder in [
            ExportEncoder::Auto,
            ExportEncoder::Software,
            ExportEncoder::Vaapi,
            ExportEncoder::Nvenc,
            ExportEncoder::Tarang,
        ] {
            let config = ExportConfig {
                output_path: "/tmp/enc_test.mp4".into(),
                format: ExportFormat::Mp4,
                width: 1920,
                height: 1080,
                frame_rate: (30, 1),
                sample_rate: 48000,
                channels: 2,
                audio_codec: None,
                encoder: encoder.clone(),
            };
            let json = serde_json::to_string(&config).unwrap();
            let back: ExportConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(back.encoder, encoder);
        }
    }

    #[test]
    fn export_progress_serde_roundtrip() {
        let p = ExportProgress {
            frames_written: 42,
            total_frames: 100,
            done: false,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ExportProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frames_written, 42);
        assert_eq!(back.total_frames, 100);
        assert!(!back.done);
    }

    #[test]
    fn export_progress_done_serde() {
        let p = ExportProgress {
            frames_written: 200,
            total_frames: 200,
            done: true,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ExportProgress = serde_json::from_str(&json).unwrap();
        assert!(back.done);
        assert!((back.fraction() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn available_encoders_contains_software_and_auto() {
        let encoders = available_encoders();
        assert!(
            encoders.contains(&ExportEncoder::Software),
            "Software encoder must always be available"
        );
        assert!(
            encoders.contains(&ExportEncoder::Auto),
            "Auto must always be in the list"
        );
        // Auto should be last
        assert_eq!(
            encoders.last(),
            Some(&ExportEncoder::Auto),
            "Auto should be the last element"
        );
    }

    #[test]
    fn available_encoders_software_is_first() {
        let encoders = available_encoders();
        assert_eq!(
            encoders.first(),
            Some(&ExportEncoder::Software),
            "Software should be the first element"
        );
    }

    #[test]
    fn export_format_clone_and_copy() {
        let f = ExportFormat::ProRes;
        let f2 = f; // Copy
        let f3 = f;
        assert_eq!(f, f2);
        assert_eq!(f, f3);
    }

    #[test]
    fn export_config_clone() {
        let config = ExportConfig {
            output_path: "/tmp/clone_test.mp4".into(),
            format: ExportFormat::WebM,
            width: 640,
            height: 480,
            frame_rate: (25, 1),
            sample_rate: 22050,
            channels: 1,
            audio_codec: Some(ExportAudioCodec::Opus),
            encoder: ExportEncoder::Software,
        };
        let cloned = config.clone();
        assert_eq!(cloned.output_path, config.output_path);
        assert_eq!(cloned.format, config.format);
        assert_eq!(cloned.width, config.width);
        assert_eq!(cloned.encoder, config.encoder);
        assert_eq!(cloned.audio_codec, config.audio_codec);
    }
}
