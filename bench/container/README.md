# Trial isolation

`run.py --isolation docker` runs every agent session in a fresh container,
and evaluates the finished repo in a second one. Build the image once per
harness commit:

```console
$ bench/container/build.sh          # tauten-bench:local; records the claude version and sha256 as labels
```

The image's `dev.tauten.bench.harness` label is the commit it was built
from. Every row records the image id and that label, and `run.py` refuses
a held-out model run unless the label equals the current harness commit
(clean `bench/`, `crates/` and workspace manifests): rebuild after the
freeze commit.

The image is the pinned Rust image (`rust:1.98.0-slim-trixie@sha256:17d1…`,
the same base as tauten's gate) plus clippy, `cargo-tauten` built from this
checkout, the `claude` executable copied from the host install, git,
ripgrep and tinyproxy. The build context holds only tauten's `crates/`, the
workspace manifests, the `claude` binary and the files in this directory:
nothing from `bench/tasks`, and no credentials.

## What the agent container can and cannot see

| Path in the container | Source | Mode |
|---|---|---|
| `/trial/repo` | the trial repo (host copy + arm files, git-initialised) | read-write |
| `/trial/prompt.md` | this step's prompt | read-only |
| `/trial/cargo-home` | the trial's own `CARGO_HOME` (crates the agent downloads) | read-write |
| `/opt/bench/agent.sh` | `bench/agents/claude-code.sh` | read-only |
| `/run/bench-creds/.credentials.json` | only with `--creds file` | read-only |
| `/trial/stub` | built-in agents and `--dry-run` only: one solution dir of one task | read-only |
| `/home/bench`, `/tmp` | tmpfs, discarded with the container | read-write |
| everything else | the image | read-only (`--read-only`) |

It cannot see `bench/` (no oracles, references, sloppy or evasive
solutions, lock, other tasks), the tauten source tree, the host home
directory, other trials, or any other host path. It runs as the invoking
host UID with CPU, memory and PID limits. A probe (`ls`, `/proc/mounts`,
`find` for oracle, lock and credential files, `env`) inside the container
confirmed this on 2026-11-18.

Every session's stdout is the full `--output-format stream-json --verbose`
transcript, saved as `transcript-<step>.jsonl` in the trial's work dir. The
row records `outside_paths`: absolute paths named in any tool call outside
`/trial/repo`, `/trial/prompt.md`, `/tmp`, `/home/bench` and the toolchain.

## Network

Agent containers sit on `tauten-bench-egress`, a Docker network created
with `--internal`: no route out and no DNS. The only exit is the
`tauten-bench-proxy` container (tinyproxy), reached through `HTTPS_PROXY`,
which allows `CONNECT` only to the domains in [`allowlist`](allowlist):

- `api.anthropic.com`: the model API;
- `crates.io`, `index.crates.io`, `static.crates.io`: crates, should an
  agent add a dependency (the hosts themselves have none). This is open in
  every arm. The crates land in the trial's own `CARGO_HOME`
  (`<work>/cargo-home`), which is mounted into the trial's evaluation
  container, so they build there offline.

Claude Code's non-essential traffic (telemetry, error reports, update
checks) is switched off by environment variables in the image. Checked on
2026-11-18 from inside the network: `api.anthropic.com` and
`index.crates.io` → `200 Connection established`; `example.com`,
`github.com` → `403 Filtered`; a direct connection to `1.1.1.1:443` →
`Network is unreachable`; DNS lookup → fails.

Evaluation containers (`git add`, the commit after each non-final step,
build, visible tests, `cargo tauten check`, the oracle in the dev and the
release profile) run with `--network none`, `CARGO_NET_OFFLINE=true` and
`CARGO_HOME=/trial/cargo-home`. The
agent's code is never built or run on the host, and git is never run on the
agent's `.git` on the host (a `.git/config` can execute commands).

## Credentials

What `claude -p` accepts on Linux (checked with `claude auth status --json`
against CLI 2.1.327, offline, with dummy values):

| Source | `authMethod` | Notes |
|---|---|---|
| `ANTHROPIC_API_KEY` | `api_key` | API billing; `total_cost_usd` is the real charge |
| `CLAUDE_CODE_OAUTH_TOKEN` | `oauth_token` | long-lived token from `claude setup-token` (subscription) |
| `~/.claude/.credentials.json` (`claudeAiOauth`) | OAuth | the interactive login; on Linux a plain file |

`~/.claude.json` (settings and history) is not needed.

`--creds env` (the default, recommended) passes `ANTHROPIC_API_KEY` and/or
`CLAUDE_CODE_OAUTH_TOKEN` from the runner's environment **by name**
(`docker run -e NAME`), so the value never appears on a command line, in
the image, in the repo or in the results. Create a dedicated token for the
benchmark and revoke it afterwards.

`--creds file` mounts the host's `~/.claude/.credentials.json` (or
`$BENCH_CREDENTIALS`) read-only. Not recommended for long runs: the access
token in that file lives for hours (the owner's had 20 minutes left when
checked), the read-only mount stops the CLI from saving a refreshed token,
and a refresh inside a container may rotate the refresh token and log the
host session out.

The agent can read whatever credential it is given (for example with
`env`). The proxy allowlist limits where it could send it.

## Dry run (no model)

```console
$ env -u ANTHROPIC_API_KEY -u CLAUDE_CODE_OAUTH_TOKEN \
    python3 -I bench/run.py --agent claude --dry-run --isolation docker \
    --tasks T01-fee --arms A,B,C --out results/dry.jsonl
```

Runs the real agent script and CLI inside the agent container with no
credentials (the CLI stops at `Not logged in` and never reaches the model;
`total_cost_usd` 0), keeps its transcript, then copies the task's reference
solution over the repo inside the container and evaluates it in the
evaluation container. Each row must show the six tools, `api_key_source:
none`, empty `outside_paths`, and the reference passing.
