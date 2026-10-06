use serde::Serialize;
use serde_json::json;

use crate::engine;

#[derive(Serialize)]
pub struct AccountInfo {
    // "ely" | "offline"
    pub mode: String,
    pub name: String,
    pub uuid: String,
}

#[tauri::command]
pub async fn account_info() -> AccountInfo {
    match engine::core::ely::current_account().await {
        Some(a) => AccountInfo {
            mode: "ely".into(),
            name: a.name,
            uuid: a.uuid,
        },
        None => AccountInfo {
            mode: "offline".into(),
            name: String::new(),
            uuid: String::new(),
        },
    }
}

#[tauri::command]
pub async fn ely_login_start() -> Result<engine::core::ely::DeviceStart, String> {
    engine::core::ely::device_start().await
}

// Один опрос: фронт повторяет его с интервалом из device_start
#[tauri::command]
pub async fn ely_login_poll(device_code: String) -> Result<serde_json::Value, String> {
    match engine::core::ely::device_poll(&device_code).await? {
        engine::core::ely::Poll::Pending => Ok(json!({ "status": "pending" })),
        engine::core::ely::Poll::Done(acc) => {
            engine::core::settings::update(|s| s.account = Some(acc))?;
            Ok(json!({ "status": "done" }))
        }
        engine::core::ely::Poll::Error(e) => Ok(json!({ "status": "error", "message": e })),
    }
}

#[tauri::command]
pub async fn ely_logout() -> Result<(), String> {
    engine::core::ely::logout().await
}

// Скин из системы скинов Ely.by, base64 png
#[tauri::command]
pub async fn ely_skin(nick: String) -> Option<String> {
    let nick = nick.trim().to_string();
    if nick.is_empty() {
        return None;
    }
    engine::core::ely::skin_png(&nick).await
}
