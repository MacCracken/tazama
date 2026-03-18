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

    #[tokio::test]
    async fn save_overwrites_existing_file() {
        let dir = std::env::temp_dir().join("tazama-test-project-overwrite");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let path = dir.join("overwrite.tazama");

        let project1 = Project::new("first", ProjectSettings::default());
        ProjectStore::save(&project1, &path).await.unwrap();

        let project2 = Project::new("second", ProjectSettings::default());
        ProjectStore::save(&project2, &path).await.unwrap();

        let loaded = ProjectStore::load(&path).await.unwrap();
        assert_eq!(loaded.name, "second");
        assert_eq!(loaded.id, project2.id);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_invalid_json_returns_error() {
        let dir = std::env::temp_dir().join("tazama-test-project-invalid");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let path = dir.join("invalid.tazama");

        tokio::fs::write(&path, "this is not valid json {{{")
            .await
            .unwrap();
        let result = ProjectStore::load(&path).await;
        assert!(result.is_err());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn round_trip_preserves_timeline_data() {
        let dir = std::env::temp_dir().join("tazama-test-project-timeline");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let path = dir.join("timeline.tazama");

        let mut project = Project::new("timeline project", ProjectSettings::default());
        let track =
            tazama_core::timeline::Track::new("V1", tazama_core::timeline::TrackKind::Video);
        project.timeline.add_track(track);
        let track_id = project.timeline.tracks[0].id;
        let clip = tazama_core::clip::Clip::new("clip1", tazama_core::clip::ClipKind::Video, 0, 60);
        let clip_id = clip.id;
        project
            .timeline
            .track_mut(track_id)
            .unwrap()
            .add_clip(clip)
            .unwrap();

        ProjectStore::save(&project, &path).await.unwrap();
        let loaded = ProjectStore::load(&path).await.unwrap();

        assert_eq!(loaded.timeline.tracks.len(), 1);
        assert_eq!(loaded.timeline.tracks[0].name, "V1");
        assert_eq!(loaded.timeline.tracks[0].clips.len(), 1);
        assert_eq!(loaded.timeline.tracks[0].clips[0].id, clip_id);
        assert_eq!(loaded.timeline.tracks[0].clips[0].duration, 60);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
