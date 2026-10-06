use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::paths;

const API: &str = "https://api.github.com/repos/mel0k1/Daylo/releases/latest";

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    pub url: String,
    pub asset_name: String,
    pub size: u64,
}

#[derive(Clone, Serialize)]
pub struct UpdateProgress {
    pub received: u64,
    pub total: u64,
    pub done: bool,
}

pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Deserialize)]
struct ReleaseRaw {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<AssetRaw>,
}

#[derive(Deserialize)]
struct AssetRaw {
    name: String,
    #[serde(default)]
    size: u64,
    browser_download_url: String,
}

// Свежее ли: сравнение числовых сегментов 0.2.10 > 0.2.9
pub fn newer(current: &str, candidate: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim()
            .trim_start_matches('v')
            .split('.')
            .map(|p| {
                let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse().unwrap_or(0)
            })
            .collect()
    };
    let a = parse(current);
    let b = parse(candidate);
    for i in 0..3 {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if y != x {
            return y > x;
        }
    }
    false
}

fn pick_asset(assets: &[AssetRaw]) -> Option<&AssetRaw> {
    let want: &[&str] = if cfg!(target_os = "windows") {
        &["x64-setup.exe", ".msi"]
    } else if cfg!(target_os = "macos") {
        &[".dmg", ".app.tar.gz"]
    } else {
        &[".AppImage", ".deb"]
    };
    for w in want {
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(w)) {
            return Some(a);
        }
    }
    None
}

// Свежий релиз GitHub, если он новее текущей версии
pub async fn check() -> Result<Option<UpdateInfo>, String> {
    let raw: ReleaseRaw = serde_json::from_str(&super::http::text(API).await?)
        .map_err(|e| format!("ответ GitHub: {e}"))?;
    if raw.assets.is_empty() {
        return Ok(None);
    }
    let tag = raw.tag_name.trim_start_matches('v').to_string();
    if !newer(current(), &tag) {
        return Ok(None);
    }
    let Some(a) = pick_asset(&raw.assets) else {
        return Ok(None);
    };
    Ok(Some(UpdateInfo {
        version: tag,
        notes: raw.body,
        url: a.browser_download_url.clone(),
        asset_name: a.name.clone(),
        size: a.size,
    }))
}

// Скачивает установщик с прогрессом и запускает его
pub async fn install(app: &tauri::AppHandle, info: &UpdateInfo) -> Result<(), String> {
    let dir = paths::data_dir().join("update");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("папка обновления: {e}"))?;
    let to = dir.join(&info.asset_name);
    download(app, &info.url, &to, info.size).await?;
    run_installer(&to)?;
    Ok(())
}

async fn download(
    app: &tauri::AppHandle,
    url: &str,
    to: &PathBuf,
    total: u64,
) -> Result<(), String> {
    let res = super::http::client()
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("скачивание {url}: {e}"))?;

    let part = to.with_extension("part");
    let mut file = tokio::fs::File::create(&part)
        .await
        .map_err(|e| format!("создание {}: {e}", part.display()))?;
    let mut stream = res.bytes_stream();
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    let mut received: u64 = 0;
    let mut last: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("поток {url}: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("запись {}: {e}", part.display()))?;
        received += chunk.len() as u64;
        if received - last > 262_144 {
            last = received;
            let _ = app.emit(
                "update-progress",
                UpdateProgress {
                    received,
                    total,
                    done: false,
                },
            );
        }
    }
    file.flush()
        .await
        .map_err(|e| format!("запись {}: {e}", part.display()))?;
    drop(file);
    tokio::fs::rename(&part, to)
        .await
        .map_err(|e| format!("переименование: {e}"))?;
    let _ = app.emit(
        "update-progress",
        UpdateProgress {
            received,
            total,
            done: true,
        },
    );
    Ok(())
}

fn run_installer(file: &Path) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let mut cmd = if name.ends_with(".msi") {
            let mut c = std::process::Command::new("msiexec");
            c.arg("/i").arg(file).arg("/quiet");
            c
        } else {
            // NSIS: тихая установка с перезапуском приложения
            let mut c = std::process::Command::new(file);
            c.arg("/S").arg("/R");
            c
        };
        cmd.spawn()
            .map_err(|e| format!("запуск установщика: {e}"))?;
        Ok(())
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg(file)
            .spawn()
            .map_err(|e| format!("открытие образа: {e}"))?;
        Ok(())
    } else {
        #[cfg(target_os = "linux")]
        {
            linux_install(file)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err("платформа не поддерживается для автоустановки".into())
        }
    }
}

// Linux: AppImage подменяем себя и перезапускаемся; прочее — открываем папку
#[cfg(target_os = "linux")]
fn linux_install(file: &Path) -> Result<(), String> {
    if file.extension().map(|e| e == "AppImage").unwrap_or(false) {
        if let Ok(appimage) = std::env::var("APPIMAGE") {
            let appimage = PathBuf::from(appimage);
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755));
            // mv поверх работающего файла допустим, поэтому меняем уже после выхода
            let script = format!(
                "sleep 1; mv -f '{}' '{}'; exec '{}'",
                file.display(),
                appimage.display(),
                appimage.display()
            );
            std::process::Command::new("sh")
                .arg("-c")
                .arg(&script)
                .spawn()
                .map_err(|e| format!("перезапуск: {e}"))?;
            return Ok(());
        }
    }
    let dir = file.parent().unwrap_or(Path::new("."));
    std::process::Command::new("xdg-open")
        .arg(dir)
        .spawn()
        .map_err(|e| format!("открытие папки: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_compares_segments() {
        assert!(newer("0.1.0", "0.2.0"));
        assert!(newer("0.1.9", "0.2.0"));
        assert!(newer("0.2.9", "0.2.10"));
        assert!(newer("0.1.0", "1.0.0"));
        assert!(!newer("0.2.0", "0.1.0"));
        assert!(!newer("0.2.0", "0.2.0"));
        assert!(!newer("0.2.0", "v0.2.0"));
        assert!(newer("0.2.0", "v0.3.0"));
    }

    #[test]
    fn newer_ignores_garbage() {
        assert!(newer("0.1.0", "0.2.0-beta"));
        assert!(!newer("abc", "abc"));
    }
}
