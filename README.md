# Agent-Ping

`agent-ping` is a two-way communication daemon for agent platform.

It handles:
- channel ingress/egress (Slack, Telegram, WhatsApp sidecar),
- session key routing,
- business/user/agent binding resolution,
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
- `AGENT_PING_ADAPTER_RUNTIME_URL`
- `AGENT_PING_SESSION_AGENT_ID`
- `AGENT_PING_SESSION_DM_SCOPE`
- `AGENT_PING_SESSION_MAIN_KEY`
- `AGENT_PING_IDENTITY_LINKS_JSON`
- `AGENT_PING_BINDINGS_JSON`
- `AGENT_PING_CHANNEL_SLACK_TRANSPORT`
- `AGENT_PING_CHANNEL_TELEGRAM_TRANSPORT`
- `AGENT_PING_CHANNEL_WHATSAPP_TRANSPORT`
- `AGENT_PING_CHANNEL_TEAMS_TRANSPORT`

## Business Routing

Provider credentials only make channel adapters live. They do not tell `agent-ping` which
business owns an inbound conversation.

Use `AGENT_PING_BINDINGS_JSON` to map inbound routes onto Stimulir business profiles, users,
and agent lanes. Matching is by:

- `channel` only
- `channel + account_id`
- `channel + peer_id`
- `channel + account_id + peer_id`

`agent-ping` picks the most specific match.

Example:

```bash
export AGENT_PING_BINDINGS_JSON='[
  {
    "channel": "slack",
    "account_id": "T03ACME",
    "business_profile_id": "bp_acme",
    "agent_id": "ops_router"
  },
  {
    "channel": "slack",
    "account_id": "T03ACME",
    "peer_id": "C091FINANCE",
    "business_profile_id": "bp_acme",
    "agent_id": "finance_main"
  },
  {
    "channel": "telegram",
    "peer_id": "123456789",
    "business_profile_id": "bp_acme"
  },
  {
    "channel": "whatsapp",
    "peer_id": "447700900123",
    "business_profile_id": "bp_acme"
  }
]'
```

Notes:

- `business_profile_id` is the Stimulir business profile id that should own the session.
- `user_id` is optional and can attach the session directly to a known user.
- `agent_id` is optional and selects the target agent lane for that route.
- `account_id` is usually the provider workspace/tenant/workspace-equivalent id.
- `peer_id` is usually the Slack channel/user id, Telegram chat id, WhatsApp phone/contact id,
  or Teams conversation id.

## Session Shape

Direct-message session behavior is controlled by:

- `AGENT_PING_SESSION_AGENT_ID`
- `AGENT_PING_SESSION_DM_SCOPE`
- `AGENT_PING_SESSION_MAIN_KEY`
- `AGENT_PING_IDENTITY_LINKS_JSON`

`AGENT_PING_SESSION_DM_SCOPE` supports:

- `main`
- `per-peer`
- `per-channel-peer`
- `per-account-channel-peer`

Example:

```bash
export AGENT_PING_SESSION_AGENT_ID=ops_router
export AGENT_PING_SESSION_DM_SCOPE=per-channel-peer
export AGENT_PING_SESSION_MAIN_KEY=main
export AGENT_PING_IDENTITY_LINKS_JSON='{
  "acme-owner": ["slack:U02ACME", "telegram:123456789", "whatsapp:447700900123"]
}'
```

This lets the same person keep a stable DM session identity across multiple channels when you
need it.

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

## Testing

### Running Tests

```bash
# All passing tests (unit tests + integration_lib + integration_ws)
cargo test --tests --lib

# Full test suite (when SQLite integration tests are fixed)
cargo test --tests
```

### Current Coverage

- **Total**: 18.86% (257/1363 lines)
- **Excluded**: `src/bin/main.rs` (CLI boilerplate)

Run `cargo tarpaulin --tests --lib --out Html` to generate coverage report.

## TODO

### Test Coverage (Target: 88%)

The following tests need to be implemented/fixed to reach 88% coverage:

#### SQLite Integration Tests
- [ ] Fix SQLite connection issues in CI/temp directory environments
  - Error: `SqliteError { code: 14, message: "unable to open database file" }`
  - Root cause: `sqlx::AnyPool::connect()` timing with SQLite URLs

#### Tests to Enable
- [ ] `tests/integration/db.rs` - DB operations (15 tests)
  - Covers: `src/db.rs` (199 lines)
- [ ] `tests/integration/api.rs` - HTTP API endpoints (18 tests)
  - Covers: `src/lib.rs` async functions

#### Expected Impact

| Module | Current | After Fix |
|--------|---------|-----------|
| db.rs | 8.5% | ~85% |
| lib.rs async | 5.8% | ~75% |
| **Total** | **18.86%** | **~55%+** |

Enable tests by uncommenting in `Cargo.toml`:
```toml
# [[test]]
# name = "integration_db"
# path = "tests/integration/db.rs"
# [[test]]
# name = "integration_api"
# path = "tests/integration/api.rs"
```
