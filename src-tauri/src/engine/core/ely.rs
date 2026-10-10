// Аккаунты Ely.by: device flow OAuth2, профиль и запуск с authlib-injector.
// Адреса подсмотрены у ElyPrismLauncher, agent — как у Millida.
use serde::Serialize;

use super::http;
use super::paths;
use super::settings::{self, Account};

// Публичный client_id приложения-лаунчера; при желании меняется в settings.json
const DEFAULT_CLIENT_ID: &str = "elyprism-launcher";
const SCOPES: &str = "account_info offline_access minecraft_server_session";
const DEVICE_URL: &str = "https://account.ely.by/api/oauth2/v1/devicecode";
const TOKEN_URL: &str = "https://account.ely.by/api/oauth2/v1/token";
const PROFILE_URL: &str = "https://account.ely.by/api/mojang/services/minecraft/profile";
// Корень yggdrasil API для authlib-injector
pub const AUTHLIB_ROOT: &str = "https://account.ely.by/api/authlib-injector";
const AUTHLIB_FEED: &str = "https://authlib-injector.yushi.moe/artifact/latest.json";
// Сборка на случай, когда фид недоступен; sha256 из того же фида
const AUTHLIB_PINNED_URL: &str =
    "https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar";
const AUTHLIB_PINNED_SHA256: &str =
    "9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10";

pub fn client_id() -> String {
    settings::load()
        .ely_client_id
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.into())
}

// percent-кодирование значения form-urlencoded
pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

async fn post_form(url: &str, body: &str) -> Result<serde_json::Value, String> {
    let text = http::client()
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body.to_string())
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("запрос {url}: {e}"))?
        .text()
        .await
        .map_err(|e| format!("ответ {url}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("разбор ответа ely.by: {e}"))
}

#[derive(Clone, Serialize)]
pub struct DeviceStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

// Первый шаг device flow: код, который пользователь вводит в браузере
pub async fn device_start() -> Result<DeviceStart, String> {
    let body = format!("client_id={}&scope={}", enc(&client_id()), enc(SCOPES));
    let v = post_form(DEVICE_URL, &body).await?;
    let required = ["device_code", "user_code", "verification_uri"];
    if required.iter().any(|k| v[k].as_str().is_none()) {
        return Err("ely.by вернул неполный ответ device code".into());
    }
    Ok(DeviceStart {
        device_code: v["device_code"].as_str().unwrap_or_default().into(),
        user_code: v["user_code"].as_str().unwrap_or_default().into(),
        verification_uri: v["verification_uri"].as_str().unwrap_or_default().into(),
        interval: v["interval"].as_u64().unwrap_or(5).max(1),
        expires_in: v["expires_in"].as_u64().unwrap_or(600),
    })
}

pub enum Poll {
    // Пользователь ещё не подтвердил — повторить позже
    Pending,
    Done(Account),
    Error(String),
}

struct Tokens {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

fn tokens_of(v: &serde_json::Value) -> Option<Tokens> {
    let access = v["access_token"].as_str()?.to_string();
    let refresh = v["refresh_token"].as_str().unwrap_or_default().to_string();
    let ttl = v["expires_in"].as_u64().unwrap_or(3600);
    Some(Tokens {
        access_token: access,
        refresh_token: refresh,
        expires_at: now() + ttl.saturating_sub(60),
    })
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Очередной опрос токена. Ошибки pending — норма протокола.
pub async fn device_poll(device_code: &str) -> Result<Poll, String> {
    let body = format!(
        "client_id={}&grant_type={}&device_code={}",
        enc(&client_id()),
        enc("urn:ietf:params:oauth:grant-type:device_code"),
        enc(device_code)
    );
    let text = http::client()
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("запрос токена: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text.text().await.map_err(|e| e.to_string())?)
        .map_err(|e| format!("разбор ответа токена: {e}"))?;
    if let Some(err) = v["error"].as_str() {
        return Ok(match err {
            "authorization_pending" | "slow_down" => Poll::Pending,
            _ => Poll::Error(v["error_description"].as_str().unwrap_or(err).to_string()),
        });
    }
    let Some(t) = tokens_of(&v) else {
        return Err("ely.by не отдал access_token".into());
    };
    let profile = fetch_profile(&t.access_token).await?;
    Ok(Poll::Done(Account {
        name: profile.name,
        uuid: profile.uuid,
        access_token: t.access_token,
        refresh_token: t.refresh_token,
        expires_at: t.expires_at,
        provider: "ely".into(),
    }))
}

#[derive(Clone, Serialize)]
pub struct Profile {
    pub name: String,
    pub uuid: String,
}

// 32 hex-символа UUID → представление с дефисами, как ждёт игра
pub fn dashed_uuid(hex: &str) -> String {
    let h: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if h.len() != 32 {
        return hex.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

async fn fetch_profile(access_token: &str) -> Result<Profile, String> {
    let text = http::client()
        .get(PROFILE_URL)
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("профиль ely.by: {e}"))?
        .text()
        .await
        .map_err(|e| format!("ответ профиля: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("разбор профиля: {e}"))?;
    let name = v["name"]
        .as_str()
        .ok_or("в профиле ely.by нет имени")?
        .to_string();
    let id = v["id"]
        .as_str()
        .ok_or("в профиле ely.by нет id")?
        .to_string();
    Ok(Profile {
        name,
        uuid: dashed_uuid(&id),
    })
}

// Обмен refresh-токена на новую пару
async fn refresh(refresh_token: &str) -> Result<Tokens, String> {
    let body = format!(
        "client_id={}&grant_type=refresh_token&refresh_token={}",
        enc(&client_id()),
        enc(refresh_token)
    );
    let v = post_form(TOKEN_URL, &body).await?;
    tokens_of(&v).ok_or_else(|| "ely.by не отдал refresh-токен".into())
}

// Учётка из настроек с автообновлением истёкшего токена
pub async fn current_account() -> Option<Account> {
    let acc = settings::load().account?;
    if acc.expires_at > now() + 60 || acc.refresh_token.is_empty() {
        return Some(acc);
    }
    match refresh(&acc.refresh_token).await {
        Ok(t) => {
            let updated = Account {
                access_token: t.access_token,
                refresh_token: if t.refresh_token.is_empty() {
                    acc.refresh_token.clone()
                } else {
                    t.refresh_token
                },
                expires_at: t.expires_at,
                ..acc.clone()
            };
            let _ = settings::update(|s| s.account = Some(updated.clone()));
            Some(updated)
        }
        Err(_) => None,
    }
}

pub async fn logout() -> Result<(), String> {
    settings::update(|s| s.account = None)
}

// Загрузка скина в аккаунт Ely.by: модель classic (4px руки) или slim (3px)
pub async fn upload_skin(model: &str, png: &[u8]) -> Result<(), String> {
    let model = match model {
        "slim" => "slim",
        _ => "classic",
    };
    let acc = current_account()
        .await
        .ok_or("нет аккаунта Ely.by — сначала войдите")?;
    if acc.provider != "ely" {
        return Err("скин загружается только в аккаунт Ely.by".into());
    }
    let url = format!("https://account.ely.by/api/mojang/services/minecraft/skins/{model}");
    let res = http::client()
        .put(&url)
        .header("Authorization", format!("Bearer {}", acc.access_token))
        .header("Content-Type", "image/png")
        .body(png.to_vec())
        .send()
        .await
        .map_err(|e| format!("загрузка скина: {e}"))?;
    res.error_for_status()
        .map_err(|e| format!("ely.by отклонил скин: {e}"))?;
    Ok(())
}

// authlib-injector: фид свежей сборки с sha256, при недоступности — pinned.
// Испорченный jar в JVM не загрузится, поэтому проверка суммы обязательна.
pub async fn ensure_authlib() -> Result<std::path::PathBuf, String> {
    let dir = paths::data_dir().join("agents");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("папка агентов: {e}"))?;
    let jar = dir.join("authlib-injector.jar");

    let meta: serde_json::Value =
        serde_json::from_str(&http::text(AUTHLIB_FEED).await.unwrap_or_default())
            .unwrap_or(serde_json::Value::Null);
    let signed = meta["download_url"]
        .as_str()
        .filter(|u| u.starts_with("https://"))
        .and_then(|u| {
            let sum = meta["checksums"]["sha256"].as_str().unwrap_or_default();
            (sum.len() == 64 && sum.chars().all(|c| c.is_ascii_hexdigit()))
                .then(|| (u.to_string(), sum.to_string()))
        });
    let (url, sha256) = match signed {
        Some((u, s)) => (u, s),
        None => (AUTHLIB_PINNED_URL.into(), AUTHLIB_PINNED_SHA256.into()),
    };
    if !jar.exists() {
        http::download(&url, &jar, None, Some(&sha256), None).await?;
    }
    Ok(jar)
}

// Скин пользователя Ely.by для предпросмотра: textures → SKIN.url → png в base64
pub async fn skin_png(nick: &str) -> Option<String> {
    let url = format!("https://skinsystem.ely.by/textures/{nick}?proxy=true");
    let body = http::text(&url).await.ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let skin_url = v["SKIN"]["url"].as_str()?.to_string();
    let tmp = paths::data_dir().join("cache").join("ely-skin.png");
    http::download(&skin_url, &tmp, None, None, None)
        .await
        .ok()?;
    let bytes = tokio::fs::read(&tmp).await.ok()?;
    Some(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enc_keeps_unreserved() {
        assert_eq!(enc("abc-._~"), "abc-._~");
        assert_eq!(enc("a b"), "a%20b");
        assert_eq!(enc("a+b"), "a%2Bb");
        assert_eq!(enc("urn:ietf:params:oauth"), "urn%3Aietf%3Aparams%3Aoauth");
    }

    #[test]
    fn uuid_gets_dashes() {
        assert_eq!(
            dashed_uuid("069a79f444e94726a5befca90e38abaf"),
            "069a79f4-44e9-4726-a5be-fca90e38abaf"
        );
        assert_eq!(
            dashed_uuid("069a79f4-44e9-4726-a5be-fca90e38abaf"),
            "069a79f4-44e9-4726-a5be-fca90e38abaf"
        );
        assert_eq!(dashed_uuid("коротко"), "коротко");
    }
}
