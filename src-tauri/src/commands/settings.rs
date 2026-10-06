use serde::Serialize;

use crate::engine;

#[derive(Serialize)]
pub struct LauncherSettings {
    pub use_mirrors: bool,
}

#[tauri::command]
pub async fn get_launcher_settings() -> LauncherSettings {
    let s = engine::core::settings::load();
    LauncherSettings {
        use_mirrors: s.use_mirrors,
    }
}

#[tauri::command]
pub async fn set_mirrors(enabled: bool) -> Result<(), String> {
    engine::core::settings::update(|s| s.use_mirrors = enabled)
}
