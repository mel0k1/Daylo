use std::path::Path;

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

// --- Фингерпринты: murmur2 с фильтром пробелов, как у CurseForge ---

// MurmurHash2 (32 бита, блоки little-endian) с заданным seed.
// Эталон сверен с векторами официального клиента Kafka (seed 0x9747b28c).
fn murmur2(data: &[u8], seed: u32) -> u64 {
    const M: u32 = 0x5bd1e995;
    let mut h: u32 = seed ^ data.len() as u32;
    let blocks = data.len() / 4;
    for i in 0..blocks {
        let j = i * 4;
        let mut k = u32::from_le_bytes([data[j], data[j + 1], data[j + 2], data[j + 3]]);
        k = k.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M) ^ k;
    }
    // Хвост — по падению case в референсе: старшие байты первыми, умножение одно
    let rest = &data[blocks * 4..];
    if rest.len() == 3 {
        h ^= (rest[2] as u32) << 16;
    }
    if !rest.is_empty() {
        if rest.len() >= 2 {
            h ^= (rest[1] as u32) << 8;
        }
        h ^= rest[0] as u32;
        h = h.wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    h as u64
}

// Фингерпринт jar по содержимому: seed 1, а байты 09/0A/0D/20 выкидываются
pub fn fingerprint(bytes: &[u8]) -> u64 {
    let kept: Vec<u8> = bytes
        .iter()
        .copied()
        .filter(|b| !matches!(b, 0x09 | 0x0A | 0x0D | 0x20))
        .collect();
    murmur2(&kept, 1)
}

pub async fn fingerprint_file(path: &Path) -> Option<u64> {
    let bytes = tokio::fs::read(path).await.ok()?;
    Some(fingerprint(&bytes))
}

// У API лимит 256 фингерпринтов на запрос
const FP_BATCH: usize = 256;

// /fingerprints есть у официального API; зеркало знает и старый путь
async fn post_fingerprints(body: &Value) -> Result<Value, String> {
    match post_json("/fingerprints", body).await {
        Ok(v) => Ok(v),
        Err(e) => post_json("/mods/fingerprints", body).await.map_err(|_| e),
    }
}

// Опознание jar по фингерпринтам: точные совпадения → (id мода, файл).
// Совпадения не возвращают сам фингерпринт — сопоставляем по имени файла.
pub async fn by_fingerprints(hashes: &[u64]) -> Result<Vec<(String, CfFile)>, String> {
    let mut out = Vec::new();
    for batch in hashes.chunks(FP_BATCH) {
        let body = serde_json::json!({ "fingerprints": batch });
        let v = post_fingerprints(&body).await?;
        let arr = v["data"][0]["exactMatches"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for m in &arr {
            let Some(mod_id) = m["id"].as_i64().map(|x| x.to_string()) else {
                continue;
            };
            let Some(file) = parse_file(&m["file"]) else {
                continue;
            };
            out.push((mod_id, file));
        }
    }
    Ok(out)
}

// Свежий файл мода под версию игры: files уже отсортированы по дате
pub async fn latest_file(mod_id: &str, game_version: &str) -> Option<CfFile> {
    files(mod_id, game_version).await.ok()?.into_iter().next()
}

// Обновление CF-мода: свежий файл ставится, прежний убирается
pub async fn replace_mod(instance: &str, mod_id: &str, file_id: &str, old_file: &str) -> Result<(), String> {
    install_mod(instance, mod_id, file_id).await?;
    modrinth::uninstall(instance, old_file).await
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

    #[test]
    fn murmur2_matches_reference_vectors() {
        // Эталон Kafka (seed 0x9747b28c) — покрывает блоки цикла и хвосты 1–3
        assert_eq!(murmur2(b"a", 0x9747b28c), 2731586172);
        assert_eq!(murmur2(b"abc", 0x9747b28c), 479470107);
        assert_eq!(murmur2(b"abcde", 0x9747b28c), 461995741);
        assert_eq!(murmur2(b"21", 0x9747b28c), 3321034988);
        assert_eq!(murmur2(b"foobar", 0x9747b28c), 3504634814);
        assert_eq!(murmur2(b"a-little-bit-long-string", 0x9747b28c), 3308985760);
        // CurseForge: seed 1 поверх байтов без 09/0A/0D/20
        assert_eq!(fingerprint(b""), 1540447798);
        assert_eq!(fingerprint(b" "), 1540447798);
        assert_eq!(fingerprint(b"a"), 626045324);
        assert_eq!(fingerprint(b"aa"), 1775036265);
        assert_eq!(fingerprint(b"hello world"), 2824650221);
        // Пробел, таб и перевод строки не меняют фингерпринт
        assert_eq!(fingerprint(b"hello  world"), 2824650221);
        assert_eq!(fingerprint(b"hello\tworld\n"), 2824650221);
    }
}
