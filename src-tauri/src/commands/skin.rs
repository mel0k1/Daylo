use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

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

// Локальный PNG уходит в аккаунт Ely.by: модель classic или slim
#[tauri::command]
pub async fn ely_upload_skin(model: String, png_base64: String) -> Result<(), String> {
    let png = B64
        .decode(png_base64)
        .map_err(|_| "не PNG в base64")?;
    engine::core::ely::upload_skin(&model, &png).await
}
