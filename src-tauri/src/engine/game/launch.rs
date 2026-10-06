use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use md5::{Digest, Md5};
use serde::Serialize;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use super::super::core::paths;
use super::config;
use super::install;
use super::java;
use super::mcmeta::{self, Arg, ArgValue, VersionJson};

#[derive(Clone, Serialize)]
pub struct LaunchProgress {
    pub version: String,
    pub stage: String,
}

#[derive(Clone, Serialize)]
pub struct GameLog {
    pub version: String,
    pub line: String,
}

#[derive(Clone, Serialize)]
pub struct GameExit {
    pub version: String,
    pub code: Option<i32>,
}

type Running = HashMap<String, Arc<Mutex<tokio::process::Child>>>;

fn running() -> &'static std::sync::Mutex<Running> {
    static RUNNING: std::sync::OnceLock<std::sync::Mutex<Running>> = std::sync::OnceLock::new();
    RUNNING.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

// UUID оффлайн-профиля, как в ваниле: md5 от "OfflinePlayer:<ник>"
fn offline_uuid(nick: &str) -> String {
    let mut h = Md5::new();
    h.update(format!("OfflinePlayer:{nick}").as_bytes());
    let mut d = [0u8; 16];
    d.copy_from_slice(&h.finalize());
    d[6] = (d[6] & 0x0f) | 0x30;
    d[8] = (d[8] & 0x3f) | 0x80;
    let hex: Vec<String> = d.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        hex[0..4].concat(),
        hex[4..6].concat(),
        hex[6..8].concat(),
        hex[8..10].concat(),
        hex[10..16].concat()
    )
}

fn subst(arg: &str, map: &HashMap<&str, String>) -> String {
    match map.get(arg) {
        Some(v) => v.clone(),
        None => arg.to_string(),
    }
}

// Разворачивает jvm/game аргументы из version json в плоский список
fn expand_args(args: &[Arg], map: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Arg::Plain(s) => out.push(subst(s, map)),
            Arg::Ruled { value, .. } => match value {
                ArgValue::One(s) => out.push(subst(s, map)),
                ArgValue::Many(list) => out.extend(list.iter().map(|s| subst(s, map))),
            },
        }
    }
    out
}

// Стартует игру: ставит недостающее, подбирает Java, запускает JVM
pub async fn launch(app: tauri::AppHandle, version_id: &str, nick: &str) -> Result<(), String> {
    let _ = app.emit(
        "launch-progress",
        LaunchProgress {
            version: version_id.into(),
            stage: "проверка файлов".into(),
        },
    );
    let v = install::ensure_version(app.clone(), version_id).await?;

    let _ = app.emit(
        "launch-progress",
        LaunchProgress {
            version: version_id.into(),
            stage: "java".into(),
        },
    );
    let major = v.java_version.as_ref().map(|j| j.major).unwrap_or(8);
    let java_bin = java::ensure_java(major).await?;

    let _ = app.emit(
        "launch-progress",
        LaunchProgress {
            version: version_id.into(),
            stage: "запуск".into(),
        },
    );

    let game_dir = paths::instance_dir(version_id);
    tokio::fs::create_dir_all(&game_dir)
        .await
        .map_err(|e| format!("папка сборки: {e}"))?;

    let sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let cp = install::classpath(&v, version_id);
    let cp_str = cp
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(sep);

    let assets_root = paths::assets_dir();
    let natives = install::natives_dir(version_id, &v);
    let uuid = offline_uuid(nick);

    let mut map: HashMap<&str, String> = HashMap::new();
    map.insert("auth_player_name", nick.into());
    map.insert("version_name", version_id.into());
    map.insert("game_directory", game_dir.to_string_lossy().to_string());
    map.insert("assets_root", assets_root.to_string_lossy().to_string());
    map.insert("assets_index_name", asset_index_name(&v, version_id));
    map.insert("auth_uuid", uuid);
    map.insert("auth_access_token", "0".into());
    map.insert("auth_xuid", "0".into());
    map.insert("clientid", "daylo".into());
    map.insert("user_type", "legacy".into());
    map.insert("version_type", v.kind.clone());
    map.insert("natives_directory", natives.to_string_lossy().to_string());
    map.insert("launcher_name", "Daylo".into());
    map.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    map.insert("classpath", cp_str);
    map.insert("classpath_separator", sep.into());
    map.insert(
        "library_directory",
        paths::libraries_dir().to_string_lossy().to_string(),
    );

    let cfg = config::load(version_id).await;

    // Правила с os учитываются заранее — expand_args уже отфильтровал их в install
    let features = [""];
    let mut jvm: Vec<String> = vec![
        "-Dlog4j2.formatMsgNoLookups=true".into(),
        format!("-Xmx{}M", cfg.ram_mb),
    ];
    match &v.arguments {
        Some(a) => {
            let ruled: Vec<Arg> = a
                .jvm
                .iter()
                .filter(|x| match x {
                    Arg::Ruled { rules, .. } => mcmeta::rules_allow(rules, &features),
                    Arg::Plain(_) => true,
                })
                .cloned()
                .collect();
            jvm.extend(expand_args(&ruled, &map));
        }
        None => {
            // До 1.13: фиксированный набор
            jvm.push(format!("-Djava.library.path={}", natives.to_string_lossy()));
            jvm.push("-cp".into());
            jvm.push(map.get("classpath").unwrap().clone());
        }
    }
    // Пользовательские флаги идут последними — HotSpot берёт последний -Xmx и т.п.
    jvm.extend(
        cfg.jvm_args
            .iter()
            .filter(|a| a.trim().starts_with('-'))
            .cloned(),
    );
    jvm.push(v.main_class.clone());

    let mut game_args: Vec<String> = match (&v.arguments, &v.legacy_args) {
        (Some(a), _) => {
            let ruled: Vec<Arg> = a
                .game
                .iter()
                .filter(|x| match x {
                    Arg::Ruled { rules, .. } => mcmeta::rules_allow(rules, &features),
                    Arg::Plain(_) => true,
                })
                .cloned()
                .collect();
            expand_args(&ruled, &map)
        }
        (None, Some(legacy)) => legacy.split_whitespace().map(|s| subst(s, &map)).collect(),
        (None, None) => Vec::new(),
    };
    if game_args.is_empty() {
        return Err("в version json нет игровых аргументов".into());
    }
    game_args.push("--width".into());
    game_args.push("1280".into());
    game_args.push("--height".into());
    game_args.push("720".into());

    let log_file = game_dir.join("logs");
    let _ = tokio::fs::create_dir_all(&log_file).await;
    let log_path = log_file.join("latest.txt");

    let mut cmd = tokio::process::Command::new(&java_bin);
    cmd.args(&jvm)
        .args(&game_args)
        .current_dir(&game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn().map_err(|e| format!("запуск java: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let child = Arc::new(Mutex::new(child));
    running()
        .lock()
        .map_err(|_| "реестр процессов")?
        .insert(version_id.to_string(), child.clone());

    let app_log = app.clone();
    let log_version = version_id.to_string();
    tauri::async_runtime::spawn(async move {
        read_stream(stdout, &app_log, &log_version, &log_path).await;
        read_stream(stderr, &app_log, &log_version, &log_path).await;
    });

    let app_exit = app.clone();
    let exit_version = version_id.to_string();
    tauri::async_runtime::spawn(async move {
        // Поллим, не держа лок: stop() должен успеть сделать kill()
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let code = {
                let mut guard = child.lock().await;
                match guard.try_wait() {
                    Ok(Some(status)) => status.code(),
                    Ok(None) => continue,
                    Err(_) => None,
                }
            };
            running()
                .lock()
                .ok()
                .and_then(|mut r| r.remove(&exit_version));
            let _ = app_exit.emit(
                "game-exit",
                GameExit {
                    version: exit_version,
                    code,
                },
            );
            break;
        }
    });
    Ok(())
}

fn asset_index_name(v: &VersionJson, version_id: &str) -> String {
    v.asset_index
        .as_ref()
        .map(|i| i.id.clone())
        .unwrap_or_else(|| version_id.to_string())
}

async fn read_stream(
    stream: Option<impl tokio::io::AsyncRead + Unpin>,
    app: &tauri::AppHandle,
    version: &str,
    log_path: &PathBuf,
) {
    let mut stream = match stream {
        Some(s) => s,
        None => return,
    };
    use tokio::io::AsyncReadExt;
    let mut buf = [0u8; 4096];
    let mut pending = Vec::new();
    loop {
        match stream.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                pending.extend_from_slice(&buf[..n]);
                while let Some(pos) = pending.iter().position(|b| *b == b'\n') {
                    let line: Vec<u8> = pending.drain(..=pos).collect();
                    let text = String::from_utf8_lossy(&line[..pos]).to_string();
                    append_log(log_path, &text).await;
                    let _ = app.emit(
                        "game-log",
                        GameLog {
                            version: version.into(),
                            line: text,
                        },
                    );
                }
            }
        }
    }
    if !pending.is_empty() {
        let text = String::from_utf8_lossy(&pending).to_string();
        append_log(log_path, &text).await;
        let _ = app.emit(
            "game-log",
            GameLog {
                version: version.into(),
                line: text,
            },
        );
    }
}

async fn append_log(path: &PathBuf, line: &str) {
    if let Ok(mut f) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
    {
        let _ = f.write_all(line.as_bytes()).await;
        let _ = f.write_all(b"\n").await;
    }
}

// Останавливает запущенную игру
pub async fn stop(version_id: &str) -> Result<(), String> {
    let child = running()
        .lock()
        .map_err(|_| "реестр процессов")?
        .remove(version_id);
    if let Some(child) = child {
        let mut guard = child.lock().await;
        let _ = guard.kill().await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_format() {
        let u = offline_uuid("Notch");
        assert_eq!(u.len(), 36);
        assert_eq!(u.chars().filter(|c| *c == '-').count(), 4);
        assert!(u.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }
}
