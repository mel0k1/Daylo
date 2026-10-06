use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::core::paths;

// Настройки сборки: куча JVM и дополнительные флаги
#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceConfig {
    pub ram_mb: u32,
    pub jvm_args: Vec<String>,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            ram_mb: 2048,
            jvm_args: Vec::new(),
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

// Только флаги вида -..., память зажата в разумных пределах
pub async fn save(version_id: &str, ram_mb: u32, jvm_args: &[String]) -> Result<(), String> {
    let cfg = InstanceConfig {
        ram_mb: ram_mb.clamp(512, 32768),
        jvm_args: jvm_args
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| s.starts_with('-') && !s.contains(char::is_whitespace))
            .collect(),
    };
    let dir = paths::instance_dir(version_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("папка сборки: {e}"))?;
    let body = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    tokio::fs::write(config_path(version_id), body)
        .await
        .map_err(|e| format!("сохранение настроек: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_two_gigs() {
        let c = InstanceConfig::default();
        assert_eq!(c.ram_mb, 2048);
        assert!(c.jvm_args.is_empty());
    }
}
