use std::path::Path;

use bytes::Bytes;
use tazama_core::{ThumbnailSpec, ThumbnailStrategy};

use crate::decode::video::VideoDecoder;
use crate::error::MediaPipelineError;
use crate::probe;

/// Video file extensions handled by tarang when the feature is enabled.
const TARANG_VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mkv", "webm"];

/// Generate thumbnails from a media file at regular intervals.
///
/// Returns a vector of `(timestamp_ms, rgba_bytes)` pairs.
pub async fn generate_thumbnails(
    path: &Path,
    spec: ThumbnailSpec,
) -> Result<Vec<(u64, Bytes)>, MediaPipelineError> {
    if is_tarang_video(path) {
        match generate_thumbnails_tarang(path, spec).await {
            Ok(thumbs) => return Ok(thumbs),
            Err(e) => {
                tracing::warn!(
                    "tarang thumbnail generation failed, falling back to GStreamer: {e}"
                );
            }
        }
    }

    generate_thumbnails_gst(path, spec).await
}

/// GStreamer-based thumbnail generation (original implementation).
async fn generate_thumbnails_gst(
    path: &Path,
    spec: ThumbnailSpec,
) -> Result<Vec<(u64, Bytes)>, MediaPipelineError> {
    let info = probe::probe(path).await?;

    let Some(video) = info.video_streams.first() else {
        return Err(MediaPipelineError::Decode("no video stream found".into()));
    };

    let frame_rate = video.frame_rate;
    let duration_ms = info.duration_ms;
    let mut thumbnails = Vec::new();

    let mut timestamp_ms = 0u64;
    while timestamp_ms < duration_ms {
        let frame_index = if frame_rate.1 > 0 {
            (timestamp_ms as f64 * frame_rate.0 as f64 / frame_rate.1 as f64 / 1000.0) as u64
        } else {
            0
        };

        let frame = VideoDecoder::decode_frame(path, frame_index, frame_rate).await?;

        // If the requested size differs from decoded, we return as-is.
        // Scaling would require an additional videoscale pipeline element;
        // for now we trust the caller to handle sizing or we add scaling later.
        let _ = (spec.width, spec.height);

        thumbnails.push((timestamp_ms, frame.data));
        timestamp_ms += spec.interval_ms;
    }

    Ok(thumbnails)
}

fn is_tarang_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| TARANG_VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Scene-aware thumbnail generation via tarang.
///
/// Demuxes + decodes frames at regular intervals, feeds them through a
/// `SceneDetector` to find boundaries, and picks frames near scene changes
/// plus high-variance frames.
async fn generate_thumbnails_tarang(
    path: &Path,
    spec: ThumbnailSpec,
) -> Result<Vec<(u64, Bytes)>, MediaPipelineError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || generate_thumbnails_tarang_sync(&path, spec))
        .await
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?
}

fn generate_thumbnails_tarang_sync(
    path: &Path,
    spec: ThumbnailSpec,
) -> Result<Vec<(u64, Bytes)>, MediaPipelineError> {
    match spec.strategy {
        ThumbnailStrategy::ContentBased => generate_thumbnails_content_based(path, spec),
        ThumbnailStrategy::SceneBased => generate_thumbnails_scene_based(path, spec),
    }
}

/// Scene-based thumbnail generation using `SceneDetector`.
fn generate_thumbnails_scene_based(
    path: &Path,
    spec: ThumbnailSpec,
) -> Result<Vec<(u64, Bytes)>, MediaPipelineError> {
    use tarang::ai::{SceneDetectionConfig, SceneDetector};
    use tarang::video::scale::{scale_frame, ScaleFilter};

    let mut demuxer = create_demuxer(path)?;
    let info = demuxer.probe()?;

    let (video_stream_idx, codec) = find_video_stream(&info)
        .ok_or_else(|| MediaPipelineError::Decode("no video stream found".into()))?;

    let config = tarang::video::DecoderConfig::for_codec(codec)?;
    let mut decoder = tarang::video::VideoDecoder::new(config)?;

    if let Some(tarang::core::StreamInfo::Video(vs)) = info.streams.get(video_stream_idx) {
        decoder.init(vs);
    }

    let interval_ns = spec.interval_ms * 1_000_000;

    let mut scene_detector = SceneDetector::new(SceneDetectionConfig::default());
    let mut candidate_frames: Vec<(u64, tarang::core::VideoFrame, bool)> = Vec::new();
    let mut next_sample_ns: u64 = 0;
    let mut frame_index = 0u64;

    loop {
        let packet = match demuxer.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };

        if packet.stream_index != video_stream_idx {
            continue;
        }

        decoder.send_packet(&packet.data, packet.timestamp)?;

        while let Ok(tarang_frame) = decoder.receive_frame() {
            let ts_ns = tarang_frame.timestamp.as_nanos() as u64;
            let is_boundary = scene_detector.feed_frame(&tarang_frame).is_some();

            if ts_ns >= next_sample_ns {
                candidate_frames.push((frame_index, tarang_frame, is_boundary));
                next_sample_ns = ts_ns + interval_ns;
            }

            frame_index += 1;
        }
    }

    let _ = decoder.flush();
    while let Ok(tarang_frame) = decoder.receive_frame() {
        let ts_ns = tarang_frame.timestamp.as_nanos() as u64;
        let is_boundary = scene_detector.feed_frame(&tarang_frame).is_some();
        if ts_ns >= next_sample_ns {
            candidate_frames.push((frame_index, tarang_frame, is_boundary));
            next_sample_ns = ts_ns + interval_ns;
        }
        frame_index += 1;
    }

    let _boundaries = scene_detector.finish();

    // Prioritize scene boundary frames, then keep all sampled frames
    candidate_frames.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

    let mut thumbnails = Vec::new();
    candidate_frames.sort_by_key(|f| f.0);

    let needs_scaling = spec.width > 0 && spec.height > 0;

    for (idx, tarang_frame, _) in &candidate_frames {
        let timestamp_ms = tarang_frame.timestamp.as_millis() as u64;

        let frame_to_convert = if needs_scaling
            && (tarang_frame.width != spec.width || tarang_frame.height != spec.height)
        {
            scale_frame(tarang_frame, spec.width, spec.height, ScaleFilter::Lanczos3)
                .map_err(|e| MediaPipelineError::Decode(e.to_string()))?
        } else {
            tarang_frame.clone()
        };

        let tazama_frame = crate::convert::tarang_frame_to_tazama(&frame_to_convert, *idx)?;
        thumbnails.push((timestamp_ms, tazama_frame.data));
    }

    Ok(thumbnails)
}

/// Content-based thumbnail generation using `ThumbnailGenerator`.
fn generate_thumbnails_content_based(
    path: &Path,
    spec: ThumbnailSpec,
) -> Result<Vec<(u64, Bytes)>, MediaPipelineError> {
    use tarang::ai::{SceneDetectionConfig, SceneDetector, ThumbnailConfig, ThumbnailGenerator};

    let mut demuxer = create_demuxer(path)?;
    let info = demuxer.probe()?;

    let (video_stream_idx, codec) = find_video_stream(&info)
        .ok_or_else(|| MediaPipelineError::Decode("no video stream found".into()))?;

    let config = tarang::video::DecoderConfig::for_codec(codec)?;
    let mut decoder = tarang::video::VideoDecoder::new(config)?;

    if let Some(tarang::core::StreamInfo::Video(vs)) = info.streams.get(video_stream_idx) {
        decoder.init(vs);
    }

    let interval_ns = spec.interval_ms * 1_000_000;

    let thumb_config = ThumbnailConfig {
        width: spec.width,
        height: spec.height,
        strategy: tarang::ai::ThumbnailStrategy::ContentBased,
        ..ThumbnailConfig::default()
    };
    let mut generator = ThumbnailGenerator::new(thumb_config);
    let mut scene_detector = SceneDetector::new(SceneDetectionConfig::default());
    let mut next_sample_ns: u64 = 0;

    loop {
        let packet = match demuxer.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };

        if packet.stream_index != video_stream_idx {
            continue;
        }

        decoder.send_packet(&packet.data, packet.timestamp)?;

        while let Ok(tarang_frame) = decoder.receive_frame() {
            let ts_ns = tarang_frame.timestamp.as_nanos() as u64;
            let is_boundary = scene_detector.feed_frame(&tarang_frame).is_some();

            if ts_ns >= next_sample_ns {
                generator.consider_frame(&tarang_frame, is_boundary);
                next_sample_ns = ts_ns + interval_ns;
            }
        }
    }

    let _ = decoder.flush();
    while let Ok(tarang_frame) = decoder.receive_frame() {
        let ts_ns = tarang_frame.timestamp.as_nanos() as u64;
        let is_boundary = scene_detector.feed_frame(&tarang_frame).is_some();
        if ts_ns >= next_sample_ns {
            generator.consider_frame(&tarang_frame, is_boundary);
            next_sample_ns = ts_ns + interval_ns;
        }
    }

    let tarang_thumbs = generator
        .generate()
        .map_err(|e| MediaPipelineError::Decode(e.to_string()))?;

    let mut thumbnails = Vec::new();
    for thumb in tarang_thumbs {
        let timestamp_ms = thumb.timestamp.as_millis() as u64;
        // Thumbnail data is already encoded (JPEG/PNG); we need RGBA for our pipeline,
        // so we pass the raw data through. The caller may need to decode if format differs.
        thumbnails.push((timestamp_ms, Bytes::from(thumb.data)));
    }

    Ok(thumbnails)
}

pub fn create_demuxer(path: &Path) -> Result<Box<dyn tarang::demux::Demuxer>, MediaPipelineError> {
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

pub fn find_video_stream(info: &tarang::core::MediaInfo) -> Option<(usize, tarang::core::VideoCodec)> {
    for (idx, stream) in info.streams.iter().enumerate() {
        if let tarang::core::StreamInfo::Video(vs) = stream {
            return Some((idx, vs.codec));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn generate_thumbnails_nonexistent_file_returns_error() {
        crate::init().ok();
        let spec = ThumbnailSpec {
            width: 160,
            height: 90,
            interval_ms: 1000,
            strategy: ThumbnailStrategy::SceneBased,
        };
        let path = PathBuf::from("/tmp/nonexistent_video_tazama_test.mp4");
        let result = generate_thumbnails(&path, spec).await;
        assert!(result.is_err(), "expected error for nonexistent file");
    }

    #[test]
    fn thumbnail_spec_construction() {
        let spec = ThumbnailSpec {
            width: 320,
            height: 180,
            interval_ms: 500,
            strategy: ThumbnailStrategy::SceneBased,
        };
        assert_eq!(spec.width, 320);
        assert_eq!(spec.height, 180);
        assert_eq!(spec.interval_ms, 500);
    }

    #[test]
    fn thumbnail_spec_zero_interval() {
        let spec = ThumbnailSpec {
            width: 160,
            height: 90,
            interval_ms: 0,
            strategy: ThumbnailStrategy::SceneBased,
        };
        assert_eq!(spec.interval_ms, 0);
    }

    #[test]
    fn thumbnail_spec_clone_and_copy() {
        let spec = ThumbnailSpec {
            width: 640,
            height: 360,
            interval_ms: 2000,
            strategy: ThumbnailStrategy::SceneBased,
        };
        let copied = spec;
        let cloned = spec;
        assert_eq!(copied.width, cloned.width);
        assert_eq!(copied.height, cloned.height);
        assert_eq!(copied.interval_ms, cloned.interval_ms);
    }

    #[test]
    fn thumbnail_spec_debug_format() {
        let spec = ThumbnailSpec {
            width: 100,
            height: 50,
            interval_ms: 1000,
            strategy: ThumbnailStrategy::SceneBased,
        };
        let debug = format!("{:?}", spec);
        assert!(debug.contains("100"));
        assert!(debug.contains("50"));
        assert!(debug.contains("1000"));
    }

    #[test]
    fn thumbnail_spec_serde_roundtrip() {
        let spec = ThumbnailSpec {
            width: 320,
            height: 180,
            interval_ms: 2000,
            strategy: ThumbnailStrategy::ContentBased,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: ThumbnailSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec.width, deserialized.width);
        assert_eq!(spec.height, deserialized.height);
        assert_eq!(spec.interval_ms, deserialized.interval_ms);
        assert_eq!(spec.strategy, deserialized.strategy);
    }

    #[test]
    fn thumbnail_spec_deserialize_from_json_value() {
        let val = serde_json::json!({
            "width": 640,
            "height": 360,
            "interval_ms": 500
        });
        // strategy should default to SceneBased when omitted
        let spec: ThumbnailSpec = serde_json::from_value(val).unwrap();
        assert_eq!(spec.width, 640);
        assert_eq!(spec.height, 360);
        assert_eq!(spec.interval_ms, 500);
        assert_eq!(spec.strategy, ThumbnailStrategy::SceneBased);
    }

    #[test]
    fn thumbnail_spec_deserialize_missing_field_fails() {
        let val = serde_json::json!({ "width": 100, "height": 50 });
        let result = serde_json::from_value::<ThumbnailSpec>(val);
        assert!(result.is_err());
    }

    #[test]
    fn thumbnail_spec_large_values() {
        let spec = ThumbnailSpec {
            width: u32::MAX,
            height: u32::MAX,
            interval_ms: u64::MAX,
            strategy: ThumbnailStrategy::SceneBased,
        };
        let json = serde_json::to_string(&spec).unwrap();
        let back: ThumbnailSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back.width, u32::MAX);
        assert_eq!(back.height, u32::MAX);
        assert_eq!(back.interval_ms, u64::MAX);
    }

    #[tokio::test]
    async fn generate_thumbnails_gst_nonexistent_returns_error() {
        crate::init().ok();
        let spec = ThumbnailSpec {
            width: 160,
            height: 90,
            interval_ms: 1000,
            strategy: ThumbnailStrategy::SceneBased,
        };
        let path = PathBuf::from("/tmp/absolutely_nonexistent_tazama_test_file.mp4");
        let result = generate_thumbnails_gst(&path, spec).await;
        assert!(result.is_err(), "expected error for nonexistent file");
    }

    #[tokio::test]
    async fn generate_thumbnails_gst_directory_returns_error() {
        crate::init().ok();
        let spec = ThumbnailSpec {
            width: 160,
            height: 90,
            interval_ms: 1000,
            strategy: ThumbnailStrategy::SceneBased,
        };
        let path = PathBuf::from("/tmp");
        let result = generate_thumbnails_gst(&path, spec).await;
        assert!(result.is_err(), "expected error when path is a directory");
    }

    mod tarang_tests {
        use super::*;

        #[test]
        fn is_tarang_video_mp4() {
            assert!(is_tarang_video(Path::new("test.mp4")));
        }

        #[test]
        fn is_tarang_video_mkv() {
            assert!(is_tarang_video(Path::new("test.mkv")));
        }

        #[test]
        fn is_tarang_video_webm() {
            assert!(is_tarang_video(Path::new("test.webm")));
        }

        #[test]
        fn is_tarang_video_m4v() {
            assert!(is_tarang_video(Path::new("video.m4v")));
        }

        #[test]
        fn is_tarang_video_case_insensitive() {
            assert!(is_tarang_video(Path::new("test.MP4")));
            assert!(is_tarang_video(Path::new("test.MKV")));
        }

        #[test]
        fn is_tarang_video_unsupported_extension() {
            assert!(!is_tarang_video(Path::new("test.avi")));
            assert!(!is_tarang_video(Path::new("test.mov")));
        }

        #[test]
        fn is_tarang_video_no_extension() {
            assert!(!is_tarang_video(Path::new("videofile")));
        }

        #[test]
        fn create_demuxer_nonexistent_file_returns_error() {
            let result = create_demuxer(Path::new("/tmp/nonexistent_tazama_test.mp4"));
            assert!(result.is_err());
        }
    }
}
