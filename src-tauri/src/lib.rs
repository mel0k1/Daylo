use tauri::Manager;

mod commands;
mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance регистрируется первым: повторный запуск лаунчера
        // просто поднимает существующее окно (два процесса писали бы settings.json
        // поверх друг друга)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::system::app_info,
            commands::update::update_check,
            commands::update::update_install,
            commands::pack::pack_export,
            commands::pack::pack_import,
            commands::screens::shots_list,
            commands::screens::shots_read,
            commands::screens::shots_delete,
            commands::screens::shots_open,
            commands::launch::list_versions,
            commands::launch::install_version,
            commands::launch::launch_game,
            commands::launch::launch_instance,
            commands::launch::stop_game,
            commands::launch::loader_builds,
            commands::launch::install_loader,
            commands::packs::pack_search,
            commands::packs::pack_versions,
            commands::packs::pack_install,
            commands::packs::instance_create,
            commands::packs::instance_list,
            commands::packs::instance_delete,
            commands::packs::curseforge_search,
            commands::packs::curseforge_versions,
            commands::packs::curseforge_install,
            commands::account::account_info,
            commands::account::ely_login_start,
            commands::account::ely_login_poll,
            commands::account::msa_login_start,
            commands::account::msa_login_poll,
            commands::account::msa_skin,
            commands::account::ely_logout,
            commands::account::ely_skin,
            commands::servers::ping_server,
            commands::servers::servers_list,
            commands::servers::servers_add,
            commands::servers::servers_remove,
            commands::servers::game_servers,
            commands::settings::get_launcher_settings,
            commands::settings::set_mirrors,
            commands::settings::set_cf_key,
            commands::settings::set_game_window,
            commands::settings::set_game_fullscreen,
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
            commands::skin::ely_upload_skin,
            commands::worlds::worlds_list,
            commands::worlds::world_backup_create,
            commands::worlds::world_backups_list,
            commands::worlds::world_backup_restore,
            commands::worlds::world_backup_delete,
            commands::worlds::world_backups_open,
            commands::ai::ai_build_plan,
        ])
        .setup(|_app| {
            // Временные файлы прошлых сессий: архивы сборок, .part, установщики
            tauri::async_runtime::spawn(engine::core::cleanup::run());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("не удалось запустить Daylo");
}
