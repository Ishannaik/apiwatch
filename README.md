# apiwatch

Tiny Rust oneshot that polls **public** LLM model catalogs, diffs IDs, and writes `state.json` for a static dashboard.

This is the cheatyyyy-style model-list watch without API keys.

Lab list endpoints return 401/403 without a key. That is auth, not a gap to bypass. Those IDs are still watched from **public docs markdown** (`html_regex`), official SDK type files, community JSON snapshots, and aggregators. Portkey's gateway `/v1/models` is workspace-scoped, but `configs.portkey.ai/pricing/{provider}.json` is a keyless public pricing dump.

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
| Catwalk | `https://catwalk.charm.land/v2/providers` | none |
| OpenAI docs | `https://platform.openai.com/docs/models.md` | none |
| OpenAI SDK ChatModel | jsDelivr `openai-python` `chat_model.py` | none |
| Anthropic docs | `https://docs.anthropic.com/en/docs/about-claude/models.md` | none |
| Anthropic models.json | jsDelivr `combinatrix-ai/claude-models-list` | none (public snapshot) |
| xAI docs | `https://docs.x.ai/developers/models` (`.md` 404s) | none |
| xAI SDK chat models | jsDelivr `vercel/ai` `xai-chat-language-model-options.ts` | none |
| Groq docs | `https://console.groq.com/docs/models.md` | none |
| Together docs | `https://docs.together.ai/docs/serverless-models.md` | none |
| Fireworks docs | `https://docs.fireworks.ai/getting-started/models.md` | none |
| Mistral docs | `https://docs.mistral.ai/getting-started/models/models_overview` | none |
| Google Gemini docs | `https://ai.google.dev/gemini-api/docs/models.md.txt` | none |
| Google Gemini llms.txt | `https://ai.google.dev/gemini-api/docs/models/llms.txt` | none |
| Google Vertex versions | `.../model-versions.md.txt` | none |
| Portkey xAI/Groq/Mistral/Vertex prices | `https://configs.portkey.ai/pricing/{x-ai,groq,mistral-ai,vertex-ai}.json` | none |

ETags are stored so unchanged catalogs return 304 and skip parse.

## What does not work

No keys, no cookie jars, no 401 bypass.

| Target | Result |
|---|---|
| OpenAI / Anthropic / xAI / Groq / Together / Fireworks / Mistral / Portkey `/v1/models` | 401/403 without Bearer |
| `generativelanguage.googleapis.com/v1beta/models` | 403 without API key |
| `aistudio.google.com/models` | JS SPA, no model IDs in HTML |
| Vertex HTML (`docs.cloud.google.com/.../learn/models`) | 200, but docs slugs (`gemini-capabilities`) not API IDs |
| `ai.google.dev/gemini-api/docs/models.md` (no `.txt`) | 200 HTML, not markdown. Use `.md.txt` |
| `ai.google.dev/gemini-api/docs/llms.txt` | 200, full docs sitemap. Model index is `docs/models/llms.txt` |
| `docs.x.ai/developers/models.md` and `docs.x.ai/docs/models.md` | 404. Use the HTML page |
| `docs.mistral.ai/.../models_overview.md` | 404 |
| Mistral `llms-full.txt` | 200, API dump, not a model catalog |
| Portkey GitHub `gateway/src/data/models.json` | stale (Claude 3.5, Groq Llama 3.1). Prefer `configs.portkey.ai` |
| Portkey `pricing/google.json` | 200, but every ID is split into `-gt-128k` / `-lte-128k`. Use `vertex-ai.json` |
| Portkey `pricing/palm.json` | 200, PaLM bison only |
| Catwalk `crush.json` | editor config, not a model catalog. Use `/v2/providers` |
| Fireworks `getting-started/models.md` | still Mixtral/Llama 3; will miss Kimi K3 / GLM 5.3. Aggregators catch those |
| Official Anthropic TypeScript `Model` union | now open `string`, not a closed ID list |

## Run

```
apiwatch --config config.toml --cache cache.json --out web
```

Optional Discord webhook: set `WEBHOOK` (or the env name in `webhook_env`) to a Discord incoming webhook URL. The first scan is a baseline and does not notify.

## Oracle

systemd timer (`Type=oneshot`) so idle RSS is zero. Dashboard: `https://watch.ishannaik.com` on the Tailscale IP, same pattern as `domains.ishannaik.com`.
