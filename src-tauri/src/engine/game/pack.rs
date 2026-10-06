use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::super::core::paths;
use super::config;
use super::install;
use super::modrinth::{self, ModMeta};

pub const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct PackMod {
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub file: String,
}

// Файл-сборка: одного json хватает, моды докачаются с Modrinth
#[derive(Serialize, Deserialize, Clone)]
pub struct Pack {
    pub format: u32,
    pub name: String,
    pub base: String,
    pub ram_mb: u32,
    pub jvm_args: Vec<String>,
    pub mods: Vec<PackMod>,
}

#[derive(Clone, Serialize)]
pub struct PackProgress {
    pub stage: String,
    pub done: u64,
    pub total: u64,
    pub error: Option<String>,
}

fn fail(app: &tauri::AppHandle, id: &str, e: String) {
    let _ = app.emit(
        "pack-progress",
        PackProgress {
            stage: "ошибка".into(),
            done: 0,
            total: 1,
            error: Some(e.clone()),
        },
    );
    let _ = app.emit("pack-imported", serde_json::json!({ "id": id, "error": e }));
}

// Манифест установленной сборки
pub async fn build(version: &str) -> Result<Pack, String> {
    let cfg = config::load(version).await;
    let base = install::version_json(version)
        .await?
        .inherits_from
        .unwrap_or_else(|| version.to_string());
    let mods = modrinth::tracked(version)
        .await
        .into_iter()
        .map(|m: ModMeta| PackMod {
            project_id: m.project_id,
            version_id: m.version_id,
            version_number: m.version_number,
            file: m.file,
        })
        .collect();
    Ok(Pack {
        format: FORMAT,
        name: version.to_string(),
        base,
        ram_mb: cfg.ram_mb,
        jvm_args: cfg.jvm_args,
        mods,
    })
}

// Диалог сохранения, потом запись json
pub async fn export(app: &tauri::AppHandle, version: &str) -> Result<Option<PathBuf>, String> {
    let pack = build(version).await?;
    let Some(path) = pick_save(app, &format!("daylo-{version}.pack.json")).await? else {
        return Ok(None);
    };
    let body =
        serde_json::to_string_pretty(&pack).map_err(|e| format!("сериализация сборки: {e}"))?;
    tokio::fs::write(&path, body)
        .await
        .map_err(|e| format!("запись файла: {e}"))?;
    Ok(Some(path))
}

// Диалог выбора файла, создание сборки и установка содержимого в фоне
pub async fn import(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let Some(path) = pick_open(app).await? else {
        return Ok(None);
    };
    let id = create_from(app, &path).await?;
    Ok(Some(id))
}

async fn create_from(app: &tauri::AppHandle, path: &Path) -> Result<String, String> {
    let body = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("чтение файла: {e}"))?;
    let pack: Pack =
        serde_json::from_str(&body).map_err(|_| "файл не похож на сборку Daylo".to_string())?;
    if pack.format != FORMAT {
        return Err(format!("формат сборки {} не поддерживается", pack.format));
    }
    if pack.base.is_empty() {
        return Err("в сборке не указана версия игры".into());
    }
    let id = unique_id(&pack.name);
    config::save(&id, pack.ram_mb, &pack.jvm_args).await?;
    modrinth::seed_meta(
        &id,
        &pack
            .mods
            .iter()
            .map(|m| ModMeta {
                file: m.file.clone(),
                project_id: m.project_id.clone(),
                version_id: m.version_id.clone(),
                version_number: m.version_number.clone(),
            })
            .collect::<Vec<_>>(),
    )
    .await?;

    let app2 = app.clone();
    let total = pack.mods.len() as u64;
    tauri::async_runtime::spawn(async move {
        match install::ensure_version(app2.clone(), &pack.base).await {
            Ok(_) => {
                let stage = String::from("моды");
                for (i, m) in pack.mods.iter().enumerate() {
                    let _ = app2.emit(
                        "pack-progress",
                        PackProgress {
                            stage: stage.clone(),
                            done: i as u64,
                            total,
                            error: None,
                        },
                    );
                    if let Err(e) = modrinth::install(&pack.name, &m.version_id).await {
                        // Неудачный мод не роняет сборку целиком
                        eprintln!("мод {}: {e}", m.file);
                    }
                }
                let _ = app2.emit(
                    "pack-progress",
                    PackProgress {
                        stage: "готово".into(),
                        done: total,
                        total,
                        error: None,
                    },
                );
                let _ = app2.emit(
                    "pack-imported",
                    serde_json::json!({ "id": pack.name, "error": null }),
                );
            }
            Err(e) => fail(&app2, &pack.name, e),
        }
    });
    Ok(id)
}

// Имя сборки без коллизий: суффиксы -2, -3…
fn unique_id(name: &str) -> String {
    let base = name.trim();
    let start = if base.is_empty() { "imported" } else { base };
    let mut id = start.to_string();
    let mut i = 2;
    while paths::instance_dir(&id).exists() {
        id = format!("{start}-{i}");
        i += 1;
    }
    id
}

async fn pick_save(app: &tauri::AppHandle, name: &str) -> Result<Option<PathBuf>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Сборка Daylo", &["json"])
        .set_file_name(name)
        .save_file(move |p| {
            let _ = tx.send(p.and_then(|f| match f {
                FilePath::Path(p) => Some(p),
                _ => None,
            }));
        });
    rx.await.map_err(|e| e.to_string())
}

async fn pick_open(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Сборка Daylo", &["json"])
        .pick_file(move |p| {
            let _ = tx.send(p.and_then(|f| match f {
                FilePath::Path(p) => Some(p),
                _ => None,
            }));
        });
    rx.await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_ids_do_not_collide() {
        // instance_dir вычисляется от данных пользователя; проверяем сам алгоритм
        let name = "my-pack";
        assert_eq!(unique_id_inner(name, &[]), "my-pack");
        assert_eq!(unique_id_inner(name, &["my-pack"]), "my-pack-2");
        assert_eq!(
            unique_id_inner(name, &["my-pack", "my-pack-2"]),
            "my-pack-3"
        );
        assert_eq!(unique_id_inner("", &[]), "imported");
    }

    fn unique_id_inner(name: &str, taken: &[&str]) -> String {
        let base = name.trim();
        let start = if base.is_empty() { "imported" } else { base };
        let mut id = start.to_string();
        let mut i = 2;
        while taken.contains(&id.as_str()) {
            id = format!("{start}-{i}");
            i += 1;
        }
        id
    }

    #[test]
    fn pack_json_roundtrip() {
        let pack = Pack {
            format: FORMAT,
            name: "1.21.4".into(),
            base: "1.21.4".into(),
            ram_mb: 4096,
            jvm_args: vec!["-XX:+UseG1GC".into()],
            mods: vec![PackMod {
                project_id: "AANobbMI".into(),
                version_id: "v1".into(),
                version_number: "0.6.0".into(),
                file: "sodium.jar".into(),
            }],
        };
        let body = serde_json::to_string(&pack).unwrap();
        let back: Pack = serde_json::from_str(&body).unwrap();
        assert_eq!(back.base, "1.21.4");
        assert_eq!(back.ram_mb, 4096);
        assert_eq!(back.mods[0].project_id, "AANobbMI");
    }
}
