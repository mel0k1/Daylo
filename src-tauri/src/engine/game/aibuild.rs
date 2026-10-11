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
            .map(|l| l.trim().trim_start_matches(['-', '*', '•', ' ']).trim())
            .filter(|l| l.len() >= 3 && !l.contains('['))
            .map(|l| l.to_string())
            .collect();
    }
    let mut seen = HashSet::new();
    out.into_iter()
        .map(|q| q.trim().chars().take(80).collect::<String>())
        .filter(|q| !q.is_empty() && seen.insert(q.clone()))
        .collect()
}

// Просим модель назвать моды; ответ — список поисковых запросов
async fn llm_queries(prompt: &str) -> Result<Vec<String>, String> {
    let body = serde_json::json!({
        "model": MODEL,
        "messages": [
            {
                "role": "system",
                "content": "Ты помощник по сборкам Minecraft. Отвечай только валидным JSON без markdown."
            },
            {"role": "user", "content": prompt}
        ]
    });
    let res = http::client()
        .post(LLM)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ИИ-шлюз: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("ИИ-шлюз: статус {}", res.status()));
    }
    let raw: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("ответ ИИ-шлюза: {e}"))?;
    let content = raw
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or("пустой ответ ИИ-шлюза")?;
    Ok(parse_queries(content))
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
            vec![
                "1. Create".to_string(),
                "2. Create".to_string(),
                "JEI".to_string()
            ]
        );
    }
}
