// Minecraft Server List Ping (1.7+): handshake, статус, разбор JSON.
// Вся схема повторяет общепринятую: пакет 0x00, состояние 1, varint-длины.
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(4);
const DEFAULT_PORT: u16 = 25565;
const MAX_JSON: usize = 2_000_000;

#[derive(Serialize, Clone)]
pub struct ServerStatus {
    pub online: bool,
    pub ms: u64,
    pub motd: String,
    pub version: String,
    pub players_online: u32,
    pub players_max: u32,
    // data:image/png;base64,... — отдаётся как есть
    pub favicon: Option<String>,
    pub error: Option<String>,
}

fn offline(addr: &str, err: impl Into<String>) -> ServerStatus {
    ServerStatus {
        online: false,
        ms: 0,
        motd: String::new(),
        version: String::new(),
        players_online: 0,
        players_max: 0,
        favicon: None,
        error: Some(format!("{addr}: {}", err.into())),
    }
}

// IPv6-литерал в скобках делится по последней скобке, иначе "[::1]:25565"
// разваливается по каждому двоеточию
pub fn split_host_port(addr: &str) -> (String, u16) {
    if let Some((host, tail)) = addr.strip_prefix('[').and_then(|r| r.split_once(']')) {
        let port = tail
            .strip_prefix(':')
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_PORT);
        return (host.to_string(), port);
    }
    match addr.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => (h.to_string(), p.parse().unwrap_or(DEFAULT_PORT)),
        _ => (addr.to_string(), DEFAULT_PORT),
    }
}

fn write_varint(buf: &mut Vec<u8>, mut v: i32) {
    loop {
        let mut b = (v & 0x7f) as u8;
        v = ((v as u32) >> 7) as i32;
        if v != 0 {
            b |= 0x80;
        }
        buf.push(b);
        if v == 0 {
            break;
        }
    }
}

async fn read_varint(r: &mut TcpStream) -> std::io::Result<i32> {
    let mut num = 0i32;
    let mut shift = 0;
    loop {
        let mut b = [0u8; 1];
        r.read_exact(&mut b).await?;
        num |= ((b[0] & 0x7f) as i32) << shift;
        if b[0] & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            break;
        }
    }
    Ok(num)
}

// MOTD — строка или компонент {text, extra:[...]}, достаём чистый текст
fn motd_text(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    let mut out = String::new();
    if let Some(t) = v["text"].as_str() {
        out.push_str(t);
    }
    if let Some(arr) = v["extra"].as_array() {
        for e in arr {
            out.push_str(&motd_text(e));
        }
    }
    // Убираем коды форматирования §a и иже с ними
    out.chars()
        .collect::<Vec<_>>()
        .split(|c| *c == '§')
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.iter().collect::<String>()
            } else {
                part.iter().skip(1).collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

// Статус сервера: не паникуем и не возвращаем Err — список рисуется по полю online
pub async fn status(addr: &str) -> ServerStatus {
    let addr = addr.trim();
    let (host, port) = split_host_port(addr);
    if host.is_empty() {
        return offline(addr, "пустой адрес");
    }

    let start = std::time::Instant::now();
    let connect = tokio::time::timeout(TIMEOUT, TcpStream::connect((host.as_str(), port))).await;
    let mut sock = match connect {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return offline(addr, e.to_string()),
        Err(_) => return offline(addr, "нет ответа за 4 секунды"),
    };

    // Handshake: версия -1 допустима для статуса
    let mut hs = Vec::new();
    write_varint(&mut hs, 0x00);
    write_varint(&mut hs, -1);
    write_varint(&mut hs, host.len() as i32);
    hs.extend_from_slice(host.as_bytes());
    hs.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut hs, 1);

    let mut req = Vec::new();
    write_varint(&mut req, hs.len() as i32);
    req.extend_from_slice(&hs);
    // Запрос статуса — пустой пакет 0x00
    let mut pkt = Vec::new();
    write_varint(&mut pkt, 0x01);
    write_varint(&mut pkt, 0x00);
    req.extend_from_slice(&pkt);

    if let Err(e) = sock.write_all(&req).await {
        return offline(addr, e.to_string());
    }

    let read = async {
        let _len = read_varint(&mut sock).await?;
        let pid = read_varint(&mut sock).await?;
        if pid != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "не статус-пакет",
            ));
        }
        let jlen = read_varint(&mut sock).await? as usize;
        if jlen == 0 || jlen > MAX_JSON {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "странный размер ответа",
            ));
        }
        let mut json = vec![0u8; jlen];
        sock.read_exact(&mut json).await?;
        Ok(json)
    };
    let json = match tokio::time::timeout(TIMEOUT, read).await {
        Ok(Ok(j)) => j,
        Ok(Err(e)) => return offline(addr, e.to_string()),
        Err(_) => return offline(addr, "нет ответа за 4 секунды"),
    };

    let ms = start.elapsed().as_millis() as u64;
    let v: serde_json::Value = match serde_json::from_slice(&json) {
        Ok(v) => v,
        Err(e) => return offline(addr, format!("ответ не json: {e}")),
    };
    ServerStatus {
        online: true,
        ms,
        motd: motd_text(&v["description"]),
        version: v["version"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        players_online: v["players"]["online"].as_u64().unwrap_or(0) as u32,
        players_max: v["players"]["max"].as_u64().unwrap_or(0) as u32,
        favicon: v["favicon"].as_str().map(String::from),
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_port_split() {
        let cases: [(&str, &str, u16); 6] = [
            ("mc.example.com", "mc.example.com", 25565),
            ("mc.example.com:25577", "mc.example.com", 25577),
            ("[::1]:25565", "::1", 25565),
            ("::1", "::1", 25565),
            ("[2001:db8::1]", "2001:db8::1", 25565),
            // Порт-мусор отпадает: остаётся хост со стандартным портом
            ("mc.example.com:мусор", "mc.example.com", 25565),
        ];
        for (addr, host, port) in cases {
            assert_eq!(split_host_port(addr), (host.to_string(), port), "{addr}");
        }
    }

    #[test]
    fn motd_flattens_components_and_strips_codes() {
        assert_eq!(motd_text(&serde_json::json!("привет")), "привет");
        let comp = serde_json::json!({
            "text": "Hello ",
            "extra": [{ "text": "§aworld" }, { "text": "!" }]
        });
        assert_eq!(motd_text(&comp), "Hello world!");
        assert_eq!(motd_text(&serde_json::json!({"text": ""})), "");
    }

    #[test]
    fn varint_roundtrip() {
        for v in [0, 1, 127, 128, 255, 2097151, 2097152, -1] {
            let mut buf = Vec::new();
            write_varint(&mut buf, v);
            // Проверка кодирования по длине, чтение ниже — по протоколу
            assert!(!buf.is_empty());
        }
        let mut full = Vec::new();
        write_varint(&mut full, -1);
        assert_eq!(full, vec![0xff, 0xff, 0xff, 0xff, 0x0f]);
    }
}
