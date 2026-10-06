use crate::engine;

#[tauri::command]
pub async fn get_instance_config(
    version: String,
) -> Result<engine::game::config::InstanceConfig, String> {
    Ok(engine::game::config::load(&version).await)
}

#[tauri::command]
pub async fn save_instance_config(
    version: String,
    ram_mb: u32,
    jvm_args: Vec<String>,
) -> Result<(), String> {
    engine::game::config::save(&version, ram_mb, &jvm_args).await
}
