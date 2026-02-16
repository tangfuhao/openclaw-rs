# Stage 1: Build
FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin openclaw \
    && strip target/release/openclaw

# Stage 2: Runtime (minimal image)
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/openclaw /usr/local/bin/openclaw

# Default data directory
RUN mkdir -p /data/.openclaw

ENV HOME=/data

EXPOSE 18789

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD ["/usr/local/bin/openclaw", "status", "--url", "http://localhost:18789"]

ENTRYPOINT ["/usr/local/bin/openclaw"]
CMD ["gateway", "--allow-unconfigured"]
