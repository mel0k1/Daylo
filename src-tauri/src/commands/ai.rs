use crate::engine;

#[tauri::command]
pub async fn ai_build_plan(
    prompt: String,
    game_version: String,
    loader: String,
) -> Result<Vec<engine::game::aibuild::AiPick>, String> {
    engine::game::aibuild::plan(&prompt, &game_version, &loader).await
}
