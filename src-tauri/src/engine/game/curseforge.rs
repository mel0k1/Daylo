use serde::Serialize;
use serde_json::Value;

use super::super::core::{http, settings};
use super::modrinth::{self, SearchHit};

// Официальному API нужен свой ключ; открытое зеркало — первый кандидат
const MIRROR: &str = "https://api.curse.tools/v1";
const OFFICIAL: &str = "https://api.curseforge.com/v1";
const GAME_ID: u64 = 432;
const CLASS_MOD: u64 = 6;
const CLASS_PACK: u64 = 4471;

#[derive(Serialize, Clone)]
pub struct CfFile {
    pub file_id: String,
    pub display: String,
    pub file_name: String,
    pub date: String,
    pub size: u64,
    pub sha1: Option<String>,
}

fn key() -> Option<String> {
    settings::load().cf_api_key.filter(|k| !k.trim().is_empty())
}

async fn fetch(base: &str, path: &str, key: Option<&str>) -> Result<Value, String> {
    let url = format!("{base}{path}");
    let mut req = http::client()
        .get(&url)
        .header("Accept", "application/json");
    if let Some(k) = key {
        req = req.header("x-api-key", k);
    }
    let res = req
        .send()
        .await
        .map_err(|e| format!("запрос CurseForge: {e}"))?;
    let res = res
        .error_for_status()
        .map_err(|e| format!("CurseForge: {e}"))?;
    let body = res.text().await.map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&body).map_err(|e| format!("ответ CurseForge: {e}"))?;
    v.get("data")
        .cloned()
        .ok_or_else(|| "в ответе CurseForge нет data".into())
}

async fn post(base: &str, path: &str, body: &Value, key: Option<&str>) -> Result<Value, String> {
    let url = format!("{base}{path}");
    let mut req = http::client()
        .post(&url)
        .header("Accept", "application/json")
        .json(body);
    if let Some(k) = key {
        req = req.header("x-api-key", k);
    }
    let res = req
        .send()
        .await
        .map_err(|e| format!("запрос CurseForge: {e}"))?;
    let res = res
        .error_for_status()
        .map_err(|e| format!("CurseForge: {e}"))?;
    let text = res.text().await.map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("ответ CurseForge: {e}"))?;
    v.get("data")
        .cloned()
        .ok_or_else(|| "в ответе CurseForge нет data".into())
}

// Зеркало без ключа, затем официальный API с ключом из настроек
async fn get(path: &str) -> Result<Value, String> {
    let mirror = fetch(MIRROR, path, None).await;
    if mirror.is_ok() {
        return mirror;
    }
    match key() {
        Some(k) => fetch(OFFICIAL, path, Some(k.trim())).await,
        None => Err(format!(
            "{}; добавьте ключ API CurseForge в настройках",
            mirror.unwrap_err()
        )),
    }
}

async fn post_json(path: &str, body: &Value) -> Result<Value, String> {
    let mirror = post(MIRROR, path, body, None).await;
    if mirror.is_ok() {
        return mirror;
    }
    match key() {
        Some(k) => post(OFFICIAL, path, body, Some(k.trim())).await,
        None => Err(format!(
            "{}; добавьте ключ API CurseForge в настройках",
            mirror.unwrap_err()
        )),
    }
}

fn clean_query<'a>(q: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    q.iter().filter(|(_, v)| !v.is_empty()).copied().collect()
}

fn query_string<'a>(q: &[(&'a str, &'a str)]) -> String {
    let pairs = clean_query(q);
    if pairs.is_empty() {
        return String::new();
    }
    let mut out = String::from("?");
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(&format!("{k}={}", super::modrinth::enc(v)));
    }
    out
}

async fn search(query: &str, game_version: &str, class_id: u64) -> Result<Vec<SearchHit>, String> {
    let qs = format!(
        "/mods/search?gameId={GAME_ID}&classId={class_id}&pageSize=40{}{}",
        if query.trim().is_empty() {
            String::new()
        } else {
            format!("&searchFilter={}", modrinth::enc(query.trim()))
        },
        if game_version.is_empty() {
            String::new()
        } else {
            format!("&gameVersion={}", modrinth::enc(game_version))
        }
    );
    let v = get(&qs).await?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_i64()?;
            let icon = m["logo"]["thumbnailUrl"]
                .as_str()
                .or_else(|| m["logo"]["url"].as_str())
                .unwrap_or_default();
            Some(SearchHit {
                project_id: id.to_string(),
                slug: m["slug"].as_str().unwrap_or_default().to_string(),
                title: m["name"].as_str().unwrap_or_default().to_string(),
                description: m["summary"].as_str().unwrap_or_default().to_string(),
                icon_url: icon.to_string(),
                downloads: m["downloadCount"].as_f64().unwrap_or(0.0) as u64,
            })
        })
        .collect())
}

pub async fn search_mods(query: &str, game_version: &str) -> Result<Vec<SearchHit>, String> {
    search(query, game_version, CLASS_MOD).await
}

pub async fn search_packs(query: &str, game_version: &str) -> Result<Vec<SearchHit>, String> {
    search(query, game_version, CLASS_PACK).await
}

fn parse_file(f: &Value) -> Option<CfFile> {
    let id = f["id"].as_i64()?;
    let name = f["fileName"].as_str()?.to_string();
    let sha1 = f["hashes"]
        .as_array()
        .and_then(|hs| {
            hs.iter()
                .find(|h| h["algorithm"].as_i64() == Some(1))
                .and_then(|h| h["value"].as_str())
        })
        .map(String::from);
    Some(CfFile {
        file_id: id.to_string(),
        display: f["displayName"].as_str().unwrap_or_default().to_string(),
        file_name: name,
        date: f["fileDate"].as_str().unwrap_or_default().to_string(),
        size: f["fileLength"].as_u64().unwrap_or(0),
        sha1,
    })
}

// CDN-ссылка по номеру файла; filename кодируется как в query
pub fn file_url(file: &CfFile) -> String {
    let id: u64 = file.file_id.parse().unwrap_or(0);
    let enc_name = modrinth::enc(&file.file_name);
    format!(
        "https://edge.forgecdn.net/files/{}/{}/{}",
        id / 1000,
        id % 1000,
        enc_name
    )
}

// Файлы проекта; модпаки и моды живут на одном эндпоинте
pub async fn files(mod_id: &str, game_version: &str) -> Result<Vec<CfFile>, String> {
    let path = format!(
        "/mods/{mod_id}/files{}",
        query_string(&[("gameVersion", game_version)])
    );
    let v = get(&path).await?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let mut out: Vec<CfFile> = arr.iter().filter_map(parse_file).collect();
    out.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(out)
}

// Метаданные одного файла: нужны URL и хеш перед скачиванием
pub async fn file_meta(mod_id: &str, file_id: &str) -> Result<CfFile, String> {
    let v = get(&format!("/mods/{mod_id}/file/{file_id}")).await?;
    parse_file(&v).ok_or_else(|| "файл не найден".into())
}

// Мод из CurseForge в папку mods указанной сборки
pub async fn install_mod(instance: &str, mod_id: &str, file_id: &str) -> Result<(), String> {
    let file = file_meta(mod_id, file_id).await?;
    let url = file_url(&file);
    let mods = super::super::core::paths::instance_dir(instance).join("mods");
    http::download(
        &url,
        &mods.join(&file.file_name),
        file.sha1.as_deref(),
        None,
        None,
    )
    .await?;
    modrinth::track(
        instance,
        modrinth::ModMeta {
            file: file.file_name.clone(),
            project_id: format!("cf:{mod_id}"),
            version_id: file.file_id.clone(),
            version_number: file.display.clone(),
        },
    )
    .await
}

// Пачка файлов сборки: один запрос на 50 id вместо сотен одиночных
pub async fn bulk_files(file_ids: &[String]) -> Result<Vec<CfFile>, String> {
    let mut out = Vec::new();
    for chunk in file_ids.chunks(50) {
        let body = serde_json::json!({ "fileIds": chunk.iter().filter_map(|s| s.parse::<i64>().ok()).collect::<Vec<_>>() });
        let v = post_json("/mods/files", &body).await?;
        let arr = v.as_array().cloned().unwrap_or_default();
        for f in arr.iter().filter_map(parse_file) {
            out.push(f);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_by_number() {
        let f = CfFile {
            file_id: "2912957".into(),
            display: "Bookshelf".into(),
            file_name: "Bookshelf-1.15.2-5.3.9.jar".into(),
            date: String::new(),
            size: 0,
            sha1: None,
        };
        assert_eq!(
            file_url(&f),
            "https://edge.forgecdn.net/files/2912/957/Bookshelf-1.15.2-5.3.9.jar"
        );
    }

    #[test]
    fn clean_query_drops_empty() {
        let q = clean_query(&[("a", "1"), ("b", "")]);
        assert_eq!(q.len(), 1);
        assert_eq!(query_string(&[("a", "1"), ("b", "")]), "?a=1");
    }
}
