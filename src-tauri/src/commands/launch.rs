pub mod launch;
pub mod system;

use crate::engine;

#[tauri::command]
pub async fn list_versions() -> Result<Vec<engine::game::mcmeta::VersionEntry>, String> {
    engine::game::mcmeta::list().await
}

#[tauri::command]
pub async fn install_version(app: tauri::AppHandle, version: String) -> Result<(), String> {
    // Прогресс и ошибка уходят событием install-progress
    tauri::async_runtime::spawn(async move {
        let _ = engine::game::install::ensure_version(app, &version).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn launch_game(
    app: tauri::AppHandle,
    version: String,
    nick: String,
) -> Result<(), String> {
    let nick = nick.trim();
    if nick.is_empty() {
        return Err("введите ник".into());
    }
    engine::game::launch::launch(app, &version, nick).await
}

#[tauri::command]
pub async fn stop_game(version: String) -> Result<(), String> {
    engine::game::launch::stop(&version).await
}
