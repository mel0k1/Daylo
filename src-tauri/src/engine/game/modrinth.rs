use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::core::{http, paths};

const API: &str = "https://api.modrinth.com/v2";

// Процентное кодирование query-параметров
fn enc(s: &str) -> String {
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
    let mut facets = vec![r#""project_type:mod""#.to_string()];
    if !game_version.is_empty() {
        facets.push(format!(r#""versions:{game_version}""#));
    }
    let facets = format!("[{}]", facets.join(","));
    let url = format!(
        "{API}/search?query={}&limit=40&index=relevance&facets={}",
        enc(query),
        enc(&facets)
    );
    let raw: SearchRaw = serde_json::from_str(&http::text(&url).await?)
        .map_err(|e| format!("ответ Modrinth: {e}"))?;
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

// Для модов без учёта ищем проект по sha1 файла и записываем в учёт
pub async fn resolve_untracked(instance: &str) -> usize {
    let files = installed(instance).await;
    let known: HashSet<String> = load_meta(&meta_path(instance))
        .iter()
        .map(|m| m.file.clone())
        .collect();
    let mut list = load_meta(&meta_path(instance));
    let mut n = 0;
    for f in files {
        if known.contains(&f) || !f.ends_with(".jar") {
            continue;
        }
        let path = paths::instance_dir(instance).join("mods").join(&f);
        let Ok(hash) = http::sha1_of(&path) else {
            continue;
        };
        let url = format!("{API}/version_file/{hash}?algorithm=sha1");
        let Ok(body) = http::text(&url).await else {
            continue;
        };
        let Ok(raw) = serde_json::from_str::<VersionRaw>(&body) else {
            continue;
        };
        if raw.project_id.is_empty() {
            continue;
        }
        list.push(ModMeta {
            file: f,
            project_id: raw.project_id.clone(),
            version_id: raw.id.clone(),
            version_number: raw.version_number.clone(),
        });
        n += 1;
    }
    if n > 0 {
        let _ = save_meta(&meta_path(instance), &list);
    }
    n
}

// Свежие версии для учтённых модов; сеть по одному проекту — сбой пропускается
pub async fn check_updates(instance: &str, game_version: &str) -> Vec<ModUpdate> {
    resolve_untracked(instance).await;
    let metas = tracked(instance).await;
    let mut out = Vec::new();
    for m in metas {
        if m.project_id.is_empty() {
            continue;
        }
        let Ok(vs) = versions(&m.project_id, game_version).await else {
            continue;
        };
        if let Some(latest) = vs.first() {
            if latest.version_id != m.version_id {
                out.push(ModUpdate {
                    file: m.file,
                    project_id: m.project_id,
                    current_version: m.version_number,
                    latest_version_id: latest.version_id.clone(),
                    latest_version_number: latest.version_number.clone(),
                });
            }
        }
    }
    out
}

// Ставит свежую версию мода и убирает старый файл
pub async fn update(instance: &str, old_file: &str, mod_version_id: &str) -> Result<(), String> {
    install(instance, mod_version_id).await?;
    uninstall(instance, old_file).await
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
