mod commands;
mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::system::app_info,
            commands::launch::list_versions,
            commands::launch::install_version,
            commands::launch::launch_game,
            commands::launch::stop_game,
            commands::config::get_instance_config,
            commands::config::save_instance_config,
            commands::mods::modrinth_search,
            commands::mods::modrinth_versions,
            commands::mods::modrinth_install,
            commands::mods::mods_list,
            commands::mods::mods_delete,
        ])
        .run(tauri::generate_context!())
        .expect("не удалось запустить Daylo");
}
