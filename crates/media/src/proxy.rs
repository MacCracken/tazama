//! Proxy generation for lower-resolution preview playback.

use crate::error::MediaPipelineError;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Generate a lower-resolution proxy file for preview playback.
///
/// Transcodes the source video to a smaller resolution using GStreamer.
/// The proxy is stored in `proxy_dir` with the same filename + "_proxy.mp4".
pub async fn generate_proxy(
    source: &Path,
    proxy_dir: &Path,
    target_width: u32,
) -> Result<PathBuf, MediaPipelineError> {
    // Ensure proxy directory exists
    tokio::fs::create_dir_all(proxy_dir).await?;

    // Determine output path
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "proxy".to_string());
    let proxy_path = proxy_dir.join(format!("{stem}_proxy_{target_width}.mp4"));

    // If proxy already exists and is newer than source, skip
    if proxy_path.exists() {
        let source_meta = tokio::fs::metadata(source).await?;
        let proxy_meta = tokio::fs::metadata(&proxy_path).await?;
        if let (Ok(src_time), Ok(proxy_time)) = (source_meta.modified(), proxy_meta.modified())
            && proxy_time > src_time
        {
            debug!("proxy already up-to-date: {}", proxy_path.display());
            return Ok(proxy_path);
        }
    }

    let source_str = source.to_string_lossy().to_string();
    let proxy_str = proxy_path.to_string_lossy().to_string();

    info!(
        "generating proxy: {} -> {} (width={})",
        source_str, proxy_str, target_width
    );

    tokio::task::spawn_blocking(move || generate_proxy_sync(&source_str, &proxy_str, target_width))
        .await
        .map_err(|e| MediaPipelineError::Export(format!("proxy task failed: {e}")))?
        .map(|()| proxy_path)
}

fn generate_proxy_sync(
    source: &str,
    output: &str,
    target_width: u32,
) -> Result<(), MediaPipelineError> {
    use gstreamer::prelude::*;

    // Reject non-video inputs that would cause the GStreamer pipeline to silently fail
    if let Some(ext) = Path::new(source).extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        match ext_lower.as_str() {
            "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac" => {
                return Err(MediaPipelineError::UnsupportedFormat(
                    "proxy generation not supported for audio-only files".into(),
                ));
            }
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tiff" | "svg" => {
                return Err(MediaPipelineError::UnsupportedFormat(
                    "proxy generation not supported for image files".into(),
                ));
            }
            _ => {}
        }
    }

    let pipeline = gstreamer::Pipeline::new();

    let filesrc = gstreamer::ElementFactory::make("filesrc")
        .property("location", source)
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let decodebin = gstreamer::ElementFactory::make("decodebin")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let videoconvert = gstreamer::ElementFactory::make("videoconvert")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let videoscale = gstreamer::ElementFactory::make("videoscale")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Scale to target_width, maintaining aspect ratio
    let capsfilter = gstreamer::ElementFactory::make("capsfilter")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let scale_caps = gstreamer_video::VideoCapsBuilder::new()
        .width(target_width as i32)
        .build();
    capsfilter.set_property("caps", &scale_caps);

    let encoder = gstreamer::ElementFactory::make("x264enc")
        .property_from_str("speed-preset", "ultrafast")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let muxer = gstreamer::ElementFactory::make("mp4mux")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let filesink = gstreamer::ElementFactory::make("filesink")
        .property("location", output)
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    pipeline
        .add_many([
            &filesrc,
            &decodebin,
            &videoconvert,
            &videoscale,
            &capsfilter,
            &encoder,
            &muxer,
            &filesink,
        ])
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    filesrc
        .link(&decodebin)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Link the rest statically
    gstreamer::Element::link_many([
        &videoconvert,
        &videoscale,
        &capsfilter,
        &encoder,
        &muxer,
        &filesink,
    ])
    .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Connect decodebin's pad-added signal to videoconvert
    let videoconvert_weak = videoconvert.downgrade();
    decodebin.connect_pad_added(move |_dbin, src_pad| {
        let Some(videoconvert) = videoconvert_weak.upgrade() else {
            return;
        };
        if let Some(sink_pad) = videoconvert.static_pad("sink")
            && !sink_pad.is_linked()
        {
            let _ = src_pad.link(&sink_pad);
        }
    });

    pipeline
        .set_state(gstreamer::State::Playing)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    let bus = pipeline
        .bus()
        .ok_or_else(|| MediaPipelineError::Export("pipeline has no bus".into()))?;

    for msg in bus.iter_timed(gstreamer::ClockTime::from_seconds(600)) {
        match msg.view() {
            gstreamer::MessageView::Eos(..) => break,
            gstreamer::MessageView::Error(err) => {
                pipeline
                    .set_state(gstreamer::State::Null)
                    .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;
                return Err(MediaPipelineError::Export(format!(
                    "proxy generation error: {}: {:?}",
                    err.error(),
                    err.debug()
                )));
            }
            _ => {}
        }
    }

    pipeline
        .set_state(gstreamer::State::Null)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    info!("proxy generated: {output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn proxy_path_construction() {
        // Verify the proxy path naming convention
        let source = Path::new("/videos/my_clip.mp4");
        let proxy_dir = Path::new("/tmp/proxies");
        let target_width = 640u32;

        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "proxy".to_string());
        let proxy_path = proxy_dir.join(format!("{stem}_proxy_{target_width}.mp4"));

        assert_eq!(
            proxy_path,
            PathBuf::from("/tmp/proxies/my_clip_proxy_640.mp4")
        );
    }

    #[test]
    fn proxy_path_no_extension() {
        // Source file without extension
        let source = Path::new("/videos/raw_footage");
        let proxy_dir = Path::new("/tmp/proxies");
        let target_width = 1280u32;

        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "proxy".to_string());
        let proxy_path = proxy_dir.join(format!("{stem}_proxy_{target_width}.mp4"));

        assert_eq!(
            proxy_path,
            PathBuf::from("/tmp/proxies/raw_footage_proxy_1280.mp4")
        );
    }

    #[tokio::test]
    async fn generate_proxy_nonexistent_source_errors() {
        // Attempting to generate a proxy from a file that doesn't exist
        // should return an error (GStreamer will fail to open the file).
        let source = Path::new("/tmp/tazama_test_nonexistent_video_12345.mp4");
        let proxy_dir = Path::new("/tmp/tazama_proxy_test");

        let result = generate_proxy(source, proxy_dir, 640).await;
        assert!(
            result.is_err(),
            "expected error for nonexistent source, got Ok"
        );

        // Clean up proxy dir if created
        let _ = tokio::fs::remove_dir(proxy_dir).await;
    }

    #[test]
    fn proxy_path_with_spaces() {
        let source = Path::new("/videos/my video clip.mp4");
        let proxy_dir = Path::new("/tmp/proxy dir");
        let target_width = 640u32;

        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "proxy".to_string());
        let proxy_path = proxy_dir.join(format!("{stem}_proxy_{target_width}.mp4"));

        assert_eq!(
            proxy_path,
            PathBuf::from("/tmp/proxy dir/my video clip_proxy_640.mp4")
        );
    }

    #[test]
    fn proxy_path_with_unicode() {
        let source = Path::new("/videos/\u{00e9}dition_vid\u{00e9}o.mp4");
        let proxy_dir = Path::new("/tmp/proxies");
        let target_width = 720u32;

        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "proxy".to_string());
        let proxy_path = proxy_dir.join(format!("{stem}_proxy_{target_width}.mp4"));

        assert_eq!(
            proxy_path,
            PathBuf::from("/tmp/proxies/\u{00e9}dition_vid\u{00e9}o_proxy_720.mp4")
        );
    }

    #[test]
    fn proxy_path_preserves_directory_structure() {
        // The proxy should be placed in proxy_dir, not in the source's directory
        let source = Path::new("/deep/nested/path/to/video.mp4");
        let proxy_dir = Path::new("/tmp/proxies");
        let target_width = 480u32;

        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "proxy".to_string());
        let proxy_path = proxy_dir.join(format!("{stem}_proxy_{target_width}.mp4"));

        // Proxy goes in proxy_dir, not in source's parent
        assert_eq!(proxy_path.parent().unwrap(), Path::new("/tmp/proxies"));
        assert_eq!(
            proxy_path,
            PathBuf::from("/tmp/proxies/video_proxy_480.mp4")
        );
    }
}
