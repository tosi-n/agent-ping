# Agent-Ping

`agent-ping` is a two-way communication daemon for agent platform.

It handles:
- channel ingress/egress (Slack, Telegram, WhatsApp sidecar),
- session key routing,
- queueing and reliable inbound webhook delivery,
- media handoff to backend storage,
- control-plane websocket monitoring.

It does **not** run the LLM or agent logic. Agent platform backend owns that.

## Runtime

- Default DB: SQLite at `~/.agent-ping/state.sqlite`
- Optional DB: Postgres via `AGENT_PING_DATABASE_URL`
- Default port: `8091`

## Config

Default config path:
- `~/.agent-ping/agent-ping.json`

Override:
- `AGENT_PING_CONFIG=/path/to/agent-ping.json`

Example file:
- `agent-ping.example.json`

## Environment

- `AGENT_PING_TOKEN`
- `AGENT_PING_DATABASE_URL`
- `AGENT_PING_SQLITE_PATH`
- `AGENT_PING_BACKEND_WEBHOOK_URL`
- `AGENT_PING_BACKEND_MEDIA_UPLOAD_URL`
- `AGENT_PING_BACKEND_TOKEN`

## HTTP API

Public:
- `GET /v1/health`
- `GET /v1/status`
- `POST /v1/channels/slack/events`
- `POST /v1/channels/whatsapp/inbound`

Authenticated (`X-Agent-Ping-Token`):
- `POST /v1/messages/send`
- `POST /v1/messages/send-bulk`
- `GET /v1/sessions`
- `GET /v1/sessions/{session_key}`
- `GET /v1/sessions/{session_key}/messages`
- `POST /v1/inbound/ack`
- `GET /v1/ws`

## WS Control Plane

Connect:
```json
{"type":"connect","token":"..."}
```

Subscribe:
```json
{"type":"subscribe","events":["chat","delivery","monitor","health","presence"]}
```

Ping:
```json
{"type":"ping"}
```

## Run

```bash
export AGENT_PING_TOKEN=changeme
export AGENT_PING_BACKEND_WEBHOOK_URL=http://localhost:8000/api/v1/agent-ping/inbound
export AGENT_PING_BACKEND_MEDIA_UPLOAD_URL=http://localhost:8000/api/v1/agent-ping/media/upload
export AGENT_PING_BACKEND_TOKEN=internal-token
cargo run
```

## Docker

Build:
```bash
docker build -t agent-ping:local .
```

Run:
```bash
docker run --rm -p 8091:8091 \
  -e AGENT_PING_TOKEN=changeme \
  -e AGENT_PING_BACKEND_WEBHOOK_URL=http://host.docker.internal:8000/api/v1/agent-ping/inbound \
  -e AGENT_PING_BACKEND_MEDIA_UPLOAD_URL=http://host.docker.internal:8000/api/v1/agent-ping/media/upload \
  -e AGENT_PING_BACKEND_TOKEN=changeme \
  -v agent-ping-data:/home/appuser/.agent-ping \
  agent-ping:local
```

## Release

- Build and publish Docker images via release workflow: `.github/workflows/release.yml`
- Release and versioning process: `docs/RELEASE.md`
