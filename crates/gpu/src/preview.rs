use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tokio::time::{self, Duration};
use tazama_core::{FrameRate, PlaybackPosition, PlaybackState, ProjectSettings, Timeline};

use crate::frame_source::{FrameSource, GpuFrame};
use crate::render::Renderer;

/// Real-time preview loop that renders frames at the project frame rate.
pub struct PreviewLoop {
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl PreviewLoop {
    /// Start the preview loop.
    ///
    /// Reads playback position from `position_rx`, renders frames, and sends
    /// them to `frame_tx`. Drops frames if the consumer falls behind.
    ///
    /// If `audio_preview` is provided, audio for each frame's time window is
    /// decoded and fed to the audio output.
    pub fn start(
        renderer: Arc<Renderer>,
        timeline: Arc<Timeline>,
        settings: ProjectSettings,
        frame_source: Arc<dyn FrameSource>,
        position_rx: watch::Receiver<PlaybackPosition>,
        frame_tx: mpsc::Sender<GpuFrame>,
        audio_preview: Option<Arc<tazama_media::AudioPreview>>,
    ) -> Self {
        let handle = tokio::spawn(async move {
            let fps = frame_rate_to_fps(&settings.frame_rate);
            let interval_duration = Duration::from_secs_f64(1.0 / fps);
            let mut interval = time::interval(interval_duration);

            loop {
                interval.tick().await;

                let position = position_rx.borrow().clone();

                // Update audio preview playing state
                if let Some(ref audio) = audio_preview {
                    audio.set_playing(position.state == PlaybackState::Playing);
                }

                if position.state == PlaybackState::Stopped {
                    continue;
                }

                let frame_index = position.frame;

                match renderer.render_frame(&timeline, frame_index, frame_source.as_ref(), &settings) {
                    Ok(frame) => {
                        // Try to send, drop frame if channel is full
                        let _ = frame_tx.try_send(frame);
                    }
                    Err(e) => {
                        tracing::warn!("Preview render failed at frame {frame_index}: {e}");
                    }
                }
            }
        });

        Self {
            handle: Some(handle),
        }
    }

    /// Stop the preview loop.
    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

impl Drop for PreviewLoop {
    fn drop(&mut self) {
        self.stop();
    }
}

fn frame_rate_to_fps(rate: &FrameRate) -> f64 {
    rate.numerator as f64 / rate.denominator as f64
}
