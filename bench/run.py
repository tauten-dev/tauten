#!/usr/bin/env python3
"""Benchmark runner: does the tauten profile reduce defects that escape an
agent's "done"? See bench/README.md and bench/PREREGISTRATION.md.

Each trial: copy the host repo, apply the arm, run the agent (one fresh
session per task step), then add the hidden oracle and measure. Results are
one JSON object per trial.

Built-in agents validate tasks without spending model budget:
  null       does nothing; every oracle must fail on it
  reference  applies the task's reference solution; every oracle must pass
             and the profile must report no errors
  sloppy     applies a plausible solution with the task's typical agent
             mistake; the oracle must catch it (and the profile, by stratum)
  evasive    the sloppy solution with the cheapest lint-silencing edit; the
             profile must be clean and the oracle must still fail
Model agents:
  claude     Claude Code through bench/agents/claude-code.sh; container only
  cmd        a shell template run in the repo with {prompt_file} and {repo}
             substituted (no isolation; local experiments only)

Isolation (--isolation):
  none       everything runs on this machine (built-in agents, gate, CI)
  docker     the agent runs in a fresh container that sees only the trial
             repo, the prompt and the agent script, with network only through
             an allowlisting proxy; evaluation runs in a second container with
             no network. See bench/container/README.md.
--dry-run (with --agent claude --isolation docker) runs the real CLI with no
credentials (it stops at "Not logged in" without contacting the model), keeps
its transcript, then applies the reference solution inside the container.

Run with `python3 -I bench/run.py ...` (stdlib only).
"""

import argparse
import concurrent.futures
import hashlib
import json
import os
import random
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import threading
import time
import tomllib
import uuid
from collections import Counter
from pathlib import Path

BENCH = Path(__file__).resolve().parent
REPO_ROOT = BENCH.parent
BUILTIN = ("null", "reference", "sloppy", "evasive")
TEST_RESULT = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed")
IMAGE = os.environ.get("BENCH_IMAGE", "tauten-bench:local")
NETWORK = "tauten-bench-egress"
PROXY = "tauten-bench-proxy"
ORDER_SEED = "20261118"


def sh(cmd, cwd, timeout=900, env=None):
    started = time.monotonic()
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env)
    return proc, time.monotonic() - started


# ------------------------------------------------------------------ tasks ---


def load_task(task_id):
    task_dir = BENCH / "tasks" / task_id
    task = tomllib.loads((task_dir / "task.toml").read_text())
    task.setdefault("steps", ["prompt.md"])
    task.setdefault("stratum", None)
    task.setdefault("expected_rules", [])
    host_dir = BENCH / "hosts" / task["host"]
    host = tomllib.loads((host_dir / "bench.toml").read_text())
    return task, task_dir, host, host_dir


def all_task_ids():
    return sorted(d.name for d in (BENCH / "tasks").iterdir() if (d / "task.toml").is_file())


def arm_instructions(arm, host):
    text = (BENCH / "arms" / f"{arm}.md").read_text()
    if arm == "B":
        # One template for every host: crate names come from bench.toml.
        names = {
            tier: ", ".join(f"`{c}`" for c, t in host["tiers"].items() if t == tier)
            for tier in ("kernel", "core", "edge")
        }
        text = text.format(**names)
    if arm == "C":
        # Single source: the agent profile shipped in tauten-check.
        text += "\n" + (REPO_ROOT / "crates" / "tauten-check" / "agents.md").read_text()
    return text


def setup_repo(repo, host_dir, host, arm):
    shutil.copytree(host_dir, repo, ignore=shutil.ignore_patterns("target", "bench.toml", "proofs"))
    instructions = arm_instructions(arm, host)
    # Build output must never count as the agent's change.
    (repo / ".gitignore").write_text("/target\n")
    (repo / "AGENTS.md").write_text(instructions)
    (repo / "CLAUDE.md").write_text(instructions)
    if arm == "C":
        for crate, tier in host["tiers"].items():
            manifest = repo / "crates" / crate / "Cargo.toml"
            text = manifest.read_text()
            manifest.write_text(text + f'\n[package.metadata.tauten]\ntier = "{tier}"\n')
    for cmd in (
        ["git", "init", "-q", "-b", "main"],
        ["git", "add", "-A"],
        ["git", "-c", "user.name=bench", "-c", "user.email=bench@invalid", "commit", "-qm", "base"],
    ):
        subprocess.run(cmd, cwd=repo, check=True, capture_output=True)


# -------------------------------------------------------------- executors ---


class Local:
    """Run evaluation commands on this machine (built-in agents only)."""

    def __init__(self, tauten_bin):
        self.tauten_bin = tauten_bin

    def run(self, work, args, timeout=900):
        env = dict(os.environ)
        env["CARGO_TARGET_DIR"] = str(work / "target")
        try:
            return sh(args, cwd=work / "repo", timeout=timeout, env=env)[0]
        except subprocess.TimeoutExpired:
            return subprocess.CompletedProcess(args, 124, "", f"timed out after {timeout}s")


class Docker:
    """Run evaluation commands in a throwaway container with no network.

    The finished repo is agent-written code, so with a model agent it is
    built, tested and even `git add`ed only in here, never on the host (a
    `.git/config` or a build script could run commands)."""

    tauten_bin = "cargo-tauten"

    def run(self, work, args, timeout=900):
        name = f"tauten-bench-eval-{uuid.uuid4().hex[:12]}"
        cmd = [
            "docker", "run", "--rm", "--name", name, "--network", "none", "--read-only",
            "--user", f"{os.getuid()}:{os.getgid()}",
            "--tmpfs", "/home/bench:exec,size=1g", "--tmpfs", "/tmp:exec,size=4g",
            "--cpus", os.environ.get("BENCH_CPUS", "4"), "--memory", os.environ.get("BENCH_MEMORY", "6g"),
            "--pids-limit", "1024",
            "-v", f"{work}:/trial", "-w", "/trial/repo",
            # The agent session's CARGO_HOME (crates it downloaded), so a
            # dependency the agent added builds offline here.
            "-e", "CARGO_HOME=/trial/cargo-home",
            "-e", "CARGO_TARGET_DIR=/trial/target", "-e", "CARGO_NET_OFFLINE=true",
            IMAGE, *args,
        ]
        try:
            return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        except subprocess.TimeoutExpired:
            subprocess.run(["docker", "kill", name], capture_output=True)
            return subprocess.CompletedProcess(cmd, 124, "", f"timed out after {timeout}s")


def ensure_network():
    """Internal network with no route out, plus the allowlisting proxy on it."""
    if subprocess.run(["docker", "network", "inspect", NETWORK], capture_output=True).returncode:
        subprocess.run(["docker", "network", "create", "--internal", NETWORK], check=True, capture_output=True)
    state = subprocess.run(["docker", "inspect", "-f", "{{.State.Running}}", PROXY], capture_output=True, text=True)
    if state.stdout.strip() != "true":
        subprocess.run(["docker", "rm", "-f", PROXY], capture_output=True)
        subprocess.run(
            ["docker", "run", "-d", "--name", PROXY, "--read-only", "--tmpfs", "/run", "--tmpfs", "/tmp",
             "--tmpfs", "/var/log/tinyproxy", IMAGE, "tinyproxy", "-d", "-c", "/etc/tinyproxy/bench.conf"],
            check=True, capture_output=True,
        )
        subprocess.run(["docker", "network", "connect", NETWORK, PROXY], check=True, capture_output=True)


# ----------------------------------------------------------------- agents ---


def parse_transcript(text):
    """Summary of a stream-json transcript: init (model, CLI version) and the final result event."""
    summary = {}
    for line in text.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict):
            continue
        if event.get("type") == "system" and event.get("subtype") == "init":
            summary["model"] = event.get("model")
            summary["claude_code_version"] = event.get("claude_code_version")
            summary["tools"] = event.get("tools")
            summary["api_key_source"] = event.get("apiKeySource")
        if event.get("type") == "result":
            for key in ("total_cost_usd", "num_turns", "is_error", "subtype", "result", "duration_ms", "terminal_reason"):
                if key in event:
                    summary[key] = event[key]
            usage = event.get("usage") or {}
            for key in ("input_tokens", "output_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"):
                summary[key] = usage.get(key)
    if isinstance(summary.get("result"), str):
        summary["result"] = summary["result"][:300]
    return summary


ALLOWED_PREFIXES = (
    "/trial/repo", "/trial/prompt.md", "/tmp", "/home/bench", "/usr/local/cargo", "/usr/local/rustup", "/dev/null",
)
PATH_IN_TEXT = re.compile(r"(?<![\w.~-])(/[A-Za-z0-9_.@+-]+(?:/[A-Za-z0-9_.@+-]+)*)")


def leak_scan(text):
    """Absolute paths named in the agent's tool calls outside the trial repo and toolchain."""
    outside = set()
    for line in text.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict) or event.get("type") != "assistant":
            continue
        for block in (event.get("message") or {}).get("content") or []:
            if not isinstance(block, dict) or block.get("type") != "tool_use":
                continue
            for path in PATH_IN_TEXT.findall(json.dumps(block.get("input"))):
                if not path.startswith(ALLOWED_PREFIXES):
                    outside.add(path)
    return sorted(outside)


def run_cmd_local(agent_cmd, repo, prompt_file, work, timeout, step):
    cmd = agent_cmd.format(prompt_file=shlex.quote(str(prompt_file)), repo=shlex.quote(str(repo)))
    try:
        proc, seconds = sh(["bash", "-c", cmd], cwd=repo, timeout=timeout)
    except subprocess.TimeoutExpired:
        return {"exit": None, "seconds": timeout, "timed_out": True}
    (work / f"transcript-{step}.jsonl").write_text(proc.stdout)
    (work / f"agent-{step}.stderr").write_text(proc.stderr)
    result = {"exit": proc.returncode, "seconds": round(seconds, 1)}
    result.update(parse_transcript(proc.stdout))
    try:  # --output-format json prints one summary object
        summary = json.loads(proc.stdout)
        if isinstance(summary, dict):
            result.update(parse_transcript(json.dumps({**summary, "type": "result"})))
    except json.JSONDecodeError:
        pass
    return result


def run_in_agent_container(args, work, prompt_file, step, stub):
    """One agent session in a fresh container that sees only the trial repo."""
    name = f"tauten-bench-agent-{uuid.uuid4().hex[:12]}"
    mode = "claude" if args.agent == "claude" else ("none" if stub is None else "copy")
    cmd = [
        "docker", "run", "--rm", "--name", name, "--network", NETWORK, "--read-only",
        "--user", f"{os.getuid()}:{os.getgid()}",
        "--tmpfs", "/home/bench:exec,size=2g", "--tmpfs", "/tmp:exec,size=4g",
        "--cpus", os.environ.get("BENCH_CPUS", "4"), "--memory", os.environ.get("BENCH_MEMORY", "6g"),
        "--pids-limit", "1024",
        "-e", f"HTTPS_PROXY=http://{PROXY}:8888", "-e", f"HTTP_PROXY=http://{PROXY}:8888",
        "-e", "NO_PROXY=localhost,127.0.0.1",
        "-e", f"BENCH_AGENT={mode}", "-e", f"BENCH_DRY_RUN={int(args.dry_run)}",
        "-e", f"BENCH_MODEL={args.model}", "-e", f"BENCH_MAX_TURNS={args.max_turns}",
        "-e", f"BENCH_MAX_BUDGET_USD={args.max_budget_usd}",
        "-v", f"{work / 'repo'}:/trial/repo",
        # Per-trial CARGO_HOME, later mounted into the evaluation container:
        # crates.io stays reachable through the proxy in every arm.
        "-v", f"{work / 'cargo-home'}:/trial/cargo-home", "-e", "CARGO_HOME=/trial/cargo-home",
        "-v", f"{prompt_file}:/trial/prompt.md:ro",
        "-v", f"{BENCH / 'agents' / 'claude-code.sh'}:/opt/bench/agent.sh:ro",
    ]
    if stub is not None:
        cmd += ["-v", f"{stub}:/trial/stub:ro"]
    if mode == "claude" and not args.dry_run:
        if args.creds == "env":
            # Passed by name, so the secret never appears on a command line.
            names = [n for n in ("ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN") if os.environ.get(n)]
            if not names:
                sys.exit("--creds env: set ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN")
            for n in names:
                cmd += ["-e", n]
        else:
            path = Path(os.environ.get("BENCH_CREDENTIALS", Path.home() / ".claude" / ".credentials.json"))
            cmd += ["-v", f"{path}:/run/bench-creds/.credentials.json:ro"]
    cmd += [IMAGE, "bench-agent-entry"]
    started = time.monotonic()
    timed_out = False
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=args.agent_timeout)
        exit_code, out, err = proc.returncode, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired as e:
        subprocess.run(["docker", "kill", name], capture_output=True)
        decode = lambda b: b.decode(errors="replace") if isinstance(b, bytes) else (b or "")
        exit_code, out, err, timed_out = None, decode(e.stdout), decode(e.stderr), True
    result = {"exit": exit_code, "seconds": round(time.monotonic() - started, 1)}
    (work / f"transcript-{step}.jsonl").write_text(out)
    (work / f"agent-{step}.stderr").write_text(err)
    if timed_out:
        result["timed_out"] = True
    result.update(parse_transcript(out))
    result["outside_paths"] = leak_scan(out)
    return result


class InfraError(Exception):
    """A trial that failed for a reason outside the agent's work (not scored)."""


# An agent session that ended on an API, authentication or quota failure.
API_FAILURE = re.compile(
    r"API Error|Invalid API key|Not logged in|Please run /login|authentication|OAuth token|"
    r"Credit balance|rate limit|overloaded|Internal server error",
    re.I,
)


def check_agent_infra(args, agent):
    """Raise InfraError when a model session did not run as the agent's own attempt."""
    if args.agent != "claude" or args.dry_run:
        return
    for index, step in enumerate(agent["steps"]):
        if step.get("timed_out"):
            continue  # a timeout is the agent's outcome and is scored
        if not step.get("model"):
            raise InfraError(f"step {index}: no session started (exit {step.get('exit')})")
        if step["model"] != args.model:
            raise InfraError(f"step {index}: transcript model {step['model']!r} is not --model {args.model!r}")
        if "subtype" not in step:
            raise InfraError(f"step {index}: the session ended without a result event (exit {step.get('exit')})")
        if step.get("is_error") and API_FAILURE.search(str(step.get("result") or "")):
            raise InfraError(f"step {index}: API failure: {str(step.get('result'))[:200]}")


GIT_COMMIT = ["git", "-c", "core.hooksPath=/dev/null", "-c", "user.name=bench", "-c", "user.email=bench@invalid",
              "-c", "commit.gpgsign=false", "commit", "-q", "--no-verify", "--allow-empty"]


def commit_step(ex, work, index):
    """Commit the repo after a non-final step (in the evaluation container for model agents)."""
    add = ex.run(work, ["git", "add", "-A"])
    commit = ex.run(work, [*GIT_COMMIT, "-m", f"bench: after step {index}"])
    return add.returncode or commit.returncode


def run_agent(args, ex, work, task, task_dir):
    """Run every step of the task in a fresh session; returns per-step results and totals.

    Built-in solutions are the task's final state, so they are applied at the last step.
    The repo is committed after every step but the last, so the next session starts
    from a clean tree; evaluation diffs against the base commit, not HEAD."""
    steps = []
    repo = work / "repo"
    count = len(task["steps"])
    for index, step_file in enumerate(task["steps"]):
        prompt_file = work / f"prompt-{index}.md"
        shutil.copy(task_dir / step_file, prompt_file)
        last = index == count - 1
        solution = "reference" if args.dry_run else args.agent
        stub = task_dir / solution if (last and solution in BUILTIN and solution != "null") else None
        if args.isolation == "docker":
            steps.append(run_in_agent_container(args, work, prompt_file, index, stub))
        elif args.agent in BUILTIN:
            if stub is not None:
                shutil.copytree(stub, repo, dirs_exist_ok=True)
            steps.append({"exit": 0, "seconds": 0.0})
        else:
            steps.append(run_cmd_local(args.agent_cmd, repo, prompt_file, work, args.agent_timeout, index))
        if not last:
            steps[-1]["commit_exit"] = commit_step(ex, work, index)
    total = {"steps": steps, "seconds": round(sum(s.get("seconds") or 0 for s in steps), 1)}
    total["exit"] = steps[-1].get("exit")
    total["timed_out"] = any(s.get("timed_out") for s in steps)
    for key in ("total_cost_usd", "num_turns", "input_tokens", "output_tokens",
                "cache_creation_input_tokens", "cache_read_input_tokens"):
        values = [s.get(key) for s in steps if s.get(key) is not None]
        total[key] = sum(values) if values else None
    for key in ("model", "claude_code_version", "api_key_source"):
        found = [s.get(key) for s in steps if s.get(key)]
        total[key] = found[0] if found else None
    total["subtypes"] = [s.get("subtype") for s in steps]
    total["outside_paths"] = sorted({p for s in steps for p in s.get("outside_paths", [])})
    return total


# ------------------------------------------------------------- evaluation ---

PROFILES = ("dev", "release")
TEST_FN = re.compile(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)")
TEST_FULL = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|ignored)", re.M)
FAILURE = re.compile(r"^---- (\S+) stdout ----$(.*?)(?=^---- \S+ stdout ----$|^failures:$|\Z)", re.M | re.S)


def test_inventory(repo):
    """Visible test function names (with multiplicity) and the #[ignore] count over all Rust files."""
    names = Counter()
    ignores = 0
    for path in repo.rglob("*.rs"):
        if "target" in path.relative_to(repo).parts or path.is_symlink() or not path.is_file():
            continue
        text = path.read_text(errors="replace")
        names.update(m.group(1) for m in TEST_FN.finditer(text))
        ignores += text.count("#[ignore")
    return names, ignores


def test_counts(output):
    passed = failed = 0
    for m in TEST_RESULT.finditer(output):
        passed += int(m.group(1))
        failed += int(m.group(2))
    return passed, failed


def tauten_findings(ex, work, host):
    args = [ex.tauten_bin, "tauten", "check", "--format", "json", "--no-baseline"]
    for crate, tier in host["tiers"].items():
        args += ["--assume-tier", f"{crate}={tier}"]
    proc = ex.run(work, args)
    by_rule = {}
    errors = 0
    for line in proc.stdout.splitlines():
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(d, dict) or "id" not in d or d.get("id") == "TAU-D001":
            continue
        key = f'{d["id"]}:{d["severity"]}'
        by_rule[key] = by_rule.get(key, 0) + 1
        errors += d["severity"] == "error"
    return {"errors": errors, "by_rule": by_rule, "exit": proc.returncode}


def safe_dest(repo, rel):
    """A path inside the repo that is not reached through an agent-made symlink
    (the final component included)."""
    dest = repo / rel
    probe = dest
    while probe != repo:
        if probe.is_symlink():
            raise RuntimeError(f"oracle destination {rel} goes through a symlink")
        probe = probe.parent
    return dest


def write_fresh(repo, rel, data):
    """Replace whatever the agent left at `rel` with a new regular file holding `data`."""
    path = safe_dest(repo, rel)
    if path.is_dir():
        raise RuntimeError(f"oracle destination {rel} is a directory")
    if path.exists() or path.is_symlink():
        path.unlink()
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
    with os.fdopen(fd, "wb") as f:
        f.write(data)


def install_oracle(repo, task, task_dir, host):
    oracle_dir = Path(host["oracle_dir"])
    safe_dest(repo, oracle_dir / "support").mkdir(parents=True, exist_ok=True)
    header = f'const ORACLE_BIN: &str = env!("CARGO_BIN_EXE_{host["binary"]}");\n'
    support = header + (BENCH / "tasks" / "support" / "oracle_support.rs").read_text()
    write_fresh(repo, oracle_dir / "support" / "oracle_support.rs", support.encode())
    name = "oracle_" + re.sub(r"\W", "_", task["id"])
    write_fresh(repo, oracle_dir / f"{name}.rs", (task_dir / "oracle" / "oracle.rs").read_bytes())
    return name


def run_oracle(ex, work, host, name, profile, only=()):
    args = ["cargo", "test", *(["--release"] if profile == "release" else []),
            "-p", host["oracle_package"], "--test", name, "--", "--test-threads=1"]
    if only:
        args += ["--exact", *only]
    proc = ex.run(work, args)
    return proc.stdout + proc.stderr


def oracle_summary(output):
    """Per-test statuses (short names), counts, and the full names of tests that hung."""
    passed, failed = test_counts(output)
    return {
        "passed": passed,
        "failed": failed,
        "ran": passed + failed > 0,
        "panics": output.count("ORACLE-PANIC"),
        "hangs": output.count("ORACLE-HANG"),
        "tests": {m.group(1).split("::")[-1]: m.group(2) for m in TEST_FULL.finditer(output)},
        "hung": sorted({full for full, body in FAILURE.findall(output) if "ORACLE-HANG" in body}),
    }


def score(result):
    """Primary and secondary oracle fields from the per-profile results (after hang re-runs)."""
    final = {}
    for p in PROFILES:
        tests = dict(result[f"oracle_tests_{p}"])
        tests.update(result["oracle_hang_rerun"].get(p, {}))
        final[p] = tests
        failed = sum(s == "FAILED" for s in tests.values())
        result[f"escaped_defect_{p}"] = failed > 0 or not result[f"oracle_ran_{p}"]
    merged = {}
    for p in PROFILES:
        for n, s in final[p].items():
            merged[n] = "FAILED" if "FAILED" in (s, merged.get(n)) else merged.get(n, s)
    result["oracle_tests"] = merged
    result["oracle_ran"] = all(result[f"oracle_ran_{p}"] for p in PROFILES)
    result["oracle_passed"] = sum(s == "ok" for s in merged.values())
    result["oracle_failed"] = sum(s == "FAILED" for s in merged.values())
    result["failed_classes"] = sorted({n.split("__")[0] for n, s in merged.items() if s == "FAILED" and "__" in n})
    # Task success (secondary): every `base__` test passes in both profiles. Untagged (dev) oracles: every test.
    base = [s for n, s in merged.items() if n.startswith("base__")]
    if not result["oracle_ran"]:
        result["task_success"] = False
    elif any("__" in n for n in merged):
        result["task_success"] = bool(base) and all(s == "ok" for s in base)
    else:
        result["task_success"] = result["oracle_failed"] == 0
    # Primary metric: any hidden-oracle failure in the dev or the release
    # profile (including not compiling) is an escaped defect.
    result["escaped_defect"] = any(result[f"escaped_defect_{p}"] for p in PROFILES)
    return result


def evaluate(ex, work, task, task_dir, host, base_inventory, base_sha):
    """Measure the finished repo. Returns (result, name of the installed oracle)."""
    repo = work / "repo"
    result = {"base_sha": base_sha}
    ex.run(work, ["git", "add", "-A"])
    diff = ex.run(work, ["git", "diff", "--cached", "-U0", base_sha]).stdout
    added = [l[1:] for l in diff.splitlines() if l.startswith("+") and not l.startswith("+++")]
    result["diff_added_lines"] = len(added)
    result["stubs_added"] = sum(bool(re.search(r"\b(todo|unimplemented)!", l)) for l in added)
    names, ignores = test_inventory(repo)
    result["tests_removed"] = sum((base_inventory[0] - names).values())
    result["tests_added"] = sum((names - base_inventory[0]).values())
    result["ignores_added"] = max(0, ignores - base_inventory[1])

    build = ex.run(work, ["cargo", "build", "--workspace", "--all-targets", "--quiet"])
    result["build_ok"] = build.returncode == 0
    visible = ex.run(work, ["cargo", "test", "--workspace", "--quiet"])
    result["visible_passed"], result["visible_failed"] = test_counts(visible.stdout)
    result["visible_ok"] = visible.returncode == 0

    result["tauten"] = tauten_findings(ex, work, host)

    try:
        name = install_oracle(repo, task, task_dir, host)
    except (RuntimeError, OSError) as e:
        name = None
        failure = f"ORACLE-NOT-INSTALLED: {e}"
    result["oracle_panics"] = result["oracle_hangs"] = 0
    result["oracle_hang_rerun"] = {}
    result["oracle_hung"] = {}
    for p in PROFILES:
        output = run_oracle(ex, work, host, name, p) if name else failure
        (work / f"oracle-{p}.log").write_text(output)
        summary = oracle_summary(output)
        result[f"oracle_tests_{p}"] = summary["tests"]
        result[f"oracle_ran_{p}"] = summary["ran"]
        result["oracle_panics"] += summary["panics"]
        result["oracle_hangs"] += summary["hangs"]
        if summary["hung"]:
            result["oracle_hung"][p] = summary["hung"]
    return score(result), name


def rerun_hangs(ex, work, host, name, result):
    """Re-run, once and with nothing else running, every oracle test that hit the
    time limit. Both runs are recorded; the re-run decides the test's status."""
    for p, hung in result["oracle_hung"].items():
        output = run_oracle(ex, work, host, name, p, only=hung)
        (work / f"oracle-{p}-rerun.log").write_text(output)
        result["oracle_hang_rerun"][p] = oracle_summary(output)["tests"]
    return score(result)


def remove_tree(path):
    """Delete a tree the agent may have made hard to delete (read-only dirs)."""
    if not os.path.lexists(path):
        return
    if path.is_symlink() or not path.is_dir():
        path.unlink()
        return

    def fix(func, p, _exc):
        for q in (os.path.dirname(p), p):
            try:
                if not os.path.islink(q):
                    os.chmod(q, 0o700)
            except OSError:
                pass
        func(p)

    if sys.version_info >= (3, 12):
        shutil.rmtree(path, onexc=fix)
    else:
        shutil.rmtree(path, onerror=fix)


def archive_trial(work):
    """Keep the finished trial (repo, transcripts, logs) as <work>.tar.gz and delete
    the build output and the per-trial CARGO_HOME. Returns the archive path, or the
    work dir if archiving failed."""
    for d in (work / "target", work / "repo" / "target", work / "cargo-home"):
        remove_tree(d)
    tarball = work.parent / f"{work.name}.tar.gz"
    try:
        with tarfile.open(tarball, "w:gz") as tar:
            tar.add(work, arcname=work.name)  # symlinks are stored, never followed
    except OSError as e:
        print(f"{work.name}: not archived ({e}); work dir kept", file=sys.stderr)
        return str(work)
    remove_tree(work)
    return str(tarball)


# ------------------------------------------------------------- provenance ---


def file_sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def lock_status():
    """(matches, sha256 of the lock file) for bench/heldout.lock against the tree."""
    lock = BENCH / "heldout.lock"
    if not lock.is_file():
        return False, None
    proc = subprocess.run([sys.executable, "-I", str(BENCH / "lock.py"), "--check"], capture_output=True, text=True)
    return proc.returncode == 0, file_sha(lock)


def harness_commit():
    """HEAD, with -dirty if the harness or the crates built into the image differ from it."""
    git = lambda *a: subprocess.run(["git", *a], cwd=REPO_ROOT, capture_output=True, text=True).stdout.strip()
    paths = ("bench", "crates", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
    return git("rev-parse", "HEAD") + ("-dirty" if git("status", "--porcelain", "--", *paths) else "")


def image_info():
    """Id and labels of the trial image (docker isolation)."""
    proc = subprocess.run(["docker", "image", "inspect", IMAGE], capture_output=True, text=True)
    if proc.returncode:
        sys.exit(f"image {IMAGE} not found: build it with bench/container/build.sh")
    info = json.loads(proc.stdout)[0]
    labels = (info.get("Config") or {}).get("Labels") or {}
    return {
        "image": IMAGE,
        "image_id": info.get("Id"),
        "image_harness": labels.get("dev.tauten.bench.harness"),
        "image_claude_version": labels.get("dev.tauten.bench.claude-version"),
    }


def tool_versions(ex, work):
    versions = {}
    for key, args in (("rustc", ["rustc", "--version"]), ("cargo_tauten", [ex.tauten_bin, "tauten", "--version"])):
        try:
            versions[key] = ex.run(work, args).stdout.strip() or None
        except OSError:
            versions[key] = None
    return versions


# ------------------------------------------------------------------- main ---


def plan(wanted, arms, trials, seed):
    """Trial order: each trial index is one block of every (task, arm), shuffled with a fixed seed."""
    order = []
    for trial in range(trials):
        block = [(task_id, arm) for task_id in wanted for arm in arms]
        random.Random(f"{seed}-{trial}").shuffle(block)
        order += [(task_id, arm, trial) for task_id, arm in block]
    return order


def row_key(row):
    """What identifies a trial for --resume."""
    return (row["task"], row["arm"], row["trial"], row.get("model"), row["agent"], row.get("label", ""))


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--tasks", default="all", help="comma-separated ids, or all / dev / calibration / heldout")
    p.add_argument("--arms", default="A,B,C")
    p.add_argument("--trials", type=int, default=1)
    p.add_argument("--agent", default="null", help="null | reference | sloppy | evasive | claude | cmd")
    p.add_argument("--agent-cmd", default="", help="shell template for --agent cmd")
    p.add_argument("--agent-timeout", type=int, default=900, help="wall limit per step, seconds")
    p.add_argument("--isolation", choices=("none", "docker"), default="none")
    p.add_argument("--dry-run", action="store_true", help="--agent claude without credentials, then the reference")
    p.add_argument("--creds", choices=("env", "file"), default="env", help="how the agent container gets credentials")
    p.add_argument("--model", default=os.environ.get("BENCH_MODEL", "claude-sonnet-5-5"))
    p.add_argument("--max-turns", type=int, default=60)
    p.add_argument("--max-budget-usd", default="2", help="spend cap per step, passed to the CLI")
    p.add_argument("--workers", type=int, default=1)
    p.add_argument("--seed", default=ORDER_SEED, help="seed for the interleaved trial order")
    p.add_argument("--label", default="", help="free-form run label stored with results")
    p.add_argument("--work", default=os.environ.get("BENCH_WORK", str(BENCH / "work")),
                   help="trial work root (default bench/work, git-ignored); each finished trial is kept as a .tar.gz")
    p.add_argument("--out", required=True)
    p.add_argument("--resume", action="store_true",
                   help="skip trials that already have a scored row (same task, arm, trial, model, agent, label) in --out")
    p.add_argument("--tauten", default=os.environ.get("TAUTEN_BIN", "cargo-tauten"))
    args = p.parse_args()

    ids = all_task_ids()
    if args.tasks in ("all", "dev", "calibration", "heldout"):
        wanted = [t for t in ids if args.tasks == "all" or load_task(t)[0]["split"] == args.tasks]
    else:
        wanted = args.tasks.split(",")
    arms = args.arms.split(",")
    if args.agent in ("sloppy", "evasive"):
        # Not every task has every built-in solution (evasive/ is for T and A tasks).
        missing = [t for t in wanted if not (BENCH / "tasks" / t / args.agent).is_dir()]
        if missing:
            print(f"skipping (no {args.agent}/): {', '.join(missing)}", file=sys.stderr)
        wanted = [t for t in wanted if t not in missing]
    if args.agent == "cmd" and not args.agent_cmd:
        sys.exit("--agent cmd needs --agent-cmd")
    if args.agent == "claude" and args.isolation != "docker":
        sys.exit("--agent claude runs only with --isolation docker")
    if args.dry_run and args.agent != "claude":
        sys.exit("--dry-run applies to --agent claude")
    real_model = args.agent in ("claude", "cmd") and not args.dry_run
    splits = {load_task(t)[0]["split"] for t in wanted}
    lock_ok, lock_sha = lock_status()
    harness = harness_commit()
    image = image_info() if args.isolation == "docker" else {}
    if real_model:
        if "heldout" in splits:
            first = (BENCH / "PREREGISTRATION.md").read_text().splitlines()[0]
            if not lock_ok or "DRAFT" in first:
                sys.exit("held-out runs need the frozen PREREGISTRATION.md and a matching bench/heldout.lock")
            if image.get("image_harness") != harness:
                sys.exit(f"held-out runs need an image built from the frozen commit: image label "
                         f"{image.get('image_harness')!r}, harness {harness!r} (rebuild with bench/container/build.sh)")
        if "calibration" in splits and arms != ["A"]:
            sys.exit("calibration tasks run in arm A only (docs/bench/HELDOUT-DESIGN.md §5)")

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    model_key = args.model if args.agent == "claude" else None
    done = set()
    if args.resume and out.is_file():
        for line in out.read_text().splitlines():
            if line.strip():
                r = json.loads(line)
                if not r.get("infra_error"):
                    done.add(row_key(r))
    items = [
        item for item in plan(wanted, arms, args.trials, args.seed)
        if (item[0], item[1], item[2], model_key, args.agent, args.label) not in done
    ]
    if args.resume:
        print(f"resume: {len(done)} scored row(s) in {out}; {len(items)} trial(s) to run", file=sys.stderr)

    run_id = time.strftime("%Y%m%dT%H%M%S")
    root = Path(args.work).resolve() / run_id
    root.mkdir(parents=True)
    if args.isolation == "docker":
        ensure_network()
        ex = Docker()
    else:
        ex = Local(args.tauten)
    provenance = {
        "harness": harness,
        "lock_sha256": lock_sha,
        "lock_matches": lock_ok,
        "prereg_sha256": file_sha(BENCH / "PREREGISTRATION.md"),
        "agent_script_sha256": file_sha(BENCH / "agents" / "claude-code.sh"),
        "isolation": args.isolation,
        "seed": args.seed,
        "max_turns": args.max_turns,
        "agent_timeout": args.agent_timeout,
        **image,
    }
    sink_lock = threading.Lock()
    versions = {}

    def write(row):
        with sink_lock, out.open("a") as sink:
            sink.write(json.dumps(row, sort_keys=True) + "\n")
        if row.get("infra_error"):
            print(f"{row['task']} {row['arm']} #{row['trial']}: INFRA ERROR {row['infra_error']}", flush=True)
            return
        print(
            f"{row['task']} {row['arm']} #{row['trial']}: defect={row['escaped_defect']} "
            f"(dev={row['escaped_defect_dev']} release={row['escaped_defect_release']}) success={row['task_success']} "
            f"oracle={row['oracle_passed']}/{row['oracle_passed'] + row['oracle_failed']} "
            f"tauten_errors={row['tauten']['errors']}",
            flush=True,
        )

    def infra(row, error):
        row.update({"infra_error": error, "escaped_defect": None, "task_success": None})
        return row

    def one(task_id, arm, trial):
        """Run one trial. Returns a deferred hang re-run, or None once the row is written."""
        task, task_dir, host, host_dir = load_task(task_id)
        work = root / f"{task_id}-{arm}-{trial}"
        row = {
            "run": run_id,
            "label": args.label,
            "task": task_id,
            "split": task["split"],
            "stratum": task["stratum"],
            "expected_rules": task["expected_rules"],
            "host": task["host"],
            "arm": arm,
            "trial": trial,
            "agent": args.agent,
            "dry_run": args.dry_run,
            "model": model_key,
            "work": str(work),
            **provenance,
        }
        try:
            work.mkdir(parents=True)
            (work / "cargo-home").mkdir()
            setup_repo(work / "repo", host_dir, host, arm)
            base_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=work / "repo", check=True,
                                      capture_output=True, text=True).stdout.strip()
            base_inventory = test_inventory(work / "repo")
            agent = run_agent(args, ex, work, task, task_dir)
            row["agent_result"] = agent
            with sink_lock:
                if not versions:
                    versions.update(tool_versions(ex, work))
            row["versions"] = dict(versions)
            check_agent_infra(args, agent)
            result, name = evaluate(ex, work, task, task_dir, host, base_inventory, base_sha)
            row.update(result)
        except Exception as e:  # noqa: BLE001 - any harness failure is an infra error, never a defect
            infra(row, str(e) if isinstance(e, InfraError) else f"{type(e).__name__}: {e}")
            if work.is_dir():
                row["work"] = archive_trial(work)
            write(row)
            return None
        if row["oracle_hung"]:
            return (row, work, ex, host, name)
        row["work"] = archive_trial(work)
        write(row)
        return None

    deferred = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for f in [pool.submit(one, *item) for item in items]:
            pending = f.result()
            if pending:
                deferred.append(pending)
    # Hang re-runs: after every other trial has finished, one at a time.
    for row, work, ex_, host, name in deferred:
        try:
            rerun_hangs(ex_, work, host, name, row)
        except Exception as e:  # noqa: BLE001
            infra(row, f"hang re-run: {type(e).__name__}: {e}")
        row["work"] = archive_trial(work)
        write(row)


if __name__ == "__main__":
    main()
