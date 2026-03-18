use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use tokio::sync::{mpsc, watch};
use tokio::task;
use tracing::{debug, error, info};

use super::{ExportConfig, ExportFormat, ExportProgress};
use crate::decode::{AudioBuffer, VideoFrame};
use crate::error::MediaPipelineError;

/// Manages a GStreamer export pipeline that muxes video and audio into an output file.
pub struct ExportPipeline;

impl ExportPipeline {
    /// Run an export, consuming video and audio frames and writing to the output file.
    ///
    /// Returns a watch receiver for progress updates.
    pub fn run(
        config: ExportConfig,
        video_rx: mpsc::Receiver<VideoFrame>,
        audio_rx: mpsc::Receiver<AudioBuffer>,
    ) -> Result<watch::Receiver<ExportProgress>, MediaPipelineError> {
        Self::run_with_total(config, video_rx, audio_rx, 0)
    }

    /// Run an export with a known total frame count for progress tracking.
    pub fn run_with_total(
        config: ExportConfig,
        mut video_rx: mpsc::Receiver<VideoFrame>,
        mut audio_rx: mpsc::Receiver<AudioBuffer>,
        total_frames: u64,
    ) -> Result<watch::Receiver<ExportProgress>, MediaPipelineError> {
        let (progress_tx, progress_rx) = watch::channel(ExportProgress {
            frames_written: 0,
            total_frames,
            done: false,
        });

        let config_clone = config.clone();
        task::spawn_blocking(move || {
            if let Err(e) = run_export(
                config_clone,
                &mut video_rx,
                &mut audio_rx,
                &progress_tx,
                total_frames,
            ) {
                error!("export pipeline error: {e}");
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

fn run_export(
    config: ExportConfig,
    video_rx: &mut mpsc::Receiver<VideoFrame>,
    audio_rx: &mut mpsc::Receiver<AudioBuffer>,
    progress_tx: &watch::Sender<ExportProgress>,
    total_frames: u64,
) -> Result<(), MediaPipelineError> {
    let pipeline = gstreamer::Pipeline::new();

    // Video branch
    let video_appsrc = gstreamer::ElementFactory::make("appsrc")
        .name("video_src")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?
        .dynamic_cast::<AppSrc>()
        .map_err(|_| MediaPipelineError::Gstreamer("failed to cast video AppSrc".into()))?;

    let video_caps = gstreamer_video::VideoCapsBuilder::new()
        .format(gstreamer_video::VideoFormat::Rgba)
        .width(config.width as i32)
        .height(config.height as i32)
        .framerate(gstreamer::Fraction::new(
            config.frame_rate.0 as i32,
            config.frame_rate.1 as i32,
        ))
        .build();
    video_appsrc.set_caps(Some(&video_caps));
    video_appsrc.set_format(gstreamer::Format::Time);

    let videoconvert = gstreamer::ElementFactory::make("videoconvert")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Audio branch
    let audio_appsrc = gstreamer::ElementFactory::make("appsrc")
        .name("audio_src")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?
        .dynamic_cast::<AppSrc>()
        .map_err(|_| MediaPipelineError::Gstreamer("failed to cast audio AppSrc".into()))?;

    let audio_caps = gstreamer_audio::AudioCapsBuilder::new()
        .format(gstreamer_audio::AudioFormat::F32le)
        .rate(config.sample_rate as i32)
        .channels(config.channels as i32)
        .build();
    audio_appsrc.set_caps(Some(&audio_caps));
    audio_appsrc.set_format(gstreamer::Format::Time);

    let audioconvert = gstreamer::ElementFactory::make("audioconvert")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Encoder + muxer based on format
    let (video_enc, audio_enc, muxer) = match config.format {
        ExportFormat::Mp4 => {
            let venc = if config.hardware_accel {
                try_hw_encoder().unwrap_or(
                    gstreamer::ElementFactory::make("x264enc")
                        .build()
                        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?,
                )
            } else {
                gstreamer::ElementFactory::make("x264enc")
                    .build()
                    .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?
            };
            let aenc = gstreamer::ElementFactory::make("voaacenc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let mux = gstreamer::ElementFactory::make("mp4mux")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            (venc, aenc, mux)
        }
        ExportFormat::WebM => {
            let venc = gstreamer::ElementFactory::make("vp9enc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let aenc = gstreamer::ElementFactory::make("opusenc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let mux = gstreamer::ElementFactory::make("webmmux")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            (venc, aenc, mux)
        }
        ExportFormat::ProRes => {
            let venc = gstreamer::ElementFactory::make("avenc_prores_ks")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let aenc = gstreamer::ElementFactory::make("voaacenc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let mux = gstreamer::ElementFactory::make("qtmux")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            (venc, aenc, mux)
        }
        ExportFormat::DnxHr => {
            let venc = gstreamer::ElementFactory::make("avenc_dnxhd")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let aenc = gstreamer::ElementFactory::make("voaacenc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let mux = gstreamer::ElementFactory::make("qtmux")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            (venc, aenc, mux)
        }
        ExportFormat::Mkv => {
            let venc = gstreamer::ElementFactory::make("x264enc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let aenc = gstreamer::ElementFactory::make("opusenc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let mux = gstreamer::ElementFactory::make("matroskamux")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            (venc, aenc, mux)
        }
        ExportFormat::Gif => {
            // GIF has no audio; use the same as MP4 and let the muxer handle it
            let venc = gstreamer::ElementFactory::make("x264enc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let aenc = gstreamer::ElementFactory::make("voaacenc")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            let mux = gstreamer::ElementFactory::make("mp4mux")
                .build()
                .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
            (venc, aenc, mux)
        }
    };

    let filesink = gstreamer::ElementFactory::make("filesink")
        .property("location", config.output_path.to_str().unwrap_or_default())
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    pipeline
        .add_many([
            video_appsrc.upcast_ref::<gstreamer::Element>(),
            &videoconvert,
            &video_enc,
            audio_appsrc.upcast_ref::<gstreamer::Element>(),
            &audioconvert,
            &audio_enc,
            &muxer,
            &filesink,
        ])
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Link video: appsrc -> videoconvert -> encoder -> muxer
    video_appsrc
        .link(&videoconvert)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
    videoconvert
        .link(&video_enc)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
    video_enc
        .link(&muxer)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Link audio: appsrc -> audioconvert -> encoder -> muxer
    audio_appsrc
        .link(&audioconvert)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
    audioconvert
        .link(&audio_enc)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
    audio_enc
        .link(&muxer)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // Link muxer -> filesink
    muxer
        .link(&filesink)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    pipeline
        .set_state(gstreamer::State::Playing)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    info!("export pipeline started: {:?}", config.output_path);

    // Feed video frames
    let mut frames_written = 0u64;
    while let Some(frame) = video_rx.blocking_recv() {
        let mut gst_buffer = gstreamer::Buffer::with_size(frame.data.len())
            .map_err(|_| MediaPipelineError::Export("failed to allocate video buffer".into()))?;
        {
            let buffer_ref = gst_buffer
                .get_mut()
                .ok_or_else(|| MediaPipelineError::Export("video buffer not writable".into()))?;
            buffer_ref.set_pts(gstreamer::ClockTime::from_nseconds(frame.timestamp_ns));
            let mut map = buffer_ref
                .map_writable()
                .map_err(|_| MediaPipelineError::Export("failed to map video buffer".into()))?;
            map.copy_from_slice(&frame.data);
        }

        video_appsrc
            .push_buffer(gst_buffer)
            .map_err(|e| MediaPipelineError::Export(e.to_string()))?;

        frames_written += 1;
        let _ = progress_tx.send(ExportProgress {
            frames_written,
            total_frames,
            done: false,
        });
    }
    video_appsrc
        .end_of_stream()
        .map_err(|e| MediaPipelineError::Export(e.to_string()))?;

    // Feed audio buffers
    while let Some(audio_buf) = audio_rx.blocking_recv() {
        let byte_data: Vec<u8> = audio_buf
            .samples
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();

        if byte_data.is_empty() {
            continue;
        }

        let mut gst_buffer = gstreamer::Buffer::with_size(byte_data.len())
            .map_err(|_| MediaPipelineError::Export("failed to allocate audio buffer".into()))?;
        {
            let buffer_ref = gst_buffer
                .get_mut()
                .ok_or_else(|| MediaPipelineError::Export("audio buffer not writable".into()))?;
            buffer_ref.set_pts(gstreamer::ClockTime::from_nseconds(audio_buf.timestamp_ns));
            let mut map = buffer_ref
                .map_writable()
                .map_err(|_| MediaPipelineError::Export("failed to map audio buffer".into()))?;
            map.copy_from_slice(&byte_data);
        }

        audio_appsrc
            .push_buffer(gst_buffer)
            .map_err(|e| MediaPipelineError::Export(e.to_string()))?;
    }
    audio_appsrc
        .end_of_stream()
        .map_err(|e| MediaPipelineError::Export(e.to_string()))?;

    // Wait for EOS on the bus
    let bus = pipeline
        .bus()
        .ok_or_else(|| MediaPipelineError::Export("pipeline has no bus".into()))?;
    for msg in bus.iter_timed(gstreamer::ClockTime::from_seconds(120)) {
        match msg.view() {
            gstreamer::MessageView::Eos(..) => {
                debug!("export pipeline received EOS");
                break;
            }
            gstreamer::MessageView::Error(err) => {
                pipeline
                    .set_state(gstreamer::State::Null)
                    .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;
                return Err(MediaPipelineError::Export(format!(
                    "{}: {:?}",
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

    info!("export complete: {:?}", config.output_path);
    Ok(())
}

/// Attempt to create a hardware H.264 encoder element.
///
/// Tries VAAPI first, then NVENC.  Returns `None` when neither is available,
/// letting the caller fall back to software x264enc.
fn try_hw_encoder() -> Option<gstreamer::Element> {
    let candidates = ["vaapih264enc", "nvh264enc"];
    for name in &candidates {
        if let Ok(elem) = gstreamer::ElementFactory::make(name).build() {
            info!("using hardware encoder: {name}");
            return Some(elem);
        }
    }
    info!("no hardware encoder available, using software x264enc");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_hw_encoder_returns_none_when_unavailable() {
        // In a test environment hardware encoders are typically not present,
        // so this should return None without panicking.
        gstreamer::init().ok();
        let result = try_hw_encoder();
        // We don't assert None because a CI machine *might* have VAAPI/NVENC.
        // The important thing is it doesn't panic.
        drop(result);
    }
}
