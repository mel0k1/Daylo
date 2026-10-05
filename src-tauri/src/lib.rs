mod commands;
mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::system::app_info])
        .run(tauri::generate_context!())
        .expect("не удалось запустить Daylo");
}
