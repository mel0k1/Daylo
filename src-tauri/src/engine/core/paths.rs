use std::path::PathBuf;

// Папка данных лаунчера
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("net.daylo.launcher")
}

// Корень файлов игры
pub fn game_root() -> PathBuf {
    data_dir().join("minecraft")
}
