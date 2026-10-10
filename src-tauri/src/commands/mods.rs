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

#[tauri::command]
pub async fn mods_updates(
    version: String,
) -> Result<Vec<engine::game::modrinth::ModUpdate>, String> {
    let gv = game_version_for(&version).await;
    Ok(engine::game::modrinth::check_updates(&version, &gv).await)
}

// Обновление по источнику мода: «cf:ид» — CurseForge, остальное Modrinth
#[tauri::command]
pub async fn mods_update(
    version: String,
    file: String,
    mod_version_id: String,
    project_id: String,
) -> Result<(), String> {
    if let Some(mod_id) = project_id.strip_prefix("cf:") {
        engine::game::curseforge::replace_mod(&version, mod_id, &mod_version_id, &file).await
    } else {
        engine::game::modrinth::update(&version, &file, &mod_version_id).await
    }
}

// Базовая версия игры: у профилей загрузчиков — родительская ванила
async fn game_version_for(id: &str) -> String {
    for (pid, _loader, base) in engine::game::loaders::local_profiles().await {
        if pid == id && !base.is_empty() {
            return base;
        }
    }
    id.to_string()
}
