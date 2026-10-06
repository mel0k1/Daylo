use std::path::PathBuf;

use serde::Serialize;

use super::super::core::paths;

#[derive(Serialize, Clone)]
pub struct Shot {
    pub name: String,
    pub modified: u64,
    pub size: u64,
}

fn dir(version: &str) -> PathBuf {
    paths::instance_dir(version).join("screenshots")
}

fn allowed(name: &str) -> bool {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|x| x.to_str())
        .map(|x| x.to_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg")
}

// Свежие снимки сверху
pub async fn list(version: &str) -> Vec<Shot> {
    let mut out = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(dir(version)).await else {
        return out;
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        if !e.path().is_file() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if !allowed(&name) {
            continue;
        }
        let Ok(meta) = e.metadata().await else {
            continue;
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push(Shot {
            name,
            modified,
            size: meta.len(),
        });
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.modified));
    out
}

// Отсекает обход путей: только файл в корне screenshots
fn resolve(version: &str, name: &str) -> Result<PathBuf, String> {
    let p = std::path::Path::new(name);
    let plain =
        p.file_name().map(|f| f.to_string_lossy()) == Some(std::borrow::Cow::Borrowed(name));
    if !plain || name.starts_with('.') {
        return Err("плохое имя файла".into());
    }
    Ok(dir(version).join(name))
}

// Картинка как data-URL для вебвью
pub async fn read(version: &str, name: &str) -> Result<String, String> {
    let path = resolve(version, name)?;
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .map(|x| x.to_lowercase())
        .unwrap_or_default();
    let mime = if ext == "png" {
        "image/png"
    } else {
        "image/jpeg"
    };
    let data = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("чтение снимка: {e}"))?;
    use base64::Engine;
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(data)
    ))
}

pub async fn delete(version: &str, name: &str) -> Result<(), String> {
    let path = resolve(version, name)?;
    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| format!("удаление: {e}"))
}

// Показывает папку снимков в проводнике
pub async fn open_dir(version: &str) -> Result<(), String> {
    let d = dir(version);
    tokio::fs::create_dir_all(&d)
        .await
        .map_err(|e| format!("папка снимков: {e}"))?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&d)
            .spawn()
            .map_err(|e| format!("проводник: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&d)
            .spawn()
            .map_err(|e| format!("finder: {e}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&d)
            .spawn()
            .map_err(|e| format!("открытие папки: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_blocks_traversal() {
        assert!(resolve("v", "shot.png").is_ok());
        assert!(resolve("v", "../daylo.json").is_err());
        assert!(resolve("v", "sub/shot.png").is_err());
        assert!(resolve("v", "..").is_err());
        assert!(resolve("v", ".hidden").is_err());
        assert!(resolve("v", "notes.txt").is_ok()); // имя ок, расширение фильтрует list
    }

    #[test]
    fn list_filters_by_extension() {
        assert!(allowed("shot.png"));
        assert!(allowed("Shot.JPG"));
        assert!(!allowed("notes.txt"));
        assert!(!allowed("noext"));
    }
}
