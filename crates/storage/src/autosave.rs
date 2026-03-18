use std::path::{Path, PathBuf};
use std::sync::Arc;

use tazama_core::Project;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Manages periodic autosave of a project file.
///
/// When started, saves `{path}.autosave` at a regular interval whenever the
/// project is marked dirty. The autosave file is cleaned up on manual save.
pub struct AutosaveManager {
    interval_secs: u64,
    dirty: Arc<Mutex<bool>>,
    project: Arc<Mutex<Option<Project>>>,
    path: Arc<Mutex<Option<PathBuf>>>,
    stop_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
}

impl AutosaveManager {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            interval_secs,
            dirty: Arc::new(Mutex::new(false)),
            project: Arc::new(Mutex::new(None)),
            path: Arc::new(Mutex::new(None)),
            stop_tx: None,
            task: None,
        }
    }

    /// Start the autosave loop.
    pub fn start(&mut self) {
        if self.task.is_some() {
            return;
        }

        let (stop_tx, mut stop_rx) = watch::channel(false);
        self.stop_tx = Some(stop_tx);

        let dirty = Arc::clone(&self.dirty);
        let project = Arc::clone(&self.project);
        let path = Arc::clone(&self.path);
        let interval = self.interval_secs;

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(interval)) => {}
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            debug!("autosave loop stopped");
                            return;
                        }
                    }
                }

                let is_dirty = {
                    let mut d = dirty.lock().await;
                    if !*d {
                        continue;
                    }
                    *d = false;
                    true
                };

                if !is_dirty {
                    continue;
                }

                let proj = project.lock().await.clone();
                let save_path = path.lock().await.clone();

                if let (Some(proj), Some(p)) = (proj, save_path) {
                    let autosave_path = autosave_path_for(&p);
                    match save_autosave(&proj, &autosave_path).await {
                        Ok(()) => debug!("autosaved to {}", autosave_path.display()),
                        Err(e) => warn!("autosave failed: {e}"),
                    }
                }
            }
        });

        self.task = Some(handle);
        info!("autosave started (interval: {}s)", self.interval_secs);
    }

    /// Stop the autosave loop.
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(handle) = self.task.take() {
            handle.abort();
        }
        info!("autosave stopped");
    }

    /// Mark the project as dirty (needs autosave).
    pub async fn mark_dirty(&self) {
        *self.dirty.lock().await = true;
    }

    /// Update the project snapshot for autosave.
    pub async fn update_project(&self, project: Project, path: PathBuf) {
        *self.project.lock().await = Some(project);
        *self.path.lock().await = Some(path);
        self.mark_dirty().await;
    }
}

impl Drop for AutosaveManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Get the autosave file path for a given project path.
fn autosave_path_for(path: &Path) -> PathBuf {
    let mut autosave = path.to_path_buf();
    let ext = autosave
        .extension()
        .map(|e| format!("{}.autosave", e.to_string_lossy()))
        .unwrap_or_else(|| "autosave".to_string());
    autosave.set_extension(ext);
    autosave
}

async fn save_autosave(project: &Project, path: &Path) -> Result<(), std::io::Error> {
    let json = serde_json::to_string(project)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    tokio::fs::write(path, json).await
}

/// Check if a newer autosave file exists for the given project path.
/// Returns the recovered project if the autosave is newer.
pub async fn recover(path: &Path) -> Option<Project> {
    let autosave = autosave_path_for(path);
    if !autosave.exists() {
        return None;
    }

    // Compare modification times
    let original_meta = tokio::fs::metadata(path).await.ok();
    let autosave_meta = tokio::fs::metadata(&autosave).await.ok()?;

    let autosave_newer = match original_meta {
        Some(orig) => {
            let orig_time = orig.modified().ok();
            let auto_time = autosave_meta.modified().ok();
            match (orig_time, auto_time) {
                (Some(o), Some(a)) => a > o,
                (None, Some(_)) => true,
                _ => false,
            }
        }
        None => true, // No original file, autosave is "newer"
    };

    if !autosave_newer {
        return None;
    }

    let data = tokio::fs::read_to_string(&autosave).await.ok()?;
    let project: Project = serde_json::from_str(&data).ok()?;
    info!("recovered project from autosave: {}", autosave.display());
    Some(project)
}

/// Remove the autosave file after a manual save.
pub async fn cleanup(path: &Path) {
    let autosave = autosave_path_for(path);
    if autosave.exists() {
        if let Err(e) = tokio::fs::remove_file(&autosave).await {
            warn!("failed to remove autosave: {e}");
        } else {
            debug!("autosave cleaned up: {}", autosave.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tazama_core::ProjectSettings;

    #[test]
    fn autosave_path_for_adds_suffix() {
        let path = PathBuf::from("/tmp/project.tazama");
        let result = autosave_path_for(&path);
        assert_eq!(result, PathBuf::from("/tmp/project.tazama.autosave"));
    }

    #[test]
    fn autosave_path_for_no_extension() {
        let path = PathBuf::from("/tmp/project");
        let result = autosave_path_for(&path);
        assert_eq!(result, PathBuf::from("/tmp/project.autosave"));
    }

    #[tokio::test]
    async fn save_and_recover() {
        let dir = std::env::temp_dir().join("tazama-autosave-test");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let path = dir.join("test.tazama");

        let project = Project::new("autosave test", ProjectSettings::default());
        let id = project.id;

        // Save autosave
        let autosave = autosave_path_for(&path);
        save_autosave(&project, &autosave).await.unwrap();

        // Recover
        let recovered = recover(&path).await.unwrap();
        assert_eq!(recovered.id, id);
        assert_eq!(recovered.name, "autosave test");

        // Cleanup
        cleanup(&path).await;
        assert!(!autosave.exists());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn recover_no_autosave_returns_none() {
        let path = PathBuf::from("/tmp/nonexistent-tazama-project.tazama");
        assert!(recover(&path).await.is_none());
    }

    #[tokio::test]
    async fn cleanup_nonexistent_is_noop() {
        let path = PathBuf::from("/tmp/nonexistent-tazama-project.tazama");
        cleanup(&path).await; // Should not panic
    }

    #[test]
    fn autosave_manager_new() {
        let mgr = AutosaveManager::new(30);
        assert_eq!(mgr.interval_secs, 30);
    }
}
