use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaPipelineError {
    #[error("GStreamer error: {0}")]
    Gstreamer(String),

    #[error("probe failed for {path}: {reason}")]
    ProbeFailed { path: String, reason: String },

    #[error("decode error: {0}")]
    Decode(String),

    #[error("export error: {0}")]
    Export(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("pipeline state change failed: {0}")]
    StateChange(String),
}

impl From<gstreamer::glib::Error> for MediaPipelineError {
    fn from(err: gstreamer::glib::Error) -> Self {
        Self::Gstreamer(err.to_string())
    }
}

impl From<gstreamer::glib::BoolError> for MediaPipelineError {
    fn from(err: gstreamer::glib::BoolError) -> Self {
        Self::Gstreamer(err.to_string())
    }
}
