use serde::Serialize;
use serde_json::json;

use crate::engine;
use crate::engine::game::launch::{offline_uuid, ok_nick};

#[derive(Serialize)]
pub struct AccountInfo {
    // "ely" | "msa" | "offline"
    pub mode: String,
    pub name: String,
    pub uuid: String,
}

// Запись в списке аккаунтов — без токенов
#[derive(Serialize)]
pub struct AccountBrief {
    // "ely" | "msa" | "offline"
    pub mode: String,
    pub name: String,
    pub uuid: String,
    pub active: bool,
}

fn mode_of(provider: &str) -> String {
    match provider {
        "msa" => "msa".into(),
        "offline" => "offline".into(),
        _ => "ely".into(),
    }
}

// Активный аккаунт: онлайн-провайдеры обновляют токен, оффлайн — как есть.
// Пустое имя при оффлайне значит «аккаунтов нет вообще»
#[tauri::command]
pub async fn account_info() -> AccountInfo {
    let saved = engine::core::settings::active();
    let acc = match saved.as_ref().map(|a| a.provider.as_str()) {
        Some("msa") => engine::core::msa::current_account().await,
        Some("ely") => engine::core::ely::current_account().await,
        _ => saved,
    };
    match acc {
        Some(a) => AccountInfo {
            mode: mode_of(&a.provider),
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

// Все сохранённые аккаунты: ely, msa и оффлайн-профили
#[tauri::command]
pub async fn account_list() -> Vec<AccountBrief> {
    let s = engine::core::settings::load();
    s.accounts
        .iter()
        .map(|a| AccountBrief {
            mode: mode_of(&a.provider),
            name: a.name.clone(),
            uuid: a.uuid.clone(),
            active: s.active_account.as_deref() == Some(a.uuid.as_str()),
        })
        .collect()
}

// Сменить активный аккаунт
#[tauri::command]
pub async fn account_switch(uuid: String) -> Result<(), String> {
    let uuid = uuid.trim().to_string();
    engine::core::settings::set_active(&uuid)
}

// Оффлайн-профиль: без токенов, uuid выводится из ника, как в ваниле.
// Повторное добавление того же ника обновляет существующую запись
#[tauri::command]
pub async fn account_add_offline(nick: String) -> Result<AccountBrief, String> {
    let nick = nick.trim().to_string();
    if !ok_nick(&nick) {
        return Err("ник: только латиница, цифры и подчёркивания, 1–16 символов".into());
    }
    let uuid = offline_uuid(&nick);
    engine::core::settings::update(|s| {
        s.upsert_account(engine::core::settings::Account {
            name: nick.clone(),
            uuid: uuid.clone(),
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at: 0,
            provider: "offline".into(),
        })
    })?;
    Ok(AccountBrief {
        mode: "offline".into(),
        name: nick,
        uuid,
        active: true,
    })
}

// Убрать аккаунт из списка; если он был активным — активен первый из оставшихся
#[tauri::command]
pub async fn account_logout(uuid: String) -> Result<(), String> {
    let uuid = uuid.trim().to_string();
    engine::core::settings::remove_account(&uuid)
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
            engine::core::settings::update(|s| s.upsert_account(acc))?;
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
            engine::core::settings::update(|s| s.upsert_account(acc))?;
            Ok(json!({ "status": "done" }))
        }
        engine::core::msa::Poll::Error(e) => Ok(json!({ "status": "error", "message": e })),
    }
}

// Текущий провайдер определяет, чем обновлять токен; скины — только онлайн-аккаунты
async fn live_account() -> Option<engine::core::settings::Account> {
    match engine::core::settings::active()?.provider.as_str() {
        "msa" => engine::core::msa::current_account().await,
        "ely" => engine::core::ely::current_account().await,
        _ => None,
    }
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
