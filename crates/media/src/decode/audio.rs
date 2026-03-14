use std::path::Path;

use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use tokio::sync::mpsc;
use tokio::task;
use tracing::{debug, error};

use super::AudioBuffer;
use crate::error::MediaPipelineError;

/// RAII guard that sets a GStreamer pipeline to Null on drop.
struct PipelineGuard(gstreamer::Pipeline);

impl Drop for PipelineGuard {
    fn drop(&mut self) {
        let _ = self.0.set_state(gstreamer::State::Null);
    }
}

/// Decodes audio from a media file into interleaved f32 buffers.
pub struct AudioDecoder;

impl AudioDecoder {
    /// Decode all audio from a file, sending buffers over a channel.
    pub fn decode(path: &Path) -> Result<mpsc::Receiver<AudioBuffer>, MediaPipelineError> {
        let path = path.to_path_buf();
        let (tx, rx) = mpsc::channel(64);

        task::spawn_blocking(move || {
            if let Err(e) = decode_audio(&path, tx) {
                error!("audio decode error: {e}");
            }
        });

        Ok(rx)
    }
}

fn build_audio_pipeline(
    path: &Path,
    sample_rate: i32,
) -> Result<(gstreamer::Pipeline, AppSink), MediaPipelineError> {
    let pipeline = gstreamer::Pipeline::new();

    let filesrc = gstreamer::ElementFactory::make("filesrc")
        .property("location", path.to_str().unwrap_or_default())
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let decodebin = gstreamer::ElementFactory::make("decodebin")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let audioconvert = gstreamer::ElementFactory::make("audioconvert")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let audioresample = gstreamer::ElementFactory::make("audioresample")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    let appsink = gstreamer::ElementFactory::make("appsink")
        .build()
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?
        .dynamic_cast::<AppSink>()
        .map_err(|_| MediaPipelineError::Gstreamer("failed to cast to AppSink".into()))?;

    let caps = gstreamer_audio::AudioCapsBuilder::new()
        .format(gstreamer_audio::AudioFormat::F32le)
        .rate(sample_rate)
        .build();
    appsink.set_caps(Some(&caps));
    appsink.set_drop(false);
    appsink.set_sync(false);

    pipeline
        .add_many([
            &filesrc,
            &decodebin,
            &audioconvert,
            &audioresample,
            appsink.upcast_ref::<gstreamer::Element>(),
        ])
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    filesrc
        .link(&decodebin)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    // decodebin uses dynamic pads — connect audio pads
    let audioconvert_weak = audioconvert.downgrade();
    decodebin.connect_pad_added(move |_, src_pad| {
        let Some(audioconvert) = audioconvert_weak.upgrade() else {
            return;
        };

        let caps = src_pad
            .current_caps()
            .unwrap_or_else(|| src_pad.query_caps(None));
        let structure = caps.structure(0);
        if let Some(s) = structure
            && s.name().starts_with("audio/")
        {
            let Some(sink_pad) = audioconvert.static_pad("sink") else {
                return;
            };
            if !sink_pad.is_linked() {
                let _ = src_pad.link(&sink_pad);
            }
        }
    });

    audioconvert
        .link(&audioresample)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;
    audioresample
        .link(&appsink)
        .map_err(|e| MediaPipelineError::Gstreamer(e.to_string()))?;

    Ok((pipeline, appsink))
}

fn decode_audio(path: &Path, tx: mpsc::Sender<AudioBuffer>) -> Result<(), MediaPipelineError> {
    let (pipeline, appsink) = build_audio_pipeline(path, 48000)?;
    let _guard = PipelineGuard(pipeline.clone());

    pipeline
        .set_state(gstreamer::State::Playing)
        .map_err(|e| MediaPipelineError::StateChange(e.to_string()))?;

    loop {
        let sample = match appsink.pull_sample() {
            Ok(sample) => sample,
            Err(_) => break, // EOS
        };

        if let Some(buffer) = sample_to_audio_buffer(&sample)
            && tx.blocking_send(buffer).is_err()
        {
            debug!("audio decode receiver dropped");
            break;
        }
    }

    // Guard handles pipeline cleanup on all exit paths
    Ok(())
}

fn sample_to_audio_buffer(sample: &gstreamer::Sample) -> Option<AudioBuffer> {
    let buffer = sample.buffer()?;
    let caps = sample.caps()?;
    let structure = caps.structure(0)?;

    let sample_rate = structure.get::<i32>("rate").ok()? as u32;
    let channels = structure.get::<i32>("channels").ok()? as u16;

    let map = buffer.map_readable().ok()?;
    let byte_slice = map.as_slice();

    // Validate alignment — f32 samples require 4-byte aligned data
    let aligned_len = byte_slice.len() - (byte_slice.len() % 4);
    let samples: Vec<f32> = byte_slice[..aligned_len]
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    let timestamp_ns = buffer.pts().map(|pts| pts.nseconds()).unwrap_or(0);

    Some(AudioBuffer {
        sample_rate,
        channels,
        samples,
        timestamp_ns,
    })
}
