use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::core::paths;

// Настройки сборки: куча JVM, флаги и происхождение (для сборок из каталога)
#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceConfig {
    pub ram_mb: u32,
    pub jvm_args: Vec<String>,
    // Метаданные модпаков; для ванильных сборок поля пустые
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub launch_version: String,
    #[serde(default)]
    pub mc_version: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub pack_name: String,
    #[serde(default)]
    pub pack_id: String,
    #[serde(default)]
    pub pack_version: String,
    #[serde(default)]
    pub icon_url: String,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            ram_mb: 2048,
            jvm_args: Vec::new(),
            kind: String::new(),
            launch_version: String::new(),
            mc_version: String::new(),
            source: String::new(),
            pack_name: String::new(),
            pack_id: String::new(),
            pack_version: String::new(),
            icon_url: String::new(),
        }
    }
}

fn config_path(version_id: &str) -> PathBuf {
    paths::instance_dir(version_id).join("daylo.json")
}

pub async fn load(version_id: &str) -> InstanceConfig {
    match tokio::fs::read_to_string(config_path(version_id)).await {
        Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
        Err(_) => InstanceConfig::default(),
    }
}

// Запись готового конфига целиком
pub async fn write(version_id: &str, cfg: &InstanceConfig) -> Result<(), String> {
    let dir = paths::instance_dir(version_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("папка сборки: {e}"))?;
    let body = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    tokio::fs::write(config_path(version_id), body)
        .await
        .map_err(|e| format!("сохранение настроек: {e}"))
}

// Только флаги вида -..., память зажата в разумных пределах; метаданные сохраняются
pub async fn save(version_id: &str, ram_mb: u32, jvm_args: &[String]) -> Result<(), String> {
    let mut cfg = load(version_id).await;
    cfg.ram_mb = ram_mb.clamp(512, 32768);
    cfg.jvm_args = jvm_args
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| s.starts_with('-') && !s.contains(char::is_whitespace))
        .collect();
    write(version_id, &cfg).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_two_gigs() {
        let c = InstanceConfig::default();
        assert_eq!(c.ram_mb, 2048);
        assert!(c.jvm_args.is_empty());
        assert!(c.kind.is_empty());
    }

    #[test]
    fn meta_fields_survive_serde() {
        // Старый daylo.json без новых полей читается, новые поля дописываются
        let body = r#"{"ram_mb": 4096, "jvm_args": ["-Xms512M"]}"#;
        let c: InstanceConfig = serde_json::from_str(body).unwrap();
        assert_eq!(c.ram_mb, 4096);
        assert_eq!(c.jvm_args, vec!["-Xms512M".to_string()]);
        assert!(c.kind.is_empty());
    }
}
