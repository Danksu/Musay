FROM rust:1.97-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg libopus0 \
    && useradd --system --create-home --uid 10001 musay \
    && mkdir -p /data \
    && chown -R musay:musay /data \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/musay /usr/local/bin/musay
COPY --chown=musay:musay .env.example /data/.env.example
USER musay
WORKDIR /data
ENV DATABASE_PATH=musay.json
ENTRYPOINT ["/usr/local/bin/musay"]
