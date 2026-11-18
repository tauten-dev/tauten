# syntax=docker/dockerfile:1
# Tool image for tauten's gate: pinned Rust toolchain plus the cargo tools the
# gate runs. Base image pinned by digest; cargo tools pinned by version and
# installed with --locked.
FROM rust:1.98.0-slim-trixie@sha256:17d1ba895198f9934c6314ec5346a0d5115372f3243390c3d731e242f35c2f27

ARG CARGO_DENY_VERSION=0.21.0
ARG CARGO_MUTANTS_VERSION=27.1.0

RUN --mount=type=cache,id=tauten-apt,target=/var/cache/apt,sharing=locked \
    apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt

RUN --mount=type=cache,id=tauten-tool-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=tauten-tool-target,target=/tmp/tool-target \
    CARGO_TARGET_DIR=/tmp/tool-target cargo install --locked \
      cargo-deny@${CARGO_DENY_VERSION} cargo-mutants@${CARGO_MUTANTS_VERSION}

ENV CARGO_TARGET_DIR=/cache/target \
    CARGO_TERM_COLOR=always \
    GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0=/work/tauten
WORKDIR /work/tauten
