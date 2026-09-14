# apiwatch

Tiny Rust oneshot that polls **public** LLM model catalogs, diffs IDs, and writes `state.json` for a static dashboard.

This is the cheatyyyy-style model-list watch without API keys. Provider endpoints that 401 without a key (OpenAI, Anthropic, Gemini, xAI, Groq, Mistral, Together, Fireworks, Cohere) are not scanned.

## Public sources

| Source | URL | Auth |
|---|---|---|
| OpenRouter | `https://openrouter.ai/api/v1/models` | none |
| Vercel AI Gateway | `https://ai-gateway.vercel.sh/v1/models` | none |
| Hugging Face router | `https://router.huggingface.co/v1/models` | none |
| Naga | `https://api.naga.ac/v1/models` | none |
| DeepInfra | `https://api.deepinfra.com/v1/openai/models` | none |
| models.dev | `https://models.dev/api.json` | none |
| LiteLLM prices | jsDelivr `model_prices_and_context_window.json` | none |

ETags are stored so unchanged catalogs return 304 and skip parse.

## Run

```
apiwatch --config config.toml --cache cache.json --out web
```

Optional Discord webhook: set `WEBHOOK` (or the env name in `webhook_env`) to a Discord incoming webhook URL. The first scan is a baseline and does not notify.

## Oracle

systemd timer (`Type=oneshot`) so idle RSS is zero. Dashboard: `https://watch.ishannaik.com` on the Tailscale IP, same pattern as `domains.ishannaik.com`.
