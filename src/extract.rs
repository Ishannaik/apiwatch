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

fn push_id(ids: &mut Vec<String>, id: Option<&str>) {
    if let Some(id) = id {
        let id = id.trim();
        if !id.is_empty() {
            ids.push(id.to_string());
        }
    }
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
}
