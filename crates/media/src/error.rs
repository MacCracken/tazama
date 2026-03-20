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

    #[error("tarang error: {0}")]
    Tarang(String),
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

impl From<tarang::core::TarangError> for MediaPipelineError {
    fn from(err: tarang::core::TarangError) -> Self {
        Self::Tarang(err.to_string())
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

    #[test]
    fn io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let e: MediaPipelineError = io_err.into();
        let msg = e.to_string();
        assert!(
            msg.contains("I/O error"),
            "Io variant should start with 'I/O error': {msg}"
        );
        assert!(
            msg.contains("access denied"),
            "should contain inner message: {msg}"
        );
    }

    #[test]
    fn probe_failed_display_exact_format() {
        let e = MediaPipelineError::ProbeFailed {
            path: "/videos/clip.mp4".into(),
            reason: "no video stream".into(),
        };
        assert_eq!(
            e.to_string(),
            "probe failed for /videos/clip.mp4: no video stream"
        );
    }

    #[test]
    fn all_string_variants_display() {
        // Exhaustive check of all string-based variants with specific messages
        let cases: Vec<(MediaPipelineError, &str)> = vec![
            (
                MediaPipelineError::Gstreamer("init failed".into()),
                "GStreamer error: init failed",
            ),
            (
                MediaPipelineError::Decode("corrupt frame".into()),
                "decode error: corrupt frame",
            ),
            (
                MediaPipelineError::Export("mux error".into()),
                "export error: mux error",
            ),
            (
                MediaPipelineError::UnsupportedFormat("webm".into()),
                "unsupported format: webm",
            ),
            (
                MediaPipelineError::FileNotFound("/missing.mp4".into()),
                "file not found: /missing.mp4",
            ),
            (
                MediaPipelineError::StateChange("null to playing".into()),
                "pipeline state change failed: null to playing",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn error_is_debug() {
        let e = MediaPipelineError::Gstreamer("test".into());
        let debug = format!("{:?}", e);
        assert!(
            debug.contains("Gstreamer"),
            "Debug should contain variant name: {debug}"
        );
    }

    #[test]
    fn error_variants_with_empty_strings() {
        assert_eq!(
            MediaPipelineError::Gstreamer(String::new()).to_string(),
            "GStreamer error: "
        );
        assert_eq!(
            MediaPipelineError::Decode(String::new()).to_string(),
            "decode error: "
        );
        assert_eq!(
            MediaPipelineError::Export(String::new()).to_string(),
            "export error: "
        );
        assert_eq!(
            MediaPipelineError::FileNotFound(String::new()).to_string(),
            "file not found: "
        );
        assert_eq!(
            MediaPipelineError::UnsupportedFormat(String::new()).to_string(),
            "unsupported format: "
        );
        assert_eq!(
            MediaPipelineError::StateChange(String::new()).to_string(),
            "pipeline state change failed: "
        );
    }

    #[test]
    fn probe_failed_with_empty_fields() {
        let e = MediaPipelineError::ProbeFailed {
            path: String::new(),
            reason: String::new(),
        };
        assert_eq!(e.to_string(), "probe failed for : ");
    }
}
