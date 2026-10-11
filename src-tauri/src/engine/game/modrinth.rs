use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::core::{http, paths};

const API: &str = "https://api.modrinth.com/v2";

// Процентное кодирование query-параметров (нужно и CurseForge-CDN)
pub fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Deserialize)]
struct SearchRaw {
    hits: Vec<HitRaw>,
}

#[derive(Deserialize)]
struct HitRaw {
    project_id: String,
    slug: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon_url: String,
    #[serde(default)]
    downloads: u64,
}

#[derive(Serialize, Clone)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: String,
    pub downloads: u64,
}

#[derive(Deserialize)]
struct FileRaw {
    url: String,
    filename: String,
    #[serde(default)]
    hashes: std::collections::HashMap<String, String>,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    primary: bool,
}

// Файл версии сборки (для установки .mrpack)
#[derive(Serialize, Clone)]
pub struct PackFile {
    pub url: String,
    pub filename: String,
    pub hashes: std::collections::HashMap<String, String>,
    pub size: u64,
    pub primary: bool,
}

// Версия сборки Modrinth
#[derive(Serialize, Clone)]
pub struct PackVersion {
    pub id: String,
    pub version_number: String,
    pub date_published: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<PackFile>,
}

#[derive(Deserialize)]
struct VersionRaw {
    id: String,
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    date_published: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    files: Vec<FileRaw>,
}

#[derive(Serialize, Clone)]
pub struct ModVersion {
    pub version_id: String,
    pub version_number: String,
    pub file_name: String,
    pub date: String,
    pub size: u64,
    pub loaders: Vec<String>,
}

// Запись в файле-учёте установленных модов
#[derive(Serialize, Deserialize, Clone)]
pub struct ModMeta {
    pub file: String,
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
}

#[derive(Serialize, Clone)]
pub struct ModUpdate {
    pub file: String,
    pub project_id: String,
    pub current_version: String,
    pub latest_version_id: String,
    pub latest_version_number: String,
}

fn primary_file(v: &VersionRaw) -> Option<&FileRaw> {
    v.files
        .iter()
        .find(|f| f.primary)
        .or_else(|| v.files.first())
}

fn to_mod_version(v: &VersionRaw, f: &FileRaw) -> ModVersion {
    ModVersion {
        version_id: v.id.clone(),
        version_number: v.version_number.clone(),
        file_name: f.filename.clone(),
        date: v.date_published.clone(),
        size: f.size,
        loaders: v.loaders.clone(),
    }
}

// Поиск модов: всегда project_type:mod, версия игры — если задана
pub async fn search(query: &str, game_version: &str) -> Result<Vec<SearchHit>, String> {
    search_type(query, game_version, "mod").await
}

// Facets поиска: Modrinth ждёт массив массивов — ИЛИ внутри группы, И между
// группами. Плоский список строк сервер отвергает с 400 Bad Request.
fn facets_json(project_type: &str, game_version: &str) -> String {
    let mut groups = vec![format!(r#"["project_type:{project_type}"]"#)];
    if !game_version.is_empty() {
        groups.push(format!(r#"["versions:{game_version}"]"#));
    }
    format!("[{}]", groups.join(","))
}

// Поиск по типу проекта: mod, modpack, resourcepack…
pub async fn search_type(
    query: &str,
    game_version: &str,
    project_type: &str,
) -> Result<Vec<SearchHit>, String> {
    let with_version = format!(
        "{API}/search?query={}&limit=40&index=relevance&facets={}",
        enc(query),
        enc(&facets_json(project_type, game_version))
    );
    // Modrinth ещё не занёс свежую версию игры в каталог — ищем без фильтра
    let body = match http::text(&with_version).await {
        Ok(t) => t,
        Err(e) if !game_version.is_empty() && e.contains("400") => {
            let no_version = format!(
                "{API}/search?query={}&limit=40&index=relevance&facets={}",
                enc(query),
                enc(&facets_json(project_type, ""))
            );
            http::text(&no_version).await?
        }
        Err(e) => return Err(e),
    };
    let raw: SearchRaw = serde_json::from_str(&body).map_err(|e| format!("ответ Modrinth: {e}"))?;
    Ok(raw
        .hits
        .into_iter()
        .map(|h| SearchHit {
            project_id: h.project_id,
            slug: h.slug,
            title: h.title,
            description: h.description,
            icon_url: h.icon_url,
            downloads: h.downloads,
        })
        .collect())
}

// Версии проекта, совместимые с версией игры; свежие сверху
pub async fn versions(project_id: &str, game_version: &str) -> Result<Vec<ModVersion>, String> {
    let url = format!("{API}/project/{project_id}/version");
    let raw: Vec<VersionRaw> = serde_json::from_str(&http::text(&url).await?)
        .map_err(|e| format!("ответ Modrinth: {e}"))?;
    let mut out = Vec::new();
    for v in &raw {
        if !game_version.is_empty() && !v.game_versions.iter().any(|g| g == game_version) {
            continue;
        }
        if let Some(f) = primary_file(v) {
            out.push(to_mod_version(v, f));
        }
    }
    Ok(out)
}

// Все версии проекта сборки (для установки .mrpack)
pub async fn pack_versions(project_id: &str) -> Result<Vec<PackVersion>, String> {
    let url = format!("{API}/project/{project_id}/version");
    let raw: Vec<VersionRaw> = serde_json::from_str(&http::text(&url).await?)
        .map_err(|e| format!("ответ Modrinth: {e}"))?;
    Ok(raw
        .into_iter()
        .map(|v| PackVersion {
            id: v.id,
            version_number: v.version_number,
            date_published: v.date_published,
            game_versions: v.game_versions,
            loaders: v.loaders,
            files: v
                .files
                .into_iter()
                .map(|f| PackFile {
                    url: f.url,
                    filename: f.filename,
                    hashes: f.hashes,
                    size: f.size,
                    primary: f.primary,
                })
                .collect(),
        })
        .collect())
}

// Качает файл версии мода в mods/ указанной сборки и запоминает происхождение
pub async fn install(instance: &str, mod_version_id: &str) -> Result<(), String> {
    let url = format!("{API}/version/{mod_version_id}");
    let raw: VersionRaw = serde_json::from_str(&http::text(&url).await?)
        .map_err(|e| format!("ответ Modrinth: {e}"))?;
    let file = primary_file(&raw).ok_or("у версии мода нет файлов")?;

    let mods = paths::instance_dir(instance).join("mods");
    let sha1 = file.hashes.get("sha1").map(|s| s.as_str());
    let size = (file.size > 0).then_some(file.size);
    http::download(&file.url, &mods.join(&file.filename), sha1, None, size).await?;

    let meta = ModMeta {
        file: file.filename.clone(),
        project_id: raw.project_id.clone(),
        version_id: raw.id.clone(),
        version_number: raw.version_number.clone(),
    };
    let mut list = load_meta(&meta_path(instance));
    list.retain(|m| m.project_id != meta.project_id);
    list.push(meta);
    save_meta(&meta_path(instance), &list)
}

// Файлы в папке mods сборки (без нашего файла учёта)
pub async fn installed(instance: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(paths::instance_dir(instance).join("mods")).await else {
        return out;
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        if e.path().is_file() {
            if let Some(n) = e.file_name().to_str() {
                if n != "daylo-meta.json" {
                    out.push(n.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

// Убирает мод из сборки; принимаем только имя файла
pub async fn uninstall(instance: &str, file: &str) -> Result<(), String> {
    let name = std::path::Path::new(file)
        .file_name()
        .ok_or("плохое имя файла")?;
    let path = paths::instance_dir(instance).join("mods").join(name);
    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| format!("удаление: {e}"))?;
    let mut list = load_meta(&meta_path(instance));
    list.retain(|m| m.file != name.to_string_lossy());
    save_meta(&meta_path(instance), &list)
}

// Докачать моды из учёта, чьи файлы пропали (удалили руками, сбои диска).
// Modrinth ставится по version_id, CurseForge (учёт "cf:ид") — по file_id;
// сбой одного мода не роняет остальные. Возвращает (починено, ошибки)
pub async fn repair_missing(instance: &str) -> (usize, Vec<String>) {
    let metas = load_meta(&meta_path(instance));
    let mut fixed = 0;
    let mut errs: Vec<String> = Vec::new();
    for m in &metas {
        let path = paths::instance_dir(instance).join("mods").join(&m.file);
        if path.exists() {
            continue;
        }
        let res = match m.project_id.strip_prefix("cf:") {
            Some(mod_id) => super::curseforge::install_mod(instance, mod_id, &m.version_id).await,
            None => install(instance, &m.version_id).await,
        };
        match res {
            Ok(()) => fixed += 1,
            Err(e) => errs.push(format!("{}: {}", m.file, e)),
        }
    }
    (fixed, errs)
}

// --- Учёт модов для обновлений ---

fn meta_path(instance: &str) -> std::path::PathBuf {
    paths::instance_dir(instance)
        .join("mods")
        .join("daylo-meta.json")
}

fn load_meta(path: &Path) -> Vec<ModMeta> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_meta(path: &Path, list: &[ModMeta]) -> Result<(), String> {
    let body = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(path, body).map_err(|e| format!("запись учёта модов: {e}"))
}

// Учтённые моды, у которых файл ещё на месте
pub async fn tracked(instance: &str) -> Vec<ModMeta> {
    let files: HashSet<String> = installed(instance).await.into_iter().collect();
    load_meta(&meta_path(instance))
        .into_iter()
        .filter(|m| files.contains(&m.file))
        .collect()
}

// Для модов без учёта ищем проект по sha1 файла, а неудачных опознаём
// по фингерпринтам CurseForge и записываем в учёт
pub async fn resolve_untracked(instance: &str) -> usize {
    let files = installed(instance).await;
    let known: HashSet<String> = load_meta(&meta_path(instance))
        .iter()
        .map(|m| m.file.clone())
        .collect();
    let mut list = load_meta(&meta_path(instance));
    let mods_dir = paths::instance_dir(instance).join("mods");
    let mut pending: Vec<(String, std::path::PathBuf, String)> = Vec::new();
    for f in files {
        if known.contains(&f) || !f.ends_with(".jar") {
            continue;
        }
        let path = mods_dir.join(&f);
        let Ok(hash) = http::sha1_of(&path) else {
            continue;
        };
        pending.push((f, path, hash));
    }
    let mut n = 0;
    let mut cf_pending: Vec<(String, std::path::PathBuf)> = Vec::new();
    // Опознание Modrinth батчами: один запрос на сотню файлов вместо сотни
    for chunk in pending.chunks(100) {
        let hashes: Vec<String> = chunk.iter().map(|(_, _, h)| h.clone()).collect();
        match lookup_files(&hashes).await {
            Ok(map) => {
                for (f, p, h) in chunk {
                    match map.get(h) {
                        Some(raw) if !raw.project_id.is_empty() => {
                            list.push(ModMeta {
                                file: f.clone(),
                                project_id: raw.project_id.clone(),
                                version_id: raw.id.clone(),
                                version_number: raw.version_number.clone(),
                            });
                            n += 1;
                        }
                        // Modrinth не знает jar — кандидат на опознание CurseForge
                        _ => cf_pending.push((f.clone(), p.clone())),
                    }
                }
            }
            // Батч не прошёл — вся пачка идёт на фингерпринты
            Err(_) => {
                for (f, p, _) in chunk {
                    cf_pending.push((f.clone(), p.clone()));
                }
            }
        }
    }
    n += resolve_by_fingerprints(&mut list, &cf_pending).await;
    if n > 0 {
        let _ = save_meta(&meta_path(instance), &list);
    }
    n
}

#[derive(Serialize)]
struct FilesQuery<'a> {
    hashes: &'a [String],
    algorithm: &'static str,
}

// Опознание файлов по sha1 одним запросом: map hash → версия
async fn lookup_files(hashes: &[String]) -> Result<HashMap<String, VersionRaw>, String> {
    let body = serde_json::to_string(&FilesQuery {
        hashes,
        algorithm: "sha1",
    })
    .map_err(|e| e.to_string())?;
    let res = http::client()
        .post(format!("{API}/version_files"))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("запрос Modrinth: {e}"))?;
    let text = res
        .text()
        .await
        .map_err(|e| format!("ответ Modrinth: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("ответ Modrinth: {e}"))
}

// Неопознанные jar ищем фингерпринтами CurseForge батчами и дописываем в учёт
async fn resolve_by_fingerprints(
    list: &mut Vec<ModMeta>,
    pending: &[(String, std::path::PathBuf)],
) -> usize {
    if pending.is_empty() {
        return 0;
    }
    let mut pairs: Vec<(String, u64)> = Vec::new();
    for (name, path) in pending {
        if let Some(fp) = super::curseforge::fingerprint_file(path).await {
            pairs.push((name.clone(), fp));
        }
    }
    let hashes: Vec<u64> = pairs.iter().map(|(_, h)| *h).collect();
    let Ok(matches) = super::curseforge::by_fingerprints(&hashes).await else {
        return 0;
    };
    let mut n = 0;
    for (name, _) in pairs {
        let Some((mod_id, file)) = matches
            .iter()
            .find(|(_, f)| f.file_name.eq_ignore_ascii_case(&name))
            .map(|(m, f)| (m.clone(), f.clone()))
        else {
            continue;
        };
        list.push(ModMeta {
            file: name,
            project_id: format!("cf:{mod_id}"),
            version_id: file.file_id.clone(),
            version_number: file.display.clone(),
        });
        n += 1;
    }
    n
}

// Свежие версии для учтённых модов: Modrinth — батчем по sha1 всех файлов
// (один запрос на сотню), CurseForge — параллельно по 8. Сбой одного
// мода/батча пропускается, записи «cf:ид» идут в CurseForge.
pub async fn check_updates(instance: &str, game_version: &str) -> Vec<ModUpdate> {
    resolve_untracked(instance).await;
    let metas = tracked(instance).await;
    let (cf, mr): (Vec<ModMeta>, Vec<ModMeta>) = metas
        .into_iter()
        .partition(|m| m.project_id.starts_with("cf:"));
    let mut out = check_modrinth_batch(instance, mr, game_version).await;
    out.extend(check_curseforge_parallel(cf, game_version).await);
    out
}

// Modrinth: локальный sha1 каждого учтённого файла → POST /version_files/update
async fn check_modrinth_batch(
    instance: &str,
    metas: Vec<ModMeta>,
    game_version: &str,
) -> Vec<ModUpdate> {
    let mods_dir = paths::instance_dir(instance).join("mods");
    let mut jobs: Vec<(ModMeta, String)> = Vec::new();
    for m in metas {
        if m.project_id.is_empty() {
            continue;
        }
        let Ok(hash) = http::sha1_of(&mods_dir.join(&m.file)) else {
            continue;
        };
        jobs.push((m, hash));
    }
    let mut out = Vec::new();
    for chunk in jobs.chunks(100) {
        let hashes: Vec<String> = chunk.iter().map(|(_, h)| h.clone()).collect();
        let Ok(map) = update_files(&hashes, game_version).await else {
            continue;
        };
        for (m, h) in chunk {
            if let Some(v) = map.get(h) {
                if v.id != m.version_id {
                    out.push(ModUpdate {
                        file: m.file.clone(),
                        project_id: m.project_id.clone(),
                        current_version: m.version_number.clone(),
                        latest_version_id: v.id.clone(),
                        latest_version_number: v.version_number.clone(),
                    });
                }
            }
        }
    }
    out
}

#[derive(Serialize)]
struct UpdateQuery<'a> {
    hashes: &'a [String],
    algorithm: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    game_versions: Vec<String>,
}

// Свежие версии файлов одним запросом: map hash → версия
async fn update_files(
    hashes: &[String],
    game_version: &str,
) -> Result<HashMap<String, VersionRaw>, String> {
    let gv = if game_version.is_empty() {
        Vec::new()
    } else {
        vec![game_version.to_string()]
    };
    let body = serde_json::to_string(&UpdateQuery {
        hashes,
        algorithm: "sha1",
        game_versions: gv,
    })
    .map_err(|e| e.to_string())?;
    let res = http::client()
        .post(format!("{API}/version_files/update"))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("запрос Modrinth: {e}"))?;
    let text = res
        .text()
        .await
        .map_err(|e| format!("ответ Modrinth: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("ответ Modrinth: {e}"))
}

// CurseForge: latest_file по каждому моду, но восемь запросов одновременно
async fn check_curseforge_parallel(metas: Vec<ModMeta>, game_version: &str) -> Vec<ModUpdate> {
    use futures::stream::{self, StreamExt};
    let jobs = metas.into_iter().filter_map(|m| {
        let mod_id = m.project_id.strip_prefix("cf:")?.to_string();
        Some((m, mod_id))
    });
    stream::iter(jobs)
        .map(|(m, mod_id)| async move {
            let latest = super::curseforge::latest_file(&mod_id, game_version).await;
            (m, latest)
        })
        .buffer_unordered(8)
        .filter_map(|(m, latest)| async move { latest.map(|l| (m, l)) })
        .filter_map(|(m, l)| async move {
            if l.file_id != m.version_id {
                Some(ModUpdate {
                    file: m.file,
                    project_id: m.project_id,
                    current_version: m.version_number,
                    latest_version_id: l.file_id,
                    latest_version_number: l.display,
                })
            } else {
                None
            }
        })
        .collect()
        .await
}

// Ставит свежую версию мода и убирает старый файл
pub async fn update(instance: &str, old_file: &str, mod_version_id: &str) -> Result<(), String> {
    install(instance, mod_version_id).await?;
    uninstall(instance, old_file).await
}

// Заполняет учёт модов при импорте сборки: файлы докачаются позже
pub async fn seed_meta(instance: &str, mods: &[ModMeta]) -> Result<(), String> {
    save_meta(&meta_path(instance), mods)
}

// Добавляет или обновляет запись об одном файле (моды CurseForge)
pub async fn track(instance: &str, meta: ModMeta) -> Result<(), String> {
    let mut list = load_meta(&meta_path(instance));
    list.retain(|m| m.file != meta.file);
    list.push(meta);
    save_meta(&meta_path(instance), &list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facets_are_nested_arrays() {
        // Плоский список строк Modrinth отвечает 400 — только группы
        assert_eq!(
            facets_json("modpack", "1.21.1"),
            r#"[["project_type:modpack"],["versions:1.21.1"]]"#
        );
        assert_eq!(facets_json("mod", ""), r#"[["project_type:mod"]]"#);
    }

    #[test]
    fn enc_keeps_unreserved() {
        assert_eq!(enc("1.21.9"), "1.21.9");
        assert_eq!(enc("farm simulator"), "farm%20simulator");
        assert_eq!(
            enc(r#"["project_type:mod"]"#),
            r#"%5B%22project_type%3Amod%22%5D"#
        );
    }

    #[test]
    fn meta_roundtrip() {
        let dir = std::env::temp_dir().join(format!("daylo-test-meta-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("daylo-meta.json");
        let list = vec![ModMeta {
            file: "sodium.jar".into(),
            project_id: "AANobbMI".into(),
            version_id: "v1".into(),
            version_number: "0.6.0".into(),
        }];
        save_meta(&path, &list).unwrap();
        let back = load_meta(&path);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].project_id, "AANobbMI");
        assert_eq!(back[0].version_number, "0.6.0");
        // Мусор в файле не должен ронять чтение
        std::fs::write(&path, "{").unwrap();
        assert!(load_meta(&path).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_response_maps_hashes() {
        let body = r#"{
            "abc123": {"id": "v2", "project_id": "p1", "version_number": "1.1.0"},
            "def456": {"id": "v1", "project_id": "p2", "version_number": "0.9"}
        }"#;
        let map: HashMap<String, VersionRaw> = serde_json::from_str(body).unwrap();
        assert_eq!(map["abc123"].id, "v2");
        assert_eq!(map["abc123"].version_number, "1.1.0");
        assert_eq!(map["def456"].version_number, "0.9");
    }

    #[test]
    fn update_query_omits_empty_game_versions() {
        let hashes = vec!["h1".to_string()];
        let empty = serde_json::to_string(&UpdateQuery {
            hashes: &hashes,
            algorithm: "sha1",
            game_versions: Vec::new(),
        })
        .unwrap();
        assert_eq!(empty, r#"{"hashes":["h1"],"algorithm":"sha1"}"#);
        let with_gv = serde_json::to_string(&UpdateQuery {
            hashes: &hashes,
            algorithm: "sha1",
            game_versions: vec!["1.21".into()],
        })
        .unwrap();
        assert_eq!(
            with_gv,
            r#"{"hashes":["h1"],"algorithm":"sha1","game_versions":["1.21"]}"#
        );
    }
}
