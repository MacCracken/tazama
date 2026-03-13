pub mod decode;
pub mod error;
pub mod export;
pub mod probe;
pub mod thumbnail;
pub mod waveform;

use std::sync::Once;

pub use decode::{AudioBuffer, DecoderConfig, FrameRange, VideoFrame};
pub use error::MediaPipelineError;
pub use export::{ExportConfig, ExportFormat, ExportProgress};

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
