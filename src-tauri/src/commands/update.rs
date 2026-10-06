use crate::engine;

#[tauri::command]
pub async fn update_check() -> Result<Option<engine::core::selfupdate::UpdateInfo>, String> {
    engine::core::selfupdate::check().await
}

#[tauri::command]
pub async fn update_install(
    app: tauri::AppHandle,
    info: engine::core::selfupdate::UpdateInfo,
) -> Result<(), String> {
    engine::core::selfupdate::install(&app, &info).await?;
    // Установщики Windows/Linux сами перезапускают лаунчер — выходим
    if !cfg!(target_os = "macos") {
        app.exit(0);
    }
    Ok(())
}
