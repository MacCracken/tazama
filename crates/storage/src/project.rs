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

#[cfg(test)]
mod tests {
    use super::*;
    use tazama_core::ProjectSettings;

    #[tokio::test]
    async fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join("tazama-test-project");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let path = dir.join("test.tazama");

        let project = Project::new("test project", ProjectSettings::default());
        let id = project.id;
        ProjectStore::save(&project, &path).await.unwrap();

        let loaded = ProjectStore::load(&path).await.unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.name, "test project");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_nonexistent_returns_error() {
        let result = ProjectStore::load(Path::new("/nonexistent/project.tazama")).await;
        assert!(result.is_err());
    }
}
