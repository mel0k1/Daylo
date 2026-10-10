use serde::Serialize;
use serde_json::json;

use crate::engine;

#[derive(Serialize)]
pub struct AccountInfo {
    // "ely" | "msa" | "offline"
    pub mode: String,
    pub name: String,
    pub uuid: String,
}

#[tauri::command]
pub async fn account_info() -> AccountInfo {
    match engine::core::ely::current_account().await {
        Some(a) => AccountInfo {
            mode: if a.provider == "msa" {
                "msa".into()
            } else {
                "ely".into()
            },
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

// Текущий провайдер определяет, чем обновлять токен; без аккаунта — оффлайн
async fn live_account() -> Option<engine::core::settings::Account> {
    let provider = engine::core::settings::load()
        .account
        .map(|a| a.provider)
        .unwrap_or_default();
    if provider == "msa" {
        engine::core::msa::current_account().await
    } else {
        engine::core::ely::current_account().await
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
pub async fn msa_login_start() -> Result<engine::core::msa::DeviceStart, String> {
    engine::core::msa::device_start().await
}

#[tauri::command]
pub async fn msa_login_poll(device_code: String) -> Result<serde_json::Value, String> {
    match engine::core::msa::device_poll(&device_code).await? {
        engine::core::msa::Poll::Pending => Ok(json!({ "status": "pending" })),
        engine::core::msa::Poll::Done(acc) => {
            engine::core::settings::update(|s| s.account = Some(acc))?;
            Ok(json!({ "status": "done" }))
        }
        engine::core::msa::Poll::Error(e) => Ok(json!({ "status": "error", "message": e })),
    }
}

#[tauri::command]
pub async fn ely_logout() -> Result<(), String> {
    engine::core::ely::logout().await
}

// Скин аккаунта Microsoft через sessionserver Mojang, base64 png
#[tauri::command]
pub async fn msa_skin() -> Option<String> {
    let acc = live_account().await?;
    engine::core::msa::skin_png(&acc.uuid).await
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
