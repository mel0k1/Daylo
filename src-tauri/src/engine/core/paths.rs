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

pub fn versions_dir() -> PathBuf {
    game_root().join("versions")
}

pub fn libraries_dir() -> PathBuf {
    game_root().join("libraries")
}

pub fn assets_dir() -> PathBuf {
    game_root().join("assets")
}

pub fn java_dir() -> PathBuf {
    data_dir().join("java")
}

// Папка запускаемой сборки
pub fn instance_dir(version: &str) -> PathBuf {
    game_root().join("instances").join(safe_name(version))
}

pub fn version_dir(version: &str) -> PathBuf {
    versions_dir().join(safe_name(version))
}

// Отсекает обход путей и мусор в именах версий
pub fn safe_name(name: &str) -> String {
    let mut out = String::new();
    let mut dot = false;
    for c in name.chars() {
        if c == '.' {
            if !dot {
                out.push('.');
                dot = true;
            }
        } else if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '+') {
            out.push(c);
            dot = false;
        }
    }
    let trimmed = out.trim_matches('.').trim();
    if trimmed.is_empty() {
        "unknown".into()
    } else {
        trimmed.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_name_cuts_traversal() {
        assert_eq!(safe_name("../../etc"), "etc");
        assert_eq!(safe_name("..\\windows"), "windows");
        assert_eq!(safe_name("..."), "unknown");
        assert_eq!(safe_name(""), "unknown");
        assert_eq!(safe_name("1.21.9"), "1.21.9");
        assert_eq!(safe_name("21w44a"), "21w44a");
    }
}
