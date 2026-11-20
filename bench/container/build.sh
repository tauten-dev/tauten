#!/usr/bin/env bash
# Build the benchmark image (tauten-bench:local). The context holds only:
# tauten's workspace manifests and crates/ (to build cargo-tauten), the claude
# binary, and the files in this directory.
#   BENCH_CLAUDE_BIN  path to the claude executable (default: the one on PATH)
set -euo pipefail
cd "$(dirname "$0")/../.."
ctx="$(mktemp -d)"
trap 'rm -rf "$ctx"' EXIT
mkdir "$ctx/src"
git ls-files -co --exclude-standard -- Cargo.toml Cargo.lock rust-toolchain.toml crates \
  | tar -cf - -T - | tar -xf - -C "$ctx/src"
claude_bin="$(readlink -f "${BENCH_CLAUDE_BIN:-$(command -v claude)}")"
cp "$claude_bin" "$ctx/claude"
claude_version="$("$ctx/claude" --version | awk '{print $1}')"
claude_sha="$(sha256sum "$ctx/claude" | awk '{print $1}')"
cp bench/container/Dockerfile bench/container/agent-entry.sh bench/container/tinyproxy.conf \
   bench/container/allowlist "$ctx/"
docker build -t tauten-bench:local \
  --label "dev.tauten.bench.claude-version=$claude_version" \
  --label "dev.tauten.bench.claude-sha256=$claude_sha" \
  --label "dev.tauten.bench.harness=$(git rev-parse HEAD)" \
  "$ctx"
echo "built tauten-bench:local (claude $claude_version, sha256 $claude_sha)"
