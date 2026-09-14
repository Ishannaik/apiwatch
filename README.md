# apiwatch

Tiny Rust oneshot that polls **public** LLM model catalogs, diffs IDs, and writes `state.json` for a static dashboard.

This is the cheatyyyy-style model-list watch without API keys.

Lab `/v1/models` endpoints (OpenAI, Anthropic, xAI, Groq, Together, Fireworks, Mistral, Portkey) return 401 without a key. That is auth, not a gap to bypass. Those IDs are still watched from **public docs markdown** (`html_regex`) plus aggregators that already list the same labs. Portkey's catalog is workspace-scoped, so there is no public model list to poll; the same IDs show up on OpenRouter / models.dev / LiteLLM.

## Public sources

| Source | URL | Auth |
|---|---|---|
| OpenRouter | `https://openrouter.ai/api/v1/models` | none |
| Vercel AI Gateway | `https://ai-gateway.vercel.sh/v1/models` | none |
| Hugging Face router | `https://router.huggingface.co/v1/models` | none |
| Naga | `https://api.naga.ac/v1/models` | none |
| AIMLAPI | `https://api.aimlapi.com/models` | none |
| DeepInfra | `https://api.deepinfra.com/v1/openai/models` | none |
| models.dev | `https://models.dev/api.json` | none |
| LiteLLM prices | jsDelivr `model_prices_and_context_window.json` | none |
| OpenAI docs | `https://platform.openai.com/docs/models.md` | none |
| Anthropic docs | `https://docs.anthropic.com/en/docs/about-claude/models.md` | none |
| Anthropic models.json | jsDelivr `combinatrix-ai/claude-models-list` | none (public snapshot) |
| xAI docs | `https://docs.x.ai/developers/models.md` | none |
| Groq docs | `https://console.groq.com/docs/models.md` | none |
| Together docs | `https://docs.together.ai/docs/serverless-models.md` | none |
| Fireworks docs | `https://docs.fireworks.ai/getting-started/models.md` | none |
| Mistral docs | `https://docs.mistral.ai/getting-started/models/models_overview` | none |

ETags are stored so unchanged catalogs return 304 and skip parse.

## Run

```
apiwatch --config config.toml --cache cache.json --out web
```

Optional Discord webhook: set `WEBHOOK` (or the env name in `webhook_env`) to a Discord incoming webhook URL. The first scan is a baseline and does not notify.

## Oracle

systemd timer (`Type=oneshot`) so idle RSS is zero. Dashboard: `https://watch.ishannaik.com` on the Tailscale IP, same pattern as `domains.ishannaik.com`.
