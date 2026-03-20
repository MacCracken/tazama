use sqlx::Row;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use thiserror::Error;

use tazama_core::{MediaInfo, Project};

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("{0}")]
    Other(String),
}

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open(path: &str) -> Result<Self, DbError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite:{path}?mode=rwc"))
            .await?;

        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Look up cached media info. Returns `None` if not cached or if file size/mtime changed.
    pub async fn get_cached_media_info(
        &self,
        path: &str,
        file_size: i64,
        modified_at: &str,
    ) -> Result<Option<MediaInfo>, DbError> {
        let row = sqlx::query(
            "SELECT media_info FROM media_cache WHERE file_path = ? AND file_size = ? AND modified_at = ?",
        )
        .bind(path)
        .bind(file_size)
        .bind(modified_at)
        .fetch_optional(&self.pool)
        .await?;

        const MAX_JSON_SIZE: usize = 50 * 1024 * 1024; // 50 MB

        match row {
            Some(row) => {
                let json: String = row.get("media_info");
                if json.len() > MAX_JSON_SIZE {
                    return Err(DbError::Other(
                        "JSON data exceeds maximum size of 50MB".into(),
                    ));
                }
                let info: MediaInfo = serde_json::from_str(&json)?;
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }

    /// Cache media info for a file path.
    pub async fn cache_media_info(
        &self,
        path: &str,
        file_size: i64,
        modified_at: &str,
        info: &MediaInfo,
    ) -> Result<(), DbError> {
        let json = serde_json::to_string(info)?;
        sqlx::query(
            "INSERT OR REPLACE INTO media_cache (file_path, file_size, modified_at, media_info) VALUES (?, ?, ?, ?)",
        )
        .bind(path)
        .bind(file_size)
        .bind(modified_at)
        .bind(json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Save a project to the database.
    pub async fn save_project(&self, project: &Project) -> Result<(), DbError> {
        let json = serde_json::to_string(project)?;
        let id = project.id.0.to_string();
        let created = project.created_at.to_rfc3339();
        let modified = project.modified_at.to_rfc3339();
        sqlx::query(
            "INSERT OR REPLACE INTO projects (id, name, project_json, created_at, modified_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&project.name)
        .bind(&json)
        .bind(&created)
        .bind(&modified)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Load a project by ID.
    pub async fn load_project(&self, id: &str) -> Result<Project, DbError> {
        let row = sqlx::query("SELECT project_json FROM projects WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        const MAX_JSON_SIZE: usize = 50 * 1024 * 1024; // 50 MB

        match row {
            Some(row) => {
                let json: String = row.get("project_json");
                if json.len() > MAX_JSON_SIZE {
                    return Err(DbError::Other(
                        "JSON data exceeds maximum size of 50MB".into(),
                    ));
                }
                let project: Project = serde_json::from_str(&json)?;
                Ok(project)
            }
            None => Err(DbError::ProjectNotFound(id.to_string())),
        }
    }

    /// List all projects (id, name, modified_at).
    pub async fn list_projects(&self) -> Result<Vec<(String, String, String)>, DbError> {
        let rows =
            sqlx::query("SELECT id, name, modified_at FROM projects ORDER BY modified_at DESC")
                .fetch_all(&self.pool)
                .await?;

        let projects = rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("id"),
                    row.get::<String, _>("name"),
                    row.get::<String, _>("modified_at"),
                )
            })
            .collect();

        Ok(projects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tazama_core::ProjectSettings;

    async fn test_db() -> Database {
        Database::open(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn media_cache_round_trip() {
        let db = test_db().await;
        let info = MediaInfo {
            duration_ms: 5000,
            duration_frames: 150,
            container: tazama_core::ContainerFormat::Mp4,
            video_streams: vec![],
            audio_streams: vec![],
            file_size: 1024,
        };

        db.cache_media_info("/tmp/test.mp4", 1024, "2026-01-01T00:00:00", &info)
            .await
            .unwrap();

        // Hit: same path, size, mtime
        let cached = db
            .get_cached_media_info("/tmp/test.mp4", 1024, "2026-01-01T00:00:00")
            .await
            .unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().duration_ms, 5000);

        // Miss: different size
        let cached = db
            .get_cached_media_info("/tmp/test.mp4", 2048, "2026-01-01T00:00:00")
            .await
            .unwrap();
        assert!(cached.is_none());

        // Miss: different mtime
        let cached = db
            .get_cached_media_info("/tmp/test.mp4", 1024, "2026-02-01T00:00:00")
            .await
            .unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn project_round_trip() {
        let db = test_db().await;
        let project = Project::new("Test Project", ProjectSettings::default());
        let id = project.id.0.to_string();

        db.save_project(&project).await.unwrap();

        let loaded = db.load_project(&id).await.unwrap();
        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.id, project.id);
    }

    #[tokio::test]
    async fn list_projects_returns_saved() {
        let db = test_db().await;
        let p1 = Project::new("Project A", ProjectSettings::default());
        let p2 = Project::new("Project B", ProjectSettings::default());

        db.save_project(&p1).await.unwrap();
        db.save_project(&p2).await.unwrap();

        let list = db.list_projects().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn load_missing_project_returns_error() {
        let db = test_db().await;
        let result = db.load_project("nonexistent").await;
        assert!(matches!(result, Err(DbError::ProjectNotFound(_))));
    }
}
