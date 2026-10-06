use serde::Serialize;

use super::super::core::paths;
use super::nbt;

#[derive(Serialize, Clone)]
pub struct GameServer {
    pub name: String,
    pub address: String,
    // PNG в data-URL: иконка прямо из servers.dat
    pub icon: Option<String>,
}

// Серверы, добавленные игроком в самой игре: instances/<v>/servers.dat
pub async fn list(version: &str) -> Vec<GameServer> {
    let path = paths::instance_dir(version).join("servers.dat");
    match tokio::fs::read(&path).await {
        Ok(data) => from_nbt(&data),
        Err(_) => Vec::new(),
    }
}

fn from_nbt(data: &[u8]) -> Vec<GameServer> {
    let Ok(root) = nbt::parse(data) else {
        return Vec::new();
    };
    let Some(list) = root.get("servers").and_then(|v| v.as_list()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in list {
        let address = item
            .get("ip")
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();
        if address.is_empty() {
            continue;
        }
        let name = item
            .get("name")
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();
        let icon = item
            .get("icon")
            .and_then(|v| v.as_bytes())
            .filter(|b| b.len() > 8)
            .map(|b| format!("data:image/png;base64,{}", b64(b)));
        out.push(GameServer {
            name,
            address,
            icon,
        });
    }
    out
}

fn b64(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_empty() {
        assert!(from_nbt(&[1, 2, 3, 4]).is_empty());
        assert!(from_nbt(&[]).is_empty());
    }

    #[test]
    fn missing_servers_key_is_empty() {
        let mut data = vec![10u8, 0, 0];
        data.push(0);
        assert!(from_nbt(&data).is_empty());
    }
}
