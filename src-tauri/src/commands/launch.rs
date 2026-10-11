use std::collections::HashSet;

use crate::engine;
use tauri::Emitter;

#[tauri::command]
pub async fn list_versions() -> Result<Vec<engine::game::mcmeta::VersionEntry>, String> {
    let mut list = engine::game::mcmeta::list().await?;
    // Профили загрузчиков с диска дописываются к манифесту Mojang
    for (id, loader, base) in engine::game::loaders::local_profiles().await {
        if !list.iter().any(|v| v.id == id) {
            list.push(engine::game::mcmeta::VersionEntry {
                id,
                kind: loader.clone(),
                release_time: String::new(),
                loader,
                base,
                // Профиль попадает в список только если его json существует
                installed: true,
            });
        }
    }
    // Скачанные ванильные версии: на диске лежит клиент-jar
    let jars = installed_jars();
    mark_installed(&mut list, &jars);
    Ok(list)
}

// Идентификаторы версий с клиентом на диске: versions/<id>/<id>.jar
fn installed_jars() -> HashSet<String> {
    let mut out = HashSet::new();
    let Ok(rd) = std::fs::read_dir(engine::core::paths::versions_dir()) else {
        return out;
    };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if path.is_dir() && path.join(format!("{name}.jar")).is_file() {
            out.insert(name);
        }
    }
    out
}

// Отметить скачанные ванильные версии; профили загрузчиков приходят только с
// диска и помечаются при добавлении в список
fn mark_installed(list: &mut [engine::game::mcmeta::VersionEntry], jars: &HashSet<String>) {
    for v in list.iter_mut() {
        if v.loader.is_empty() && jars.contains(&v.id) {
            v.installed = true;
        }
    }
}

#[tauri::command]
pub async fn loader_builds(
    loader: String,
    version: String,
) -> Result<Vec<engine::game::loaders::LoaderBuild>, String> {
    engine::game::loaders::list_builds(&loader, &version).await
}

#[tauri::command]
pub async fn install_loader(
    app: tauri::AppHandle,
    loader: String,
    version: String,
    build: String,
) -> Result<(), String> {
    // Долго: инсталлер качает библиотеки. Прогресс — install-progress, итог — loader-installed
    tauri::async_runtime::spawn(async move {
        match engine::game::loaders::install(app.clone(), &loader, &version, &build).await {
            Ok(id) => {
                let _ = app.emit(
                    "loader-installed",
                    serde_json::json!({ "id": id, "base": version }),
                );
            }
            Err(e) => {
                let _ = app.emit(
                    "install-progress",
                    engine::game::install::InstallProgress {
                        version,
                        stage: "ошибка".into(),
                        done: 0,
                        total: 1,
                        error: Some(e),
                    },
                );
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn install_version(app: tauri::AppHandle, version: String) -> Result<(), String> {
    // Прогресс и ошибка уходят событием install-progress
    tauri::async_runtime::spawn(async move {
        let _ = engine::game::install::ensure_version(app, &version).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn launch_game(
    app: tauri::AppHandle,
    version: String,
    nick: String,
) -> Result<(), String> {
    let nick = nick.trim();
    if nick.is_empty() {
        return Err("введите ник".into());
    }
    // Ванильная сборка: папка совпадает с версией
    engine::game::launch::launch(app, &version, nick, &version).await
}

#[tauri::command]
pub async fn launch_instance(
    app: tauri::AppHandle,
    instance: String,
    nick: String,
) -> Result<(), String> {
    let nick = nick.trim();
    if nick.is_empty() {
        return Err("введите ник".into());
    }
    // У модпаков в daylo.json записана версия, которой нужно запускать
    let cfg = engine::game::config::load(&instance).await;
    let version = if cfg.launch_version.is_empty() {
        instance.clone()
    } else {
        cfg.launch_version
    };
    engine::game::launch::launch(app, &version, nick, &instance).await
}

#[tauri::command]
pub async fn stop_game(version: String) -> Result<(), String> {
    engine::game::launch::stop(&version).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, loader: &str) -> engine::game::mcmeta::VersionEntry {
        engine::game::mcmeta::VersionEntry {
            id: id.into(),
            kind: if loader.is_empty() {
                "release".into()
            } else {
                loader.into()
            },
            release_time: String::new(),
            loader: loader.into(),
            base: String::new(),
            installed: false,
        }
    }

    #[test]
    fn installed_marks_only_vanilla_with_jar() {
        let mut list = vec![
            entry("1.21.10", ""),
            entry("1.21.9", ""),
            entry("fabric-loader-0.16.9", "fabric"),
        ];
        let jars: HashSet<String> = ["1.21.10".to_string()].into();
        mark_installed(&mut list, &jars);
        assert!(list[0].installed);
        assert!(!list[1].installed);
        // профили загрузчиков помечаются при добавлении из local_profiles,
        // mark_installed их не трогает
        assert!(!list[2].installed);
    }
}
