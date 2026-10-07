use futures::StreamExt;
use serde_json::Value;

use super::super::core::{http, paths};
use super::modpack::{PackHit, PackVersionInfo};

const API: &str = "https://api.modpacks.ch/public";
const CACHE_HOURS: u64 = 24;

// Прямые ссылки на файлы версии сборки
pub struct FtbFile {
    pub url: String,
    pub path: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

async fn get_json(url: &str) -> Result<Value, String> {
    let body = http::text(url).await?;
    let v: Value =
        serde_json::from_str(&body).map_err(|e| format!("ответ FTB: {e}"))?;
    if v["status"].as_str() == Some("error") {
        return Err(format!(
            "FTB: {}",
            v["message"].as_str().unwrap_or("ошибка")
        ));
    }
    Ok(v)
}

fn pack_hit(id: u64, v: &Value) -> PackHit {
    PackHit {
        source: "ftb".into(),
        id: id.to_string(),
        slug: String::new(),
        title: v["name"].as_str().unwrap_or("FTB сборка").to_string(),
        description: v["synopsis"].as_str().unwrap_or_default().to_string(),
        icon_url: art_url(v),
        downloads: v["installs"].as_u64().unwrap_or(0),
    }
}

// Квадратная обложка сборки
pub fn art_url(v: &Value) -> String {
    v["art"]
        .as_array()
        .and_then(|arts| {
            arts.iter()
                .find(|a| a["type"].as_str() == Some("square"))
                .or_else(|| arts.first())
        })
        .and_then(|a| a["url"].as_str())
        .unwrap_or_default()
        .to_string()
}

// Каталог кэшируется на сутки: ~100+ карточек за один проход
async fn catalog() -> Result<Vec<PackHit>, String> {
    let cache = paths::data_dir().join("cache").join("ftb-catalog.json");
    if let Ok(meta) = std::fs::metadata(&cache) {
        if let Ok(age) = meta.modified() {
            if age.elapsed().map(|d| d.as_secs() < CACHE_HOURS * 3600).unwrap_or(false) {
                if let Ok(body) = std::fs::read_to_string(&cache) {
                    if let Ok(hits) = serde_json::from_str::<Vec<PackHit>>(&body) {
                        if !hits.is_empty() {
                            return Ok(hits);
                        }
                    }
                }
            }
        }
    }

    let body = http::text(&format!("{API}/modpack/all")).await?;
    let v: Value = serde_json::from_str(&body).map_err(|e| format!("ответ FTB: {e}"))?;
    let ids: Vec<u64> = v["packs"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|x| x.as_u64())
        .collect();

    let tasks: Vec<_> = ids
        .iter()
        .map(|id| {
            let url = format!("{API}/modpack/{id}");
            async move { get_json(&url).await.map(|v| (*id, v)).ok() }
        })
        .collect();

    let mut hits = Vec::new();
    let mut stream = futures::stream::iter(tasks).buffer_unordered(16);
    while let Some(res) = stream.next().await {
        if let Some((id, v)) = res {
            hits.push(pack_hit(id, &v));
        }
    }
    hits.sort_by_key(|p| std::cmp::Reverse(p.downloads));

    if !hits.is_empty() {
        if let Some(dir) = cache.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(body) = serde_json::to_string(&hits) {
            let _ = std::fs::write(&cache, body);
        }
    }
    Ok(hits)
}

// Поиск по локально закэшированному каталогу
pub async fn search(term: &str) -> Result<Vec<PackHit>, String> {
    let packs = catalog().await?;
    let term = term.trim().to_lowercase();
    Ok(packs
        .into_iter()
        .filter(|p| {
            term.is_empty()
                || p.title.to_lowercase().contains(&term)
                || p.description.to_lowercase().contains(&term)
        })
        .take(60)
        .collect())
}

pub async fn pack_detail(pack_id: &str) -> Result<Value, String> {
    get_json(&format!("{API}/modpack/{pack_id}")).await
}

// Версии сборки: id, имя, дата, версия игры, загрузчик
pub async fn versions(pack_id: &str) -> Result<Vec<PackVersionInfo>, String> {
    let v = pack_detail(pack_id).await?;
    Ok(v["versions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|ver| {
            let (mc, loader) = parse_targets(ver);
            PackVersionInfo {
                version_id: ver["id"].as_u64().map(|i| i.to_string()).unwrap_or_default(),
                name: ver["name"].as_str().unwrap_or_default().to_string(),
                date: ver["updated"].as_u64().unwrap_or(0).to_string(),
                mc_versions: if mc.is_empty() { Vec::new() } else { vec![mc] },
                loaders: loader.map(|l| vec![l.0]).unwrap_or_default(),
            }
        })
        .filter(|x| !x.version_id.is_empty())
        .collect())
}

pub async fn version_detail(pack_id: &str, version_id: &str) -> Result<Value, String> {
    get_json(&format!("{API}/modpack/{pack_id}/{version_id}")).await
}

// Из targets: версия игры и загрузчик
pub fn parse_targets(v: &Value) -> (String, Option<(String, String)>) {
    let mut mc = String::new();
    let mut loader = None;
    for t in v["targets"].as_array().cloned().unwrap_or_default() {
        let name = t["name"].as_str().unwrap_or_default().to_lowercase();
        let ver = t["version"].as_str().unwrap_or_default().to_string();
        if ver.is_empty() {
            continue;
        }
        match name.as_str() {
            "minecraft" => mc = ver,
            "forge" | "fabric" | "neoforge" | "quilt" => {
                loader = Some((name, ver));
            }
            _ => {}
        }
    }
    (mc, loader)
}

pub fn parse_files(v: &Value) -> Vec<FtbFile> {
    v["files"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|f| {
            let url = f["url"].as_str()?.to_string();
            let path = f["path"].as_str().unwrap_or("./mods").to_string();
            Some(FtbFile {
                url,
                path,
                sha1: f["sha1"].as_str().map(String::from),
                size: f["size"].as_u64(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn targets_extract_mc_and_loader() {
        let v = json!({ "targets": [
            { "name": "minecraft", "type": "game", "version": "1.12.2" },
            { "name": "forge", "type": "modloader", "version": "14.23.5.2860" },
            { "name": "java", "type": "runtime", "version": "8" }
        ]});
        let (mc, loader) = parse_targets(&v);
        assert_eq!(mc, "1.12.2");
        assert_eq!(loader, Some(("forge".into(), "14.23.5.2860".into())));
    }

    #[test]
    fn files_keep_path_and_hash() {
        let v = json!({ "files": [
            { "url": "https://edge.forgecdn.net/files/1/2/Mod.jar", "path": "./mods", "sha1": "abc", "size": 10 },
            { "url": "https://x/cfg.zip", "path": "./config", "sha1": null, "size": null }
        ]});
        let files = parse_files(&v);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "./mods");
        assert_eq!(files[0].sha1.as_deref(), Some("abc"));
        assert_eq!(files[1].path, "./config");
    }
}
