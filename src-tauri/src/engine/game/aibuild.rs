use std::collections::HashSet;

use futures::future::join_all;
use serde::Serialize;

use super::super::core::http;
use super::modrinth;

// Бесплатный безключевой LLM-шлюз с OpenAI-совместимым ответом.
// Никаких чужих платных API: сервис публичный и рассчитан на прямой доступ.
const LLM: &str = "https://text.pollinations.ai/openai";
const MODEL: &str = "openai";

#[derive(Serialize, Clone)]
pub struct AiPick {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: String,
    pub downloads: u64,
    pub version_id: String,
    pub file_name: String,
}

// Промпт: модель отдаёт только JSON-массив англ. поисковых запросов к Modrinth.
// LLM-шки любят экономить и выдают 3-5 модов даже когда просят больше — поэтому
// требуем большой список с обязательными категориями: сборка из пяти модов
// никому не нужна
fn plan_prompt(user: &str, mc_version: &str, loader: &str) -> String {
    format!(
        "Собери основу большой сборки Minecraft {mc_version} на загрузчике {loader}. \
Тема от пользователя: {user} — это сюжет сборки, а не ограничение количества модов. \
Верни ТОЛЬКО JSON-массив из 18-22 строк — точные английские названия модов из каталога Modrinth, \
совместимых с этой версией игры и загрузчиком. Минимум 15, меньше — брак. \
Обязательно накрой категории: оптимизация, библиотеки/API, рецепты и инвентарь, миникарта или карта, \
хранилище, генерация мира, исследования и данжи, еда или ферма, инструменты, декор и стройка — \
и добавь моды по теме пользователя. Без номера версии в названиях, без markdown, без пояснений, \
без дубликатов."
    )
}

// Добор: второй раунд, когда модов всё ещё меньше десяти — просим другие,
// не дублируя уже найденные
fn more_prompt(user: &str, mc_version: &str, loader: &str, have: &[String]) -> String {
    format!(
        "Собираем сборку Minecraft {mc_version} на загрузчике {loader}. Тема: {user}. \
Уже выбраны: {}. \
Верни ТОЛЬКО JSON-массив из 12 строк — названия ДРУГИХ модов с Modrinth, совместимых \
с этой версией и загрузчиком, не повторяя уже выбранные. Без markdown и пояснений.",
        have.join(", ")
    )
}

// Фолбэк-строка без буллетов и нумерации: «- Create», «1. Create» → «Create»
fn strip_list_marker(line: &str) -> &str {
    let l = line.trim().trim_start_matches(['-', '*', '•']).trim();
    let digits = l.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0 {
        let rest = l[digits..].trim_start();
        if let Some(r) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')')) {
            let r = r.trim_start();
            // «1.21 mods» — это версия игры, а не нумерация: после точки цифры
            if !r.starts_with(|c: char| c.is_ascii_digit()) {
                return r;
            }
        }
    }
    l
}

// Вытаскивает JSON-массив строк из ответа модели (терпит ```json и болтовню вокруг)
fn parse_queries(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let (Some(a), Some(b)) = (body.find('['), body.rfind(']')) {
        if a < b {
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(&body[a..=b]) {
                out = arr;
            }
        }
    }
    if out.is_empty() {
        // Фолбэк: непустые строки без буллетов и нумерации
        out = body
            .lines()
            .map(strip_list_marker)
            .filter(|l| l.len() >= 3 && !l.contains('['))
            .map(|l| l.to_string())
            .collect();
    }
    let mut seen = HashSet::new();
    out.into_iter()
        .map(|q| q.trim().chars().take(80).collect::<String>())
        .filter(|q| !q.is_empty() && seen.insert(q.to_lowercase()))
        .collect()
}

// Просим модель назвать моды; ответ — список поисковых запросов.
// Бесплатный шлюз упирается в квоты (402/429) и иногда отдаёт пустой
// content — до трёх попыток с паузой, черновик ищем и в reasoning
async fn llm_queries(prompt: &str) -> Result<Vec<String>, String> {
    let body = serde_json::json!({
        "model": MODEL,
        "referrer": "daylo-launcher",
        "messages": [
            {
                "role": "system",
                "content": "Ты помощник по сборкам Minecraft. Отвечай только валидным JSON без markdown."
            },
            {"role": "user", "content": prompt}
        ]
    });
    let mut last = String::from("ИИ-шлюз не ответил");
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2 * attempt as u64)).await;
        }
        let res = match http::client().post(LLM).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                last = format!("ИИ-шлюз: {e}");
                continue;
            }
        };
        if !res.status().is_success() {
            last = gateway_status(res.status().as_u16());
            continue;
        }
        let raw: serde_json::Value = match res.json().await {
            Ok(v) => v,
            Err(e) => {
                last = format!("ответ ИИ-шлюза: {e}");
                continue;
            }
        };
        let msg = raw
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"));
        let content = msg
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let queries = parse_queries(content);
        if !queries.is_empty() {
            return Ok(queries);
        }
        // Reasoning-модели съедают лимит токенов рассуждениями — тогда
        // черновик JSON остаётся только в поле reasoning
        if let Some(r) = msg
            .and_then(|m| m.get("reasoning"))
            .and_then(|c| c.as_str())
        {
            let draft = parse_queries(r);
            if !draft.is_empty() {
                return Ok(draft);
            }
        }
        last = "ИИ-шлюз вернул пустой ответ — повторяем".into();
    }
    Err(last)
}

fn gateway_status(code: u16) -> String {
    match code {
        402 => "бесплатный лимит ИИ-шлюза исчерпан — попробуйте через несколько минут".into(),
        429 => "ИИ-шлюз перегружен запросами — попробуйте через минуту".into(),
        500..=599 => format!("ИИ-шлюз временно недоступен (статус {code})"),
        _ => format!("ИИ-шлюз: статус {code}"),
    }
}

// Версии для пачки проектов одним параллельным заходом: сначала вариант под
// наш загрузчик, иначе любая совместимая с версией игры. Проекты без
// подходящей версии просто выпадают, порядок сохраняется
async fn resolve(hits: &[modrinth::SearchHit], gv: &str, loader: &str) -> Vec<AiPick> {
    let versions: Vec<Vec<modrinth::ModVersion>> = join_all(hits.iter().map(|h| {
        let pid = h.project_id.clone();
        let gv = gv.to_string();
        async move { modrinth::versions(&pid, &gv).await.ok().unwrap_or_default() }
    }))
    .await;
    hits.iter()
        .zip(versions)
        .filter_map(|(hit, vers)| {
            let chosen = vers
                .iter()
                .find(|v| v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader)))
                .or_else(|| vers.first())?;
            Some(AiPick {
                project_id: hit.project_id.clone(),
                slug: hit.slug.clone(),
                title: hit.title.clone(),
                description: hit.description.clone(),
                icon_url: hit.icon_url.clone(),
                downloads: hit.downloads,
                version_id: chosen.version_id.clone(),
                file_name: chosen.file_name.clone(),
            })
        })
        .collect()
}

// План сборки: промпт -> список модов -> живые проекты Modrinth с версиями.
// Чтобы план не съёживался до трёх модов, поиск идёт с запасом: до 24 запросов
// параллельно, у каждого держим до трёх хитов (первый — в работу, остальные —
// в резерв), короткий план добираем из резерва, а если и там меньше десяти —
// второй раунд к модели с просьбой назвать другие моды
pub async fn plan(user: &str, mc_version: &str, loader: &str) -> Result<Vec<AiPick>, String> {
    if user.trim().is_empty() {
        return Err("опиши сборку словами".into());
    }
    let queries = llm_queries(&plan_prompt(user, mc_version, loader)).await?;
    let gv = mc_version.to_string();

    // Волна 1: параллельные поиски, до трёх хитов на запрос
    let per_query: Vec<Vec<modrinth::SearchHit>> = join_all(queries.iter().take(24).map(|q| {
        let q = q.clone();
        let gv = gv.clone();
        async move {
            modrinth::search(&q, &gv)
                .await
                .map(|hits| hits.into_iter().take(3).collect())
                .unwrap_or_default()
        }
    }))
    .await;

    // Дубликаты выкидываем, порядок сохраняем — план читается в порядке
    // запросов модели. Первый неповторяющийся хит запроса — в работу,
    // остальные — в резерв на случай отсутствия версии под игру
    let mut seen: HashSet<String> = HashSet::new();
    let mut primary: Vec<modrinth::SearchHit> = Vec::new();
    let mut reserve: Vec<modrinth::SearchHit> = Vec::new();
    for hits in per_query {
        let mut took_top = false;
        for h in hits {
            if seen.insert(h.project_id.clone()) {
                if took_top {
                    reserve.push(h);
                } else {
                    primary.push(h);
                    took_top = true;
                }
            }
        }
    }
    if primary.is_empty() {
        return Err("модель не подобрала модов под эту игру — переформулируй запрос".into());
    }

    // Волна 2: параллельные списки версий под каждый найденный проект
    let mut picks = resolve(&primary, &gv, loader).await;

    // Топ-ап: короткий план добираем резервными хитами тех же поисков —
    // это дешевле второго раунда к модели
    if picks.len() < 12 && !reserve.is_empty() {
        let extra = resolve(&reserve[..reserve.len().min(24)], &gv, loader).await;
        let mut have: HashSet<String> = picks.iter().map(|p| p.project_id.clone()).collect();
        for p in extra {
            if have.insert(p.project_id.clone()) {
                picks.push(p);
            }
        }
    }

    // Всё ещё меньше десяти — второй раунд к модели: другие моды, кроме уже
    // выбранных. Не критично, если раунд сорвался — план уже непустой
    if picks.len() < 10 {
        let have: Vec<String> = picks.iter().map(|p| p.title.clone()).collect();
        if let Ok(more) = llm_queries(&more_prompt(user, mc_version, loader, &have)).await {
            let tops: Vec<Option<modrinth::SearchHit>> = join_all(more.iter().take(12).map(|q| {
                let q = q.clone();
                let gv = gv.clone();
                async move {
                    modrinth::search(&q, &gv)
                        .await
                        .ok()
                        .and_then(|hits| hits.into_iter().next())
                }
            }))
            .await;
            let tops: Vec<modrinth::SearchHit> = tops.into_iter().flatten().collect();
            let extra = resolve(&tops, &gv, loader).await;
            let mut have: HashSet<String> = picks.iter().map(|p| p.project_id.clone()).collect();
            for p in extra {
                if have.insert(p.project_id.clone()) {
                    picks.push(p);
                }
            }
        }
    }

    if picks.is_empty() {
        return Err("модель не подобрала модов под эту игру — переформулируй запрос".into());
    }
    // Подрезаем слишком щедрую модель: больше тридцати модов в основе не нужно
    picks.truncate(30);
    Ok(picks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_carries_context() {
        let p = plan_prompt("техно и магия", "1.20.1", "fabric");
        assert!(p.contains("1.20.1"));
        assert!(p.contains("fabric"));
        assert!(p.contains("техно и магия"));
        assert!(p.contains("JSON"));
        // Просим большую сборку, иначе модель выдаёт три мода
        assert!(p.contains("18-22"));
        assert!(p.contains("Минимум 15"));
    }

    #[test]
    fn more_prompt_lists_existing() {
        let p = more_prompt(
            "техно и магия",
            "1.20.1",
            "fabric",
            &["Create".into(), "JEI".into()],
        );
        assert!(p.contains("Create"));
        assert!(p.contains("JEI"));
        assert!(p.contains("12"));
        assert!(p.contains("JSON"));
    }

    #[test]
    fn parse_plain_array() {
        assert_eq!(
            parse_queries(r#"["Create","JEI"]"#),
            vec!["Create".to_string(), "JEI".to_string()]
        );
    }

    #[test]
    fn parse_tolerates_fence_and_chatter() {
        let body =
            "Вот план:\n```json\n[\"Create\", \"Applied Energistics 2\"]\n```\nУдачной игры!";
        assert_eq!(
            parse_queries(body),
            vec!["Create".to_string(), "Applied Energistics 2".to_string()]
        );
    }

    #[test]
    fn parse_falls_back_to_lines_and_dedupes() {
        let body = "1. Create\n2. Create\n- JEI\n";
        assert_eq!(
            parse_queries(body),
            vec!["Create".to_string(), "JEI".to_string()]
        );
    }

    #[test]
    fn strip_marker_handles_numbering() {
        assert_eq!(strip_list_marker("1. Create"), "Create");
        assert_eq!(strip_list_marker("12) Applied"), "Applied");
        assert_eq!(strip_list_marker("- JEI"), "JEI");
        assert_eq!(strip_list_marker("JEI"), "JEI");
        // Это не нумерация — ничего не срезаем
        assert_eq!(strip_list_marker("1.21 mods"), "1.21 mods");
    }

    #[test]
    fn gateway_status_is_friendly() {
        assert!(gateway_status(402).contains("лимит"));
        assert!(gateway_status(429).contains("перегружен"));
        assert!(gateway_status(503).contains("503"));
        assert!(gateway_status(400).contains("400"));
    }
}
