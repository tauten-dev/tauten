#!/usr/bin/env bash
# Benchmark agent: one headless Claude Code session in the trial repo.
# Same command, tools and limits for every arm; only the repo's AGENTS.md/CLAUDE.md differ.
set -u
prompt_file="$1"
exec claude -p "$(cat "$prompt_file")" \
  --model "${BENCH_MODEL:-claude-sonnet-5-5}" \
  --permission-mode acceptEdits \
  --allowedTools "Bash(cargo:*)" "Bash(git diff:*)" "Bash(git status:*)" "Read" "Edit" "Write" "Glob" "Grep" \
  --setting-sources project \
  --strict-mcp-config \
  --no-session-persistence \
  --output-format json
