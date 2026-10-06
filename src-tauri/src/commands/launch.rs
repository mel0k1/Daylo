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
            });
        }
    }
    Ok(list)
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
    engine::game::launch::launch(app, &version, nick).await
}

#[tauri::command]
pub async fn stop_game(version: String) -> Result<(), String> {
    engine::game::launch::stop(&version).await
}
