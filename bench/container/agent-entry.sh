#!/usr/bin/env bash
# Entry point of the agent container. Sees only /trial/repo (rw),
# /trial/prompt.md (ro), the agent script at /opt/bench/agent.sh (ro) and, in
# stub modes only, /trial/stub (ro): one solution directory of one task.
#   BENCH_AGENT=claude  run the agent script (needs credentials)
#   BENCH_AGENT=copy    copy /trial/stub over the repo (built-in agents)
#   BENCH_AGENT=none    do nothing
#   BENCH_DRY_RUN=1     with claude: run the real CLI with no credentials (it
#                       stops at "Not logged in" without contacting the model),
#                       then copy /trial/stub over the repo
set -u
mkdir -p "$HOME/.claude"
if [[ -r /run/bench-creds/.credentials.json ]]; then
  # Read-only mount of the host credentials file, exposed where the CLI looks.
  ln -s /run/bench-creds/.credentials.json "$HOME/.claude/.credentials.json"
fi
cd /trial/repo
case "${BENCH_AGENT:-none}" in
  claude)
    if [[ "${BENCH_DRY_RUN:-0}" == 1 ]]; then
      env -u ANTHROPIC_API_KEY -u CLAUDE_CODE_OAUTH_TOKEN bash /opt/bench/agent.sh /trial/prompt.md
      status=$?
      cp -a /trial/stub/. /trial/repo/
      exit "$status"
    fi
    exec bash /opt/bench/agent.sh /trial/prompt.md
    ;;
  copy) cp -a /trial/stub/. /trial/repo/ ;;
  none) ;;
  *) echo "bench-agent-entry: unknown BENCH_AGENT=${BENCH_AGENT}" >&2; exit 2 ;;
esac
