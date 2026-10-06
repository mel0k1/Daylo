mod commands;
mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::system::app_info,
            commands::update::update_check,
            commands::update::update_install,
            commands::launch::list_versions,
            commands::launch::install_version,
            commands::launch::launch_game,
            commands::launch::stop_game,
            commands::launch::loader_builds,
            commands::launch::install_loader,
            commands::account::account_info,
            commands::account::ely_login_start,
            commands::account::ely_login_poll,
            commands::account::ely_logout,
            commands::account::ely_skin,
            commands::servers::ping_server,
            commands::servers::servers_list,
            commands::servers::servers_add,
            commands::servers::servers_remove,
            commands::settings::get_launcher_settings,
            commands::settings::set_mirrors,
            commands::config::get_instance_config,
            commands::config::save_instance_config,
            commands::mods::modrinth_search,
            commands::mods::modrinth_versions,
            commands::mods::modrinth_install,
            commands::mods::mods_list,
            commands::mods::mods_delete,
            commands::mods::mods_updates,
            commands::mods::mods_update,
            commands::skin::skin_save,
            commands::skin::skin_load,
            commands::skin::skin_delete,
        ])
        .run(tauri::generate_context!())
        .expect("не удалось запустить Daylo");
}
