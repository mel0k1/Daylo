use std::collections::HashSet;

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

// Промпт: модель отдаёт только JSON-массив англ. поисковых запросов к Modrinth
fn plan_prompt(user: &str, mc_version: &str, loader: &str) -> String {
    format!(
        "Собери список модов для сборки Minecraft {mc_version} на загрузчике {loader}. \
Тема сборки от пользователя: {user}. \
Верни ТОЛЬКО JSON-массив из 5-10 строк — точные английские названия модов из каталога Modrinth, \
совместимых с этой версией игры и загрузчиком. Без markdown, без пояснений, без дубликатов."
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

// План сборки: промпт -> список модов -> живые проекты Modrinth с версиями
pub async fn plan(user: &str, mc_version: &str, loader: &str) -> Result<Vec<AiPick>, String> {
    if user.trim().is_empty() {
        return Err("опиши сборку словами".into());
    }
    let queries = llm_queries(&plan_prompt(user, mc_version, loader)).await?;
    let mut picks: Vec<AiPick> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for q in queries.iter().take(12) {
        let hits = match modrinth::search(q, mc_version).await {
            Ok(h) => h,
            Err(_) => continue,
        };
        let Some(hit) = hits.into_iter().next() else {
            continue;
        };
        if !seen.insert(hit.project_id.clone()) {
            continue;
        }
        let vers = match modrinth::versions(&hit.project_id, mc_version).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Сначала версия под наш загрузчик, иначе любая совместимая с игрой
        let chosen = vers
            .iter()
            .find(|v| v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader)))
            .or_else(|| vers.first());
        let Some(v) = chosen else {
            continue;
        };
        picks.push(AiPick {
            project_id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            icon_url: hit.icon_url,
            downloads: hit.downloads,
            version_id: v.version_id.clone(),
            file_name: v.file_name.clone(),
        });
    }
    if picks.is_empty() {
        return Err("модель не подобрала модов под эту игру — переформулируй запрос".into());
    }
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
