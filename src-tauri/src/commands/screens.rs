use crate::engine;

#[tauri::command]
pub async fn shots_list(version: String) -> Vec<engine::game::shots::Shot> {
    engine::game::shots::list(&version).await
}

#[tauri::command]
pub async fn shots_read(version: String, name: String) -> Result<String, String> {
    engine::game::shots::read(&version, &name).await
}

#[tauri::command]
pub async fn shots_delete(version: String, name: String) -> Result<(), String> {
    engine::game::shots::delete(&version, &name).await
}

#[tauri::command]
pub async fn shots_open(version: String) -> Result<(), String> {
    engine::game::shots::open_dir(&version).await
}
