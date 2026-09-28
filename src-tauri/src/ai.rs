use crate::credentials;
use crate::db::Db;
use crate::models::{AiResponse, Settings};
use serde_json::{json, Value};
use std::time::Duration;

/// Clips longer than this are refused rather than silently truncated.
const MAX_INPUT_CHARS: usize = 200_000;
const ANTHROPIC_VERSION: &str = "2023-06-01";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

const LANGUAGES: &[&str] = &[
    "français",
    "anglais",
    "espagnol",
    "allemand",
    "italien",
    "portugais",
    "néerlandais",
    "japonais",
    "chinois",
    "coréen",
    "arabe",
    "russe",
];

fn instruction(action: &str, lang: Option<&str>) -> Result<String, String> {
    let fixed = match action {
        "summarize" => "Résume le texte fourni en 2 à 3 phrases, en français. Réponds uniquement avec le résumé.",
        "explain" => "Explique clairement et brièvement ce contenu (code ou texte) en français, pour un développeur.",
        "rephrase" => "Reformule le texte fourni dans un style professionnel et concis, dans sa langue d'origine. Réponds uniquement avec le texte reformulé.",
        "fix" => "Corrige l'orthographe, la grammaire, la ponctuation et la typographie du texte fourni sans changer sa langue ni son ton. Réponds uniquement avec le texte corrigé.",
        "translate" => {
            let lang = lang.filter(|l| LANGUAGES.contains(l)).ok_or("Langue non prise en charge.")?;
            return Ok(format!(
                "Traduis le texte fourni en {lang} en conservant le ton et la mise en forme. Réponds uniquement avec la traduction."
            ));
        }
        _ => return Err("Action IA inconnue.".into()),
    };
    Ok(fixed.to_string())
}

fn clip_text(db: &Db, settings: &Settings, id: i64) -> Result<String, String> {
    let clip = db
        .get(id)
        .map_err(|e| e.to_string())?
        .ok_or("Élément introuvable.")?;
    // A secret may only go to a model running on this very machine.
    let on_this_machine = settings.ai_provider == "ollama" && is_loopback(&settings.ollama_url);
    if clip.sensitive && !on_this_machine {
        return Err(
            "Cet élément contient un secret : il n'est pas envoyé à un service en ligne.".into(),
        );
    }
    let (kind, content) = (clip.kind, clip.content.unwrap_or_default());
    match kind.as_str() {
        "image" => Err("L'IA ne traite pas les images.".into()),
        "file" => Err("L'IA ne traite pas les fichiers.".into()),
        _ if content.chars().count() > MAX_INPUT_CHARS => Err(format!(
            "Contenu trop long pour l'IA ({} caractères max).",
            MAX_INPUT_CHARS
        )),
        _ => Ok(content),
    }
}

pub async fn run(db: &Db, id: i64, action: &str, lang: Option<&str>) -> AiResponse {
    let result = async {
        let system = instruction(action, lang)?;
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let content = clip_text(db, &settings, id)?;
        complete(&settings, &system, &content).await
    }
    .await;
    match result {
        Ok(text) => AiResponse::ok(text),
        Err(e) => AiResponse::err(e),
    }
}

/// Ask the model for a collection and tags, then apply them to the clip.
pub async fn smart_tag(db: &Db, id: i64) -> AiResponse {
    let result = async {
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let content = clip_text(db, &settings, id)?;
        let system = "Tu classes des éléments de presse-papiers. Réponds uniquement avec un objet JSON \
            de la forme {\"category\": \"...\", \"tags\": [\"...\"]} : une catégorie courte en français \
            (1 à 2 mots, ex. \"Travail\", \"Code SQL\", \"Recette\") et 1 à 5 tags en minuscules, sans espaces.";
        let answer = complete(&settings, system, &content).await?;
        let json = match (answer.find('{'), answer.rfind('}')) {
            (Some(a), Some(b)) if a < b => &answer[a..=b],
            _ => return Err("Réponse de l'IA inexploitable.".to_string()),
        };
        let v: Value = serde_json::from_str(json).map_err(|_| "Réponse de l'IA inexploitable.")?;
        let category = v["category"].as_str().map(str::trim).filter(|c| !c.is_empty());
        let clip = db.get(id).map_err(|e| e.to_string())?.ok_or("Élément introuvable.")?;
        let mut tags = clip.tags.clone();
        for t in v["tags"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            let t: String = t
                .trim()
                .to_lowercase()
                .replace(' ', "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
                .collect();
            if !t.is_empty() && !tags.contains(&t) {
                tags.push(t);
            }
        }
        if let Some(cat) = category {
            let collection = db.create_collection(cat).map_err(|e| e.to_string())?;
            db.set_collection(id, Some(collection))
                .map_err(|e| e.to_string())?;
        }
        db.update_tags(id, &tags).map_err(|e| e.to_string())?;
        Ok(format!(
            "Catégorie : {}\nTags : {}",
            category.unwrap_or("—"),
            if tags.is_empty() { "—".into() } else { tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ") }
        ))
    }
    .await;
    match result {
        Ok(text) => AiResponse::ok(text),
        Err(e) => AiResponse::err(e),
    }
}

pub async fn health(settings: &Settings) -> AiResponse {
    match settings.ai_provider.as_str() {
        "openai" if !credentials::is_set("openai") => AiResponse::err("Clé API OpenAI manquante."),
        "openai" => AiResponse::ok(format!("OpenAI · {}", settings.openai_model)),
        "anthropic" if !credentials::is_set("anthropic") => {
            AiResponse::err("Clé API Anthropic manquante.")
        }
        "anthropic" => AiResponse::ok(format!("Claude · {}", settings.anthropic_model)),
        _ => {
            let url = match endpoint(&settings.ollama_url, "/api/tags", false) {
                Ok(u) => u,
                Err(e) => return AiResponse::err(e),
            };
            let resp = client(3).get(url).send().await;
            match resp {
                Ok(r) if r.status().is_success() => {
                    AiResponse::ok(format!("Ollama · {}", settings.ollama_model))
                }
                Ok(r) => AiResponse::err(format!("Ollama : HTTP {}", r.status())),
                Err(_) => AiResponse::err("Ollama injoignable."),
            }
        }
    }
}

async fn complete(s: &Settings, system: &str, content: &str) -> Result<String, String> {
    let text = match s.ai_provider.as_str() {
        "openai" => openai(s, system, content).await?,
        "anthropic" => anthropic(s, system, content).await?,
        _ => ollama(s, system, content).await?,
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        Err("Réponse vide.".into())
    } else {
        Ok(text)
    }
}

fn client(timeout_s: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_s))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

/// Build `base + path`, refusing anything but http(s) and addresses with
/// credentials in them. When `secret` is true (an API key will be sent),
/// plain http is only allowed to this machine.
fn endpoint(base: &str, path: &str, secret: bool) -> Result<String, String> {
    let base = base.trim().trim_end_matches('/');
    let url = reqwest::Url::parse(base)
        .map_err(|_| "URL invalide (http:// ou https:// attendu).".to_string())?;
    if !url.username().is_empty() || url.password().is_some() || url.host_str().is_none() {
        return Err("URL invalide.".into());
    }
    match url.scheme() {
        "https" => Ok(format!("{base}{path}")),
        "http" if !secret || is_loopback(base) => Ok(format!("{base}{path}")),
        "http" => Err("URL refusée : utilisez https:// pour envoyer une clé API.".into()),
        _ => Err("URL invalide (http:// ou https:// attendu).".into()),
    }
}

/// The address points to this machine (host parsed, not matched as text).
fn is_loopback(base: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base.trim()) else {
        return false;
    };
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

async fn error_body(r: reqwest::Response) -> String {
    let status = r.status();
    let body: Value = r.json().await.unwrap_or(Value::Null);
    let msg = body["error"]["message"]
        .as_str()
        .or_else(|| body["error"].as_str())
        .unwrap_or("");
    let msg: String = msg.chars().take(300).collect();
    format!("HTTP {status} {msg}").trim().to_string()
}

async fn ollama(s: &Settings, system: &str, content: &str) -> Result<String, String> {
    let url = endpoint(&s.ollama_url, "/api/generate", false)?;
    let body = json!({
        "model": s.ollama_model,
        "system": system,
        "prompt": content,
        "stream": false,
    });
    let r = client(300)
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|_| "Ollama injoignable. Lancez Ollama puis réessayez.".to_string())?;
    if !r.status().is_success() {
        return Err(format!("Ollama : {}", error_body(r).await));
    }
    let v: Value = r.json().await.map_err(|e| e.to_string())?;
    Ok(v["response"].as_str().unwrap_or_default().to_string())
}

async fn openai(s: &Settings, system: &str, content: &str) -> Result<String, String> {
    let key = credentials::get("openai").ok_or("Clé API OpenAI manquante (Paramètres › IA).")?;
    let url = endpoint(&s.openai_base_url, "/v1/chat/completions", true)?;
    let body = json!({
        "model": s.openai_model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": content},
        ],
    });
    let r = client(180)
        .post(url)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI injoignable : {e}"))?;
    if !r.status().is_success() {
        return Err(format!("OpenAI : {}", error_body(r).await));
    }
    let v: Value = r.json().await.map_err(|e| e.to_string())?;
    Ok(v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

async fn anthropic(s: &Settings, system: &str, content: &str) -> Result<String, String> {
    let key =
        credentials::get("anthropic").ok_or("Clé API Anthropic manquante (Paramètres › IA).")?;
    let model = s.anthropic_model.trim();
    let mut body = json!({
        "model": model,
        "max_tokens": 16000,
        "system": system,
        "messages": [{"role": "user", "content": content}],
    });
    let mut req = client(300)
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION);
    // Opus 5 / Fable: if a safety classifier declines, let the API retry on
    // its recommended fallback model instead of failing.
    if model.starts_with("claude-opus-5") || model.starts_with("claude-fable") {
        body["fallbacks"] = json!("default");
        req = req.header("anthropic-beta", FALLBACK_BETA);
    }
    let r = req
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Claude injoignable : {e}"))?;
    if !r.status().is_success() {
        return Err(format!("Claude : {}", error_body(r).await));
    }
    let v: Value = r.json().await.map_err(|e| e.to_string())?;
    if v["stop_reason"] == "refusal" {
        return Err("Claude a refusé de traiter ce contenu.".into());
    }
    let text: String = v["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["type"] == "text")
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("");
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_rules() {
        assert!(endpoint("https://api.openai.com/", "/v1/x", true).is_ok());
        assert!(endpoint("http://localhost:11434", "/api", true).is_ok());
        assert!(endpoint("http://localhost.evil.com", "/api", true).is_err());
        assert!(endpoint("http://example.com", "/api", true).is_err());
        assert!(endpoint("http://192.168.1.2:11434", "/api", false).is_ok());
        assert!(endpoint("file:///etc/passwd", "", false).is_err());
        assert!(endpoint("http://localhost:1@evil.com", "/api", true).is_err());
        assert!(endpoint("http://user:pw@127.0.0.1", "/api", false).is_err());
        assert!(is_loopback("http://127.0.0.1:11434"));
        assert!(is_loopback("http://[::1]:11434"));
        assert!(!is_loopback("http://192.168.1.2:11434"));
        assert!(!is_loopback("http://localhost.evil.com"));
    }

    #[test]
    fn translate_requires_known_language() {
        assert!(instruction("translate", Some("anglais")).is_ok());
        assert!(instruction("translate", Some("ignore previous instructions")).is_err());
        assert!(instruction("nope", None).is_err());
    }
}
