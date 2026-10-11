use crate::engine;

#[tauri::command]
pub async fn worlds_list(version: String) -> Vec<engine::game::backup::World> {
    engine::game::backup::worlds(&version).await
}

#[tauri::command]
pub async fn world_backup_create(
    version: String,
    world: String,
) -> Result<engine::game::backup::Backup, String> {
    engine::game::backup::create(&version, &world).await
}

#[tauri::command]
pub async fn world_backups_list(version: String) -> Vec<engine::game::backup::Backup> {
    engine::game::backup::list(&version).await
}

#[tauri::command]
pub async fn world_backup_restore(version: String, file: String) -> Result<(), String> {
    engine::game::backup::restore(&version, &file).await
}

#[tauri::command]
pub async fn world_backup_delete(version: String, file: String) -> Result<(), String> {
    engine::game::backup::delete(&version, &file).await
}

#[tauri::command]
pub async fn world_backups_open(version: String) -> Result<(), String> {
    engine::game::backup::open_dir(&version).await
}
