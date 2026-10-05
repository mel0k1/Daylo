use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::super::core::http;
use super::super::core::paths;

// Java качается из Adoptium: JRE нужной мажорной версии
#[derive(Deserialize)]
struct Asset {
    binary: Binary,
}

#[derive(Deserialize)]
struct Binary {
    package: Pkg,
}

#[derive(Deserialize)]
struct Pkg {
    name: String,
    link: String,
    #[serde(default)]
    checksum: Option<String>,
}

fn os_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "mac",
        _ => "linux",
    }
}

fn ext() -> &'static str {
    if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    }
}

// Путь к java-бинарнику нужной мажорной версии, с докачкой при необходимости
pub async fn ensure_java(major: u32) -> Result<PathBuf, String> {
    let exe = if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    };
    let home = paths::java_dir().join(major.to_string()).join("jre");
    let bin = home.join("bin").join(exe);
    if bin.exists() {
        return Ok(bin);
    }

    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        _ => "x64",
    };
    let url = format!(
        "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?os={}&architecture={arch}&image_type=jre",
        os_name()
    );
    let list: Vec<Asset> = serde_json::from_str(&http::text(&url).await?)
        .map_err(|e| format!("ответ Adoptium: {e}"))?;
    let asset = list
        .iter()
        .find(|a| a.binary.package.name.ends_with(ext()))
        .ok_or("Adoptium не отдал подходящую сборку JRE")?;

    let root = paths::java_dir().join(major.to_string());
    let archive = root.join(&asset.binary.package.name);
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|e| format!("папка java: {e}"))?;
    http::download(
        &asset.binary.package.link,
        &archive,
        None,
        asset.binary.package.checksum.as_deref(),
        None,
    )
    .await?;

    let extract_dir = root.join("unpack");
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;
    if ext() == "zip" {
        unpack_zip(&archive, &extract_dir)?;
    } else {
        unpack_targz(&archive, &extract_dir)?;
    }

    // Архив лежит в подпапке вида jdk-21.x — переносим содержимое в jre/
    let inner = first_dir_with(&extract_dir, "bin")?;
    let _ = tokio::fs::remove_dir_all(&home).await;
    if tokio::fs::rename(&inner, &home).await.is_err() {
        copy_dir(&inner, &home)
            .await
            .map_err(|e| format!("перенос JRE: {e}"))?;
    }
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;
    let _ = tokio::fs::remove_file(&archive).await;

    if !cfg!(target_os = "windows") {
        use std::os::unix::fs::PermissionsExt;
        let _ = tokio::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).await;
    }
    if !bin.exists() {
        return Err("JRE распакована, но java-бинарник не найден".into());
    }
    Ok(bin)
}

fn first_dir_with(root: &Path, marker: &str) -> Result<PathBuf, String> {
    for entry in std::fs::read_dir(root).map_err(|e| format!("распаковка: {e}"))? {
        let p = entry.map_err(|e| e.to_string())?.path();
        if p.is_dir() && p.join(marker).exists() {
            return Ok(p);
        }
    }
    Err("в архиве JRE нет папки bin".into())
}

fn unpack_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("архив: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("архив: {e}"))?;
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let out = dest.join(entry.mangled_name());
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut w).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn unpack_targz(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("архив: {e}"))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);
    tar.unpack(dest).map_err(|e| format!("распаковка: {e}"))
}

async fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    tokio::fs::create_dir_all(to).await?;
    let mut rd = tokio::fs::read_dir(from).await?;
    while let Some(e) = rd.next_entry().await? {
        let dst = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &dst).await?;
        } else {
            tokio::fs::copy(e.path(), &dst).await?;
        }
    }
    Ok(())
}
