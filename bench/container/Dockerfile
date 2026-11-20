# syntax=docker/dockerfile:1
# One image for every container a benchmark trial uses:
#   agent  - runs the agent on the trial repo (network only through the proxy)
#   eval   - builds and tests the finished repo and runs the oracle (no network)
#   proxy  - tinyproxy with a domain allowlist; the only route out
# Built by bench/container/build.sh from a context holding only tauten's
# crates (for cargo-tauten), the pinned claude binary and these files. No
# bench/tasks content and no credentials are ever part of the context.
ARG BASE=rust:1.98.0-slim-trixie@sha256:17d1ba895198f9934c6314ec5346a0d5115372f3243390c3d731e242f35c2f27

FROM ${BASE} AS tauten
COPY src/ /src/
RUN --mount=type=cache,id=tauten-bench-registry,target=/usr/local/cargo/registry \
    cd /src && CARGO_TARGET_DIR=/tmp/t cargo install --locked --path crates/cargo-tauten --root /opt/tauten

FROM ${BASE}
RUN --mount=type=cache,id=tauten-bench-apt,target=/var/cache/apt,sharing=locked \
    apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates tinyproxy ripgrep \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add clippy rustfmt
COPY --from=tauten /opt/tauten/bin/cargo-tauten /usr/local/cargo/bin/cargo-tauten
# The Claude Code CLI, copied from the host install at the version the owner
# approved (build.sh records its version and sha256 in the image labels).
COPY claude /usr/local/bin/claude
COPY agent-entry.sh /usr/local/bin/bench-agent-entry
COPY tinyproxy.conf /etc/tinyproxy/bench.conf
COPY allowlist /etc/tinyproxy/allowlist
RUN chmod 0755 /usr/local/bin/claude /usr/local/bin/bench-agent-entry \
    && mkdir -p /trial && chmod 0777 /trial
# Containers run as the invoking host user with a tmpfs HOME and a read-only
# root filesystem; cargo's registry cache lives under HOME.
ENV CARGO_HOME=/home/bench/.cargo \
    PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin \
    HOME=/home/bench \
    CARGO_TERM_COLOR=never \
    DISABLE_TELEMETRY=1 \
    DISABLE_ERROR_REPORTING=1 \
    DISABLE_AUTOUPDATER=1 \
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
WORKDIR /trial/repo
