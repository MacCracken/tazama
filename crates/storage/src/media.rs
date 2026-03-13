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

        let dest = self.asset_path(
            source
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("unknown"),
        );

        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::copy(source, &dest).await?;
        Ok(dest)
    }
}
