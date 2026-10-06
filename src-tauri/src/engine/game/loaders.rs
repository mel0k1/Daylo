use serde::Serialize;
use tauri::Emitter;

use super::super::core::{http, paths};
use super::install;
use super::java;

const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
const FORGE_META: &str = "https://files.minecraftforge.net/net/minecraftforge/forge";
const NEOFORGE_META: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

#[derive(Serialize, Clone)]
pub struct LoaderBuild {
    pub version: String,
    pub stable: bool,
    pub recommended: bool,
}

// Список сборок загрузчика под версию игры, свежие сверху
pub async fn list_builds(loader: &str, mc: &str) -> Result<Vec<LoaderBuild>, String> {
    match loader {
        "fabric" => fabric_builds(mc).await,
        "forge" => forge_builds(mc).await,
        "neoforge" => neoforge_builds(mc).await,
        _ => Err(format!("неизвестный загрузчик {loader}")),
    }
}

async fn fabric_builds(mc: &str) -> Result<Vec<LoaderBuild>, String> {
    let url = format!("{FABRIC_META}/versions/loader/{mc}");
    let raw: serde_json::Value = serde_json::from_str(&http::text(&url).await?)
        .map_err(|e| format!("ответ meta.fabricmc: {e}"))?;
    let arr = raw.as_array().cloned().unwrap_or_default();
    let mut out: Vec<LoaderBuild> = arr
        .iter()
        .filter_map(|l| {
            let version = l["loader"]["version"].as_str()?.to_string();
            let stable = l["loader"]["stable"].as_bool().unwrap_or(false);
            Some(LoaderBuild {
                version,
                stable,
                recommended: false,
            })
        })
        .collect();
    if let Some(i) =
        out.iter()
            .position(|b| b.stable)
            .or(if out.is_empty() { None } else { Some(0) })
    {
        out[i].recommended = true;
    }
    Ok(out)
}

// Forge перечисляет сборки как "<mc>-<build>", у старых "<build>-<mc>" в хвосте
async fn forge_builds(mc: &str) -> Result<Vec<LoaderBuild>, String> {
    let url = format!("{FORGE_META}/maven-metadata.json");
    let raw: serde_json::Value = serde_json::from_str(&http::text(&url).await?)
        .map_err(|e| format!("ответ forge meta: {e}"))?;
    let mut builds: Vec<String> = raw[mc]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(String::from))
        .filter(|full| full.starts_with(&format!("{mc}-")))
        .map(|full| full[mc.len() + 1..].to_string())
        .collect();
    builds.reverse();
    let promo_url = format!("{FORGE_META}/promotions_slim.json");
    let promos: serde_json::Value = serde_json::from_str(&http::text(&promo_url).await?)
        .map_err(|e| format!("ответ forge promos: {e}"))
        .unwrap_or(serde_json::Value::Null);
    let rec = promos["promos"][format!("{mc}-recommended")]
        .as_str()
        .or_else(|| promos["promos"][format!("{mc}-latest")].as_str())
        .map(String::from);
    Ok(builds
        .into_iter()
        .map(|b| LoaderBuild {
            recommended: rec.as_deref() == Some(b.as_str()),
            stable: true,
            version: b,
        })
        .collect())
}

// NeoForge именует сборки как "<mc без ведущей единицы>.<build>", годовые версии — как есть
fn neoforge_prefixes(mc: &str) -> Vec<String> {
    let stripped = mc.strip_prefix("1.").unwrap_or(mc);
    let mut out = vec![format!("{stripped}.")];
    if stripped != mc {
        out.push(format!("{mc}-"));
    }
    out
}

fn neoforge_is_stable(v: &str) -> bool {
    !v.ends_with("-beta") && !v.ends_with("-alpha") && !v.ends_with("-rc")
}

async fn neoforge_builds(mc: &str) -> Result<Vec<LoaderBuild>, String> {
    let raw: serde_json::Value = serde_json::from_str(&http::text(NEOFORGE_META).await?)
        .map_err(|e| format!("ответ meta.neoforged: {e}"))?;
    let list: Vec<String> = raw["versions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let cands: Vec<String> = list
        .into_iter()
        .filter(|v| neoforge_prefixes(mc).iter().any(|p| v.starts_with(p)))
        .collect();
    if cands.is_empty() {
        return Err(format!("NeoForge для {mc} не выпускался"));
    }
    let rec = cands
        .iter()
        .find(|v| neoforge_is_stable(v))
        .or_else(|| cands.first())
        .cloned();
    // API отдаёт старые первыми — переворачиваем
    let mut out: Vec<LoaderBuild> = cands
        .into_iter()
        .rev()
        .map(|v| {
            let stable = neoforge_is_stable(&v);
            LoaderBuild {
                recommended: rec.as_deref() == Some(v.as_str()),
                stable,
                version: v,
            }
        })
        .collect();
    out.sort_by_key(|b| std::cmp::Reverse(num_key(&b.version)));
    Ok(out)
}

// Сравнение сборок по числовым сегментам: 21.1.100 > 21.1.99
fn num_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit())
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .collect()
}

// Установка Fabric: profile json с meta, клиент подтягивается через inheritsFrom
async fn install_fabric(mc: &str, build: &str) -> Result<String, String> {
    let url = format!("{FABRIC_META}/versions/loader/{mc}/{build}/profile/json");
    let body = http::text(&url).await?;
    let raw: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("ответ meta.fabricmc: {e}"))?;
    let id = raw["id"]
        .as_str()
        .ok_or("у профиля Fabric нет id")?
        .to_string();
    let _ = install::version_json(mc).await?;
    let dir = paths::version_dir(&id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("папка профиля: {e}"))?;
    tokio::fs::write(dir.join(format!("{id}.json")), body)
        .await
        .map_err(|e| format!("сохранение профиля: {e}"))?;
    Ok(id)
}

// Java для инсталлера и игры по версии Minecraft
fn java_major_for(mc: &str) -> u32 {
    let p: Vec<u32> = mc.split('.').filter_map(|s| s.parse().ok()).collect();
    if p.first() != Some(&1) {
        return 21;
    }
    let minor = p.get(1).copied().unwrap_or(0);
    let patch = p.get(2).copied().unwrap_or(0);
    if minor > 20 || (minor == 20 && patch >= 5) {
        21
    } else if minor >= 18 {
        17
    } else if minor == 17 {
        16
    } else {
        8
    }
}

fn installer_url(loader: &str, full: &str) -> String {
    match loader {
        "forge" => format!("{FORGE_MAVEN}/{full}/forge-{full}-installer.jar"),
        _ => format!("{NEOFORGE_MAVEN}/{full}/neoforge-{full}-installer.jar"),
    }
}

// Установка Forge/NeoForge: официальный инсталлер, запущенный нашей Java
async fn install_installer(
    app: tauri::AppHandle,
    loader: &str,
    mc: &str,
    build: &str,
) -> Result<String, String> {
    let url = installer_url(loader, build);
    let cache = paths::data_dir().join("cache").join("loaders");
    let jar = cache.join(format!("{loader}-{build}-installer.jar"));

    let _ = emit_stage(&app, mc, "загрузчик: скачивание инсталлера");
    // Сумма берётся с maven; для инсталлера это обязательная проверка
    let sha1 = match http::text(&format!("{url}.sha1")).await {
        Ok(s) => s
            .trim()
            .split_whitespace()
            .next()
            .filter(|s| s.len() == 40)
            .map(String::from),
        Err(_) => None,
    };
    http::download(&url, &jar, sha1.as_deref(), None, None).await?;

    let _ = emit_stage(&app, mc, "загрузчик: java");
    let _ = install::version_json(mc).await?;
    let java_bin = java::ensure_java(java_major_for(mc)).await?;

    let root = paths::game_root();
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|e| format!("папка игры: {e}"))?;
    let before = profile_snapshot();

    let _ = emit_stage(&app, mc, "загрузчик: работа инсталлера");
    let mut cmd = tokio::process::Command::new(&java_bin);
    cmd.arg("-jar")
        .arg(&jar)
        .arg("--installClient")
        .arg(&root)
        .current_dir(&root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd
        .output()
        .await
        .map_err(|e| format!("запуск инсталлера: {e}"))?;
    if !out.status.success() {
        let tail: String = String::from_utf8_lossy(&out.stderr)
            .lines()
            .rev()
            .take(6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "инсталлер {loader} завершился с кодом {:?}: {tail}",
            out.status.code()
        ));
    }

    let id = newest_profile(&before).ok_or("инсталлер не записал version json")?;
    let _ = tokio::fs::remove_file(&jar).await;
    Ok(id)
}

fn emit_stage(app: &tauri::AppHandle, version: &str, stage: &str) {
    let _ = app.emit(
        "install-progress",
        install::InstallProgress {
            version: version.to_string(),
            stage: stage.to_string(),
            done: 0,
            total: 1,
            error: None,
        },
    );
}

// Кэп snapshot json-профилей в versions/, чтобы найти созданный инсталлером
fn profile_snapshot() -> Vec<(std::path::PathBuf, std::time::SystemTime)> {
    list_profile_jsons()
        .into_iter()
        .filter_map(|p| {
            let t = p.metadata().ok()?.modified().ok()?;
            Some((p, t))
        })
        .collect()
}

fn list_profile_jsons() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(paths::versions_dir()) else {
        return out;
    };
    for entry in rd.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&dir) else {
            continue;
        };
        for f in files.flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "json") {
                out.push(p);
            }
        }
    }
    out
}

fn newest_profile(before: &[(std::path::PathBuf, std::time::SystemTime)]) -> Option<String> {
    let mut best: Option<(std::time::SystemTime, String)> = None;
    for p in list_profile_jsons() {
        let already = before.iter().any(|(b, _)| b == &p);
        if already {
            continue;
        }
        let t = p.metadata().ok()?.modified().ok()?;
        if best.as_ref().is_none_or(|(bt, _)| t > *bt) {
            best = Some((t, p.file_stem()?.to_string_lossy().to_string()));
        }
    }
    best.map(|(_, id)| id)
}

// Ставит загрузчик, возвращает id созданного профиля
pub async fn install(
    app: tauri::AppHandle,
    loader: &str,
    mc: &str,
    build: &str,
) -> Result<String, String> {
    let id = match loader {
        "fabric" => install_fabric(mc, build).await,
        "forge" | "neoforge" => install_installer(app.clone(), loader, mc, build).await,
        _ => Err(format!("неизвестный загрузчик {loader}")),
    }?;
    let _ = app.emit(
        "install-progress",
        install::InstallProgress {
            version: mc.to_string(),
            stage: "готово".into(),
            done: 1,
            total: 1,
            error: None,
        },
    );
    Ok(id)
}

// Локальные профили загрузчиков: id, тег загрузчика и базовая версия игры
pub async fn local_profiles() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for p in list_profile_jsons() {
        let Some(id) = p.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        let Ok(body) = tokio::fs::read_to_string(&p).await else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
            continue;
        };
        if v["id"].as_str().is_none() {
            continue;
        }
        let base = v["inheritsFrom"].as_str().unwrap_or(&id).to_string();
        let loader = loader_tag(&id);
        if !loader.is_empty() {
            out.push((id, loader.to_string(), base));
        }
    }
    out.sort();
    out
}

// Тег загрузчика по id профиля
pub fn loader_tag(id: &str) -> &'static str {
    if id.starts_with("fabric-loader") {
        "fabric"
    } else if id.starts_with("neoforge") {
        "neoforge"
    } else if id.starts_with("forge") {
        "forge"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_prefixes_follow_mc() {
        assert_eq!(neoforge_prefixes("1.21.1"), vec!["21.1.", "1.21.1-"]);
        assert_eq!(neoforge_prefixes("26.2"), vec!["26.2."]);
    }

    #[test]
    fn neoforge_stability() {
        assert!(!neoforge_is_stable("26.2.0.12-beta"));
        assert!(neoforge_is_stable("21.1.233"));
    }

    #[test]
    fn java_major_by_mc() {
        assert_eq!(java_major_for("1.16.5"), 8);
        assert_eq!(java_major_for("1.17.1"), 16);
        assert_eq!(java_major_for("1.20.4"), 17);
        assert_eq!(java_major_for("1.20.5"), 21);
        assert_eq!(java_major_for("1.21.9"), 21);
        assert_eq!(java_major_for("26.1"), 21);
    }

    #[test]
    fn installer_urls() {
        assert_eq!(
            installer_url("forge", "1.20.1-47.4.0"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.0/forge-1.20.1-47.4.0-installer.jar"
        );
        assert_eq!(
            installer_url("neoforge", "21.1.233"),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.233/neoforge-21.1.233-installer.jar"
        );
    }

    #[test]
    fn num_key_sorts_numerically() {
        assert!(num_key("21.1.100") > num_key("21.1.99"));
    }
}
