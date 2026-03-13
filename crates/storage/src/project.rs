use std::path::Path;

use tazama_core::{Project, ProjectId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectStoreError {
    #[error("project not found: {0:?}")]
    NotFound(ProjectId),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// File-based project persistence.
pub struct ProjectStore;

impl ProjectStore {
    pub async fn save(project: &Project, path: &Path) -> Result<(), ProjectStoreError> {
        let json = serde_json::to_string_pretty(project)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    pub async fn load(path: &Path) -> Result<Project, ProjectStoreError> {
        let data = tokio::fs::read_to_string(path).await?;
        let project = serde_json::from_str(&data)?;
        Ok(project)
    }
}
