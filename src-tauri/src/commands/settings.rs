use serde::Serialize;

use crate::engine;

#[derive(Serialize)]
pub struct LauncherSettings {
    pub use_mirrors: bool,
    pub cf_api_key: Option<String>,
    pub game_width: u32,
    pub game_height: u32,
    pub game_fullscreen: bool,
    pub backup_on_launch: bool,
    pub backup_keep: u32,
}

#[tauri::command]
pub async fn get_launcher_settings() -> LauncherSettings {
    let s = engine::core::settings::load();
    LauncherSettings {
        use_mirrors: s.use_mirrors,
        cf_api_key: s.cf_api_key,
        game_width: s.game_width,
        game_height: s.game_height,
        game_fullscreen: s.game_fullscreen,
        backup_on_launch: s.backup_on_launch,
        backup_keep: s.backup_keep,
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

#[tauri::command]
pub async fn set_game_window(width: u32, height: u32) -> Result<(), String> {
    // Клампит и на фронте, и здесь — файл настроек пишет только бэкенд
    let (width, height) = engine::core::settings::clamp_window(width, height);
    engine::core::settings::update(move |s| {
        s.game_width = width;
        s.game_height = height;
    })
}

#[tauri::command]
pub async fn set_game_fullscreen(on: bool) -> Result<(), String> {
    engine::core::settings::update(move |s| s.game_fullscreen = on)
}

// Бэкапы миров: автобэкап перед запуском и ретенция копий на мир
#[tauri::command]
pub async fn set_backups(on: bool, keep: u32) -> Result<(), String> {
    let keep = keep.clamp(1, engine::core::settings::MAX_BACKUP_KEEP);
    engine::core::settings::update(move |s| {
        s.backup_on_launch = on;
        s.backup_keep = keep;
    })
}
