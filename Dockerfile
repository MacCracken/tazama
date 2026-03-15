# Stage 1: Build
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libasound2-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release --package tazama-mcp

# Stage 2: Runtime — AGNOS base OS
FROM ghcr.io/maccracken/agnosticos:latest

LABEL org.opencontainers.image.title="Tazama MCP"
LABEL org.opencontainers.image.description="Tazama MCP server on AGNOS"
LABEL org.opencontainers.image.source="https://github.com/anomalyco/tazama"
LABEL org.opencontainers.image.licenses="AGPL-3.0"
LABEL org.opencontainers.image.base.name="ghcr.io/maccracken/agnosticos:latest"

USER root
RUN apt-get update && apt-get install -y --no-install-recommends \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    libgstreamer1.0-0 \
    libasound2 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1007 tazama && useradd -u 1007 -g tazama -m -s /bin/bash tazama
RUN mkdir -p /data && chown tazama:tazama /data

COPY --from=builder /build/target/release/tazama-mcp /usr/local/bin/tazama-mcp

ENV RUST_LOG=info

USER tazama

ENTRYPOINT ["tazama-mcp"]
