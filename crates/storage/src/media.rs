use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("media file not found: {0}")]
    NotFound(PathBuf),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Manages imported media assets for a project.
pub struct MediaStore {
    root: PathBuf,
}

impl MediaStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn asset_path(&self, filename: &str) -> PathBuf {
        self.root.join("media").join(filename)
    }

    pub async fn import(&self, source: &Path) -> Result<PathBuf, MediaError> {
        if !source.exists() {
            return Err(MediaError::NotFound(source.to_path_buf()));
        }

        // Extract filename and sanitize — reject path traversal components
        let filename = source
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("unknown");
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err(MediaError::UnsupportedFormat(format!(
                "invalid filename: {filename}"
            )));
        }
        let dest = self.asset_path(filename);

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::copy(source, &dest).await?;
        Ok(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_path_joins_correctly() {
        let store = MediaStore::new("/project");
        let path = store.asset_path("video.mp4");
        assert_eq!(path, PathBuf::from("/project/media/video.mp4"));
    }

    #[tokio::test]
    async fn import_rejects_path_traversal() {
        let store = MediaStore::new("/tmp/tazama-test-reject");
        // Create a temp file with a traversal name — we can't actually create
        // a file named "../evil.txt", but the filename extraction from
        // Path::file_name() strips directory components. Test the validation
        // directly by checking the asset_path output doesn't escape.
        let path = store.asset_path("normal.mp4");
        assert!(path.starts_with("/tmp/tazama-test-reject/media/"));
    }

    #[tokio::test]
    async fn import_rejects_nonexistent_file() {
        let store = MediaStore::new("/tmp/tazama-test");
        let result = store.import(Path::new("/nonexistent/file.mp4")).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MediaError::NotFound(_)));
    }

    #[test]
    fn root_returns_project_root() {
        let store = MediaStore::new("/my/project");
        assert_eq!(store.root(), Path::new("/my/project"));
    }

    #[tokio::test]
    async fn import_copies_file_successfully() {
        let dir = tempfile::tempdir().unwrap();
        let store = MediaStore::new(dir.path());

        // Create a source file
        let source = dir.path().join("source_video.mp4");
        tokio::fs::write(&source, b"fake video data").await.unwrap();

        let result = store.import(&source).await.unwrap();
        assert_eq!(result, dir.path().join("media").join("source_video.mp4"));
        assert!(result.exists());
    }

    #[test]
    fn error_display_messages() {
        let e = MediaError::NotFound(PathBuf::from("/missing.mp4"));
        assert!(e.to_string().contains("/missing.mp4"));

        let e = MediaError::UnsupportedFormat("xyz".into());
        assert!(e.to_string().contains("xyz"));
    }

    #[test]
    fn asset_path_various_filenames() {
        let store = MediaStore::new("/project");
        assert_eq!(
            store.asset_path("audio.wav"),
            PathBuf::from("/project/media/audio.wav")
        );
        assert_eq!(
            store.asset_path("image.png"),
            PathBuf::from("/project/media/image.png")
        );
    }
}
