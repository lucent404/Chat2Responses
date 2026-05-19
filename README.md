# Chat2Responses

A Rust service proxy that translates OpenAI **Responses API** traffic into upstream **Chat Completions API** requests. It now runs as a managed service: configure upstream providers, model routes, and service-issued API keys from the admin UI, then let callers use the new keys against `/v1/responses`.

## Quick Start

Build the admin UI first:

```bash
cd admin-ui
npm install
npm run build
cd ..
```

Start the service:

```bash
CHAT2RESPONSES_SECRET="change-this-long-random-secret" \
CHAT2RESPONSES_DATABASE_URL="sqlite://data/chat2responses.db" \
CHAT2RESPONSES_PORT=4444 \
chat2responses
```

Open `http://127.0.0.1:4444/admin/`, create the first administrator, then:

1. Add an upstream provider with its Chat Completions base URL and provider API key.
2. Publish one or more global model routes, mapping a public model name to an upstream model name.
3. Create a service API key for callers.
4. Point clients at `http://127.0.0.1:4444/v1` with `Authorization: Bearer <service-key>`.

## Runtime Interfaces

| Endpoint | Purpose |
|---|---|
| `GET /v1/models` | Returns enabled global model routes for an authenticated service key. |
| `POST /v1/responses` | Accepts Responses API requests, routes by `model`, and forwards to the configured upstream Chat Completions provider. |
| `/admin/` | React/Vite admin console served by Axum. |
| `/admin/api/*` | Admin JSON API protected by cookie session login. |

## Configuration

| Variable | Default | Description |
|---|---|---|
| `CHAT2RESPONSES_PORT` | `4444` | Local listen port. |
| `CHAT2RESPONSES_DATABASE_URL` | `sqlite://data/chat2responses.db` | SQLite database URL. |
| `CHAT2RESPONSES_SECRET` | development fallback | Secret used to encrypt provider keys and hash service keys. Set this explicitly in any real deployment. |
| `RUST_LOG` | `chat2responses=info,tower_http=warn` | Log verbosity. |

Provider keys are encrypted in SQLite. Service-issued caller keys are shown only once and stored as keyed hashes.

## Model Routing

Models are global, not per caller key. Each public model name maps to exactly one enabled upstream and upstream model name. If two providers expose the same model id, create distinct public names such as `deepseek-chat` and `openrouter-deepseek-chat`.

## Python API

```python
from chat2responses import start

proc = start(
    port=4444,
    database_url="sqlite://data/chat2responses.db",
    secret="change-this-long-random-secret",
)
proc.terminate()
```

## Development

```bash
cargo test
cd admin-ui && npm run build
```

Live provider compatibility still depends on real provider keys and should remain gated outside the default test path.

## Notes

This project remains focused on Responses API compatibility for Codex-style clients. The first service version does not implement `/v1/chat/completions`, embeddings, images, audio, per-key model allowlists, or quota enforcement.
