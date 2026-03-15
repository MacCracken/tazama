pub mod decode;
pub mod error;
pub mod export;
pub mod mix;
pub mod playback;
pub mod probe;
pub mod thumbnail;
pub mod waveform;

use std::sync::Once;

pub use decode::{AudioBuffer, DecoderConfig, FrameRange, VideoFrame};
pub use error::MediaPipelineError;
pub use export::{ExportConfig, ExportFormat, ExportProgress};
pub use playback::AudioPreview;

static GST_INIT: Once = Once::new();

/// Initialize GStreamer. Safe to call multiple times.
pub fn init() -> Result<(), MediaPipelineError> {
    let mut result = Ok(());
    GST_INIT.call_once(|| {
        if let Err(e) = gstreamer::init() {
            result = Err(MediaPipelineError::Gstreamer(e.to_string()));
        }
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gstreamer_init_succeeds() {
        assert!(init().is_ok());
    }

    #[test]
    fn gstreamer_init_idempotent() {
        assert!(init().is_ok());
        assert!(init().is_ok());
    }
}
