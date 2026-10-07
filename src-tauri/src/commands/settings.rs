use serde::Serialize;

use crate::engine;

#[derive(Serialize)]
pub struct LauncherSettings {
    pub use_mirrors: bool,
    pub cf_api_key: Option<String>,
}

#[tauri::command]
pub async fn get_launcher_settings() -> LauncherSettings {
    let s = engine::core::settings::load();
    LauncherSettings {
        use_mirrors: s.use_mirrors,
        cf_api_key: s.cf_api_key,
    }
}

#[tauri::command]
pub async fn set_mirrors(enabled: bool) -> Result<(), String> {
    engine::core::settings::update(|s| s.use_mirrors = enabled)
}

#[tauri::command]
pub async fn set_cf_key(key: String) -> Result<(), String> {
    let key = key.trim().to_string();
    engine::core::settings::update(|s| s.cf_api_key = if key.is_empty() { None } else { Some(key) })
}
