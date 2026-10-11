use std::path::Path;

use super::paths;

// Чистит временные артефакты, которые иначе копятся вечно:
// архивы сборок cache/packs (нужны только на время установки),
// недокачанные .part и установщики самообновления data/update.
// Вызывается один раз при старте лаунчера: активных скачиваний
// в этот момент нет, удалять безопасно. Файл, занятый на Windows
// ещё работающим установщиком, просто переживёт до следующего старта.
pub async fn run() {
    purge(&paths::data_dir().join("cache").join("packs")).await;
    purge(&paths::data_dir().join("update")).await;
}

// Удаляет файлы в папке (не трогая подпапки); отсутствие папки — не ошибка
async fn purge(dir: &Path) {
    let Ok(mut rd) = tokio::fs::read_dir(dir).await else {
        return;
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        let path = e.path();
        if path.is_file() {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn purge_removes_files_keeps_dirs() {
        let base = std::env::temp_dir().join(format!("daylo-purge-{}", std::process::id()));
        let dir = base.join("packs");
        let _ = tokio::fs::remove_dir_all(&base).await;
        tokio::fs::create_dir_all(dir.join("nested")).await.unwrap();
        tokio::fs::write(dir.join("pack.zip"), "x").await.unwrap();
        tokio::fs::write(dir.join("pack.zip.part"), "y")
            .await
            .unwrap();
        tokio::fs::write(dir.join("nested").join("deep.txt"), "z")
            .await
            .unwrap();

        purge(&dir).await;

        assert!(!dir.join("pack.zip").exists());
        assert!(!dir.join("pack.zip.part").exists());
        // Подпапки и их содержимое не трогаем
        assert!(dir.join("nested").join("deep.txt").exists());

        // Несуществующая папка — тихо выходим
        purge(&base.join("missing")).await;
        let _ = tokio::fs::remove_dir_all(&base).await;
    }
}
