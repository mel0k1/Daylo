use crate::engine;

#[tauri::command]
pub fn app_info() -> engine::core::AppInfo {
    engine::app_info()
}
