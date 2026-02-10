# Agent-Ping Release Process

## Registry

- Docker images are published to GHCR.
- Repository image path: `ghcr.io/<org-or-user>/agent-ping`.
- Workflow file: `.github/workflows/release.yml`.

## Automatic Publish Triggers

- Push tag `v*` (example `v0.2.0`):
  - Publishes `latest` and semver tag (for example `0.2.0`) to GHCR.
  - Creates a GitHub Release with generated notes.
- Manual trigger:
  - Run `Release` from GitHub Actions UI (publishes `latest` and `manual-<sha7>`).

## Local Verification Before Release

```bash
cargo test -- --test-threads=1
docker build -t agent-ping:local .
```

## Create a Versioned Release

1. Update version in `Cargo.toml`.
2. Commit changes.
3. Create and push git tag:

```bash
git tag v0.2.0
git push origin main --tags
```

4. Confirm GH Action completed and image exists in GHCR.

## Pull and Run Tagged Image

```bash
docker pull ghcr.io/<org-or-user>/agent-ping:v0.2.0
docker run --rm -p 8091:8091 \
  -e AGENT_PING_TOKEN=changeme \
  -e AGENT_PING_BACKEND_WEBHOOK_URL=http://host.docker.internal:8000/api/v1/agent-ping/inbound \
  -e AGENT_PING_BACKEND_MEDIA_UPLOAD_URL=http://host.docker.internal:8000/api/v1/agent-ping/media/upload \
  -e AGENT_PING_BACKEND_TOKEN=changeme \
  ghcr.io/<org-or-user>/agent-ping:v0.2.0
```
