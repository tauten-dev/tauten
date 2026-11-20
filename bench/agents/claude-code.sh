#!/usr/bin/env bash
# Benchmark agent: one headless Claude Code session in the trial repo.
# Same command, tools and limits for every arm; only the repo's AGENTS.md/CLAUDE.md differ.
# Runs inside the agent container (bench/container); run.py captures stdout,
# the full stream-json transcript, into the trial's transcript file.
#   BENCH_MODEL           model id (default claude-sonnet-5-5)
#   BENCH_MAX_TURNS       turn limit per step (default 60)
#   BENCH_MAX_BUDGET_USD  spend cap per step (default 2)
set -u
prompt_file="$1"
exec claude -p "$(cat "$prompt_file")" \
  --model "${BENCH_MODEL:-claude-sonnet-5-5}" \
  --max-turns "${BENCH_MAX_TURNS:-60}" \
  --max-budget-usd "${BENCH_MAX_BUDGET_USD:-2}" \
  --permission-mode acceptEdits \
  --tools "Bash,Read,Edit,Write,Glob,Grep" \
  --allowedTools "Bash(cargo:*)" "Bash(git diff:*)" "Bash(git status:*)" "Read" "Edit" "Write" "Glob" "Grep" \
  --setting-sources project \
  --strict-mcp-config \
  --no-session-persistence \
  --output-format stream-json \
  --verbose
