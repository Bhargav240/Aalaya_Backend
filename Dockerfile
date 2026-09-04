FROM rust:1.95.0-bookworm AS builder

WORKDIR /build
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release -p pg-server \
    && cp /build/target/release/pg-server /tmp/pg-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 hamsa

WORKDIR /app
COPY --from=builder /tmp/pg-server /usr/local/bin/pg-server
RUN mkdir -p /app/data/objectstore && chown -R hamsa:hamsa /app

USER hamsa
EXPOSE 8080
HEALTHCHECK --interval=5s --timeout=3s --retries=20 \
    CMD curl --fail --silent http://127.0.0.1:8080/health >/dev/null || exit 1

ENTRYPOINT ["pg-server"]
