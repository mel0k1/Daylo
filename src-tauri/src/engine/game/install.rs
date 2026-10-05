use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use serde::Serialize;
use tauri::Emitter;

use super::super::core::http;
use super::super::core::paths;
use super::mcmeta::{self, Artifact, Library, VersionJson};

#[derive(Clone, Serialize)]
pub struct InstallProgress {
    pub version: String,
    pub stage: String,
    pub done: u64,
    pub total: u64,
    pub error: Option<String>,
}

struct Reporter {
    app: tauri::AppHandle,
    version: String,
    last: AtomicUsize,
}

impl Reporter {
    fn new(app: tauri::AppHandle, version: &str) -> Self {
        Self {
            app,
            version: version.into(),
            last: AtomicUsize::new(usize::MAX),
        }
    }

    // Не спамим событиями: каждые 25 файлов и на границах этапа
    fn emit(&self, stage: &str, done: u64, total: u64) {
        let n = done as usize;
        if n != self.last.swap(n, Ordering::Relaxed) && n % 25 != 0 && done < total {
            return;
        }
        let _ = self.app.emit(
            "install-progress",
            InstallProgress {
                version: self.version.clone(),
                stage: stage.into(),
                done,
                total,
                error: None,
            },
        );
    }
}

fn fail(rep: &Reporter, e: String) -> String {
    let _ = rep.app.emit(
        "install-progress",
        InstallProgress {
            version: rep.version.clone(),
            stage: "ошибка".into(),
            done: 0,
            total: 1,
            error: Some(e.clone()),
        },
    );
    e
}

// Ставит версию целиком: клиент, библиотеки, нативы, ассеты
pub async fn ensure_version(
    app: tauri::AppHandle,
    version_id: &str,
) -> Result<VersionJson, String> {
    let rep = Arc::new(Reporter::new(app, version_id));
    let vjson = match version_json(version_id).await {
        Ok(v) => v,
        Err(e) => return Err(fail(&rep, e)),
    };

    rep.emit("клиент", 0, 1);
    if let Err(e) = client_jar(version_id, &vjson).await {
        return Err(fail(&rep, e));
    }
    rep.emit("клиент", 1, 1);

    if let Err(e) = libraries(&rep, &vjson).await {
        return Err(fail(&rep, e));
    }
    if let Err(e) = natives(version_id, &vjson).await {
        return Err(fail(&rep, e));
    }
    if let Err(e) = assets(&rep, version_id, &vjson).await {
        return Err(fail(&rep, e));
    }
    rep.emit("готово", 1, 1);
    Ok(vjson)
}

pub async fn version_json(version_id: &str) -> Result<VersionJson, String> {
    let dir = paths::version_dir(version_id);
    let file = dir.join(format!("{version_id}.json"));
    let body = match tokio::fs::read_to_string(&file).await {
        Ok(b) => b,
        Err(_) => {
            let url = mcmeta::version_url(version_id).await?;
            let b = http::text(&url).await?;
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| format!("папка версии: {e}"))?;
            tokio::fs::write(&file, &b)
                .await
                .map_err(|e| format!("сохранение version json: {e}"))?;
            b
        }
    };
    serde_json::from_str(&body).map_err(|e| format!("разбор version json: {e}"))
}

async fn client_jar(version_id: &str, v: &VersionJson) -> Result<(), String> {
    let c = v
        .downloads
        .client
        .as_ref()
        .ok_or("в version json нет клиента")?;
    let to = paths::version_dir(version_id).join(format!("{version_id}.jar"));
    http::download(&c.url, &to, c.sha1.as_deref(), None, c.size).await
}

fn allowed_libraries(v: &VersionJson) -> Vec<&Library> {
    v.libraries
        .iter()
        .filter(|l| mcmeta::rules_allow(&l.rules, &[]))
        .collect()
}

async fn libraries(rep: &Reporter, v: &VersionJson) -> Result<(), String> {
    let libs = allowed_libraries(v);
    let total = libs.len() as u64;
    rep.emit("библиотеки", 0, total);

    let tasks: Vec<_> = libs
        .iter()
        .filter_map(|lib| {
            lib.downloads
                .as_ref()?
                .artifact
                .as_ref()
                .map(|art| (lib, art))
        })
        .map(|(lib, art)| {
            let path = art
                .path
                .clone()
                .unwrap_or_else(|| mcmeta::maven_path(&lib.name));
            let to = paths::libraries_dir().join(path);
            let url = art.url.clone();
            let sha1 = art.sha1.clone();
            let size = art.size;
            async move { http::download(&url, &to, sha1.as_deref(), None, size).await }
        })
        .collect();

    let mut stream = futures::stream::iter(tasks).buffer_unordered(24);
    let mut count: u64 = 0;
    let mut errors: Vec<String> = Vec::new();
    while let Some(res) = stream.next().await {
        count += 1;
        rep.emit("библиотеки", count, total);
        if let Err(e) = res {
            errors.push(e);
        }
    }
    if !errors.is_empty() {
        return Err(format!(
            "библиотеки: {} не скачалось, первая ошибка: {}",
            errors.len(),
            errors[0]
        ));
    }
    Ok(())
}

// Нативные библиотеки текущей ОС распаковываются в versions/<id>/natives
async fn natives(version_id: &str, v: &VersionJson) -> Result<(), String> {
    let os = mcmeta::current_os();
    let dest = natives_dir(version_id, v);
    tokio::fs::create_dir_all(&dest)
        .await
        .map_err(|e| format!("папка нативов: {e}"))?;

    for lib in allowed_libraries(v) {
        let classifier = match lib.natives.as_ref().and_then(|n| n.get(os)) {
            Some(c) => c.clone(),
            None => continue,
        };
        let art: Artifact = lib
            .downloads
            .as_ref()
            .and_then(|d| d.classifiers.as_ref())
            .and_then(|c| c.get(&classifier))
            .cloned()
            .unwrap_or_else(|| fallback_native(&lib.name, &classifier));
        let path = art
            .path
            .clone()
            .unwrap_or_else(|| mcmeta::maven_path(&format!("{}:{}", lib.name, classifier)));
        let jar = paths::libraries_dir().join(&path);
        http::download(&art.url, &jar, art.sha1.as_deref(), None, art.size).await?;
        let exclude = lib
            .extract
            .as_ref()
            .map(|e| e.exclude.clone())
            .unwrap_or_default();
        extract_native(&jar, &dest, exclude)?;
    }
    Ok(())
}

// Старые версии кладут классификатор только в координатах, без classifiers
fn fallback_native(name: &str, classifier: &str) -> Artifact {
    let path = mcmeta::maven_path(&format!("{name}:{classifier}"));
    Artifact {
        path: Some(path.clone()),
        url: format!("https://libraries.minecraft.net/{path}"),
        sha1: None,
        size: None,
    }
}

fn extract_native(jar: &Path, dest: &Path, exclude: Vec<String>) -> Result<(), String> {
    let file = std::fs::File::open(jar).map_err(|e| format!("натив {}: {e}", jar.display()))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("натив {}: {e}", jar.display()))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if entry.is_dir() || exclude.iter().any(|x| name.starts_with(x.as_str())) {
            continue;
        }
        let out = dest.join(&name);
        if !out.starts_with(dest) {
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

async fn assets(rep: &Reporter, version_id: &str, v: &VersionJson) -> Result<(), String> {
    let idx = match &v.asset_index {
        Some(i) => i,
        None => return Ok(()),
    };
    let index_file = paths::assets_dir()
        .join("indexes")
        .join(format!("{}.json", idx.id));
    let cached_ok = tokio::fs::read_to_string(&index_file)
        .await
        .ok()
        .filter(|_| {
            idx.sha1.as_deref().map_or(true, |want| {
                http::sha1_of(&index_file)
                    .map(|got| got.eq_ignore_ascii_case(want))
                    .unwrap_or(false)
            })
        });
    let body = match cached_ok {
        Some(b) => b,
        None => {
            let b = http::text(&idx.url).await?;
            tokio::fs::create_dir_all(index_file.parent().unwrap())
                .await
                .map_err(|e| format!("папка индексов: {e}"))?;
            tokio::fs::write(&index_file, &b)
                .await
                .map_err(|e| format!("сохранение индекса: {e}"))?;
            b
        }
    };
    let index: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("разбор asset index: {e}"))?;
    let objects = index
        .get("objects")
        .and_then(|o| o.as_object())
        .ok_or("asset index без objects")?;

    let mut need: Vec<String> = Vec::new();
    for obj in objects.values() {
        let hash = obj
            .get("hash")
            .and_then(|h| h.as_str())
            .ok_or("объект без хеша")?
            .to_string();
        let file = paths::assets_dir()
            .join("objects")
            .join(format!("{}/{}", &hash[..2], hash));
        if !file.exists() {
            need.push(hash);
        }
    }
    let total = need.len() as u64;
    rep.emit("ассеты", 0, total);

    let base = "https://resources.download.minecraft.net";
    let done = Arc::new(AtomicUsize::new(0));
    let tasks: Vec<_> = need
        .into_iter()
        .map(|hash| {
            let url = format!("{base}/{}/{}", &hash[..2], hash);
            let to = paths::assets_dir()
                .join("objects")
                .join(format!("{}/{}", &hash[..2], hash));
            let done = done.clone();
            async move {
                let r = http::download(&url, &to, Some(&hash), None, None).await;
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                (n, r)
            }
        })
        .collect();

    let mut stream = futures::stream::iter(tasks).buffer_unordered(48);
    let mut errors: Vec<String> = Vec::new();
    while let Some((n, res)) = stream.next().await {
        rep.emit("ассеты", n as u64, total);
        if let Err(e) = res {
            errors.push(e);
        }
    }
    if !errors.is_empty() {
        return Err(format!(
            "ассеты: {} не скачалось, первая ошибка: {}",
            errors.len(),
            errors[0]
        ));
    }

    // Старые индексы требуют копию в assets/virtual/<id>
    if index.get("virtual").and_then(|x| x.as_bool()) == Some(true) {
        let virtual_dir = paths::assets_dir().join("virtual").join(&idx.id);
        for (name, obj) in objects {
            let hash = obj.get("hash").and_then(|h| h.as_str()).unwrap_or_default();
            let src = paths::assets_dir()
                .join("objects")
                .join(format!("{}/{}", &hash[..2], hash));
            let dst = virtual_dir.join(name);
            if src.exists() && !dst.exists() {
                let _ = tokio::fs::create_dir_all(dst.parent().unwrap_or(Path::new("."))).await;
                let _ = tokio::fs::copy(&src, &dst).await;
            }
        }
    }

    let _ = version_id;
    Ok(())
}

// Готовый classpath из разрешённых библиотек и клиента
pub fn classpath(v: &VersionJson, version_id: &str) -> Vec<PathBuf> {
    let mut cp: Vec<PathBuf> = Vec::new();
    for lib in allowed_libraries(v) {
        if let Some(art) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            let path = art
                .path
                .clone()
                .unwrap_or_else(|| mcmeta::maven_path(&lib.name));
            cp.push(paths::libraries_dir().join(path));
        }
    }
    cp.push(paths::version_dir(version_id).join(format!("{version_id}.jar")));
    cp
}

// Папка с распакованными нативами версии
pub fn natives_dir(version_id: &str, _v: &VersionJson) -> PathBuf {
    paths::version_dir(version_id).join("natives")
}
