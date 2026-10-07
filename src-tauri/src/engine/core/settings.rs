use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

// Настройки лаунчера одним файлом. Читается часто (каждая загрузка),
// поэтому простой синхронный путь без блокировок на время записи.
static LOCK: Mutex<()> = Mutex::new(());

const MAX_SERVERS: usize = 20;
const MAX_TOKEN_LEN: usize = 4096;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Account {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    // Момент истечения access-токена, секунды от эпохи
    pub expires_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub use_mirrors: bool,
    pub servers: Vec<String>,
    pub account: Option<Account>,
    // Переопределение client_id приложения Ely.by
    pub ely_client_id: Option<String>,
    // Ключ API CurseForge для официального эндпоинта, если зеркало недоступно
    #[serde(default)]
    pub cf_api_key: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            use_mirrors: true,
            servers: Vec::new(),
            account: None,
            ely_client_id: None,
            cf_api_key: None,
        }
    }
}

fn file() -> PathBuf {
    super::paths::data_dir().join("settings.json")
}

fn ok_host(h: &str) -> bool {
    !h.is_empty()
        && h.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '[' | ']' | ':'))
}

pub fn normalize_server(raw: &str) -> Option<String> {
    let s = raw.trim().trim_end_matches('.').trim();
    if s.is_empty() || s.len() > 255 || s.contains(char::is_whitespace) {
        return None;
    }
    match s.rsplit_once(':') {
        // host:port — порт обязан быть числом, скобкаIPv6 не трогаем
        Some((h, p)) if !h.ends_with(']') => {
            if !ok_host(h) || p.parse::<u16>().is_err() {
                return None;
            }
        }
        _ => {
            if !ok_host(s) {
                return None;
            }
        }
    }
    Some(s.to_ascii_lowercase())
}

pub fn load() -> Settings {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::read_to_string(file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .map(|s: Settings| {
            // Файл писали мы, но страховка от мусора лишней не бывает
            let mut servers: Vec<String> = s
                .servers
                .iter()
                .filter_map(|x| normalize_server(x))
                .collect();
            servers.truncate(MAX_SERVERS);
            servers.dedup();
            Settings {
                use_mirrors: s.use_mirrors,
                servers,
                account: s.account.filter(|a| {
                    !a.name.is_empty()
                        && a.name.len() <= 16
                        && a.uuid.len() == 36
                        && a.access_token.len() <= MAX_TOKEN_LEN
                        && a.refresh_token.len() <= MAX_TOKEN_LEN
                }),
                ely_client_id: s.ely_client_id.filter(|c| !c.is_empty() && c.len() <= 128),
                cf_api_key: s.cf_api_key.filter(|c| !c.is_empty() && c.len() <= 256),
            }
        })
        .unwrap_or_default()
}

pub fn save(s: &Settings) -> Result<(), String> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = file();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("папка настроек: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    let body =
        serde_json::to_string_pretty(s).map_err(|e| format!("сериализация настроек: {e}"))?;
    std::fs::write(&tmp, body).map_err(|e| format!("запись настроек: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("сохранение настроек: {e}"))?;
    Ok(())
}

pub fn update(f: impl FnOnce(&mut Settings)) -> Result<(), String> {
    let mut s = load();
    f(&mut s);
    save(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servers_are_normalized() {
        let cases: [(&str, Option<&str>); 8] = [
            ("mc.example.com", Some("mc.example.com")),
            ("Play.Example.RU:25577", Some("play.example.ru:25577")),
            ("mc.example.com.", Some("mc.example.com")),
            ("  mc.example.com  ", Some("mc.example.com")),
            ("", None),
            ("два слова", None),
            ("mc.example.com:99999", None),
            ("mc.example.com:", None),
        ];
        for (raw, want) in cases {
            assert_eq!(normalize_server(raw).as_deref(), want, "{raw}");
        }
        assert_eq!(
            normalize_server("mc.example.com:0").as_deref(),
            Some("mc.example.com:0")
        );
    }

    #[test]
    fn account_needs_both_tokens() {
        let ok = Account {
            name: "Steve".into(),
            uuid: "069a79f4-44e9-4726-a5be-fca90e38abaf".into(),
            access_token: "t".into(),
            refresh_token: "r".into(),
            expires_at: 1,
        };
        assert_eq!(ok.uuid.len(), 36);
        let bad = Account {
            name: String::new(),
            ..ok.clone()
        };
        assert!(bad.name.is_empty());
    }
}
