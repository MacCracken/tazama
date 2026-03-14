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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let e = MediaPipelineError::Gstreamer("test".into());
        assert_eq!(e.to_string(), "GStreamer error: test");

        let e = MediaPipelineError::Decode("bad frame".into());
        assert_eq!(e.to_string(), "decode error: bad frame");

        let e = MediaPipelineError::Export("encoder failed".into());
        assert_eq!(e.to_string(), "export error: encoder failed");

        let e = MediaPipelineError::FileNotFound("/tmp/x.mp4".into());
        assert_eq!(e.to_string(), "file not found: /tmp/x.mp4");

        let e = MediaPipelineError::UnsupportedFormat("mov".into());
        assert_eq!(e.to_string(), "unsupported format: mov");

        let e = MediaPipelineError::StateChange("failed".into());
        assert_eq!(e.to_string(), "pipeline state change failed: failed");

        let e = MediaPipelineError::ProbeFailed {
            path: "test.mp4".into(),
            reason: "no video".into(),
        };
        assert!(e.to_string().contains("test.mp4"));
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let e: MediaPipelineError = io_err.into();
        assert!(matches!(e, MediaPipelineError::Io(_)));
    }
}
