use crate::engine;

#[tauri::command]
pub async fn modrinth_search(
    query: String,
    game_version: String,
) -> Result<Vec<engine::game::modrinth::SearchHit>, String> {
    engine::game::modrinth::search(&query, &game_version).await
}

#[tauri::command]
pub async fn modrinth_versions(
    project_id: String,
    game_version: String,
) -> Result<Vec<engine::game::modrinth::ModVersion>, String> {
    engine::game::modrinth::versions(&project_id, &game_version).await
}

#[tauri::command]
pub async fn modrinth_install(version: String, mod_version_id: String) -> Result<(), String> {
    engine::game::modrinth::install(&version, &mod_version_id).await
}

#[tauri::command]
pub async fn mods_list(version: String) -> Result<Vec<String>, String> {
    Ok(engine::game::modrinth::installed(&version).await)
}

#[tauri::command]
pub async fn mods_delete(version: String, file: String) -> Result<(), String> {
    engine::game::modrinth::uninstall(&version, &file).await
}
