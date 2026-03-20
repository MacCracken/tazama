use std::path::Path;

use gstreamer_pbutils::Discoverer;
use gstreamer_pbutils::prelude::*;
use tazama_core::{AudioStreamInfo, Codec, ContainerFormat, MediaInfo, VideoStreamInfo};
use tokio::task;

use crate::error::MediaPipelineError;

/// Audio-only file extensions handled by tarang when the feature is enabled.
#[cfg(feature = "tarang")]
const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "ogg", "m4a", "aac"];

/// Video file extensions handled by tarang demux when the feature is enabled.
#[cfg(feature = "tarang")]
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mkv", "webm"];

/// Probe a media file and extract its metadata.
pub async fn probe(path: &Path) -> Result<MediaInfo, MediaPipelineError> {
    let path = path.to_path_buf();
    task::spawn_blocking(move || probe_sync(&path))
        .await
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?
}

fn probe_sync(path: &Path) -> Result<MediaInfo, MediaPipelineError> {
    if !path.exists() {
        return Err(MediaPipelineError::FileNotFound(path.display().to_string()));
    }

    #[cfg(feature = "tarang")]
    if is_video_file(path) {
        match probe_tarang_video(path) {
            Ok(info) => return Ok(info),
            Err(e) => {
                tracing::warn!("tarang video probe failed, falling back to GStreamer: {e}");
            }
        }
    }

    #[cfg(feature = "tarang")]
    if is_audio_file(path) {
        return probe_tarang(path);
    }

    let timeout = gstreamer::ClockTime::from_seconds(10);
    let discoverer = Discoverer::new(timeout)?;

    let uri = if path.is_absolute() {
        format!("file://{}", path.display())
    } else {
        let abs = std::fs::canonicalize(path)?;
        format!("file://{}", abs.display())
    };

    let info = discoverer
        .discover_uri(&uri)
        .map_err(|e| MediaPipelineError::ProbeFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

    let duration_ns = info.duration().map(|d| d.nseconds()).unwrap_or(0);
    let duration_ms = duration_ns / 1_000_000;

    let container = detect_container(path);

    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();

    for stream in info.video_streams() {
        let caps = stream.caps();
        let (width, height, frame_rate, bit_depth, pixel_format) = if let Some(caps) = caps {
            parse_video_caps(&caps)
        } else {
            (0, 0, (0, 1), 8, "unknown".to_string())
        };

        video_streams.push(VideoStreamInfo {
            codec: detect_video_codec(&stream),
            width,
            height,
            frame_rate,
            bit_depth,
            pixel_format,
        });
    }

    for stream in info.audio_streams() {
        let caps = stream.caps();
        let (sample_rate, channels, bit_depth) = if let Some(caps) = caps {
            parse_audio_caps(&caps)
        } else {
            (0, 0, 0)
        };

        audio_streams.push(AudioStreamInfo {
            codec: detect_audio_codec(&stream),
            sample_rate,
            channels,
            bit_depth,
        });
    }

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    // Estimate duration in frames from first video stream
    let duration_frames = if let Some(vs) = video_streams.first() {
        let (num, den) = vs.frame_rate;
        if den > 0 {
            (duration_ms as f64 * num as f64 / den as f64 / 1000.0).round() as u64
        } else {
            0
        }
    } else {
        0
    };

    Ok(MediaInfo {
        duration_ms,
        duration_frames,
        container,
        video_streams,
        audio_streams,
        file_size,
    })
}

#[cfg(feature = "tarang")]
fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(feature = "tarang")]
fn probe_tarang(path: &Path) -> Result<MediaInfo, MediaPipelineError> {
    let file = std::fs::File::open(path)?;
    let info = tarang::audio::probe_audio(file)?;

    let duration_ms = info.duration.map(|d| d.as_millis() as u64).unwrap_or(0);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let container = detect_container(path);

    let audio_streams = info
        .audio_streams()
        .into_iter()
        .map(|s| AudioStreamInfo {
            codec: map_tarang_audio_codec(s.codec),
            sample_rate: s.sample_rate,
            channels: s.channels,
            bit_depth: s.sample_format.bytes_per_sample() as u32 * 8,
        })
        .collect();

    Ok(MediaInfo {
        duration_ms,
        duration_frames: 0,
        container,
        video_streams: Vec::new(),
        audio_streams,
        file_size,
    })
}

#[cfg(feature = "tarang")]
fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(feature = "tarang")]
fn probe_tarang_video(path: &Path) -> Result<MediaInfo, MediaPipelineError> {
    use std::io::Read;
    use tarang::demux::Demuxer;

    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 32];
    let n = file.read(&mut header)?;
    drop(file);

    let format = tarang::demux::detect_format(&header[..n]).map_err(|e| {
        MediaPipelineError::ProbeFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        }
    })?;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut demuxer: Box<dyn Demuxer> = match format {
        tarang::core::ContainerFormat::Mp4 => Box::new(tarang::demux::Mp4Demuxer::new(reader)),
        tarang::core::ContainerFormat::Mkv | tarang::core::ContainerFormat::WebM => {
            Box::new(tarang::demux::MkvDemuxer::new(reader))
        }
        other => {
            return Err(MediaPipelineError::UnsupportedFormat(format!("{other:?}")));
        }
    };

    let tarang_info = demuxer
        .probe()
        .map_err(|e| MediaPipelineError::ProbeFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

    let container = detect_container(path);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let duration_ms = tarang_info
        .duration
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();

    for stream in &tarang_info.streams {
        match stream {
            tarang::core::StreamInfo::Video(vs) => {
                let frame_rate = f64_to_rational(vs.frame_rate);
                video_streams.push(VideoStreamInfo {
                    codec: map_tarang_video_codec(vs.codec),
                    width: vs.width,
                    height: vs.height,
                    frame_rate,
                    bit_depth: 8,
                    pixel_format: format!("{:?}", vs.pixel_format).to_lowercase(),
                });
            }
            tarang::core::StreamInfo::Audio(aus) => {
                audio_streams.push(AudioStreamInfo {
                    codec: map_tarang_audio_codec(aus.codec),
                    sample_rate: aus.sample_rate,
                    channels: aus.channels,
                    bit_depth: aus.sample_format.bytes_per_sample() as u32 * 8,
                });
            }
            _ => {}
        }
    }

    let duration_frames = if let Some(vs) = video_streams.first() {
        let (num, den) = vs.frame_rate;
        if den > 0 {
            (duration_ms as f64 * num as f64 / den as f64 / 1000.0).round() as u64
        } else {
            0
        }
    } else {
        0
    };

    Ok(MediaInfo {
        duration_ms,
        duration_frames,
        container,
        video_streams,
        audio_streams,
        file_size,
    })
}

#[cfg(feature = "tarang")]
fn map_tarang_video_codec(codec: tarang::core::VideoCodec) -> Codec {
    match codec {
        tarang::core::VideoCodec::H264 => Codec::H264,
        tarang::core::VideoCodec::H265 => Codec::H265,
        tarang::core::VideoCodec::Vp9 => Codec::Vp9,
        tarang::core::VideoCodec::Av1 => Codec::Av1,
        _ => Codec::Other,
    }
}

/// Convert an f64 frame rate to a (numerator, denominator) rational approximation.
#[cfg(feature = "tarang")]
fn f64_to_rational(fps: f64) -> (u32, u32) {
    if fps <= 0.0 {
        return (0, 1);
    }
    // Common frame rates — check exact matches first
    let common = [
        (24000, 1001, 23.976),
        (24, 1, 24.0),
        (25, 1, 25.0),
        (30000, 1001, 29.97),
        (30, 1, 30.0),
        (50, 1, 50.0),
        (60000, 1001, 59.94),
        (60, 1, 60.0),
    ];
    for (num, den, expected) in common {
        if (fps - expected).abs() < 0.01 {
            return (num, den);
        }
    }
    // Fallback: multiply by 1000 and simplify
    let num = (fps * 1000.0).round() as u32;
    let den = 1000u32;
    let g = gcd(num, den);
    (num / g, den / g)
}

#[cfg(feature = "tarang")]
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[cfg(feature = "tarang")]
fn map_tarang_audio_codec(codec: tarang::core::AudioCodec) -> Codec {
    match codec {
        tarang::core::AudioCodec::Aac => Codec::Aac,
        tarang::core::AudioCodec::Mp3 => Codec::Mp3,
        tarang::core::AudioCodec::Flac => Codec::Flac,
        tarang::core::AudioCodec::Opus => Codec::Opus,
        _ => Codec::Other,
    }
}

fn detect_container(path: &Path) -> ContainerFormat {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("mp4" | "m4v") => ContainerFormat::Mp4,
        Some("mkv") => ContainerFormat::Mkv,
        Some("webm") => ContainerFormat::WebM,
        Some("mov") => ContainerFormat::Mov,
        Some("avi") => ContainerFormat::Avi,
        _ => ContainerFormat::Other,
    }
}

fn detect_video_codec(stream: &gstreamer_pbutils::DiscovererVideoInfo) -> Codec {
    let caps = match stream.caps() {
        Some(c) => c,
        None => return Codec::Other,
    };
    let caps_str = caps.to_string();
    if caps_str.contains("h264") || caps_str.contains("x-h264") {
        Codec::H264
    } else if caps_str.contains("h265") || caps_str.contains("x-h265") {
        Codec::H265
    } else if caps_str.contains("vp9") || caps_str.contains("x-vp9") {
        Codec::Vp9
    } else if caps_str.contains("av1") || caps_str.contains("x-av1") {
        Codec::Av1
    } else {
        Codec::Other
    }
}

fn detect_audio_codec(stream: &gstreamer_pbutils::DiscovererAudioInfo) -> Codec {
    let caps = match stream.caps() {
        Some(c) => c,
        None => return Codec::Other,
    };
    let caps_str = caps.to_string();
    if caps_str.contains("aac") || caps_str.contains("mpeg") {
        Codec::Aac
    } else if caps_str.contains("opus") {
        Codec::Opus
    } else if caps_str.contains("flac") {
        Codec::Flac
    } else if caps_str.contains("mp3") || caps_str.contains("layer3") {
        Codec::Mp3
    } else {
        Codec::Other
    }
}

fn parse_video_caps(caps: &gstreamer::Caps) -> (u32, u32, (u32, u32), u32, String) {
    let structure = match caps.structure(0) {
        Some(s) => s,
        None => return (0, 0, (0, 1), 8, "unknown".to_string()),
    };

    let width = structure.get::<i32>("width").unwrap_or(0) as u32;
    let height = structure.get::<i32>("height").unwrap_or(0) as u32;

    let frame_rate = structure
        .get::<gstreamer::Fraction>("framerate")
        .map(|f| (f.numer() as u32, f.denom() as u32))
        .unwrap_or((0, 1));

    let bit_depth = structure.get::<i32>("depth").unwrap_or(8) as u32;

    let pixel_format = structure
        .get::<String>("format")
        .unwrap_or_else(|_| "unknown".to_string());

    (width, height, frame_rate, bit_depth, pixel_format)
}

fn parse_audio_caps(caps: &gstreamer::Caps) -> (u32, u16, u32) {
    let structure = match caps.structure(0) {
        Some(s) => s,
        None => return (0, 0, 0),
    };

    let sample_rate = structure.get::<i32>("rate").unwrap_or(0) as u32;
    let channels = structure.get::<i32>("channels").unwrap_or(0) as u16;
    let bit_depth = structure.get::<i32>("depth").unwrap_or(0) as u32;

    (sample_rate, channels, bit_depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detect_container_mp4() {
        assert_eq!(
            detect_container(Path::new("video.mp4")),
            ContainerFormat::Mp4
        );
        assert_eq!(
            detect_container(Path::new("video.m4v")),
            ContainerFormat::Mp4
        );
    }

    #[test]
    fn detect_container_mkv() {
        assert_eq!(
            detect_container(Path::new("video.mkv")),
            ContainerFormat::Mkv
        );
    }

    #[test]
    fn detect_container_webm() {
        assert_eq!(
            detect_container(Path::new("video.webm")),
            ContainerFormat::WebM
        );
    }

    #[test]
    fn detect_container_mov() {
        assert_eq!(
            detect_container(Path::new("video.mov")),
            ContainerFormat::Mov
        );
    }

    #[test]
    fn detect_container_avi() {
        assert_eq!(
            detect_container(Path::new("video.avi")),
            ContainerFormat::Avi
        );
    }

    #[test]
    fn detect_container_unknown() {
        assert_eq!(
            detect_container(Path::new("video.flv")),
            ContainerFormat::Other
        );
        assert_eq!(detect_container(Path::new("noext")), ContainerFormat::Other);
    }

    #[test]
    fn detect_container_case_insensitive() {
        assert_eq!(
            detect_container(Path::new("video.MP4")),
            ContainerFormat::Mp4
        );
        assert_eq!(
            detect_container(Path::new("video.MKV")),
            ContainerFormat::Mkv
        );
        assert_eq!(
            detect_container(Path::new("video.WebM")),
            ContainerFormat::WebM
        );
    }

    #[test]
    fn probe_nonexistent_file() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(probe(Path::new("/nonexistent/file.mp4")));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MediaPipelineError::FileNotFound(_)
        ));
    }
}
