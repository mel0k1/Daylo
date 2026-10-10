use serde::Serialize;
use tauri::Emitter;

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
            kind: if cfg.kind.is_empty() {
                "vanilla".into()
            } else {
                cfg.kind
            },
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

// --- Своя сборка: папка + daylo.json, ванила или загрузчик ---

#[tauri::command]
pub async fn instance_create(
    app: tauri::AppHandle,
    name: String,
    mc_version: String,
    loader: String,
    build: String,
) -> Result<String, String> {
    let mc_version = mc_version.trim().to_string();
    if mc_version.is_empty() {
        return Err("выберите версию игры".into());
    }
    // Имя — безопасное и уникальное, как у импортированных сборок
    let mut id = engine::core::paths::safe_name(&name);
    if id == "unknown" {
        id = "instance".into();
    }
    let base = id.clone();
    let mut i = 2;
    while engine::core::paths::instance_dir(&id).exists() {
        id = format!("{base}-{i}");
        i += 1;
    }
    let loader = loader.trim().to_lowercase();
    let custom = loader.is_empty() || loader == "vanilla";
    if !custom && build.trim().is_empty() {
        return Err("выберите сборку загрузчика".into());
    }

    // Долго: ванила качает игру, загрузчик — инсталлер. Прогресс — install-progress,
    // итог — pack-installed, как у установки из каталога
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let id = task_id;
        let built = if custom {
            engine::game::install::ensure_version(app.clone(), &mc_version)
                .await
                .map(|_| String::new())
        } else {
            engine::game::loaders::install(app.clone(), &loader, &mc_version, build.trim()).await
        };
        match built {
            Ok(profile) => {
                let cfg = engine::game::config::InstanceConfig {
                    kind: "custom".into(),
                    source: "custom".into(),
                    pack_name: name,
                    mc_version: mc_version.clone(),
                    // ванила запускается своей версией, загрузчик — профилем
                    launch_version: if profile.is_empty() {
                        mc_version
                    } else {
                        profile
                    },
                    ..Default::default()
                };
                let result = engine::game::config::write(&id, &cfg).await;
                let _ = app.emit(
                    "pack-installed",
                    serde_json::json!({ "instance": id, "error": result.err() }),
                );
            }
            Err(e) => {
                let _ = app.emit(
                    "pack-installed",
                    serde_json::json!({ "instance": id, "error": e }),
                );
            }
        }
    });
    Ok(id)
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
