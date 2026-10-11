use std::path::{Path, PathBuf};

use serde::Serialize;

use super::super::core::paths;
use super::super::core::settings;

#[derive(Serialize, Clone)]
pub struct World {
    pub name: String,
    pub modified: u64,
    pub size: u64,
}

#[derive(Serialize, Clone)]
pub struct Backup {
    pub file: String,
    pub world: String,
    pub created: u64,
    pub size: u64,
}

fn saves_dir(version: &str) -> PathBuf {
    paths::instance_dir(version).join("saves")
}

fn backups_dir(version: &str) -> PathBuf {
    paths::instance_dir(version).join("backups")
}

fn epoch_secs(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Имя мира: одна папка внутри saves, без обхода путей и скрытых
pub fn world_ok(name: &str) -> bool {
    let p = Path::new(name);
    let plain =
        p.file_name().map(|f| f.to_string_lossy()) == Some(std::borrow::Cow::Borrowed(name));
    // бэкслеш режем всегда: на Unix он легален, но мир уедет и в zip, и на Windows
    plain && !name.is_empty() && !name.starts_with('.') && name != ".." && !name.contains('\\')
}

// Имя файла бэкапа: plain-файл с расширением .zip
fn backup_ok(name: &str) -> bool {
    let p = Path::new(name);
    let plain =
        p.file_name().map(|f| f.to_string_lossy()) == Some(std::borrow::Cow::Borrowed(name));
    plain && name.ends_with(".zip") && !name.starts_with('.')
}

// «Мир-1728652800.zip» -> («Мир», 1728652800); без метки времени -> created = 0
fn parse_backup_name(file: &str) -> (String, u64) {
    let stem = file.strip_suffix(".zip").unwrap_or(file);
    match stem.rsplit_once('-') {
        Some((w, ts)) if !ts.is_empty() && ts.chars().all(|c| c.is_ascii_digit()) => {
            (w.to_string(), ts.parse().unwrap_or(0))
        }
        _ => (stem.to_string(), 0),
    }
}

// Рекурсивный обход файлов (блокирующий, звать из spawn_blocking)
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.is_file() {
            out.push(p);
        }
    }
}

fn dir_size(dir: &Path) -> u64 {
    let mut files = Vec::new();
    walk(dir, &mut files);
    files
        .iter()
        .filter_map(|f| std::fs::metadata(f).ok())
        .map(|m| m.len())
        .sum()
}

// Список миров инстанса: папки внутри saves
pub async fn worlds(version: &str) -> Vec<World> {
    let mut out: Vec<(String, u64, PathBuf)> = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(saves_dir(version)).await else {
        return Vec::new();
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        let path = e.path();
        if !path.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if !world_ok(&name) {
            continue;
        }
        let modified = e
            .metadata()
            .await
            .ok()
            .and_then(|m| m.modified().ok())
            .map(epoch_secs)
            .unwrap_or(0);
        out.push((name, modified, path));
    }
    // Размеры считаются одним блокирующим проходом
    let dirs: Vec<PathBuf> = out.iter().map(|(_, _, p)| p.clone()).collect();
    let sizes =
        tokio::task::spawn_blocking(move || dirs.iter().map(|d| dir_size(d)).collect::<Vec<u64>>())
            .await
            .unwrap_or_default();
    let mut worlds: Vec<World> = out
        .iter()
        .zip(sizes)
        .map(|((name, modified, _), size)| World {
            name: name.clone(),
            modified: *modified,
            size,
        })
        .collect();
    worlds.sort_by_key(|w| std::cmp::Reverse(w.modified));
    worlds
}

// Запаковывает мир в backups/<мир>-<epoch>.zip, возвращает метаданные бэкапа.
// После копии срабатывает ретенция: остаются keep свежайших (из настроек)
pub async fn create(version: &str, world: &str) -> Result<Backup, String> {
    if !world_ok(world) {
        return Err("плохое имя мира".into());
    }
    let src = saves_dir(version).join(world);
    if !src.is_dir() {
        return Err("мира нет в saves".into());
    }
    let d = backups_dir(version);
    tokio::fs::create_dir_all(&d)
        .await
        .map_err(|e| format!("папка бэкапов: {e}"))?;
    let dst_name = format!("{world}-{}.zip", epoch_secs(std::time::SystemTime::now()));
    let dst = d.join(&dst_name);
    let src2 = src.clone();
    let dst2 = dst.clone();
    tokio::task::spawn_blocking(move || zip_dir(&src2, &dst2))
        .await
        .map_err(|e| format!("поток: {e}"))??;
    let meta = tokio::fs::metadata(&dst).await.map_err(|e| e.to_string())?;
    let keep = settings::load().backup_keep;
    prune(version, world, keep).await;
    Ok(Backup {
        file: dst_name,
        world: world.to_string(),
        created: meta.modified().ok().map(epoch_secs).unwrap_or(0),
        size: meta.len(),
    })
}

// Максимальное время изменения файла в мире (блокирующий обход, звать из
// spawn_blocking): метка свежайшей правки для решения «нужен ли бэкап»
fn world_mtime_max(dir: &Path) -> u64 {
    let mut files = Vec::new();
    walk(dir, &mut files);
    files
        .iter()
        .filter_map(|f| std::fs::metadata(f).ok())
        .filter_map(|m| m.modified().ok())
        .map(epoch_secs)
        .max()
        .unwrap_or(0)
}

// Мир менялся после свежайшего бэкапа? Нет бэкапов — считаем что нужен
fn world_needs_backup(max_mtime: u64, latest: u64) -> bool {
    latest == 0 || max_mtime > latest
}

// Ретенция: оставляем keep свежайших копий мира, старшие удаляем.
// Касается только *.zip — папки keep-* от восстановления не трогаются
async fn prune(version: &str, world: &str, keep: u32) {
    let stale: Vec<String> = list(version)
        .await
        .into_iter()
        .filter(|b| b.world == world)
        .skip(keep as usize)
        .map(|b| b.file)
        .collect();
    for f in stale {
        let _ = delete(version, &f).await;
    }
}

// Метка времени из имени keep-папки (создана при восстановлении).
// mtime папки не годится — это время её содержимого, оно древнее
fn keep_dir_epoch(name: &str) -> Option<u64> {
    if !name.starts_with("keep-") {
        return None;
    }
    name.rsplit_once('-')
        .and_then(|(_, ts)| if ts.is_empty() { None } else { ts.parse().ok() })
}

// Папки keep-* живут неделю по метке в имени, потом вон
async fn prune_keeps(version: &str) {
    let Ok(mut rd) = tokio::fs::read_dir(backups_dir(version)).await else {
        return;
    };
    let week = 7 * 24 * 3600;
    let now = epoch_secs(std::time::SystemTime::now());
    while let Ok(Some(e)) = rd.next_entry().await {
        if !e.path().is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(ts) = keep_dir_epoch(&name) {
            if ts + week < now {
                let _ = tokio::fs::remove_dir_all(e.path()).await;
            }
        }
    }
}

// Автобэкап перед запуском: архивируем только миры, изменившиеся после
// свежайшей копии. Ошибки отдельных миров не мешают остальным и запуску,
// возвращаем число сделанных копий
pub async fn auto_backup(version: &str) -> usize {
    prune_keeps(version).await;
    let mut names: Vec<String> = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(saves_dir(version)).await else {
        return 0;
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        if !e.path().is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if world_ok(&name) {
            names.push(name);
        }
    }
    let mut created = 0usize;
    for w in names {
        let latest = list(version)
            .await
            .into_iter()
            .filter(|b| b.world == w)
            .map(|b| b.created)
            .max()
            .unwrap_or(0);
        let dir = saves_dir(version).join(&w);
        let max = tokio::task::spawn_blocking(move || world_mtime_max(&dir))
            .await
            .unwrap_or(0);
        if !world_needs_backup(max, latest) {
            continue;
        }
        if create(version, &w).await.is_ok() {
            created += 1;
        }
    }
    created
}

fn zip_dir(src: &Path, dst: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    walk(src, &mut files);
    let file = std::fs::File::create(dst).map_err(|e| format!("создание архива: {e}"))?;
    let mut zw = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    for f in &files {
        let rel = f
            .strip_prefix(src)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let data = std::fs::read(f).map_err(|e| format!("чтение {}: {e}", rel))?;
        zw.start_file(rel, opts).map_err(|e| e.to_string())?;
        use std::io::Write;
        zw.write_all(&data).map_err(|e| e.to_string())?;
    }
    zw.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn resolve_backup(version: &str, file: &str) -> Result<PathBuf, String> {
    if !backup_ok(file) {
        return Err("плохое имя файла".into());
    }
    Ok(backups_dir(version).join(file))
}

fn unzip_to(archive: &Path, dest: &Path) -> Result<(), String> {
    let f = std::fs::File::open(archive).map_err(|e| format!("открытие архива: {e}"))?;
    let mut za = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    for i in 0..za.len() {
        let mut e = za.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = e.enclosed_name() else {
            continue; // пути с .. пропускаем
        };
        let target = dest.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(p) = target.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut e, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Восстановление: текущая папка мира (если есть) уезжает в backups/keep-*,
// содержимое архива распаковывается в saves/<мир>
pub async fn restore(version: &str, file: &str) -> Result<(), String> {
    let path = resolve_backup(version, file)?;
    let (world, _) = parse_backup_name(file);
    if !world_ok(&world) {
        return Err("плохое имя мира в архиве".into());
    }
    let target = saves_dir(version).join(&world);
    if target.exists() {
        let keep = backups_dir(version).join(format!(
            "keep-{}-{}",
            world,
            epoch_secs(std::time::SystemTime::now())
        ));
        tokio::fs::rename(&target, &keep)
            .await
            .map_err(|e| format!("сохранение старого мира: {e}"))?;
    }
    tokio::fs::create_dir_all(&target)
        .await
        .map_err(|e| format!("папка мира: {e}"))?;
    let path2 = path.clone();
    tokio::task::spawn_blocking(move || unzip_to(&path2, &target))
        .await
        .map_err(|e| format!("поток: {e}"))??;
    Ok(())
}

pub async fn delete(version: &str, file: &str) -> Result<(), String> {
    let path = resolve_backup(version, file)?;
    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| format!("удаление: {e}"))
}

// Список бэкапов: *.zip в backups, свежие сверху
pub async fn list(version: &str) -> Vec<Backup> {
    let mut out = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(backups_dir(version)).await else {
        return out;
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        if !e.path().is_file() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if !backup_ok(&name) {
            continue;
        }
        let Ok(meta) = e.metadata().await else {
            continue;
        };
        let (world, created) = parse_backup_name(&name);
        out.push(Backup {
            file: name,
            world,
            created: if created > 0 {
                created
            } else {
                epoch_secs(meta.modified().unwrap_or(std::time::UNIX_EPOCH))
            },
            size: meta.len(),
        });
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.created));
    out
}

// Показывает папку бэкапов в проводнике
pub async fn open_dir(version: &str) -> Result<(), String> {
    let d = backups_dir(version);
    tokio::fs::create_dir_all(&d)
        .await
        .map_err(|e| format!("папка бэкапов: {e}"))?;
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
    fn world_names_guarded() {
        assert!(world_ok("My World"));
        assert!(world_ok("Мир-1"));
        assert!(!world_ok("../evil"));
        assert!(!world_ok("a/b"));
        assert!(!world_ok("a\\b"));
        assert!(!world_ok(".hidden"));
        assert!(!world_ok(".."));
        assert!(!world_ok(""));
    }

    #[test]
    fn backup_names_guarded() {
        assert!(backup_ok("Мир-1-1728652800.zip"));
        assert!(!backup_ok("Мир.zip/../x.zip"));
        assert!(!backup_ok(".hid.zip"));
        assert!(!backup_ok("notzip.txt"));
    }

    #[test]
    fn parse_keeps_world_with_dashes() {
        assert_eq!(
            parse_backup_name("My World-1728652800.zip"),
            ("My World".into(), 1728652800)
        );
        assert_eq!(
            parse_backup_name("hardcore--42.zip"),
            ("hardcore-".into(), 42)
        );
        // без метки времени — весь стем, created = 0
        assert_eq!(parse_backup_name("wanderer.zip"), ("wanderer".into(), 0));
    }

    #[test]
    fn backup_needed_only_after_changes() {
        // нет копий — бэкапим всегда
        assert!(world_needs_backup(0, 0));
        assert!(world_needs_backup(1, 0));
        // мир не менялся после копии (и даже ровно в момент копии)
        assert!(!world_needs_backup(100, 200));
        assert!(!world_needs_backup(200, 200));
        // изменился после копии
        assert!(world_needs_backup(300, 200));
    }

    #[test]
    fn keep_epoch_from_name() {
        assert_eq!(keep_dir_epoch("keep-Мир-1728652800"), Some(1728652800));
        assert_eq!(keep_dir_epoch("keep-My World-42"), Some(42));
        assert_eq!(keep_dir_epoch("keep-x"), None);
        assert_eq!(keep_dir_epoch("keep-"), None);
        assert_eq!(keep_dir_epoch("Мир-100"), None);
    }
}
