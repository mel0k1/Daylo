use crate::engine;

#[tauri::command]
pub async fn skin_save(nick: String, png_base64: String) -> Result<(), String> {
    engine::game::skin::save(&nick, &png_base64).await
}

#[tauri::command]
pub async fn skin_load(nick: String) -> Result<Option<String>, String> {
    Ok(engine::game::skin::load(&nick).await)
}

#[tauri::command]
pub async fn skin_delete(nick: String) -> Result<(), String> {
    engine::game::skin::delete(&nick).await
}
