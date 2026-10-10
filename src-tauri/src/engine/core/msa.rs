// Аккаунты Microsoft: device flow → Xbox Live → XSTS → Minecraft.
// Схема стандартная для Identity Platform; ошибки Xbox расшифровываем по XErr.
use serde::Serialize;

use super::ely::{dashed_uuid, enc};
use super::http;
use super::paths;
use super::settings::{self, Account};

// Client_id официального лаунчера Minecraft; свой меняется в settings.json
const DEFAULT_CLIENT_ID: &str = "00000000402b5328";
const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const SESSIONSERVER: &str = "https://sessionserver.mojang.com/session/minecraft/profile";

pub fn client_id() -> String {
    settings::load()
        .msa_client_id
        .unwrap_or_else(|| DEFAULT_CLIENT_ID.into())
}

#[derive(Clone, Serialize)]
pub struct DeviceStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
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
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Первый шаг device flow: код, который пользователь вводит в браузере
pub async fn device_start() -> Result<DeviceStart, String> {
    let body = format!("client_id={}&scope={}", enc(&client_id()), enc(SCOPE));
    let text = http::client()
        .post(DEVICE_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("запрос device code: {e}"))?
        .text()
        .await
        .map_err(|e| format!("ответ device code: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("разбор device code: {e}"))?;
    let required = ["device_code", "user_code", "verification_uri"];
    if required.iter().any(|k| v[k].as_str().is_none()) {
        return Err("Microsoft вернул неполный ответ device code".into());
    }
    Ok(DeviceStart {
        device_code: v["device_code"].as_str().unwrap_or_default().into(),
        user_code: v["user_code"].as_str().unwrap_or_default().into(),
        verification_uri: v["verification_uri"].as_str().unwrap_or_default().into(),
        interval: v["interval"].as_u64().unwrap_or(5).max(1),
        expires_in: v["expires_in"].as_u64().unwrap_or(900),
    })
}

// Токен Microsoft: не error_for_status — ошибки приходят телом с 400
async fn token_request(body: String) -> Result<serde_json::Value, String> {
    let text = http::client()
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("запрос токена: {e}"))?
        .text()
        .await
        .map_err(|e| format!("ответ токена: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("разбор ответа токена: {e}"))
}

fn tokens_of(v: &serde_json::Value) -> Option<Tokens> {
    let access_token = v["access_token"].as_str()?.to_string();
    let refresh_token = v["refresh_token"].as_str().unwrap_or_default().to_string();
    Some(Tokens {
        access_token,
        refresh_token,
    })
}

// JSON POST без проверки статуса: Xbox и Minecraft отдают ошибки телом
async fn post_json(url: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
    let text = http::client()
        .post(url)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("запрос {url}: {e}"))?
        .text()
        .await
        .map_err(|e| format!("ответ {url}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("разбор ответа {url}: {e}"))
}

// Дружелюбные объяснения кодов Xbox (XErr)
fn xerr_message(xerr: i64) -> String {
    match xerr {
        2148916233 => "у этого аккаунта Microsoft нет профиля Xbox".into(),
        2148916235 => "Xbox Live недоступен в вашей стране".into(),
        2148916236 | 2148916237 => {
            "детский аккаунт — добавьте его в семейную группу Xbox".into()
        }
        2148916238 => "подтвердите возраст аккаунта на xbox.com".into(),
        _ => format!("Xbox отказал во входе (код {xerr})"),
    }
}

// XBL → XSTS: обе службы отвечают Token + DisplayClaims.xui[0].uhs
async fn xsts_chain(msa_token: &str) -> Result<(String, String), String> {
    let xbl = post_json(
        XBL_URL,
        serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={msa_token}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }),
    )
    .await?;
    let Some(xbl_token) = xbl["Token"].as_str() else {
        let xerr = xbl["XErr"].as_i64().unwrap_or(0);
        return Err(if xerr == 0 {
            "Xbox Live не отдал токен".into()
        } else {
            xerr_message(xerr)
        });
    };
    let xsts = post_json(
        XSTS_URL,
        serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }),
    )
    .await?;
    let Some(xsts_token) = xsts["Token"].as_str() else {
        let xerr = xsts["XErr"].as_i64().unwrap_or(0);
        return Err(if xerr == 0 {
            "XSTS не отдал токен".into()
        } else {
            xerr_message(xerr)
        });
    };
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    Ok((uhs, xsts_token.to_string()))
}

// Обмен XSTS-токена на токен Minecraft и профиль
async fn minecraft_account(xsts: &(String, String), refresh_token: String) -> Result<Account, String> {
    let mc = post_json(
        MC_LOGIN_URL,
        serde_json::json!({
            "identityToken": format!("XBL3.0 x={};{}", xsts.0, xsts.1),
        }),
    )
    .await?;
    let Some(access_token) = mc["access_token"].as_str() else {
        return Err("Minecraft не отдал токен входа".into());
    };
    let ttl = mc["expires_in"].as_u64().unwrap_or(86400);
    let text = http::client()
        .get(PROFILE_URL)
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("профиль Minecraft: {e}"))?
        .text()
        .await
        .map_err(|e| format!("ответ профиля: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("разбор профиля: {e}"))?;
    let name = v["name"]
        .as_str()
        .ok_or("в профиле Minecraft нет имени")?
        .to_string();
    let id = v["id"]
        .as_str()
        .ok_or("в профиле Minecraft нет id")?
        .to_string();
    Ok(Account {
        name,
        uuid: dashed_uuid(&id),
        access_token: access_token.to_string(),
        refresh_token,
        expires_at: now() + ttl.saturating_sub(60),
        provider: "msa".into(),
    })
}

// Очередной опрос токена. Ошибки pending — норма протокола.
pub async fn device_poll(device_code: &str) -> Result<Poll, String> {
    let body = format!(
        "client_id={}&grant_type={}&device_code={}",
        enc(&client_id()),
        enc("urn:ietf:params:oauth:grant-type:device_code"),
        enc(device_code)
    );
    let v = token_request(body).await?;
    if let Some(err) = v["error"].as_str() {
        return Ok(match err {
            "authorization_pending" | "slow_down" => Poll::Pending,
            _ => Poll::Error(v["error_description"].as_str().unwrap_or(err).to_string()),
        });
    }
    let Some(t) = tokens_of(&v) else {
        return Err("Microsoft не отдал access_token".into());
    };
    let (uhs, xsts_token) = xsts_chain(&t.access_token).await?;
    Ok(Poll::Done(
        minecraft_account(&(uhs, xsts_token), t.refresh_token).await?,
    ))
}

// Учётка из настроек с обновлением токена через refresh-цепочку
pub async fn current_account() -> Option<Account> {
    let acc = settings::load().account?;
    if acc.provider != "msa" {
        return None;
    }
    if acc.expires_at > now() + 60 || acc.refresh_token.is_empty() {
        return Some(acc);
    }
    let v = token_request(format!(
        "client_id={}&grant_type=refresh_token&refresh_token={}",
        enc(&client_id()),
        enc(&acc.refresh_token)
    ))
    .await
    .ok()?;
    let t = tokens_of(&v)?;
    let (uhs, xsts_token) = xsts_chain(&t.access_token).await.ok()?;
    match minecraft_account(&(uhs, xsts_token), t.refresh_token).await {
        Ok(updated) => {
            let _ = settings::update(|s| s.account = Some(updated.clone()));
            Some(updated)
        }
        Err(_) => None,
    }
}

// Скин аккаунта Microsoft: sessionserver → textures → SKIN.url → png в base64
pub async fn skin_png(uuid: &str) -> Option<String> {
    let url = format!("{SESSIONSERVER}/{uuid}");
    let body = http::text(&url).await.ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let tex = v["properties"]
        .as_array()?
        .iter()
        .find(|p| p["name"].as_str() == Some("textures"))?["value"]
        .as_str()?
        .to_string();
    let raw = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &tex).ok()?;
    let tv: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    let skin_url = tv["textures"]["SKIN"]["url"].as_str()?.to_string();
    let tmp = paths::data_dir().join("cache").join("msa-skin.png");
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
    fn xerr_is_friendly() {
        assert_eq!(
            xerr_message(2148916233),
            "у этого аккаунта Microsoft нет профиля Xbox"
        );
        assert!(xerr_message(2148916236).contains("семейную"));
        assert!(xerr_message(2148916238).contains("возраст"));
        assert!(xerr_message(42).contains("42"));
    }

    #[test]
    fn tokens_need_access() {
        let t = tokens_of(&serde_json::json!({
            "access_token": "at", "refresh_token": "rt", "expires_in": 86400
        }))
        .unwrap();
        assert_eq!(t.access_token, "at");
        assert_eq!(t.refresh_token, "rt");
        assert!(tokens_of(&serde_json::json!({})).is_none());
    }
}
