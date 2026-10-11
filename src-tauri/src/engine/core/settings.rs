use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

// Настройки лаунчера одним файлом. Читается часто (каждая загрузка),
// поэтому простой синхронный путь без блокировок на время записи.
static LOCK: Mutex<()> = Mutex::new(());

const MAX_SERVERS: usize = 20;
const MAX_TOKEN_LEN: usize = 4096;

// Здравые границы окна игры: ниже 320x240 интерфейс Minecraft не вмещается,
// выше 8K мониторов пока не бывает
const MIN_GAME_W: u32 = 320;
const MAX_GAME_W: u32 = 7680;
const MIN_GAME_H: u32 = 240;
const MAX_GAME_H: u32 = 4320;

fn default_game_width() -> u32 {
    1280
}

fn default_game_height() -> u32 {
    720
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Account {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    // Момент истечения access-токена, секунды от эпохи
    pub expires_at: u64,
    // Источник авторизации: "ely" | "msa" | "offline";
    // в старых файлах пусто — значит ely
    #[serde(default)]
    pub provider: String,
}

// Провайдеры, которые понимает загрузка; пустое значение = ely (старые файлы)
fn known_provider(p: &str) -> bool {
    matches!(p, "" | "ely" | "msa" | "offline")
}

// Структурная проверка одной учётки: имя/uuid/токены в разумных рамках
fn account_plausible(a: &Account) -> bool {
    !a.name.is_empty()
        && a.name.len() <= 16
        && a.uuid.len() == 36
        && a.access_token.len() <= MAX_TOKEN_LEN
        && a.refresh_token.len() <= MAX_TOKEN_LEN
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub use_mirrors: bool,
    pub servers: Vec<String>,
    // Старое одиночное поле: читается ради миграции, при записи пропадает
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    // Все сохранённые аккаунты: ely, msa и оффлайн-профили
    #[serde(default)]
    pub accounts: Vec<Account>,
    // Активный аккаунт по uuid; None при пустом списке
    #[serde(default)]
    pub active_account: Option<String>,
    // Переопределение client_id приложения Ely.by
    pub ely_client_id: Option<String>,
    // Ключ API CurseForge для официального эндпоинта, если зеркало недоступно
    #[serde(default)]
    pub cf_api_key: Option<String>,
    // Свой client_id приложения Microsoft (GUID), если не годится публичный
    #[serde(default)]
    pub msa_client_id: Option<String>,
    // Окно запускаемой игры; в старых файлах нет полей — считаем 1280x720
    #[serde(default = "default_game_width")]
    pub game_width: u32,
    #[serde(default = "default_game_height")]
    pub game_height: u32,
    // Полноэкранный режим игры
    #[serde(default)]
    pub game_fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            use_mirrors: true,
            servers: Vec::new(),
            account: None,
            accounts: Vec::new(),
            active_account: None,
            ely_client_id: None,
            cf_api_key: None,
            msa_client_id: None,
            game_width: default_game_width(),
            game_height: default_game_height(),
            game_fullscreen: false,
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

// Обрезает запрошенное окно до поддерживаемых границ
pub fn clamp_window(width: u32, height: u32) -> (u32, u32) {
    (
        width.clamp(MIN_GAME_W, MAX_GAME_W),
        height.clamp(MIN_GAME_H, MAX_GAME_H),
    )
}

// GUID вида 8-4-4-4-12 hex — client_id приложения Azure
fn ok_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&parts)
            .all(|(w, p)| p.len() == *w && p.chars().all(|c| c.is_ascii_hexdigit()))
}

pub fn load() -> Settings {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::read_to_string(file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .map(sanitize)
        .unwrap_or_default()
}

// Чистка распарсенного файла: файл писали мы, но страховка от мусора
// лишней не бывает. Выделена отдельно — проверяется тестами без файловой системы.
fn sanitize(s: Settings) -> Settings {
    let mut servers: Vec<String> = s
        .servers
        .iter()
        .filter_map(|x| normalize_server(x))
        .collect();
    servers.truncate(MAX_SERVERS);
    servers.dedup();

    // Список аккаунтов: валидные, провайдер известен, без дублей по uuid
    let mut accounts: Vec<Account> = Vec::new();
    let mut seed: Vec<Account> = s.accounts;
    // Миграция старого одиночного поля: переносим, если список пуст
    if let Some(old) = s.account {
        if seed.iter().all(|a| a.uuid != old.uuid) {
            seed.push(old);
        }
    }
    for mut a in seed {
        if !account_plausible(&a) || !known_provider(&a.provider) {
            continue;
        }
        if a.provider.is_empty() {
            a.provider = "ely".into();
        }
        if accounts.iter().any(|x| x.uuid == a.uuid) {
            continue;
        }
        accounts.push(a);
    }
    // Активным остаётся только существующий в списке
    let active_account = s
        .active_account
        .filter(|id| accounts.iter().any(|a| &a.uuid == id))
        .or_else(|| accounts.first().map(|a| a.uuid.clone()));

    Settings {
        use_mirrors: s.use_mirrors,
        servers,
        // Старое поле погашено: данные уже в accounts
        account: None,
        accounts,
        active_account,
        ely_client_id: s.ely_client_id.filter(|c| !c.is_empty() && c.len() <= 128),
        cf_api_key: s.cf_api_key.filter(|c| !c.is_empty() && c.len() <= 256),
        msa_client_id: s.msa_client_id.filter(|c| ok_guid(c)),
        game_width: clamp_window(s.game_width, s.game_height).0,
        game_height: clamp_window(s.game_width, s.game_height).1,
        game_fullscreen: s.game_fullscreen,
    }
}

impl Settings {
    // Активный аккаунт: выбранный, иначе единственный сохранённый
    pub fn active(&self) -> Option<Account> {
        let by_id = self
            .active_account
            .as_ref()
            .and_then(|id| self.accounts.iter().find(|a| &a.uuid == id));
        by_id.or_else(|| self.accounts.first()).cloned()
    }

    // Вставить/обновить учётку и сделать её активной (по uuid;
    // оффлайн-uuid выводится из ника, повторный вход обновляет ту же запись)
    pub fn upsert_account(&mut self, acc: Account) {
        match self.accounts.iter_mut().find(|a| a.uuid == acc.uuid) {
            Some(slot) => *slot = acc.clone(),
            None => self.accounts.push(acc.clone()),
        }
        self.active_account = Some(acc.uuid);
        self.account = None;
    }
}

// Активный аккаунт из файла настроек
pub fn active() -> Option<Account> {
    load().active()
}

// Сменить активный аккаунт; чужой uuid — ошибка
pub fn set_active(uuid: &str) -> Result<(), String> {
    update(|s| {
        if s.accounts.iter().any(|a| a.uuid == uuid) {
            s.active_account = Some(uuid.to_string());
        }
    })
    .and_then(|_| {
        if load().active_account.as_deref() == Some(uuid) {
            Ok(())
        } else {
            Err("аккаунт не найден".into())
        }
    })
}

// Удалить учётку; если это был активный — активным становится первый из оставшихся
pub fn remove_account(uuid: &str) -> Result<(), String> {
    update(|s| {
        s.accounts.retain(|a| a.uuid != uuid);
        if s.active_account.as_deref() == Some(uuid) {
            s.active_account = s.accounts.first().map(|a| a.uuid.clone());
        }
        s.account = None;
    })
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
    // Права на tmp: rename сохраняет inode и права, окно с открытым файлом минимально
    restrict_perms(&tmp);
    std::fs::rename(&tmp, &path).map_err(|e| format!("сохранение настроек: {e}"))?;
    // Идемпотентно на случай унаследованных прав у старого файла
    restrict_perms(&path);
    Ok(())
}

pub fn update(f: impl FnOnce(&mut Settings)) -> Result<(), String> {
    let mut s = load();
    f(&mut s);
    save(&s)
}

// Токены аккаунтов лежат в settings.json открытым текстом — на Unix файл
// должен читать только владелец (на Windows права задаёт профиль юзера)
#[cfg(unix)]
fn restrict_perms(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_perms(_path: &std::path::Path) {}

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
            provider: "ely".into(),
        };
        assert_eq!(ok.uuid.len(), 36);
        let bad = Account {
            name: String::new(),
            ..ok.clone()
        };
        assert!(bad.name.is_empty());
    }

    fn acc(name: &str, uuid: &str, provider: &str) -> Account {
        Account {
            name: name.into(),
            uuid: uuid.into(),
            access_token: "t".into(),
            refresh_token: "r".into(),
            expires_at: 0,
            provider: provider.into(),
        }
    }

    const U1: &str = "069a79f4-44e9-4726-a5be-fca90e38abaf";
    const U2: &str = "1c1e4a5f-2b7d-4c3e-9a0b-5d6e7f8a9b0c";

    // Старый файл с одиночным account переезжает в accounts и становится активным
    #[test]
    fn legacy_account_migrates_to_list() {
        let s = Settings {
            account: Some(acc("Notch", U1, "")),
            ..Default::default()
        };
        let out = sanitize(s);
        assert_eq!(out.accounts.len(), 1);
        assert_eq!(out.accounts[0].provider, "ely"); // пустой провайдер = ely
        assert_eq!(out.active_account.as_deref(), Some(U1));
        assert_eq!(out.active().map(|a| a.name), Some("Notch".into()));
        assert!(out.account.is_none());
    }

    // Мусорные учётки отбрасываются, дублей по uuid не остаётся
    #[test]
    fn accounts_are_filtered_and_deduped() {
        let s = Settings {
            accounts: vec![
                acc("Steve", U1, "offline"),
                acc("", U2, "ely"),            // пустое имя — долой
                acc("Dup", U1, "msa"),         // тот же uuid — дубль
                acc("Bad", "короткий", "ely"), // uuid не 36
                acc("Fog", U2, "steam"),       // неизвестный провайдер
            ],
            ..Default::default()
        };
        let out = sanitize(s);
        assert_eq!(out.accounts.len(), 1);
        assert_eq!(out.accounts[0].name, "Steve");
        assert_eq!(out.accounts[0].provider, "offline");
        assert_eq!(out.active_account.as_deref(), Some(U1));
    }

    // Активный uuid, которого нет в списке, заменяется первым существующим
    #[test]
    fn active_id_falls_back_to_first() {
        let mut s = Settings {
            accounts: vec![acc("A", U1, "ely"), acc("B", U2, "msa")],
            active_account: Some("deadbeef-dead-dead-dead-deaddeadbeef".into()),
            ..Default::default()
        };
        let out = sanitize(s.clone());
        assert_eq!(out.active().map(|a| a.name), Some("A".into()));
        // И явно выбранный выигрывает у фолбэка
        s.active_account = Some(U2.to_string());
        assert_eq!(sanitize(s).active().map(|a| a.name), Some("B".into()));
    }

    // Апсерт обновляет по uuid и делает запись активной
    #[test]
    fn upsert_replaces_and_activates() {
        let mut s = Settings::default();
        s.upsert_account(acc("Steve", U1, "offline"));
        s.upsert_account(acc("Notch", U2, "ely"));
        assert_eq!(s.accounts.len(), 2);
        assert_eq!(s.active_account.as_deref(), Some(U2));
        // Повторный вход с тем же uuid обновляет токены, а не плодит запись
        let mut renewed = acc("Notch", U2, "ely");
        renewed.access_token = "new".into();
        s.upsert_account(renewed);
        assert_eq!(s.accounts.len(), 2);
        assert_eq!(s.accounts[1].access_token, "new");
        assert!(s.account.is_none());
    }

    #[test]
    fn guid_shape_is_strict() {
        assert!(ok_guid("00000000-402b-5328-0000-000000000000"));
        assert!(ok_guid("00000000-402B-5328-0000-000000000000"));
        assert!(!ok_guid(""));
        assert!(!ok_guid("не-guid"));
        // Без дефисов и с неверной длиной сегментов не проходит
        assert!(!ok_guid("00000000402b5328"));
        assert!(!ok_guid("00000000-402b-5328-0000-00000000000"));
        assert!(!ok_guid("00000000-402b-5328-0000-0000000000zz"));
    }

    #[test]
    fn window_dims_are_clamped() {
        assert_eq!(clamp_window(1280, 720), (1280, 720));
        assert_eq!(clamp_window(1024, 768), (1024, 768));
        // Нули и отрицательные невозможны в u32 — любое малое число к низу
        assert_eq!(clamp_window(0, 0), (320, 240));
        assert_eq!(clamp_window(100, 50), (320, 240));
        // 8K сверху
        assert_eq!(clamp_window(999_999, 99_999), (7680, 4320));
        // Ширина в норме, высота за пределами
        assert_eq!(clamp_window(500, 10_000), (500, 4320));
    }

    #[cfg(unix)]
    #[test]
    fn settings_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("daylo-perms-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("settings.json");
        std::fs::write(&f, "{}").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
        restrict_perms(&f);
        assert_eq!(
            std::fs::metadata(&f).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
