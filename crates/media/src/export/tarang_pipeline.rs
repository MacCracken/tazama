use std::io::Write;
use std::time::Duration;

use bytes::Bytes;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use super::{ExportAudioCodec, ExportConfig, ExportFormat, ExportProgress};
use crate::decode::{AudioBuffer, VideoFrame};
use crate::error::MediaPipelineError;

/// Export pipeline backed by tarang encoders and muxer.
///
/// This provides the same interface as the GStreamer-based [`super::pipeline::ExportPipeline`]
/// but routes encoding through tarang video encoders (openh264) and audio encoders
/// (FLAC), with a custom EBML muxer for MKV output.
///
/// **Supported natively:** MKV (H.264 video + audio via tarang encoders + EBML mux)
/// **Falls back to GStreamer:** MP4, WebM, ProRes, DnxHr, GIF
///
/// WebM would use VP9 encoding via `vpx-enc`, but that feature requires a
/// compatible libvpx build. When `vpx-enc` is unavailable, WebM falls back
/// to GStreamer.
// TODO: enable WebM via tarang when `vpx-enc` feature compiles on this system.
pub struct TarangExportPipeline;

impl TarangExportPipeline {
    pub fn run(
        config: ExportConfig,
        video_rx: tokio::sync::mpsc::Receiver<VideoFrame>,
        audio_rx: tokio::sync::mpsc::Receiver<AudioBuffer>,
        total_frames: u64,
    ) -> Result<watch::Receiver<ExportProgress>, MediaPipelineError> {
        match config.format {
            ExportFormat::Mkv => {
                info!("tarang export pipeline: MKV format, encoding via tarang (H.264 + audio)");
                Self::run_tarang(config, video_rx, audio_rx, total_frames)
            }
            ExportFormat::Mp4
            | ExportFormat::WebM
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

/// Convert RGBA pixel data to YUV420p.
fn rgba_to_yuv420p(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let chroma_w = w / 2;
    let chroma_h = h / 2;
    let mut yuv = vec![0u8; w * h + 2 * chroma_w * chroma_h];

    // Y plane
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let r = rgba[i] as f32;
            let g = rgba[i + 1] as f32;
            let b = rgba[i + 2] as f32;
            yuv[y * w + x] = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
        }
    }

    // U and V planes (subsampled 2x2)
    let u_offset = w * h;
    let v_offset = u_offset + chroma_w * chroma_h;
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let i = (y * w + x) * 4;
            let r = rgba[i] as f32;
            let g = rgba[i + 1] as f32;
            let b = rgba[i + 2] as f32;
            let u = (-0.169 * r - 0.331 * g + 0.500 * b + 128.0).clamp(0.0, 255.0) as u8;
            let v = (0.500 * r - 0.419 * g - 0.081 * b + 128.0).clamp(0.0, 255.0) as u8;
            let ux = (y / 2) * chroma_w + (x / 2);
            yuv[u_offset + ux] = u;
            yuv[v_offset + ux] = v;
        }
    }
    yuv
}

/// Convert tazama AudioBuffer (interleaved f32 samples) to tarang AudioBuffer (Bytes).
fn convert_audio_buffer(buf: &AudioBuffer) -> tarang::core::AudioBuffer {
    let byte_data: Vec<u8> = buf.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let num_samples = buf.samples.len() / buf.channels.max(1) as usize;
    tarang::core::AudioBuffer {
        data: Bytes::from(byte_data),
        sample_format: tarang::core::SampleFormat::F32,
        channels: buf.channels,
        sample_rate: buf.sample_rate,
        num_samples,
        timestamp: Duration::from_nanos(buf.timestamp_ns),
    }
}

/// Track configuration for the dual-track muxer.
struct MuxerTrackConfig<'a> {
    video_codec: &'a str,
    width: u32,
    height: u32,
    frame_rate: f64,
    audio_codec_id: &'a str,
    sample_rate: u32,
    channels: u16,
}

/// Simple dual-track MKV/WebM muxer using EBML primitives.
///
/// Writes both a video track (track 1) and an audio track (track 2)
/// into a Matroska or WebM container. Uses a streaming layout with
/// unknown-size segments and clusters.
struct DualTrackMkvMuxer<W: Write> {
    writer: W,
    is_webm: bool,
    cluster_open: bool,
    cluster_timecode_ms: u64,
    packets_in_cluster: u32,
}

impl<W: Write> DualTrackMkvMuxer<W> {
    fn new(writer: W, is_webm: bool) -> Self {
        Self {
            writer,
            is_webm,
            cluster_open: false,
            cluster_timecode_ms: 0,
            packets_in_cluster: 0,
        }
    }

    fn write_header(&mut self, tc: &MuxerTrackConfig<'_>) -> Result<(), MediaPipelineError> {
        let video_codec = tc.video_codec;
        let width = tc.width;
        let height = tc.height;
        let frame_rate = tc.frame_rate;
        let audio_codec_id = tc.audio_codec_id;
        let sample_rate = tc.sample_rate;
        let channels = tc.channels;
        use tarang::demux::ebml;

        // EBML Header
        let mut ebml_header = Vec::new();
        ebml::write_uint(&mut ebml_header, 0x4286, 1); // EBMLVersion
        ebml::write_uint(&mut ebml_header, 0x42F7, 1); // EBMLReadVersion
        ebml::write_uint(&mut ebml_header, 0x42F2, 4); // EBMLMaxIDLength
        ebml::write_uint(&mut ebml_header, 0x42F3, 8); // EBMLMaxSizeLength
        let doc_type = if self.is_webm { "webm" } else { "matroska" };
        ebml::write_string(&mut ebml_header, 0x4282, doc_type);
        ebml::write_uint(&mut ebml_header, 0x4287, 4); // DocTypeVersion
        ebml::write_uint(&mut ebml_header, 0x4285, 2); // DocTypeReadVersion

        ebml::write_master_to_writer(&mut self.writer, 0x1A45DFA3, &ebml_header)
            .map_err(|e| MediaPipelineError::Export(format!("EBML header write: {e}")))?;

        // Segment (unknown size)
        ebml::write_id_to_writer(&mut self.writer, 0x18538067)
            .map_err(|e| MediaPipelineError::Export(format!("segment ID write: {e}")))?;
        self.writer
            .write_all(&[0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
            .map_err(|e| MediaPipelineError::Export(format!("segment size write: {e}")))?;

        // Info element
        let mut info = Vec::new();
        ebml::write_uint(&mut info, 0x2AD7B1, 1_000_000); // TimecodeScale = 1ms
        ebml::write_string(&mut info, 0x4D80, "tazama"); // MuxingApp
        ebml::write_string(&mut info, 0x5741, "tazama"); // WritingApp
        let mut info_buf = Vec::new();
        ebml::write_master(&mut info_buf, 0x1549A966, &info);
        self.writer
            .write_all(&info_buf)
            .map_err(|e| MediaPipelineError::Export(format!("info write: {e}")))?;

        // Tracks element
        let mut tracks = Vec::new();

        // Track 1: Video
        {
            let mut track_entry = Vec::new();
            ebml::write_uint(&mut track_entry, 0xD7, 1); // TrackNumber
            ebml::write_uint(&mut track_entry, 0x73C5, 1); // TrackUID
            ebml::write_uint(&mut track_entry, 0x83, 1); // TrackType = video
            ebml::write_string(&mut track_entry, 0x86, video_codec); // CodecID
            ebml::write_uint(&mut track_entry, 0x9C, 0); // FlagLacing = 0

            // Video settings sub-element
            let mut video_settings = Vec::new();
            ebml::write_uint(&mut video_settings, 0xB0, width as u64); // PixelWidth
            ebml::write_uint(&mut video_settings, 0xBA, height as u64); // PixelHeight
            // DefaultDuration in nanoseconds
            if frame_rate > 0.0 {
                let default_dur_ns = (1_000_000_000.0 / frame_rate) as u64;
                ebml::write_uint(&mut track_entry, 0x23E383, default_dur_ns);
            }
            ebml::write_master(&mut track_entry, 0xE0, &video_settings);

            ebml::write_master(&mut tracks, 0xAE, &track_entry);
        }

        // Track 2: Audio
        {
            let mut track_entry = Vec::new();
            ebml::write_uint(&mut track_entry, 0xD7, 2); // TrackNumber
            ebml::write_uint(&mut track_entry, 0x73C5, 2); // TrackUID
            ebml::write_uint(&mut track_entry, 0x83, 2); // TrackType = audio
            ebml::write_string(&mut track_entry, 0x86, audio_codec_id); // CodecID

            let mut audio_settings = Vec::new();
            ebml::write_float(&mut audio_settings, 0xB5, sample_rate as f64); // SamplingFrequency
            ebml::write_uint(&mut audio_settings, 0x9F, channels as u64); // Channels
            ebml::write_master(&mut track_entry, 0xE1, &audio_settings);

            ebml::write_master(&mut tracks, 0xAE, &track_entry);
        }

        let mut tracks_buf = Vec::new();
        ebml::write_master(&mut tracks_buf, 0x1654AE6B, &tracks);
        self.writer
            .write_all(&tracks_buf)
            .map_err(|e| MediaPipelineError::Export(format!("tracks write: {e}")))?;

        // Start first cluster
        self.start_cluster(0)?;

        Ok(())
    }

    fn start_cluster(&mut self, timecode_ms: u64) -> Result<(), MediaPipelineError> {
        use tarang::demux::ebml;

        // Cluster with unknown size
        ebml::write_id_to_writer(&mut self.writer, 0x1F43B675)
            .map_err(|e| MediaPipelineError::Export(format!("cluster ID: {e}")))?;
        self.writer
            .write_all(&[0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
            .map_err(|e| MediaPipelineError::Export(format!("cluster size: {e}")))?;

        // Timecode
        let mut tc_buf = Vec::new();
        ebml::write_uint(&mut tc_buf, 0xE7, timecode_ms);
        self.writer
            .write_all(&tc_buf)
            .map_err(|e| MediaPipelineError::Export(format!("cluster timecode: {e}")))?;

        self.cluster_timecode_ms = timecode_ms;
        self.cluster_open = true;
        self.packets_in_cluster = 0;
        Ok(())
    }

    /// Write a SimpleBlock for the given track.
    fn write_simple_block(
        &mut self,
        track_number: u8,
        timestamp_ms: u64,
        data: &[u8],
        keyframe: bool,
    ) -> Result<(), MediaPipelineError> {
        use tarang::demux::ebml;

        // Start a new cluster every ~5 seconds or 500 packets
        if self.packets_in_cluster > 500
            || (self.cluster_open && timestamp_ms > self.cluster_timecode_ms + 5000)
        {
            self.start_cluster(timestamp_ms)?;
        }

        let relative_ts = timestamp_ms.saturating_sub(self.cluster_timecode_ms);
        let relative_ts_i16 = relative_ts.min(i16::MAX as u64) as i16;

        let mut block = Vec::new();
        ebml::write_vint(&mut block, track_number as u64); // track number
        block.extend_from_slice(&relative_ts_i16.to_be_bytes()); // relative timecode
        let flags = if keyframe { 0x80u8 } else { 0x00u8 };
        block.push(flags);
        block.extend_from_slice(data);

        let mut block_buf = Vec::new();
        ebml::write_id(&mut block_buf, 0xA3); // SimpleBlock ID
        ebml::write_vint(&mut block_buf, block.len() as u64);
        block_buf.extend_from_slice(&block);

        self.writer
            .write_all(&block_buf)
            .map_err(|e| MediaPipelineError::Export(format!("simple block write: {e}")))?;

        self.packets_in_cluster += 1;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), MediaPipelineError> {
        self.writer
            .flush()
            .map_err(|e| MediaPipelineError::Export(format!("finalize flush: {e}")))?;
        Ok(())
    }
}

/// Determine the MKV codec ID string for video based on format.
fn video_codec_id(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::WebM => "V_VP9",
        ExportFormat::Mkv => "V_MPEG4/ISO/AVC", // H.264
        _ => "V_MPEG4/ISO/AVC",
    }
}

/// Determine the MKV codec ID string for audio.
fn audio_codec_id(codec: tarang::core::AudioCodec) -> &'static str {
    match codec {
        tarang::core::AudioCodec::Opus => "A_OPUS",
        tarang::core::AudioCodec::Flac => "A_FLAC",
        tarang::core::AudioCodec::Aac => "A_AAC",
        tarang::core::AudioCodec::Vorbis => "A_VORBIS",
        tarang::core::AudioCodec::Mp3 => "A_MPEG/L3",
        _ => "A_PCM/INT/LIT",
    }
}

fn run_tarang_export(
    config: ExportConfig,
    video_rx: &mut tokio::sync::mpsc::Receiver<VideoFrame>,
    audio_rx: &mut tokio::sync::mpsc::Receiver<AudioBuffer>,
    progress_tx: &watch::Sender<ExportProgress>,
    total_frames: u64,
) -> Result<(), MediaPipelineError> {
    // Create output file
    let file = std::fs::File::create(&config.output_path)?;
    let mut writer = std::io::BufWriter::new(file);

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

    // Create muxer
    let is_webm = config.format == ExportFormat::WebM;
    let frame_rate = config.frame_rate.0 as f64 / config.frame_rate.1.max(1) as f64;

    let mut muxer = DualTrackMkvMuxer::new(&mut writer, is_webm);
    muxer.write_header(&MuxerTrackConfig {
        video_codec: video_codec_id(config.format),
        width: config.width,
        height: config.height,
        frame_rate,
        audio_codec_id: audio_codec_id(audio_codec),
        sample_rate: config.sample_rate,
        channels: config.channels,
    })?;

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
        let timestamp_ms = frame.timestamp_ns / 1_000_000;

        // Write encoded data to muxer
        if !encoded.is_empty() {
            muxer.write_simple_block(1, timestamp_ms, &encoded, true)?;
        }

        frames_written += 1;
        let _ = progress_tx.send(ExportProgress {
            frames_written,
            total_frames,
            done: false,
        });
    }

    // openh264 encoder does not buffer frames, so no flush needed for video.
    // If VpxEncoder is added in the future, call encoder.flush() here.

    debug!("video encoding complete: {frames_written} frames");

    // Process audio buffers
    let mut audio_packets_written = 0u64;
    while let Some(audio_buf) = audio_rx.blocking_recv() {
        if audio_buf.samples.is_empty() {
            continue;
        }

        if let Some(ref mut enc) = audio_encoder {
            let tarang_buf = convert_audio_buffer(&audio_buf);
            let timestamp_ms = audio_buf.timestamp_ns / 1_000_000;

            match enc.encode(&tarang_buf) {
                Ok(packets) => {
                    for packet in &packets {
                        if !packet.is_empty() {
                            muxer.write_simple_block(2, timestamp_ms, packet, true)?;
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
                        muxer.write_simple_block(2, 0, packet, true)?;
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
    writer
        .flush()
        .map_err(|e| MediaPipelineError::Export(format!("final flush: {e}")))?;

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
        assert_eq!(tarang_buf.num_samples, 2); // 4 samples / 2 channels
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
    fn video_codec_id_values() {
        assert_eq!(video_codec_id(ExportFormat::WebM), "V_VP9");
        assert_eq!(video_codec_id(ExportFormat::Mkv), "V_MPEG4/ISO/AVC");
    }

    #[test]
    fn audio_codec_id_values() {
        assert_eq!(audio_codec_id(tarang::core::AudioCodec::Opus), "A_OPUS");
        assert_eq!(audio_codec_id(tarang::core::AudioCodec::Flac), "A_FLAC");
        assert_eq!(audio_codec_id(tarang::core::AudioCodec::Aac), "A_AAC");
    }

    #[test]
    fn dual_track_muxer_writes_ebml_header() {
        let mut buf = Vec::new();
        let mut muxer = DualTrackMkvMuxer::new(&mut buf, false);
        muxer
            .write_header(&MuxerTrackConfig {
                video_codec: "V_MPEG4/ISO/AVC",
                width: 320,
                height: 240,
                frame_rate: 30.0,
                audio_codec_id: "A_OPUS",
                sample_rate: 48000,
                channels: 2,
            })
            .unwrap();
        // Should start with EBML magic: 0x1A 0x45 0xDF 0xA3
        assert!(buf.len() > 20);
        assert_eq!(&buf[..4], &[0x1A, 0x45, 0xDF, 0xA3]);
    }

    #[test]
    fn dual_track_muxer_webm_doctype() {
        let mut buf = Vec::new();
        let mut muxer = DualTrackMkvMuxer::new(&mut buf, true);
        muxer
            .write_header(&MuxerTrackConfig {
                video_codec: "V_VP9",
                width: 320,
                height: 240,
                frame_rate: 30.0,
                audio_codec_id: "A_OPUS",
                sample_rate: 48000,
                channels: 2,
            })
            .unwrap();
        // Should contain "webm" doctype string
        let s = String::from_utf8_lossy(&buf);
        assert!(s.contains("webm"));
    }

    #[test]
    fn dual_track_muxer_write_blocks() {
        let mut buf = Vec::new();
        {
            let mut muxer = DualTrackMkvMuxer::new(&mut buf, false);
            muxer
                .write_header(&MuxerTrackConfig {
                    video_codec: "V_MPEG4/ISO/AVC",
                    width: 320,
                    height: 240,
                    frame_rate: 30.0,
                    audio_codec_id: "A_OPUS",
                    sample_rate: 48000,
                    channels: 2,
                })
                .unwrap();
            // Write a video block
            muxer
                .write_simple_block(1, 0, &[0x00, 0x00, 0x01], true)
                .unwrap();
            // Write an audio block
            muxer.write_simple_block(2, 0, &[0xAA, 0xBB], true).unwrap();
            muxer.finalize().unwrap();
        }
        // Should contain EBML header + blocks
        assert!(buf.len() > 50, "output should have header + block data");
    }

    #[test]
    fn tarang_pipeline_struct_is_zero_sized() {
        assert_eq!(std::mem::size_of::<TarangExportPipeline>(), 0);
    }
}
