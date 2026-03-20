use std::io::{Seek, Write};
use std::time::Duration;

use bytes::Bytes;
use tarang::demux::mux::{MkvMuxer, Mp4Muxer, Muxer, MuxConfig, VideoMuxConfig};
use tokio::sync::watch;
use tracing::{debug, error, info, info_span, warn};

use super::{ExportAudioCodec, ExportConfig, ExportFormat, ExportProgress};
use crate::decode::{AudioBuffer, VideoFrame};
use crate::error::MediaPipelineError;

/// Export pipeline backed by tarang encoders and muxer.
///
/// This provides the same interface as the GStreamer-based [`super::pipeline::ExportPipeline`]
/// but routes encoding through tarang video encoders (openh264) and audio encoders
/// (FLAC), with tarang's muxers for MKV/MP4 output.
///
/// **Supported natively:** MKV, MP4 (H.264 video + audio via tarang encoders + muxers)
/// **Falls back to GStreamer:** WebM, ProRes, DnxHr, GIF
pub struct TarangExportPipeline;

impl TarangExportPipeline {
    pub fn run(
        config: ExportConfig,
        video_rx: tokio::sync::mpsc::Receiver<VideoFrame>,
        audio_rx: tokio::sync::mpsc::Receiver<AudioBuffer>,
        total_frames: u64,
    ) -> Result<watch::Receiver<ExportProgress>, MediaPipelineError> {
        match config.format {
            ExportFormat::Mkv | ExportFormat::Mp4 => {
                info!(
                    "tarang export pipeline: {:?} format, encoding via tarang (H.264 + audio)",
                    config.format
                );
                Self::run_tarang(config, video_rx, audio_rx, total_frames)
            }
            ExportFormat::WebM
            | ExportFormat::ProRes
            | ExportFormat::DnxHr
            | ExportFormat::Gif => {
                info!(
                    "tarang export pipeline: {:?} format not fully supported by tarang, \
                     falling back to GStreamer",
                    config.format
                );
                super::pipeline::ExportPipeline::run_with_total(
                    config,
                    video_rx,
                    audio_rx,
                    total_frames,
                )
            }
        }
    }

    fn run_tarang(
        config: ExportConfig,
        mut video_rx: tokio::sync::mpsc::Receiver<VideoFrame>,
        mut audio_rx: tokio::sync::mpsc::Receiver<AudioBuffer>,
        total_frames: u64,
    ) -> Result<watch::Receiver<ExportProgress>, MediaPipelineError> {
        let (progress_tx, progress_rx) = watch::channel(ExportProgress {
            frames_written: 0,
            total_frames,
            done: false,
        });

        let config_clone = config.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = run_tarang_export(
                config_clone,
                &mut video_rx,
                &mut audio_rx,
                &progress_tx,
                total_frames,
            ) {
                error!("tarang export pipeline error: {e}");
            }
            let _ = progress_tx.send(ExportProgress {
                frames_written: total_frames,
                total_frames,
                done: true,
            });
        });

        Ok(progress_rx)
    }
}

/// Video encoder wrapper around openh264 (H.264).
///
/// When the `vpx-enc` feature becomes available, this can be extended to
/// support VP9 encoding for WebM via `tarang::video::VpxEncoder`.
struct VideoEncoder {
    inner: tarang::video::OpenH264Encoder,
}

impl VideoEncoder {
    /// Encode a YUV420p frame. Returns encoded H.264 NAL units.
    fn encode(&mut self, frame: &tarang::core::VideoFrame) -> Result<Vec<u8>, MediaPipelineError> {
        self.inner
            .encode(frame)
            .map_err(|e| MediaPipelineError::Tarang(e.to_string()))
    }
}

fn create_video_encoder(config: &ExportConfig) -> Result<VideoEncoder, MediaPipelineError> {
    info!(
        "creating H.264 encoder via openh264 for {:?}",
        config.format
    );
    let h264_config = tarang::video::OpenH264EncoderConfig {
        width: config.width,
        height: config.height,
        bitrate_bps: compute_video_bitrate(config.width, config.height),
        frame_rate_num: config.frame_rate.0,
        frame_rate_den: config.frame_rate.1,
    };
    let inner = tarang::video::OpenH264Encoder::new(&h264_config)
        .map_err(|e| MediaPipelineError::Tarang(e.to_string()))?;
    Ok(VideoEncoder { inner })
}

/// Compute a reasonable video bitrate based on resolution.
fn compute_video_bitrate(width: u32, height: u32) -> u32 {
    let pixels = (width as u64) * (height as u64);
    // ~5 Mbps for 1080p, scale linearly
    let base_pixels = 1920u64 * 1080;
    let base_bitrate = 5_000_000u64;
    ((pixels * base_bitrate) / base_pixels).clamp(500_000, 50_000_000) as u32
}

fn select_audio_codec(config: &ExportConfig) -> tarang::core::AudioCodec {
    match config.audio_codec {
        Some(ExportAudioCodec::Flac) => tarang::core::AudioCodec::Flac,
        Some(ExportAudioCodec::Opus) => tarang::core::AudioCodec::Opus,
        Some(ExportAudioCodec::Aac) => tarang::core::AudioCodec::Aac,
        None => match config.format {
            // Default audio codec per container
            ExportFormat::WebM => tarang::core::AudioCodec::Opus,
            ExportFormat::Mkv => tarang::core::AudioCodec::Opus,
            ExportFormat::Mp4 => tarang::core::AudioCodec::Aac,
            _ => tarang::core::AudioCodec::Opus,
        },
    }
}

/// Convert RGBA pixel data to YUV420p via tarang's pixel format conversion.
fn rgba_to_yuv420p(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|c| &c[..3]).copied().collect();
    let rgb_frame = tarang::core::VideoFrame {
        data: Bytes::from(rgb),
        pixel_format: tarang::core::PixelFormat::Rgb24,
        width,
        height,
        timestamp: Duration::ZERO,
    };
    tarang::video::convert::rgb24_to_yuv420p(&rgb_frame)
        .expect("RGB24 to YUV420p conversion")
        .data
        .to_vec()
}

/// Convert tazama AudioBuffer (interleaved f32 samples) to tarang AudioBuffer (Bytes).
fn convert_audio_buffer(buf: &AudioBuffer) -> tarang::core::AudioBuffer {
    let byte_data: Vec<u8> = buf.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let num_frames = buf.samples.len() / buf.channels.max(1) as usize;
    tarang::core::AudioBuffer {
        data: Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels: buf.channels,
        sample_rate: buf.sample_rate,
        num_frames,
        timestamp: Duration::from_nanos(buf.timestamp_ns),
    }
}

/// Build `MuxConfig` for audio from the export configuration.
fn build_audio_mux_config(config: &ExportConfig, audio_codec: tarang::core::AudioCodec) -> MuxConfig {
    MuxConfig {
        codec: audio_codec,
        sample_rate: config.sample_rate,
        channels: config.channels,
        bits_per_sample: 16,
    }
}

/// Build `VideoMuxConfig` from the export configuration.
fn build_video_mux_config(config: &ExportConfig) -> VideoMuxConfig {
    VideoMuxConfig {
        codec: tarang::core::VideoCodec::H264,
        width: config.width,
        height: config.height,
    }
}

/// Trait object wrapper so MKV and MP4 muxers share the same write path.
trait ExportMuxer {
    fn write_header(&mut self) -> Result<(), MediaPipelineError>;
    fn write_video_packet(&mut self, data: &[u8]) -> Result<(), MediaPipelineError>;
    fn write_audio_packet(&mut self, data: &[u8]) -> Result<(), MediaPipelineError>;
    fn finalize(&mut self) -> Result<(), MediaPipelineError>;
}

struct MkvExportMuxer<W: Write>(MkvMuxer<W>);

impl<W: Write> ExportMuxer for MkvExportMuxer<W> {
    fn write_header(&mut self) -> Result<(), MediaPipelineError> {
        self.0
            .write_header()
            .map_err(|e| MediaPipelineError::Export(format!("MKV header: {e}")))
    }
    fn write_video_packet(&mut self, data: &[u8]) -> Result<(), MediaPipelineError> {
        self.0
            .write_video_packet(data)
            .map_err(|e| MediaPipelineError::Export(format!("MKV video packet: {e}")))
    }
    fn write_audio_packet(&mut self, data: &[u8]) -> Result<(), MediaPipelineError> {
        self.0
            .write_packet(data)
            .map_err(|e| MediaPipelineError::Export(format!("MKV audio packet: {e}")))
    }
    fn finalize(&mut self) -> Result<(), MediaPipelineError> {
        self.0
            .finalize()
            .map_err(|e| MediaPipelineError::Export(format!("MKV finalize: {e}")))
    }
}

struct Mp4ExportMuxer<W: Write + Seek>(Mp4Muxer<W>);

impl<W: Write + Seek> ExportMuxer for Mp4ExportMuxer<W> {
    fn write_header(&mut self) -> Result<(), MediaPipelineError> {
        self.0
            .write_header()
            .map_err(|e| MediaPipelineError::Export(format!("MP4 header: {e}")))
    }
    fn write_video_packet(&mut self, data: &[u8]) -> Result<(), MediaPipelineError> {
        self.0
            .write_video_packet(data)
            .map_err(|e| MediaPipelineError::Export(format!("MP4 video packet: {e}")))
    }
    fn write_audio_packet(&mut self, data: &[u8]) -> Result<(), MediaPipelineError> {
        self.0
            .write_packet(data)
            .map_err(|e| MediaPipelineError::Export(format!("MP4 audio packet: {e}")))
    }
    fn finalize(&mut self) -> Result<(), MediaPipelineError> {
        self.0
            .finalize()
            .map_err(|e| MediaPipelineError::Export(format!("MP4 finalize: {e}")))
    }
}

fn run_tarang_export(
    config: ExportConfig,
    video_rx: &mut tokio::sync::mpsc::Receiver<VideoFrame>,
    audio_rx: &mut tokio::sync::mpsc::Receiver<AudioBuffer>,
    progress_tx: &watch::Sender<ExportProgress>,
    total_frames: u64,
) -> Result<(), MediaPipelineError> {
    let _span = info_span!("tarang_export_pipeline", format = ?config.format,
        width = config.width, height = config.height)
    .entered();

    // Create output file
    let file = std::fs::File::create(&config.output_path)?;
    let writer = std::io::BufWriter::new(file);

    // Create video encoder
    let mut video_encoder = create_video_encoder(&config)?;

    // Select and create audio encoder
    let audio_codec = select_audio_codec(&config);
    let audio_enc_config = tarang::audio::EncoderConfig {
        codec: audio_codec,
        sample_rate: config.sample_rate,
        channels: config.channels,
        bits_per_sample: 16,
    };
    let mut audio_encoder = match tarang::audio::create_encoder(&audio_enc_config) {
        Ok(enc) => Some(enc),
        Err(e) => {
            warn!(
                "tarang audio encoder for {:?} not available: {e}, audio will be skipped",
                audio_codec
            );
            None
        }
    };

    // Build mux configs
    let audio_mux_config = build_audio_mux_config(&config, audio_codec);
    let video_mux_config = build_video_mux_config(&config);

    let frame_rate = config.frame_rate.0 as f64 / config.frame_rate.1.max(1) as f64;

    // Create muxer based on format
    let mut muxer: Box<dyn ExportMuxer> = match config.format {
        ExportFormat::Mp4 => Box::new(Mp4ExportMuxer(
            Mp4Muxer::new_with_video(writer, audio_mux_config, video_mux_config),
        )),
        _ => Box::new(MkvExportMuxer(
            MkvMuxer::new_webm(writer, audio_mux_config, video_mux_config),
        )),
    };
    muxer.write_header()?;

    info!(
        "tarang export started: {:?}, {}x{}, {:.2} fps, audio={:?}",
        config.output_path, config.width, config.height, frame_rate, audio_codec
    );

    // Process video frames
    let mut frames_written = 0u64;
    while let Some(frame) = video_rx.blocking_recv() {
        // Convert RGBA -> YUV420p
        let yuv_data = rgba_to_yuv420p(&frame.data, frame.width, frame.height);
        let tarang_frame = tarang::core::VideoFrame {
            data: Bytes::from(yuv_data),
            pixel_format: tarang::core::PixelFormat::Yuv420p,
            width: frame.width,
            height: frame.height,
            timestamp: Duration::from_nanos(frame.timestamp_ns),
        };

        // Encode
        let encoded = video_encoder.encode(&tarang_frame)?;

        // Write encoded data to muxer
        if !encoded.is_empty() {
            muxer.write_video_packet(&encoded)?;
        }

        frames_written += 1;
        let _ = progress_tx.send(ExportProgress {
            frames_written,
            total_frames,
            done: false,
        });
    }

    debug!("video encoding complete: {frames_written} frames");

    // Process audio buffers
    let mut audio_packets_written = 0u64;
    while let Some(audio_buf) = audio_rx.blocking_recv() {
        if audio_buf.samples.is_empty() {
            continue;
        }

        if let Some(ref mut enc) = audio_encoder {
            let tarang_buf = convert_audio_buffer(&audio_buf);

            match enc.encode(&tarang_buf) {
                Ok(packets) => {
                    for packet in &packets {
                        if !packet.is_empty() {
                            muxer.write_audio_packet(packet)?;
                            audio_packets_written += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("audio encode error (skipping buffer): {e}");
                }
            }
        }
    }

    // Flush audio encoder
    if let Some(ref mut enc) = audio_encoder {
        match enc.flush() {
            Ok(packets) => {
                for packet in &packets {
                    if !packet.is_empty() {
                        muxer.write_audio_packet(packet)?;
                        audio_packets_written += 1;
                    }
                }
            }
            Err(e) => {
                warn!("audio encoder flush error: {e}");
            }
        }
    }

    debug!("audio encoding complete: {audio_packets_written} packets");

    // Finalize
    muxer.finalize()?;

    info!("tarang export complete: {:?}", config.output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_to_yuv420p_basic() {
        // 4x4 white image (RGBA)
        let rgba = vec![255u8; 4 * 4 * 4];
        let yuv = rgba_to_yuv420p(&rgba, 4, 4);
        // Y plane: 4*4 = 16 bytes
        // U plane: 2*2 = 4 bytes
        // V plane: 2*2 = 4 bytes
        assert_eq!(yuv.len(), 16 + 4 + 4);
        // White → Y≈255
        assert!(yuv[0] > 250);
    }

    #[test]
    fn rgba_to_yuv420p_black() {
        let rgba = vec![0u8; 4 * 4 * 4]; // black, alpha=0
        let yuv = rgba_to_yuv420p(&rgba, 4, 4);
        assert_eq!(yuv.len(), 24);
        // Black → Y=0
        assert_eq!(yuv[0], 0);
        // U,V should be 128 (neutral chroma)
        assert_eq!(yuv[16], 128);
        assert_eq!(yuv[20], 128);
    }

    #[test]
    fn rgba_to_yuv420p_size() {
        let w = 320u32;
        let h = 240u32;
        let rgba = vec![128u8; (w * h * 4) as usize];
        let yuv = rgba_to_yuv420p(&rgba, w, h);
        let expected = (w * h + 2 * (w / 2) * (h / 2)) as usize;
        assert_eq!(yuv.len(), expected);
    }

    #[test]
    fn convert_audio_buffer_basic() {
        let buf = AudioBuffer {
            sample_rate: 48000,
            channels: 2,
            samples: vec![0.5f32, -0.5, 0.25, -0.25],
            timestamp_ns: 1_000_000,
        };
        let tarang_buf = convert_audio_buffer(&buf);
        assert_eq!(tarang_buf.sample_rate, 48000);
        assert_eq!(tarang_buf.channels, 2);
        assert_eq!(tarang_buf.num_frames, 2); // 4 samples / 2 channels
        assert_eq!(tarang_buf.data.len(), 16); // 4 floats * 4 bytes
        assert_eq!(tarang_buf.timestamp, Duration::from_millis(1));
    }

    #[test]
    fn select_audio_codec_defaults() {
        let config = ExportConfig {
            output_path: "/tmp/test.webm".into(),
            format: ExportFormat::WebM,
            width: 1920,
            height: 1080,
            frame_rate: (30, 1),
            sample_rate: 48000,
            channels: 2,
            audio_codec: None,
            encoder: super::super::ExportEncoder::default(),
        };
        assert_eq!(select_audio_codec(&config), tarang::core::AudioCodec::Opus);

        let mkv_config = ExportConfig {
            format: ExportFormat::Mkv,
            ..config.clone()
        };
        assert_eq!(
            select_audio_codec(&mkv_config),
            tarang::core::AudioCodec::Opus
        );
    }

    #[test]
    fn select_audio_codec_explicit() {
        let config = ExportConfig {
            output_path: "/tmp/test.mkv".into(),
            format: ExportFormat::Mkv,
            width: 1920,
            height: 1080,
            frame_rate: (30, 1),
            sample_rate: 48000,
            channels: 2,
            audio_codec: Some(ExportAudioCodec::Flac),
            encoder: super::super::ExportEncoder::default(),
        };
        assert_eq!(select_audio_codec(&config), tarang::core::AudioCodec::Flac);
    }

    #[test]
    fn compute_video_bitrate_1080p() {
        let br = compute_video_bitrate(1920, 1080);
        assert_eq!(br, 5_000_000);
    }

    #[test]
    fn compute_video_bitrate_4k() {
        let br = compute_video_bitrate(3840, 2160);
        assert_eq!(br, 20_000_000);
    }

    #[test]
    fn compute_video_bitrate_small() {
        let br = compute_video_bitrate(320, 240);
        assert_eq!(br, 500_000); // clamped minimum
    }

    #[test]
    fn mkv_muxer_writes_header() {
        let mut buf = Vec::new();
        let audio = MuxConfig {
            codec: tarang::core::AudioCodec::Opus,
            sample_rate: 48000,
            channels: 2,
            bits_per_sample: 16,
        };
        let video = VideoMuxConfig {
            codec: tarang::core::VideoCodec::H264,
            width: 320,
            height: 240,
        };
        let mut muxer = MkvExportMuxer(MkvMuxer::new_webm(&mut buf, audio, video));
        muxer.write_header().unwrap();
        // Should start with EBML magic: 0x1A 0x45 0xDF 0xA3
        assert!(buf.len() > 20);
        assert_eq!(&buf[..4], &[0x1A, 0x45, 0xDF, 0xA3]);
    }

    #[test]
    fn mp4_muxer_writes_header() {
        let mut buf = std::io::Cursor::new(Vec::new());
        let audio = MuxConfig {
            codec: tarang::core::AudioCodec::Aac,
            sample_rate: 48000,
            channels: 2,
            bits_per_sample: 16,
        };
        let video = VideoMuxConfig {
            codec: tarang::core::VideoCodec::H264,
            width: 320,
            height: 240,
        };
        let mut muxer = Mp4ExportMuxer(Mp4Muxer::new_with_video(&mut buf, audio, video));
        muxer.write_header().unwrap();
        let data = buf.into_inner();
        assert!(data.len() > 8, "MP4 header should produce output");
    }

    #[test]
    fn mkv_muxer_write_packets() {
        let mut buf = Vec::new();
        let audio = MuxConfig {
            codec: tarang::core::AudioCodec::Opus,
            sample_rate: 48000,
            channels: 2,
            bits_per_sample: 16,
        };
        let video = VideoMuxConfig {
            codec: tarang::core::VideoCodec::H264,
            width: 320,
            height: 240,
        };
        let mut muxer = MkvExportMuxer(MkvMuxer::new_webm(&mut buf, audio, video));
        muxer.write_header().unwrap();
        muxer.write_video_packet(&[0x00, 0x00, 0x01]).unwrap();
        muxer.write_audio_packet(&[0xAA, 0xBB]).unwrap();
        muxer.finalize().unwrap();
        assert!(buf.len() > 50, "output should have header + packet data");
    }

    #[test]
    fn build_mux_configs() {
        let config = ExportConfig {
            output_path: "/tmp/test.mkv".into(),
            format: ExportFormat::Mkv,
            width: 1920,
            height: 1080,
            frame_rate: (30, 1),
            sample_rate: 48000,
            channels: 2,
            audio_codec: None,
            encoder: super::super::ExportEncoder::default(),
        };
        let audio_cfg = build_audio_mux_config(&config, tarang::core::AudioCodec::Opus);
        assert_eq!(audio_cfg.sample_rate, 48000);
        assert_eq!(audio_cfg.channels, 2);

        let video_cfg = build_video_mux_config(&config);
        assert_eq!(video_cfg.width, 1920);
        assert_eq!(video_cfg.height, 1080);
    }

    #[test]
    fn tarang_pipeline_struct_is_zero_sized() {
        assert_eq!(std::mem::size_of::<TarangExportPipeline>(), 0);
    }
}
