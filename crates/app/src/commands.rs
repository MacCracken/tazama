use tazama_core::{Project, ProjectSettings};

#[tauri::command]
pub async fn new_project(name: String, width: u32, height: u32) -> Result<Project, String> {
    let settings = ProjectSettings {
        width,
        height,
        ..Default::default()
    };
    Ok(Project::new(name, settings))
}

#[tauri::command]
pub async fn open_project(path: String) -> Result<Project, String> {
    tazama_storage::ProjectStore::load(std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_project(project: Project, path: String) -> Result<(), String> {
    tazama_storage::ProjectStore::save(&project, std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_media(project_root: String, source: String) -> Result<String, String> {
    let store = tazama_storage::MediaStore::new(&project_root);
    let dest = store
        .import(std::path::Path::new(&source))
        .await
        .map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}
