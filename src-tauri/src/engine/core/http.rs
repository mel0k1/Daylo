use std::io::Read;
use std::path::Path;
use std::time::Duration;

use sha1::Digest;

use super::mirrors;

// Единственный клиент на всё приложение
pub fn client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("Daylo/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .expect("не удалось создать http-клиент")
    })
}

pub async fn text(url: &str) -> Result<String, String> {
    let res = client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("запрос {url}: {e}"))?;
    let res = res
        .error_for_status()
        .map_err(|e| format!("запрос {url}: {e}"))?;
    res.text().await.map_err(|e| format!("ответ {url}: {e}"))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn digest_of<D: Digest>(path: &Path) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("чтение {}: {e}", path.display()))?;
    let mut h = D::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(to_hex(&h.finalize()))
}

pub fn sha1_of(path: &Path) -> Result<String, String> {
    digest_of::<sha1::Sha1>(path)
}

pub fn sha256_of(path: &Path) -> Result<String, String> {
    digest_of::<sha2::Sha256>(path)
}

fn file_ok(to: &Path, sha1: Option<&str>, sha256: Option<&str>, size: Option<u64>) -> bool {
    if let (Ok(meta), Some(size)) = (std::fs::metadata(to), size) {
        if meta.len() != size {
            return false;
        }
    }
    if let Some(want) = sha1 {
        match sha1_of(to) {
            Ok(got) if got.eq_ignore_ascii_case(want) => {}
            _ => return false,
        }
    }
    if let Some(want) = sha256 {
        match sha256_of(to) {
            Ok(got) if got.eq_ignore_ascii_case(want) => {}
            _ => return false,
        }
    }
    true
}

// Скачивает в <файл>.part, проверяет хеш и размер, потом ставит на место
pub async fn download(
    url: &str,
    to: &Path,
    sha1: Option<&str>,
    sha256: Option<&str>,
    size: Option<u64>,
) -> Result<(), String> {
    if to.exists() && file_ok(to, sha1, sha256, size) {
        return Ok(());
    }
    let parent = to.parent().ok_or("путь без родителя")?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| format!("папка {}: {e}", parent.display()))?;

    let mut last = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(attempt)).await;
        }
        match download_once(url, to, sha1, sha256, size).await {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
    }
    Err(last)
}

// Скачивание по списку адресов-кандидатов: зеркало → оригинал. На каждый
// адрес две попытки, чтобы один сбой зеркала не ронял загрузку.
pub async fn download_mirrored(
    url: &str,
    to: &Path,
    sha1: Option<&str>,
    sha256: Option<&str>,
    size: Option<u64>,
) -> Result<(), String> {
    if to.exists() && file_ok(to, sha1, sha256, size) {
        return Ok(());
    }
    let routes = mirrors::routes(url).await;
    let mut last = String::new();
    for route in &routes {
        for attempt in 0..2 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(attempt)).await;
            }
            match download_once(route, to, sha1, sha256, size).await {
                Ok(()) => return Ok(()),
                Err(e) => last = e,
            }
        }
    }
    Err(last)
}

// Текст по маршрутам «зеркало → оригинал»: первый удачный ответ побеждает
pub async fn text_mirrored(url: &str) -> Result<String, String> {
    let routes = mirrors::routes(url).await;
    let mut last = String::new();
    for route in &routes {
        match text(route).await {
            Ok(t) => return Ok(t),
            Err(e) => last = e,
        }
    }
    Err(last)
}

async fn download_once(
    url: &str,
    to: &Path,
    sha1: Option<&str>,
    sha256: Option<&str>,
    size: Option<u64>,
) -> Result<(), String> {
    let res = client()
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("скачивание {url}: {e}"))?;

    let part = to.with_extension("part");
    let mut file = tokio::fs::File::create(&part)
        .await
        .map_err(|e| format!("создание {}: {e}", part.display()))?;
    let mut stream = res.bytes_stream();
    let mut hasher1 = sha1::Sha1::new();
    let mut hasher2 = sha2::Sha256::new();
    let mut len: u64 = 0;

    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("поток {url}: {e}"))?;
        hasher1.update(&chunk);
        hasher2.update(&chunk);
        len += chunk.len() as u64;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("запись {}: {e}", part.display()))?;
    }
    file.flush()
        .await
        .map_err(|e| format!("запись {}: {e}", part.display()))?;
    drop(file);

    if let Some(want) = size {
        if len != want {
            return Err(format!("{url}: размер {len}, ожидалось {want}"));
        }
    }
    if let Some(want) = sha1 {
        let got = to_hex(&hasher1.finalize());
        if !got.eq_ignore_ascii_case(want) {
            return Err(format!("{url}: sha1 {got}, ожидалось {want}"));
        }
    }
    if let Some(want) = sha256 {
        let got = to_hex(&hasher2.finalize());
        if !got.eq_ignore_ascii_case(want) {
            return Err(format!("{url}: sha256 {got}, ожидалось {want}"));
        }
    }

    tokio::fs::rename(&part, to)
        .await
        .map_err(|e| format!("переименование {}: {e}", to.display()))?;
    Ok(())
}
