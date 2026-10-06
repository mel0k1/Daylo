use crate::engine;

#[tauri::command]
pub async fn pack_export(app: tauri::AppHandle, version: String) -> Result<Option<String>, String> {
    let path = engine::game::pack::export(&app, &version).await?;
    Ok(path.map(|p| p.display().to_string()))
}

#[tauri::command]
pub async fn pack_import(app: tauri::AppHandle) -> Result<Option<String>, String> {
    // Диалог, создание сборки; установка идёт в фоне с событием pack-progress
    engine::game::pack::import(&app).await
}
