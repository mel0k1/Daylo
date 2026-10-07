use serde::Serialize;

use crate::engine;

// --- Каталог готовых сборок: Modrinth, CurseForge, FTB ---

#[tauri::command]
pub async fn pack_search(
    source: String,
    query: String,
    game_version: String,
) -> Result<Vec<engine::game::modpack::PackHit>, String> {
    engine::game::modpack::search(&source, &query, &game_version).await
}

#[tauri::command]
pub async fn pack_versions(
    source: String,
    pack_id: String,
    game_version: String,
) -> Result<Vec<engine::game::modpack::PackVersionInfo>, String> {
    engine::game::modpack::versions(&source, &pack_id, &game_version).await
}

// Установка идёт в фоне: прогресс — pack-progress, итог — pack-installed
#[tauri::command]
pub async fn pack_install(
    app: tauri::AppHandle,
    source: String,
    pack_id: String,
    version_id: String,
    icon: String,
) -> Result<(), String> {
    engine::game::modpack::install(app, &source, &pack_id, &version_id, &icon).await
}

// --- Установленные сборки ---

#[derive(Serialize)]
pub struct InstanceInfo {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub launch_version: String,
    pub source: String,
    pub pack_version: String,
    pub icon_url: String,
    pub ram_mb: u32,
    pub mods_count: usize,
}

#[tauri::command]
pub async fn instance_list() -> Vec<InstanceInfo> {
    let mut out = Vec::new();
    let root = engine::core::paths::game_root().join("instances");
    let Ok(mut rd) = tokio::fs::read_dir(&root).await else {
        return out;
    };
    let mut dirs: Vec<String> = Vec::new();
    while let Ok(Some(e)) = rd.next_entry().await {
        if e.path().is_dir() {
            if let Some(name) = e.file_name().to_str() {
                dirs.push(name.to_string());
            }
        }
    }
    dirs.sort();
    let profiles = engine::game::loaders::local_profiles().await;
    for id in dirs {
        let cfg = engine::game::config::load(&id).await;
        let launch_version = if cfg.launch_version.is_empty() {
            id.clone()
        } else {
            cfg.launch_version.clone()
        };
        let loader = engine::game::loaders::loader_tag(&launch_version);
        let mc_version = if cfg.mc_version.is_empty() {
            profiles
                .iter()
                .find(|(pid, _, _)| *pid == launch_version)
                .map(|(_, _, base)| base.clone())
                .filter(|b| !b.is_empty())
                .unwrap_or_else(|| id.clone())
        } else {
            cfg.mc_version.clone()
        };
        let mods_count = engine::game::modrinth::installed(&id).await.len();
        let name = if cfg.pack_name.is_empty() {
            id.clone()
        } else {
            cfg.pack_name.clone()
        };
        out.push(InstanceInfo {
            id,
            kind: if cfg.kind.is_empty() { "vanilla".into() } else { cfg.kind },
            name,
            mc_version,
            loader: loader.to_string(),
            launch_version,
            source: cfg.source,
            pack_version: cfg.pack_version,
            icon_url: cfg.icon_url,
            ram_mb: cfg.ram_mb,
            mods_count,
        });
    }
    out
}

#[tauri::command]
pub async fn instance_delete(id: String) -> Result<(), String> {
    let dir = engine::core::paths::instance_dir(&id);
    // daylo.json может не быть у ванильных сборок, созданных запуском
    if !dir.is_dir() {
        return Err("сборка не найдена".into());
    }
    tokio::fs::remove_dir_all(&dir)
        .await
        .map_err(|e| format!("удаление: {e}"))
}

// --- Моды CurseForge ---

#[tauri::command]
pub async fn curseforge_search(
    query: String,
    game_version: String,
) -> Result<Vec<engine::game::modrinth::SearchHit>, String> {
    engine::game::curseforge::search_mods(&query, &game_version).await
}

#[tauri::command]
pub async fn curseforge_versions(
    mod_id: String,
    game_version: String,
) -> Result<Vec<engine::game::modrinth::ModVersion>, String> {
    let files = engine::game::curseforge::files(&mod_id, &game_version).await?;
    Ok(files
        .into_iter()
        .take(40)
        .map(|f| engine::game::modrinth::ModVersion {
            version_id: f.file_id,
            version_number: f.display,
            file_name: f.file_name,
            date: f.date,
            size: f.size,
            loaders: vec!["curseforge".into()],
        })
        .collect())
}

#[tauri::command]
pub async fn curseforge_install(
    version: String,
    mod_id: String,
    file_id: String,
) -> Result<(), String> {
    engine::game::curseforge::install_mod(&version, &mod_id, &file_id).await
}
