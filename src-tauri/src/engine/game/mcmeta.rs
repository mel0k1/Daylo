use serde::Deserialize;

use super::super::core::http;
use super::super::core::paths;

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Deserialize, Clone, serde::Serialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Deserialize)]
struct FullEntry {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    url: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
}

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<FullEntry>,
}

// Список версий: сеть с запасным вариантом на локальный кеш
pub async fn list() -> Result<Vec<VersionEntry>, String> {
    let cache = paths::data_dir().join("mc").join("version_manifest.json");
    let from_net = async {
        let raw = http::text(MANIFEST_URL).await?;
        let parsed: Manifest =
            serde_json::from_str(&raw).map_err(|e| format!("разбор манифеста: {e}"))?;
        let list: Vec<VersionEntry> = parsed
            .versions
            .into_iter()
            .map(|v| VersionEntry {
                id: v.id,
                kind: v.kind,
                release_time: v.release_time,
            })
            .collect();
        Ok::<Vec<VersionEntry>, String>(list)
    };
    match from_net.await {
        Ok(v) => {
            let _ = tokio::fs::create_dir_all(cache.parent().unwrap_or(&cache)).await;
            let _ = tokio::fs::write(
                &cache,
                serde_json::to_string(&v).unwrap_or_else(|_| "[]".into()),
            )
            .await;
            Ok(v)
        }
        Err(e) => {
            let body = tokio::fs::read_to_string(&cache)
                .await
                .map_err(|_| format!("манифест версий: {e}"))?;
            serde_json::from_str(&body).map_err(|err| format!("кеш манифеста: {err}"))
        }
    }
}

pub async fn version_url(id: &str) -> Result<String, String> {
    let raw = http::text(MANIFEST_URL).await?;
    let parsed: Manifest = serde_json::from_str(&raw).map_err(|e| format!("манифест: {e}"))?;
    for v in parsed.versions {
        if v.id == id {
            return Ok(v.url);
        }
    }
    Err(format!("версия {id} не найдена в манифесте"))
}

// --- Разбор version json ---

#[derive(Deserialize, Clone)]
pub struct VersionJson {
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments", default)]
    pub legacy_args: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(rename = "assetIndex", default)]
    pub asset_index: Option<AssetIndex>,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub downloads: Downloads,
    #[serde(rename = "type", default)]
    pub kind: String,
}

#[derive(Deserialize, Clone)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Arg>,
    #[serde(default)]
    pub jvm: Vec<Arg>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Arg {
    Plain(String),
    Ruled { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize, Clone)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<std::collections::BTreeMap<String, bool>>,
}

#[derive(Deserialize, Clone)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    #[serde(default)]
    pub natives: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
}

#[derive(Deserialize, Clone)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<std::collections::BTreeMap<String, Artifact>>,
}

#[derive(Deserialize, Clone)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Deserialize, Clone)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct JavaVersion {
    #[serde(rename = "majorVersion")]
    pub major: u32,
}

#[derive(Deserialize, Clone, Default)]
pub struct Downloads {
    #[serde(default, rename = "client")]
    pub client: Option<Artifact>,
}

pub fn current_os() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        _ => "linux",
    }
}

pub fn current_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => "x86",
    }
}

// Правила Mojang: без rules — разрешено; allow/disallow по os и features
pub fn rules_allow(rules: &[Rule], features: &[&str]) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for r in rules {
        let os_ok = match &r.os {
            Some(o) => {
                o.name.as_deref().is_none_or(|n| n == current_os())
                    && o.arch.as_deref().is_none_or(|a| a == current_arch())
            }
            None => true,
        };
        let feat_ok = match &r.features {
            Some(f) => f.iter().all(|(k, v)| features.contains(&k.as_str()) == *v),
            None => true,
        };
        if os_ok && feat_ok {
            match r.action.as_str() {
                "allow" => allowed = true,
                "disallow" => allowed = false,
                _ => {}
            }
        }
    }
    allowed
}

// Maven-путь по координатам group:artifact:version[:classifier]
pub fn maven_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return name.replace(':', "-");
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    format!(
        "{}/{}/{}/{}-{}{}.jar",
        group.replace('.', "/"),
        artifact,
        version,
        artifact,
        version,
        classifier
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_coords_to_path() {
        assert_eq!(
            maven_path("com.mojang:brigadier:1.0.18"),
            "com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar"
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.2.2:natives-linux"),
            "org/lwjgl/lwjgl/3.2.2/lwjgl-3.2.2-natives-linux.jar"
        );
    }

    #[test]
    fn rules_default_and_os() {
        let empty: Vec<Rule> = vec![];
        assert!(rules_allow(&empty, &[]));

        let v = serde_json::json!([{"action": "allow", "os": {"name": "windows"}}]);
        let rules: Vec<Rule> = serde_json::from_value(v).unwrap();
        assert!(!rules_allow(&rules, &[]));
    }
}
