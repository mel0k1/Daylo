use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use serde::Serialize;
use tauri::Emitter;

use super::super::core::{http, paths};
use super::config;
use super::curseforge;
use super::ftb;
use super::install;
use super::loaders;
use super::modrinth;
use super::pack;

pub const SOURCE_MR: &str = "modrinth";
pub const SOURCE_CF: &str = "curseforge";
pub const SOURCE_FTB: &str = "ftb";

#[derive(Serialize, Clone, serde::Deserialize)]
pub struct PackHit {
    pub source: String,
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: String,
    pub downloads: u64,
}

#[derive(Serialize, Clone)]
pub struct PackVersionInfo {
    pub version_id: String,
    pub name: String,
    pub date: String,
    pub mc_versions: Vec<String>,
    pub loaders: Vec<String>,
}

// План установки: что и куда качать
pub struct PackPlan {
    pub instance: String,
    pub name: String,
    pub mc_version: String,
    pub loader: Option<(String, String)>,
    pub icon_url: String,
    pub source: String,
    pub pack_id: String,
    pub pack_version: String,
    pub ram_mb: u32,
    pub archive: PathBuf,
    pub files: Vec<PlannedFile>,
}

#[derive(Clone)]
pub struct PlannedFile {
    pub url: String,
    pub path: String, // относительный путь внутри папки сборки
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

// --- Поиск и версии ---

pub async fn search(source: &str, query: &str, game_version: &str) -> Result<Vec<PackHit>, String> {
    match source {
        SOURCE_MR => modrinth_packs(query, game_version).await,
        SOURCE_CF => curseforge_packs(query, game_version).await,
        SOURCE_FTB => ftb::search(query).await,
        other => Err(format!("неизвестный источник {other}")),
    }
}

async fn modrinth_packs(query: &str, game_version: &str) -> Result<Vec<PackHit>, String> {
    let hits = modrinth::search_type(query, game_version, "modpack").await?;
    Ok(hits
        .into_iter()
        .map(|h| PackHit {
            source: SOURCE_MR.into(),
            id: h.project_id,
            slug: h.slug,
            title: h.title,
            description: h.description,
            icon_url: h.icon_url,
            downloads: h.downloads,
        })
        .collect())
}

async fn curseforge_packs(query: &str, game_version: &str) -> Result<Vec<PackHit>, String> {
    let hits = curseforge::search_packs(query, game_version).await?;
    Ok(hits
        .into_iter()
        .map(|h| PackHit {
            source: SOURCE_CF.into(),
            id: h.project_id,
            slug: h.slug,
            title: h.title,
            description: h.description,
            icon_url: h.icon_url,
            downloads: h.downloads,
        })
        .collect())
}

pub async fn versions(
    source: &str,
    pack_id: &str,
    game_version: &str,
) -> Result<Vec<PackVersionInfo>, String> {
    match source {
        SOURCE_MR => {
            let raw = modrinth::pack_versions(pack_id).await?;
            Ok(raw
                .into_iter()
                .filter(|v| {
                    game_version.is_empty() || v.game_versions.iter().any(|g| g == game_version)
                })
                .take(40)
                .map(|v| PackVersionInfo {
                    version_id: v.id,
                    name: v.version_number,
                    date: v.date_published,
                    mc_versions: v.game_versions,
                    loaders: v.loaders,
                })
                .collect())
        }
        SOURCE_CF => {
            let files = curseforge::files(pack_id, game_version).await?;
            Ok(files
                .into_iter()
                .take(40)
                .map(|f| PackVersionInfo {
                    version_id: f.file_id,
                    name: f.display,
                    date: f.date,
                    mc_versions: vec![],
                    loaders: vec![],
                })
                .collect())
        }
        SOURCE_FTB => ftb::versions(pack_id).await,
        other => Err(format!("неизвестный источник {other}")),
    }
}

// --- Разбор форматов ---

// modrinth.index.json из .mrpack
fn parse_mrpack(index: &str, name: &str, icon: &str) -> Result<PackPlan, String> {
    let v: serde_json::Value =
        serde_json::from_str(index).map_err(|e| format!("разбор modrinth.index.json: {e}"))?;
    if v["game"].as_str() != Some("minecraft") {
        return Err("это не сборка Minecraft".into());
    }
    let deps = &v["dependencies"];
    let mc = deps["minecraft"].as_str().unwrap_or_default().to_string();
    if mc.is_empty() {
        return Err("в сборке не указана версия Minecraft".into());
    }
    let loader = dep_loader(deps, "fabric-loader", "fabric")
        .or_else(|| dep_loader(deps, "quilt-loader", "quilt"))
        .or_else(|| dep_loader(deps, "forge", "forge"))
        .or_else(|| dep_loader(deps, "neoforge", "neoforge"));

    let mut files = Vec::new();
    for f in v["files"].as_array().cloned().unwrap_or_default() {
        if f["env"]["client"].as_str() == Some("unsupported") {
            continue;
        }
        let path = f["path"].as_str().unwrap_or_default().to_string();
        let url = f["downloads"]
            .as_array()
            .and_then(|d| d.first())
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .to_string();
        if url.is_empty() || path.is_empty() {
            continue;
        }
        files.push(PlannedFile {
            url,
            path,
            sha1: f["hashes"]["sha1"].as_str().map(String::from),
            size: f["fileSize"].as_u64().filter(|s| *s > 0),
        });
    }
    Ok(PackPlan {
        instance: String::new(),
        name: v["name"].as_str().unwrap_or(name).to_string(),
        mc_version: mc,
        loader,
        icon_url: icon.to_string(),
        source: SOURCE_MR.into(),
        pack_id: String::new(),
        pack_version: v["versionId"].as_str().unwrap_or("1").to_string(),
        ram_mb: 4096,
        archive: PathBuf::new(),
        files,
    })
}

fn dep_loader(deps: &serde_json::Value, key: &str, tag: &str) -> Option<(String, String)> {
    deps[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|b| (tag.to_string(), b.to_string()))
}

// manifest.json из zip CurseForge; отдельный список id файлов — их url узнаём пачкой
fn parse_cf_manifest(
    manifest: &str,
    icon: &str,
) -> Result<(PackPlan, Vec<(String, String)>), String> {
    let v: serde_json::Value =
        serde_json::from_str(manifest).map_err(|e| format!("разбор manifest.json: {e}"))?;
    let mc = v["minecraft"]["version"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if mc.is_empty() {
        return Err("в сборке не указана версия Minecraft".into());
    }
    let loader = v["minecraft"]["modLoaders"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .find(|l| l["primary"].as_bool().unwrap_or(false))
        .or_else(|| {
            v["minecraft"]["modLoaders"]
                .as_array()
                .and_then(|a| a.first())
        })
        .and_then(|l| l["id"].as_str())
        .and_then(|id| id.split_once('-'))
        .map(|(tag, build)| (tag.to_lowercase(), build.to_string()));
    if let Some((tag, _)) = &loader {
        if !matches!(tag.as_str(), "fabric" | "forge" | "neoforge" | "quilt") {
            return Err(format!("загрузчик {tag} не поддерживается"));
        }
    }
    let ids: Vec<(String, String)> = v["files"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|f| f["required"].as_bool().unwrap_or(true))
        .filter_map(|f| {
            let p = f["projectID"].as_i64()?;
            let fid = f["fileID"].as_i64()?;
            Some((p.to_string(), fid.to_string()))
        })
        .collect();
    let plan = PackPlan {
        instance: String::new(),
        name: v["name"]
            .as_str()
            .unwrap_or("CurseForge сборка")
            .to_string(),
        mc_version: mc,
        loader,
        icon_url: icon.to_string(),
        source: SOURCE_CF.into(),
        pack_id: String::new(),
        pack_version: v["version"].as_str().unwrap_or("1").to_string(),
        ram_mb: 4096,
        archive: PathBuf::new(),
        files: Vec::new(),
    };
    Ok((plan, ids))
}

// --- Установка ---

fn emit(app: &tauri::AppHandle, stage: &str, done: u64, total: u64) {
    let _ = app.emit(
        "pack-progress",
        pack::PackProgress {
            stage: stage.into(),
            done,
            total,
            error: None,
        },
    );
}

// Уникальное имя папки сборки: суффиксы -2, -3…
pub fn unique_id(name: &str) -> String {
    let base = name.trim();
    let start = if base.is_empty() { "pack" } else { base };
    let mut id = start.to_string();
    let mut i = 2;
    while paths::instance_dir(&id).exists() {
        id = format!("{start}-{i}");
        i += 1;
    }
    id
}

// Читает файл из архива целиком
fn zip_entry(archive: &Path, name: &str) -> Result<Option<String>, String> {
    let f = std::fs::File::open(archive).map_err(|e| format!("открытие архива: {e}"))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| format!("архив сборки: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.name() == name {
            let mut body = String::new();
            std::io::Read::read_to_string(&mut entry, &mut body)
                .map_err(|e| format!("чтение {name}: {e}"))?;
            return Ok(Some(body));
        }
    }
    Ok(None)
}

// Распаковывает содержимое папки prefix из архива в dest
fn extract_prefix(archive: &Path, prefix: &str, dest: &Path) -> Result<usize, String> {
    let f = std::fs::File::open(archive).map_err(|e| format!("открытие архива: {e}"))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| format!("архив сборки: {e}"))?;
    let mut n = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let Some(rel) = name.strip_prefix(prefix).map(|r| r.trim_start_matches('/')) else {
            continue;
        };
        if rel.is_empty() || entry.is_dir() {
            continue;
        }
        let out = dest.join(rel);
        if !out.starts_with(dest) {
            continue; // защита от обхода путей
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut w).map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

// Точка входа: возвращает сразу, установка идёт в фоне с pack-progress
pub async fn install(
    app: tauri::AppHandle,
    source: &str,
    pack_id: &str,
    version_id: &str,
    icon: &str,
) -> Result<(), String> {
    let source = source.to_string();
    let pack_id = pack_id.to_string();
    let version_id = version_id.to_string();
    let icon = icon.to_string();
    tauri::async_runtime::spawn(async move {
        let outcome: Result<String, (String, String)> = match source.as_str() {
            SOURCE_MR => run_modrinth(app.clone(), &pack_id, &version_id, &icon).await,
            SOURCE_CF => run_curseforge(app.clone(), &pack_id, &version_id, &icon).await,
            SOURCE_FTB => run_ftb(app.clone(), &pack_id, &version_id).await,
            other => Err((String::new(), format!("неизвестный источник {other}"))),
        };
        match outcome {
            Ok(instance) => {
                emit(&app, "готово", 1, 1);
                let _ = app.emit(
                    "pack-installed",
                    serde_json::json!({ "instance": instance, "error": null }),
                );
            }
            Err((instance, e)) => {
                let _ = app.emit(
                    "pack-progress",
                    pack::PackProgress {
                        stage: "ошибка".into(),
                        done: 0,
                        total: 1,
                        error: Some(e.clone()),
                    },
                );
                // Сборка без метаданных — обломки установки убираем
                if !instance.is_empty() {
                    let dir = paths::instance_dir(&instance);
                    if !dir.join("daylo.json").exists() {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                    }
                }
                let _ = app.emit(
                    "pack-installed",
                    serde_json::json!({ "instance": instance, "error": e }),
                );
            }
        }
    });
    Ok(())
}

// Ранний сбой: сборка ещё не создана
fn early(e: String) -> (String, String) {
    (String::new(), e)
}

// Сбой после создания папки: вернуть её имя для зачистки
fn late(instance: &str, e: String) -> (String, String) {
    (instance.to_string(), e)
}

async fn run_modrinth(
    app: tauri::AppHandle,
    pack_id: &str,
    version_id: &str,
    icon: &str,
) -> Result<String, (String, String)> {
    emit(&app, "версии сборки", 0, 1);
    let versions = modrinth::pack_versions(pack_id).await.map_err(early)?;
    let ver = versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| early("версия сборки не найдена".into()))?;
    let file = ver
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| ver.files.first())
        .ok_or_else(|| early("у версии сборки нет файлов".into()))?;

    emit(&app, "загрузка архива", 0, 1);
    let archive = paths::data_dir()
        .join("cache")
        .join("packs")
        .join(&file.filename);
    http::download(
        &file.url,
        &archive,
        file.hashes.get("sha1").map(|s| s.as_str()),
        None,
        None,
    )
    .await
    .map_err(early)?;

    emit(&app, "разбор сборки", 0, 1);
    let index = zip_entry(&archive, "modrinth.index.json")
        .map_err(early)?
        .ok_or_else(|| early("в архиве нет modrinth.index.json".into()))?;
    let mut plan = parse_mrpack(&index, &file.filename, icon).map_err(early)?;
    plan.instance = unique_id(&plan.name);
    plan.pack_id = pack_id.to_string();
    plan.archive = archive;
    execute(app, plan).await
}

async fn run_curseforge(
    app: tauri::AppHandle,
    pack_id: &str,
    file_id: &str,
    icon: &str,
) -> Result<String, (String, String)> {
    emit(&app, "разбор сборки", 0, 1);
    let meta = curseforge::file_meta(pack_id, file_id)
        .await
        .map_err(early)?;
    let url = curseforge::file_url(&meta);
    let archive = paths::data_dir()
        .join("cache")
        .join("packs")
        .join(format!("cf-{pack_id}-{file_id}.zip"));

    emit(&app, "загрузка архива", 0, 1);
    http::download(&url, &archive, meta.sha1.as_deref(), None, None)
        .await
        .map_err(early)?;

    let manifest = zip_entry(&archive, "manifest.json")
        .map_err(early)?
        .ok_or_else(|| early("в архиве нет manifest.json".into()))?;
    let (mut plan, ids) = parse_cf_manifest(&manifest, icon).map_err(early)?;

    // Ссылки на файлы манифеста — одним-двумя запросами
    emit(&app, "список файлов", 0, 1);
    let file_ids: Vec<String> = ids.iter().map(|(_, fid)| fid.clone()).collect();
    let bulk = curseforge::bulk_files(&file_ids)
        .await
        .map_err(|e| early(format!("файлы сборки: {e}")))?;
    plan.files = bulk
        .into_iter()
        .map(|f| PlannedFile {
            url: curseforge::file_url(&f),
            path: format!("mods/{}", f.file_name),
            sha1: f.sha1,
            size: (f.size > 0).then_some(f.size),
        })
        .collect();

    plan.instance = unique_id(&plan.name);
    plan.pack_id = pack_id.to_string();
    plan.archive = archive;
    execute(app, plan).await
}

async fn run_ftb(
    app: tauri::AppHandle,
    pack_id: &str,
    version_id: &str,
) -> Result<String, (String, String)> {
    emit(&app, "разбор сборки", 0, 1);
    let detail = ftb::version_detail(pack_id, version_id)
        .await
        .map_err(early)?;
    let (mc, loader) = ftb::parse_targets(&detail);
    if mc.is_empty() {
        return Err(early("в сборке FTB не указана версия Minecraft".into()));
    }
    let pack = ftb::pack_detail(pack_id).await.map_err(early)?;
    let name = pack["name"].as_str().unwrap_or("FTB сборка").to_string();
    let version_name = pack["versions"]
        .as_array()
        .and_then(|vs| {
            vs.iter()
                .find(|v| v["id"].as_u64().map(|i| i.to_string()) == Some(version_id.to_string()))
                .and_then(|v| v["name"].as_str())
        })
        .unwrap_or(version_id)
        .to_string();

    // Загрузчик: FTB называет сборку forge без версии игры впереди
    let loader = loader.map(|(l, b)| {
        if (l == "forge" || l == "neoforge") && !b.contains('-') {
            (l, format!("{mc}-{b}"))
        } else {
            (l, b)
        }
    });

    let mut files = Vec::new();
    for f in ftb::parse_files(&detail) {
        let rel = f
            .path
            .trim_start_matches("./")
            .trim_matches('/')
            .to_string();
        let file_name = f.url.rsplit('/').next().unwrap_or("file.bin").to_string();
        let full = if rel.is_empty() {
            file_name
        } else {
            format!("{rel}/{file_name}")
        };
        files.push(PlannedFile {
            url: f.url,
            path: full,
            sha1: f.sha1,
            size: f.size,
        });
    }

    let plan = PackPlan {
        instance: unique_id(&name),
        name,
        mc_version: mc.clone(),
        loader,
        icon_url: ftb::art_url(&pack),
        source: SOURCE_FTB.into(),
        pack_id: pack_id.to_string(),
        pack_version: version_name,
        ram_mb: detail["specs"]["recommended"]
            .as_u64()
            .unwrap_or(4096)
            .clamp(1024, 32768) as u32,
        archive: PathBuf::new(),
        files,
    };
    execute(app, plan).await
}

// Исполнение плана: overrides, файлы, загрузчик, клиент
async fn execute(app: tauri::AppHandle, plan: PackPlan) -> Result<String, (String, String)> {
    let instance = plan.instance.clone();

    let instance_dir = paths::instance_dir(&plan.instance);
    tokio::fs::create_dir_all(&instance_dir)
        .await
        .map_err(|e| late(&instance, format!("папка сборки: {e}")))?;

    if !plan.archive.as_os_str().is_empty() {
        emit(&app, "распаковка", 0, 1);
        extract_prefix(&plan.archive, "overrides/", &instance_dir)
            .map_err(|e| late(&instance, e))?;
    }

    let total = plan.files.len() as u64;
    if total > 0 {
        emit(&app, "файлы сборки", 0, total);
        let done = Arc::new(AtomicUsize::new(0));
        let tasks: Vec<_> = plan
            .files
            .iter()
            .map(|f| {
                let to = instance_dir.join(&f.path);
                let f = f.clone();
                let dir = instance_dir.clone();
                let done = done.clone();
                async move {
                    if !to.starts_with(&dir) {
                        return (0usize, Err("плохой путь в сборке".to_string()));
                    }
                    let r = http::download(&f.url, &to, f.sha1.as_deref(), None, f.size).await;
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    (n, r)
                }
            })
            .collect();
        let mut stream = futures::stream::iter(tasks).buffer_unordered(12);
        let mut errors: Vec<String> = Vec::new();
        while let Some((n, res)) = stream.next().await {
            emit(&app, "файлы сборки", n as u64, total);
            if let Err(e) = res {
                errors.push(e);
            }
        }
        if !errors.is_empty() {
            return Err(late(
                &instance,
                format!("файлов не скачалось: {}; {}", errors.len(), errors[0]),
            ));
        }
    }

    // Загрузчик: forge в манифестах указан без версии игры впереди
    let mut launch_version = plan.mc_version.clone();
    if let Some((tag, build)) = &plan.loader {
        emit(&app, &format!("загрузчик {tag}"), 0, 1);
        let full = match tag.as_str() {
            "forge" | "neoforge" if !build.contains('-') => {
                format!("{}-{}", plan.mc_version, build)
            }
            _ => build.clone(),
        };
        launch_version = loaders::install(app.clone(), tag, &plan.mc_version, &full)
            .await
            .map_err(|e| late(&instance, e))?;
    }

    emit(&app, "файлы игры", 0, 1);
    install::ensure_version(app.clone(), &launch_version)
        .await
        .map_err(|e| late(&instance, e))?;

    let cfg = config::InstanceConfig {
        ram_mb: plan.ram_mb,
        jvm_args: Vec::new(),
        kind: "modpack".into(),
        launch_version: launch_version.clone(),
        mc_version: plan.mc_version.clone(),
        source: plan.source.clone(),
        pack_name: plan.name.clone(),
        pack_id: plan.pack_id.clone(),
        pack_version: plan.pack_version.clone(),
        icon_url: plan.icon_url.clone(),
    };
    config::write(&plan.instance, &cfg)
        .await
        .map_err(|e| late(&instance, e))?;
    Ok(instance)
}
