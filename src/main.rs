mod extract;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const UA: &str = "apiwatch/0.1 (+https://watch.ishannaik.com)";

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
    #[serde(default)]
    webhook_env: Option<String>,
    sources: Vec<Source>,
}

fn default_timeout() -> u64 {
    25
}

#[derive(Debug, Deserialize, Clone)]
struct Source {
    id: String,
    name: String,
    url: String,
    kind: String,
    #[serde(default)]
    patterns: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct Cache {
    sources: BTreeMap<String, CachedSource>,
    events: Vec<Event>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CachedSource {
    etag: Option<String>,
    ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Event {
    ts: u64,
    source: String,
    op: String,
    id: String,
}

struct Args {
    config: PathBuf,
    cache: PathBuf,
    out: PathBuf,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("apiwatch: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let cfg: Config = toml::from_str(
        &fs::read_to_string(&args.config)
            .map_err(|e| format!("read config {}: {e}", args.config.display()))?,
    )
    .map_err(|e| format!("parse config: {e}"))?;
    if cfg.sources.is_empty() {
        return Err("config has no sources".into());
    }

    let mut cache = load_cache(&args.cache);
    let baseline = cache.sources.is_empty();
    let started = Instant::now();
    let ts = now_unix();
    let mut rows = Vec::new();
    let mut new_events: Vec<Event> = Vec::new();

    for src in &cfg.sources {
        let prior = cache.sources.get(&src.id).cloned();
        let (ok, status, not_modified, ms, ids, etag, error) =
            fetch_source(&cfg, src, prior.as_ref());
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let count = if ok {
            ids.len()
        } else {
            prior.as_ref().map(|p| p.ids.len()).unwrap_or(0)
        };
        if ok {
            if !baseline {
                if let Some(old) = &prior {
                    let old_set: BTreeSet<_> = old.ids.iter().cloned().collect();
                    let new_set: BTreeSet<_> = ids.iter().cloned().collect();
                    added = new_set.difference(&old_set).cloned().collect();
                    removed = old_set.difference(&new_set).cloned().collect();
                    for id in &added {
                        new_events.push(Event {
                            ts,
                            source: src.id.clone(),
                            op: "add".into(),
                            id: id.clone(),
                        });
                    }
                    for id in &removed {
                        new_events.push(Event {
                            ts,
                            source: src.id.clone(),
                            op: "remove".into(),
                            id: id.clone(),
                        });
                    }
                }
            }
            cache.sources.insert(
                src.id.clone(),
                CachedSource {
                    etag: etag.or_else(|| prior.and_then(|p| p.etag)),
                    ids: ids.clone(),
                },
            );
        }
        rows.push(json!({
            "id": src.id,
            "name": src.name,
            "url": src.url,
            "ok": ok,
            "status": status,
            "not_modified": not_modified,
            "ms": ms,
            "count": count,
            "added": added,
            "removed": removed,
            "error": error,
        }));
    }

    cache.events.extend(new_events.iter().cloned());
    if cache.events.len() > 400 {
        let skip = cache.events.len() - 400;
        cache.events = cache.events.split_off(skip);
    }

    if let Some(parent) = args.cache.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir cache: {e}"))?;
    }
    atomic_write(
        &args.cache,
        serde_json::to_vec_pretty(&cache).map_err(|e| format!("serialize cache: {e}"))?,
    )?;

    let mut ids_out = BTreeMap::new();
    let mut models_n = 0usize;
    for src in &cfg.sources {
        if let Some(c) = cache.sources.get(&src.id) {
            models_n += c.ids.len();
            ids_out.insert(src.id.clone(), c.ids.clone());
        }
    }
    let added_n = new_events.iter().filter(|e| e.op == "add").count();
    let removed_n = new_events.iter().filter(|e| e.op == "remove").count();
    let sources_ok = rows.iter().filter(|r| r["ok"].as_bool() == Some(true)).count();

    let state = json!({
        "generated_at": ts,
        "scan_ms": started.elapsed().as_millis(),
        "baseline": baseline,
        "totals": {
            "models": models_n,
            "added": added_n,
            "removed": removed_n,
            "sources_ok": sources_ok,
            "sources": rows.len(),
        },
        "sources": rows,
        "events": cache.events.iter().rev().take(80).cloned().collect::<Vec<_>>(),
        "ids": ids_out,
    });

    fs::create_dir_all(&args.out).map_err(|e| format!("mkdir out: {e}"))?;
    atomic_write(
        &args.out.join("state.json"),
        serde_json::to_vec(&state).map_err(|e| format!("serialize state: {e}"))?,
    )?;

    if !baseline && !new_events.is_empty() {
        if let Some(url) = webhook_url(&cfg) {
            if let Err(e) = notify(&url, ts, &new_events) {
                eprintln!("webhook: {e}");
            }
        }
    }

    println!(
        "apiwatch {models_n} models +{added_n} -{removed_n} in {}ms baseline={baseline}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn fetch_source(
    cfg: &Config,
    src: &Source,
    prior: Option<&CachedSource>,
) -> (bool, u16, bool, u128, Vec<String>, Option<String>, Option<String>) {
    let t0 = Instant::now();
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .user_agent(UA)
        .build();
    let mut req = agent.get(&src.url);
    if src.kind == "html_regex" {
        req = req.set(
            "Accept",
            "text/markdown, text/plain, text/html;q=0.8, */*;q=0.5",
        );
    }
    if let Some(etag) = prior.and_then(|p| p.etag.as_deref()) {
        req = req.set("If-None-Match", etag);
    }
    match req.call() {
        Ok(resp) => {
            let status = resp.status();
            let etag = resp.header("etag").map(|s| s.to_string());
            let ms = t0.elapsed().as_millis();
            if status == 304 {
                let ids = prior.map(|p| p.ids.clone()).unwrap_or_default();
                return (true, status, true, ms, ids, etag, None);
            }
            if src.kind == "html_regex" {
                return match resp.into_string() {
                    Ok(text) => match extract::extract_text(&src.kind, &text, &src.patterns) {
                        Ok(ids) => (true, status, false, ms, ids, etag, None),
                        Err(e) => (false, status, false, ms, vec![], None, Some(e)),
                    },
                    Err(e) => (false, status, false, ms, vec![], None, Some(e.to_string())),
                };
            }
            match resp.into_json::<Value>() {
                Ok(value) => match extract::extract(&src.kind, &value) {
                    Ok(ids) => (true, status, false, ms, ids, etag, None),
                    Err(e) => (false, status, false, ms, vec![], None, Some(e)),
                },
                Err(e) => (false, status, false, ms, vec![], None, Some(e.to_string())),
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            let msg: String = body.chars().take(180).collect();
            (
                false,
                code,
                false,
                t0.elapsed().as_millis(),
                vec![],
                None,
                Some(format!("http {code}: {msg}")),
            )
        }
        Err(e) => (
            false,
            0,
            false,
            t0.elapsed().as_millis(),
            vec![],
            None,
            Some(e.to_string()),
        ),
    }
}

fn webhook_url(cfg: &Config) -> Option<String> {
    let key = cfg
        .webhook_env
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("WEBHOOK");
    std::env::var(key).ok().filter(|s| s.starts_with("https://"))
}

fn notify(url: &str, ts: u64, events: &[Event]) -> Result<(), String> {
    let mut lines = vec![format!("apiwatch {ts}")];
    let mut by: BTreeMap<&str, (Vec<&str>, Vec<&str>)> = BTreeMap::new();
    for ev in events {
        let e = by.entry(ev.source.as_str()).or_default();
        if ev.op == "add" {
            e.0.push(ev.id.as_str());
        } else {
            e.1.push(ev.id.as_str());
        }
    }
    for (src, (add, rem)) in by {
        lines.push(format!("**{src}** +{} −{}", add.len(), rem.len()));
        for id in add.iter().take(12) {
            lines.push(format!("+ `{id}`"));
        }
        if add.len() > 12 {
            lines.push(format!("+ …{} more", add.len() - 12));
        }
        for id in rem.iter().take(8) {
            lines.push(format!("− `{id}`"));
        }
    }
    let mut content = lines.join("\n");
    if content.len() > 1800 {
        content.truncate(1800);
        content.push('…');
    }
    ureq::post(url)
        .set("User-Agent", UA)
        .send_json(json!({ "content": content }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn load_cache(path: &Path) -> Cache {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn atomic_write(path: &Path, bytes: Vec<u8>) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("create {}: {e}", tmp.display()))?;
        f.write_all(&bytes)
            .map_err(|e| format!("write {}: {e}", tmp.display()))?;
        let _ = f.sync_all();
    }
    fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))?;
    Ok(())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn parse_args() -> Result<Args, String> {
    let mut config = PathBuf::from("config.toml");
    let mut cache = PathBuf::from("cache.json");
    let mut out = PathBuf::from("web");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => config = PathBuf::from(args.next().ok_or("--config needs a path")?),
            "--cache" => cache = PathBuf::from(args.next().ok_or("--cache needs a path")?),
            "--out" => out = PathBuf::from(args.next().ok_or("--out needs a path")?),
            "-h" | "--help" => {
                println!("apiwatch --config FILE --cache FILE --out DIR");
                std::process::exit(0);
            }
            other => return Err(format!("unknown arg {other}")),
        }
    }
    Ok(Args { config, cache, out })
}
