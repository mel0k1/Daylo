use serde::Serialize;

use super::super::core::paths;

#[derive(Serialize, Clone)]
pub struct CrashInfo {
    // Сборка, в которой случился краш: модал показывается на любом экране
    pub instance: String,
    pub title: String,
    pub reason: String,
    pub advice: Vec<String>,
    pub excerpt: Vec<String>,
    pub exit_code: Option<i32>,
}

// Читает лог и crash-reports упавшей сборки и объясняет причину по-человечески
pub async fn analyze(version: &str, exit_code: Option<i32>) -> Option<CrashInfo> {
    if exit_code == Some(0) {
        return None;
    }
    let lines = collect_lines(version).await;
    let (title, reason, advice) = match detect(&lines) {
        Some(x) => x,
        None => (
            "Игра завершилась аварийно".into(),
            match exit_code {
                Some(c) => format!("Игра закрылась с кодом {c}. Точных указаний в логе нет."),
                None => "Игра закрылась аварийно, точных указаний в логе нет.".to_string(),
            },
            vec![
                "Посмотрите последние строки лога ниже — часто там видна конкретная причина."
                    .into(),
                "Если в папке сборки появился файл crash-reports, пришлите его тем, кто поможет."
                    .into(),
            ],
        ),
    };
    Some(CrashInfo {
        instance: version.to_string(),
        title,
        reason,
        advice,
        excerpt: excerpt(&lines),
        exit_code,
    })
}

async fn collect_lines(version: &str) -> Vec<String> {
    let dir = paths::instance_dir(version);
    let log = dir.join("logs").join("latest.txt");
    let mut lines = read_head(&log, 4000).await;
    if let Ok(mut rd) = tokio::fs::read_dir(dir.join("crash-reports")).await {
        let mut files: Vec<std::path::PathBuf> = Vec::new();
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.path().is_file() {
                files.push(e.path());
            }
        }
        files.sort();
        if let Some(f) = files.pop() {
            lines.extend(read_head(&f, 300).await);
        }
    }
    lines
}

async fn read_head(path: &std::path::Path, max: usize) -> Vec<String> {
    match tokio::fs::read_to_string(path).await {
        Ok(body) => body
            .lines()
            .map(|l| l.trim_end().to_string())
            .take(max)
            .collect(),
        Err(_) => Vec::new(),
    }
}

type Cause = (String, String, Vec<String>);

// Таблица причин: от конкретных к общим
fn detect(lines: &[String]) -> Option<Cause> {
    let low: Vec<String> = lines.iter().map(|l| l.to_lowercase()).collect();
    let hit = |needle: &str| low.iter().any(|l| l.contains(needle));

    if hit("outofmemoryerror") {
        if hit("metaspace") {
            return Some((
                "Переполнен метаспейс Java".into(),
                "Классов в игре оказалось так много, что служебная память Java (Metaspace) кончилась. Обычно виноваты моды или сборки с огромным количеством библиотек.".into(),
                vec![
                    "Добавьте в JVM-флаги сборки: -XX:MaxMetaspaceSize=768m".into(),
                    "Уберите лишние моды из сборки.".into(),
                ],
            ));
        }
        if hit("direct buffer") {
            return Some((
                "Недостаточно памяти для буферов".into(),
                "Игра запросила больше прямой памяти, чем Java разрешает выделять.".into(),
                vec![
                    "Добавьте в JVM-флаги сборки: -XX:MaxDirectMemorySize=1G".into(),
                    "Убавьте дальность прорисовки и включите меньшее разрешение текстур.".into(),
                ],
            ));
        }
        return Some((
            "Недостаточно памяти".into(),
            "Игра израсходовала всю память, выделенную через -Xmx.".into(),
            vec![
                "Увеличьте память в настройках сборки, например до 4096 МБ.".into(),
                "Если в системе мало памяти — закройте браузер и другие программы.".into(),
                "Моды вроде шейдеров и HD-текстур требуют больше памяти, чем ванила.".into(),
            ],
        ));
    }
    if hit("unsupportedclassversionerror") {
        return Some((
            "Неподходящая версия Java".into(),
            "Игра запущена на Java другой версии, чем требует Minecraft.".into(),
            vec![
                "Удалите папку java в данных лаунчера — при следующем запуске скачается нужная."
                    .into(),
                "Проверьте пользовательские JVM-флаги: возможно, там жёстко задан путь к Java."
                    .into(),
            ],
        ));
    }
    if hit("classnotfoundexception") || hit("noclassdeffounderror") {
        return Some((
            "Повреждён или отсутствует класс".into(),
            "Часть файлов игры не загрузилась: битая библиотека, недокачанный клиент или мод."
                .into(),
            vec![
                "Удалите и заново установите сборку.".into(),
                "Если краш появился после установки мода — уберите его.".into(),
            ],
        ));
    }
    if (hit("mods.toml") && hit("missing")) || hit("fabric.mod.json") || hit("not a valid mod") {
        return Some((
            "Мод не для этого загрузчика".into(),
            "В папке mods лежит файл без метаданных Fabric/Forge — скорее всего, мод для другого загрузчика.".into(),
            vec![
                "Уберите из mods файл, упомянутый в логе ниже.".into(),
                "Скачивайте моды под тот же загрузчик, что и сборка (Fabric или Forge).".into(),
            ],
        ));
    }
    if hit("incompatible mod set") {
        return Some((
            "Несовместимый набор модов".into(),
            "Загрузчик Fabric не смог собрать рабочую связку из установленных модов.".into(),
            vec![
                "Уберите моды, которые были добавлены последними.".into(),
                "Проверьте, что все моды выпущены под вашу версию игры и Fabric.".into(),
            ],
        ));
    }
    if hit("mixinapplyerror") || hit("mixin apply") || (hit("mixin") && hit("failed")) {
        return Some((
            "Конфликт модов (Mixin)".into(),
            "Один из модов врезался в другой: оба правят один и тот же код игры.".into(),
            vec![
                "Уберите моды, установленные последними, и добавляйте их по одному.".into(),
                "Обновите конфликтующие моды до свежих версий.".into(),
            ],
        ));
    }
    if hit("duplicate mods found") || hit("duplicate mod") {
        return Some((
            "Дубликаты модов".into(),
            "Один и тот же мод положен в mods дважды (или в двух форматах .jar/.zip).".into(),
            vec!["Оставьте в mods по одной копии каждого мода.".into()],
        ));
    }
    if hit("nosuchmethoderror") || hit("nosuchfielderror") {
        return Some((
            "Моды несовместимы между собой".into(),
            "Мод рассчитан на другую версию игры или другого мода-зависимости.".into(),
            vec![
                "Обновите все моды до версий под вашу игру.".into(),
                "Проверьте, что версии Fabric API и основных библиотек совпадают с версией игры."
                    .into(),
            ],
        ));
    }
    if hit("glfw error") || hit("failed to create window") || hit("window creation") {
        return Some((
            "Не удалось создать окно игры".into(),
            "Система отказалась открывать окно Minecraft — обычно проблема с драйвером или графикой.".into(),
            vec![
                "Обновите драйвер видеокарты.".into(),
                "Если работаете через удалённый стол — запускайте игру локально.".into(),
                "Попробуйте снизить разрешение экрана.".into(),
            ],
        ));
    }
    if hit("opengl") && (hit("error") || hit("unsupported") || hit("failed")) {
        return Some((
            "Проблема с OpenGL".into(),
            "Видеодрайвер не дал игре нужную версию OpenGL.".into(),
            vec![
                "Обновите драйвер видеокарты с сайта производителя (NVIDIA/AMD/Intel).".into(),
                "На ноутбуках с двумя видеокартами запустите игру на дискретной.".into(),
            ],
        ));
    }
    if hit("accessdeniedexception") || hit("access is denied") {
        return Some((
            "Нет доступа к файлу".into(),
            "Игра не смогла прочитать или записать файл — мешают права или антивирус.".into(),
            vec![
                "Добавьте папку лаунчера и сборок в исключения антивируса.".into(),
                "Запустите лаунчер от администратора один раз.".into(),
            ],
        ));
    }
    if hit("hs_err_pid") || hit("sigsegv") || hit("exception_access_violation") {
        return Some((
            "Краш виртуальной машины Java".into(),
            "Сама Java упала на уровне системы — обычно это драйвер графики или сбойная память."
                .into(),
            vec![
                "Обновите драйвер видеокарты.".into(),
                "Проверьте память системы (memtest на Windows, mdsostress на Linux).".into(),
                "Уменьшите выделенную память сборки.".into(),
            ],
        ));
    }
    if hit("failed to download") || hit("connection timed out") || hit("unknownhostexception") {
        return Some((
            "Проблема с сетью".into(),
            "Игра или загрузчик не смогли скачать нужные файлы.".into(),
            vec![
                "Проверьте интернет и попробуйте запустить снова.".into(),
                "Включите зеркала загрузок в настройках, если провайдер режет доступ.".into(),
            ],
        ));
    }
    if hit("exception in thread \"main\"") {
        return Some((
            "Ошибка при старте игры".into(),
            "Главный поток игры упал ещё до запуска окна.".into(),
            vec![
                "Посмотрите строки ниже: чаще всего там имя недостающего файла или класса.".into(),
                "Если ставили моды — уберите последние добавленные.".into(),
            ],
        ));
    }
    None
}

// Кусок лога вокруг первой ошибки, для модалки
fn excerpt(lines: &[String]) -> Vec<String> {
    if lines.is_empty() {
        return vec!["(лог пуст — игра не успела ничего записать)".into()];
    }
    let idx = lines
        .iter()
        .position(|l| l.contains("Exception") || l.contains("Error") || l.contains("ERROR"));
    let (start, end) = match idx {
        Some(i) => (i.saturating_sub(8), (i + 18).min(lines.len())),
        None => (lines.len().saturating_sub(20), lines.len()),
    };
    lines[start..end]
        .iter()
        .map(|l| {
            if l.chars().count() > 300 {
                format!("{}…", l.chars().take(300).collect::<String>())
            } else {
                l.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn ordinary_lines_are_not_a_crash() {
        let c = detect(&lines(&["игра работает", "все ок"]));
        assert!(c.is_none());
    }

    #[test]
    fn detects_oom_heap() {
        let c = detect(&lines(&[
            "[main/INFO]: Starting minecraft",
            "java.lang.OutOfMemoryError: Java heap space",
            "\tat java.base/java.util.ArrayList.resize",
        ]))
        .unwrap();
        assert_eq!(c.0, "Недостаточно памяти");
        assert!(!c.2.is_empty());
    }

    #[test]
    fn detects_metaspace() {
        let c = detect(&lines(&["java.lang.OutOfMemoryError: Metaspace"])).unwrap();
        assert!(c.0.to_lowercase().contains("метаспейс"));
    }

    #[test]
    fn detects_mixin_conflict() {
        let c = detect(&lines(&[
            "org.spongepowered.asm.mixin.transformer.MixinApplyError: Mixin apply failed",
        ]))
        .unwrap();
        assert!(c.0.contains("Mixin"));
    }

    #[test]
    fn detects_wrong_loader_mod() {
        let c = detect(&lines(&["Mod file nope.jar is missing fabric.mod.json"])).unwrap();
        assert!(c.0.contains("загрузчика"));
    }

    #[test]
    fn detects_glfw() {
        let c = detect(&lines(&["GLFW error 65543: GLX: Failed to create context"])).unwrap();
        assert!(c.0.contains("окно") || c.0.contains("OpenGL"));
    }

    #[tokio::test]
    async fn falls_back_to_generic() {
        // Несуществующая сборка: лог пуст, но краш с кодом должен объясниться общим текстом
        let c = analyze("does-not-exist", Some(1)).await.unwrap();
        assert!(c.title.contains("аварийно"));
        assert!(c.excerpt[0].contains("лог пуст"));
    }

    #[tokio::test]
    async fn zero_exit_is_not_a_crash() {
        assert!(analyze("does-not-exist", Some(0)).await.is_none());
    }

    #[test]
    fn excerpt_centers_on_error() {
        let mut l = vec!["строка начала".to_string()];
        for i in 0..30 {
            l.push(format!("обычный шаг {i}"));
        }
        l.push("java.lang.RuntimeException: boom".to_string());
        for i in 0..40 {
            l.push(format!("после падения {i}"));
        }
        let e = excerpt(&l);
        assert!(e.iter().any(|x| x.contains("boom")));
        assert!(e.len() <= 30);
    }

    #[test]
    fn excerpt_cuts_long_lines() {
        let long = "x".repeat(1000);
        let e = excerpt(&lines(&[&long]));
        assert!(e[0].chars().count() <= 301);
    }

    #[test]
    fn excerpt_empty_log() {
        let e = excerpt(&[]);
        assert!(e[0].contains("лог пуст"));
    }
}
