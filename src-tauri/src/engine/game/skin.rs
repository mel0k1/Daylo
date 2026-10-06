use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::Value;

use super::super::core::{http, paths};
use super::loaders;

fn skins_dir() -> std::path::PathBuf {
    paths::data_dir().join("skins")
}

// Ник превращается в имя файла: только латиница, цифры и подчёркивание
fn skin_file(nick: &str) -> Result<std::path::PathBuf, String> {
    let key: String = nick
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(16)
        .collect();
    if key.is_empty() {
        return Err("пустой ник".into());
    }
    Ok(skins_dir().join(format!("{}.png", key.to_lowercase())))
}

// PNG скина 64x64 или 64x32, до полумегабайта
fn validate_skin(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 24 || bytes.len() > 512 * 1024 {
        return Err("файл слишком большой или побитый".into());
    }
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Err("это не PNG".into());
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if w != 64 || (h != 64 && h != 32) {
        return Err("скин должен быть 64x64 или 64x32".into());
    }
    Ok(())
}

pub async fn save(nick: &str, png_base64: &str) -> Result<(), String> {
    let bytes = B64.decode(png_base64).map_err(|_| "не PNG в base64")?;
    validate_skin(&bytes)?;
    let file = skin_file(nick)?;
    tokio::fs::create_dir_all(skins_dir())
        .await
        .map_err(|e| format!("папка скинов: {e}"))?;
    tokio::fs::write(&file, bytes)
        .await
        .map_err(|e| format!("сохранение скина: {e}"))
}

pub async fn load(nick: &str) -> Option<String> {
    let bytes = tokio::fs::read(skin_file(nick).ok()?).await.ok()?;
    Some(B64.encode(bytes))
}

pub async fn delete(nick: &str) -> Result<(), String> {
    let file = skin_file(nick)?;
    if file.exists() {
        tokio::fs::remove_file(&file)
            .await
            .map_err(|e| format!("удаление скина: {e}"))?;
    }
    Ok(())
}

const CSL_PROJECT: &str = "https://api.modrinth.com/v2/project/customskinloader/version";

// Свежий Universal-jar CustomSkinLoader под загрузчик и версию игры
async fn ensure_csl_jar(mc: &str, tag: &str) -> Result<std::path::PathBuf, String> {
    let raw: Vec<Value> =
        serde_json::from_str(&http::text(CSL_PROJECT).await?).map_err(|e| e.to_string())?;
    let v = raw
        .iter()
        .find(|v| {
            v["loaders"]
                .as_array()
                .is_some_and(|l| l.iter().any(|x| x.as_str() == Some(tag)))
                && v["game_versions"]
                    .as_array()
                    .is_some_and(|g| g.iter().any(|x| x.as_str() == Some(mc)))
        })
        .ok_or("CustomSkinLoader под эту сборку не найден")?;
    let file = v["files"]
        .as_array()
        .and_then(|f| f.iter().find(|x| x["primary"].as_bool() == Some(true)))
        .or_else(|| v["files"].as_array().and_then(|f| f.first()))
        .ok_or("у релиза CSL нет файлов")?;
    let name = file["filename"]
        .as_str()
        .ok_or("у файла CSL нет имени")?
        .to_string();

    let dir = paths::data_dir().join("cache").join("csl");
    let cached = dir.join(&name);
    if cached.exists() {
        return Ok(cached);
    }
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("папка кеша csl: {e}"))?;
    let sha1 = file["hashes"]["sha1"].as_str().map(String::from);
    let size = file["size"].as_u64();
    http::download(
        file["url"].as_str().ok_or("у файла CSL нет ссылки")?,
        &cached,
        sha1.as_deref(),
        None,
        size,
    )
    .await?;
    Ok(cached)
}

// Скин в игру: CSL в mods сборки с загрузчиком и копия в LocalSkin
pub async fn apply_to_instance(instance: &str, mc: &str, nick: &str) -> Result<(), String> {
    let tag = loaders::loader_tag(instance);
    if tag.is_empty() {
        return Ok(());
    }
    let jar = ensure_csl_jar(mc, tag).await?;
    let mods = paths::instance_dir(instance).join("mods");
    tokio::fs::create_dir_all(&mods)
        .await
        .map_err(|e| format!("папка mods: {e}"))?;
    let name = jar
        .file_name()
        .ok_or("плохое имя jar")?
        .to_string_lossy()
        .to_string();
    let target = mods.join(&name);
    if !target.exists() {
        tokio::fs::copy(&jar, &target)
            .await
            .map_err(|e| format!("установка CSL: {e}"))?;
    }
    if let Ok(skin) = skin_file(nick) {
        if skin.exists() {
            let local = paths::instance_dir(instance)
                .join("CustomSkinLoader")
                .join("LocalSkin");
            tokio::fs::create_dir_all(&local)
                .await
                .map_err(|e| format!("папка LocalSkin: {e}"))?;
            tokio::fs::copy(&skin, local.join(format!("{nick}.png")))
                .await
                .map_err(|e| format!("копия скина: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skin_name_is_sanitized() {
        let p = skin_file("Notch").unwrap();
        assert!(p.file_name().unwrap() == "notch.png");
        assert!(
            skin_file("../etc/passwd").is_err()
                || skin_file("../etc/passwd").unwrap().file_name().unwrap() == "etcpasswd.png"
        );
        assert!(skin_file("").is_err());
    }
}
