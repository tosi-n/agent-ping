FROM rust:bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 appuser \
    && mkdir -p /home/appuser/.agent-ping \
    && chown -R appuser:appuser /home/appuser

COPY --from=builder /app/target/release/main /usr/local/bin/agent-ping

USER appuser
WORKDIR /home/appuser

ENV AGENT_PING_CONFIG=/home/appuser/.agent-ping/agent-ping.json
EXPOSE 8091

CMD ["agent-ping"]
