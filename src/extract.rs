use serde_json::Value;

/// Pull model IDs out of a catalog JSON blob.
pub fn extract(kind: &str, value: &Value) -> Result<Vec<String>, String> {
    let mut ids = match kind {
        "openai_list" => openai_list(value),
        "models_dev" => models_dev(value),
        "object_keys" => object_keys(value),
        other => return Err(format!("unknown kind {other}")),
    };
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// Pull model IDs out of HTML/Markdown using capture-group regexes.
pub fn extract_text(kind: &str, text: &str, patterns: &[String]) -> Result<Vec<String>, String> {
    match kind {
        "html_regex" => html_regex(text, patterns),
        other => Err(format!("kind {other} is not a text extractor")),
    }
}

fn openai_list(value: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(arr) = value.get("data").and_then(|v| v.as_array()) {
        for item in arr {
            push_id(&mut ids, item.get("id").and_then(|v| v.as_str()));
        }
    }
    ids
}

fn models_dev(value: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    let Some(providers) = value.as_object() else {
        return ids;
    };
    for provider in providers.values() {
        let Some(models) = provider.get("models").and_then(|v| v.as_object()) else {
            continue;
        };
        for (key, model) in models {
            if let Some(id) = model.get("id").and_then(|v| v.as_str()) {
                push_id(&mut ids, Some(id));
            } else {
                push_id(&mut ids, Some(key));
            }
        }
    }
    ids
}

fn object_keys(value: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    let Some(map) = value.as_object() else {
        return ids;
    };
    for key in map.keys() {
        if key == "sample_spec" {
            continue;
        }
        push_id(&mut ids, Some(key));
    }
    ids
}

fn html_regex(text: &str, patterns: &[String]) -> Result<Vec<String>, String> {
    if patterns.is_empty() {
        return Err("html_regex needs at least one pattern".into());
    }
    let mut ids = Vec::new();
    for pat in patterns {
        let re = regex::Regex::new(pat).map_err(|e| format!("bad pattern {pat}: {e}"))?;
        for cap in re.captures_iter(text) {
            let raw = cap
                .get(1)
                .or_else(|| cap.get(0))
                .map(|m| m.as_str())
                .unwrap_or("");
            push_id(&mut ids, Some(raw));
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn push_id(ids: &mut Vec<String>, id: Option<&str>) {
    if let Some(id) = sanitize_id(id.unwrap_or("")) {
        ids.push(id);
    }
}

fn sanitize_id(raw: &str) -> Option<String> {
    let mut s = raw.trim().trim_matches(|c| {
        matches!(c, '"' | '\'' | '`' | ',' | ')' | '(' | '[' | ']' | '{' | '}' | '*' | '\\')
    });
    if let Some(stripped) = s.strip_suffix(".md") {
        s = stripped;
    }
    if s.len() < 2 || s.len() > 90 {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if s.contains("://") || s.contains(' ') || s.contains('<') || s.contains('\n') {
        return None;
    }
    const BAD: &[&str] = &[
        "localhost",
        "system-card",
        "favicon",
        "_next",
        "llms.txt",
        "/image",
        "woff",
        "connectors",
        "readme",
        "interpreter",
        ".tar",
        ".png",
        ".pdf",
        ".yaml",
        ".svg",
        ".webp",
        ".mp4",
        ".css",
        ".json",
        "mistral-color",
        "mistral-rag",
    ];
    if BAD.iter().any(|b| lower.contains(b)) {
        return None;
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '@'))
    {
        return None;
    }
    if let Some((a, b)) = s.split_once('/') {
        if a.is_empty()
            || b.is_empty()
            || !a.chars().any(|c| c.is_ascii_alphabetic())
            || !b.chars().any(|c| c.is_ascii_alphabetic())
        {
            return None;
        }
    }
    Some(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn openai_list_ids() {
        let v = json!({
            "data": [
                {"id": "openai/gpt-4o"},
                {"id": "google/gemini-2.5-pro"},
                {"id": "openai/gpt-4o"}
            ]
        });
        assert_eq!(
            extract("openai_list", &v).unwrap(),
            vec!["google/gemini-2.5-pro", "openai/gpt-4o"]
        );
    }

    #[test]
    fn models_dev_nested() {
        let v = json!({
            "openai": {
                "id": "openai",
                "models": {
                    "gpt-4o": {"id": "openai/gpt-4o"},
                    "o3": {"name": "o3"}
                }
            }
        });
        assert_eq!(extract("models_dev", &v).unwrap(), vec!["o3", "openai/gpt-4o"]);
    }

    #[test]
    fn object_keys_skips_sample_spec() {
        let v = json!({
            "sample_spec": {"max_tokens": 1},
            "gpt-4o": {"litellm_provider": "openai"},
            "claude-sonnet-4": {}
        });
        assert_eq!(
            extract("object_keys", &v).unwrap(),
            vec!["claude-sonnet-4", "gpt-4o"]
        );
    }

    #[test]
    fn unknown_kind_errors() {
        assert!(extract("nope", &json!({})).is_err());
    }

    #[test]
    fn html_regex_docs_paths_and_junk() {
        let text = r#"
- [GPT-6 Astra](/api/docs/models/gpt-6-astra.md)
- [o3](/api/docs/models/o3.md)
[Llama](/docs/model/llama-3.3-70b-versatile)
accounts/fireworks/models/qwen3p7-plus
meta-llama/Llama-3.3-70B-Instruct-Turbo
0.134/image
font/woff2
chatgpt-preview.localhost
"#;
        let ids = extract_text(
            "html_regex",
            text,
            &[
                r"/docs/models/([A-Za-z0-9._-]+)".into(),
                r"/docs/model/([A-Za-z0-9_./-]+)".into(),
                r"(accounts/fireworks/(?:models|routers)/[A-Za-z0-9._-]+)".into(),
                r"\b([A-Za-z][A-Za-z0-9._-]+/[A-Za-z0-9._-]+)\b".into(),
            ],
        )
        .unwrap();
        assert!(ids.contains(&"gpt-6-astra".into()));
        assert!(ids.contains(&"o3".into()));
        assert!(ids.contains(&"llama-3.3-70b-versatile".into()));
        assert!(ids.contains(&"accounts/fireworks/models/qwen3p7-plus".into()));
        assert!(ids.contains(&"meta-llama/Llama-3.3-70B-Instruct-Turbo".into()));
        assert!(!ids.iter().any(|s| s.contains("image") || s.contains("woff") || s.contains("localhost")));
    }

    #[test]
    fn html_regex_needs_patterns() {
        assert!(extract_text("html_regex", "gpt-4o", &[]).is_err());
    }
}
