#!/usr/bin/env python3
"""Benchmark runner: does the tauten profile reduce defects that escape an
agent's "done"? See bench/README.md and bench/PREREGISTRATION.md.

Each trial: copy the host repo, apply the arm, run the agent, then add the
hidden oracle and measure. Results are one JSON object per trial.

Built-in agents validate tasks without spending model budget:
  null       does nothing; every oracle must fail on it
  reference  applies the task's reference solution; every oracle must pass
             and the profile must report no errors
  sloppy     applies a plausible solution with the task's typical agent
             mistake; the oracle and the profile must both catch it
Any other agent is a shell command template run in the repo with
{prompt_file} and {repo} substituted.

Run with `python3 -I bench/run.py ...` (stdlib only).
"""

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path

BENCH = Path(__file__).resolve().parent
ARMS = ("A", "B", "C")
TEST_RESULT = re.compile(r"test result: \w+\. (\d+) passed; (\d+) failed")


def sh(cmd, cwd, timeout=900, env=None):
    started = time.monotonic()
    proc = subprocess.run(
        cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env
    )
    return proc, time.monotonic() - started


def load_task(task_id):
    task_dir = BENCH / "tasks" / task_id
    task = tomllib.loads((task_dir / "task.toml").read_text())
    host_dir = BENCH / "hosts" / task["host"]
    host = tomllib.loads((host_dir / "bench.toml").read_text())
    return task, task_dir, host, host_dir


def setup_repo(repo, host_dir, host, arm):
    shutil.copytree(host_dir, repo, ignore=shutil.ignore_patterns("target", "bench.toml", "proofs"))
    instructions = (BENCH / "arms" / f"{arm}.md").read_text()
    if arm == "C":
        # Single source: the agent profile shipped in tauten-check.
        instructions += "\n" + (BENCH.parent / "crates" / "tauten-check" / "agents.md").read_text()
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


def run_agent(agent, agent_cmd, repo, task_dir, prompt_file, log_dir, timeout):
    if agent == "null":
        return {"exit": 0, "seconds": 0.0}
    if agent in ("reference", "sloppy"):
        shutil.copytree(task_dir / agent, repo, dirs_exist_ok=True)
        return {"exit": 0, "seconds": 0.0}
    cmd = agent_cmd.format(prompt_file=shlex.quote(str(prompt_file)), repo=shlex.quote(str(repo)))
    try:
        proc, seconds = sh(["bash", "-c", cmd], cwd=repo, timeout=timeout)
        (log_dir / "agent.stdout").write_text(proc.stdout)
        (log_dir / "agent.stderr").write_text(proc.stderr)
        result = {"exit": proc.returncode, "seconds": round(seconds, 1)}
        # Agents that print a JSON summary (claude -p --output-format json)
        # get their cost and turn count recorded; others are left as is.
        try:
            summary = json.loads(proc.stdout)
        except json.JSONDecodeError:
            summary = None
        if isinstance(summary, dict):
            for key in ("total_cost_usd", "num_turns", "is_error"):
                if key in summary:
                    result[key] = summary[key]
            usage = summary.get("usage") or {}
            result["output_tokens"] = usage.get("output_tokens")
        return result
    except subprocess.TimeoutExpired:
        return {"exit": None, "seconds": timeout, "timed_out": True}


def count_tests(repo):
    """Visible #[test] and #[ignore] counts over all Rust files."""
    tests = ignores = 0
    for path in repo.rglob("*.rs"):
        if "target" in path.parts:
            continue
        text = path.read_text(errors="replace")
        tests += text.count("#[test]")
        ignores += text.count("#[ignore")
    return tests, ignores


def added_lines(repo):
    proc = subprocess.run(["git", "diff", "--cached", "-U0", "HEAD"], cwd=repo, capture_output=True, text=True)
    return [l[1:] for l in proc.stdout.splitlines() if l.startswith("+") and not l.startswith("+++")]


def test_counts(output):
    passed = failed = 0
    for m in TEST_RESULT.finditer(output):
        passed += int(m.group(1))
        failed += int(m.group(2))
    return passed, failed


def tauten_findings(repo, host, tauten_bin, env):
    args = [tauten_bin, "tauten", "check", "--format", "json", "--no-baseline"]
    for crate, tier in host["tiers"].items():
        args += ["--assume-tier", f"{crate}={tier}"]
    proc, _ = sh(args, cwd=repo, env=env)
    by_rule = {}
    errors = 0
    for line in proc.stdout.splitlines():
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        if d.get("id") == "TAU-D001":
            continue
        key = f'{d["id"]}:{d["severity"]}'
        by_rule[key] = by_rule.get(key, 0) + 1
        errors += d["severity"] == "error"
    return {"errors": errors, "by_rule": by_rule, "exit": proc.returncode}


def evaluate(repo, task, task_dir, host, base_counts, tauten_bin, env):
    result = {}
    subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
    added = added_lines(repo)
    result["diff_added_lines"] = len(added)
    result["stubs_added"] = sum(bool(re.search(r"\b(todo|unimplemented)!", l)) for l in added)
    tests, ignores = count_tests(repo)
    result["tests_removed"] = max(0, base_counts[0] - tests)
    result["ignores_added"] = max(0, ignores - base_counts[1])

    build, _ = sh(["cargo", "build", "--workspace", "--all-targets", "--quiet"], cwd=repo, env=env)
    result["build_ok"] = build.returncode == 0
    visible, _ = sh(["cargo", "test", "--workspace", "--quiet"], cwd=repo, env=env)
    result["visible_passed"], result["visible_failed"] = test_counts(visible.stdout)
    result["visible_ok"] = visible.returncode == 0

    result["tauten"] = tauten_findings(repo, host, tauten_bin, env)

    dest = repo / host["oracle_dir"]
    (dest / "support").mkdir(parents=True, exist_ok=True)
    shutil.copy(BENCH / "tasks" / "support" / "oracle_support.rs", dest / "support" / "oracle_support.rs")
    name = "oracle_" + re.sub(r"\W", "_", task["id"])
    shutil.copy(task_dir / "oracle" / "oracle.rs", dest / f"{name}.rs")
    oracle, _ = sh(
        ["cargo", "test", "--quiet", "-p", host["oracle_package"], "--test", name, "--", "--test-threads=1"],
        cwd=repo,
        env=env,
    )
    passed, failed = test_counts(oracle.stdout)
    result["oracle_passed"] = passed
    result["oracle_failed"] = failed
    result["oracle_ran"] = passed + failed > 0
    result["oracle_panics"] = (oracle.stdout + oracle.stderr).count("ORACLE-PANIC")
    # Primary metric: any hidden-oracle failure (including not compiling) is an escaped defect.
    result["escaped_defect"] = failed > 0 or not result["oracle_ran"]
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--tasks", default="all", help="comma-separated task ids, or 'all' / 'dev' / 'heldout'")
    p.add_argument("--arms", default="A,B,C")
    p.add_argument("--trials", type=int, default=1)
    p.add_argument("--agent", default="null", help="null | reference | cmd")
    p.add_argument("--agent-cmd", default="", help="shell template for --agent cmd")
    p.add_argument("--agent-timeout", type=int, default=1800)
    p.add_argument("--label", default="", help="free-form run label (model, date) stored with results")
    p.add_argument("--work", default=os.environ.get("BENCH_WORK", "/tmp/tauten-bench"))
    p.add_argument("--out", required=True)
    p.add_argument("--tauten", default=os.environ.get("TAUTEN_BIN", "cargo-tauten"))
    args = p.parse_args()

    all_tasks = sorted(d.name for d in (BENCH / "tasks").iterdir() if (d / "task.toml").is_file())
    if args.tasks in ("all", "dev", "heldout"):
        wanted = [t for t in all_tasks if args.tasks == "all" or load_task(t)[0]["split"] == args.tasks]
    else:
        wanted = args.tasks.split(",")
    arms = args.arms.split(",")
    if args.agent == "cmd" and not args.agent_cmd:
        sys.exit("--agent cmd needs --agent-cmd")

    run_id = time.strftime("%Y%m%dT%H%M%S")
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    with out.open("a") as sink:
        for task_id in wanted:
            task, task_dir, host, host_dir = load_task(task_id)
            for arm in arms:
                for trial in range(args.trials):
                    work = Path(args.work) / run_id / f"{task_id}-{arm}-{trial}"
                    repo = work / "repo"
                    work.mkdir(parents=True)
                    setup_repo(repo, host_dir, host, arm)
                    base_counts = count_tests(repo)
                    prompt_file = work / "prompt.md"
                    shutil.copy(task_dir / "prompt.md", prompt_file)
                    env["CARGO_TARGET_DIR"] = str(work / "target")
                    agent = run_agent(args.agent, args.agent_cmd, repo, task_dir, prompt_file, work, args.agent_timeout)
                    row = {
                        "run": run_id,
                        "label": args.label,
                        "task": task_id,
                        "split": task["split"],
                        "arm": arm,
                        "trial": trial,
                        "agent": args.agent,
                        "agent_result": agent,
                    }
                    row.update(evaluate(repo, task, task_dir, host, base_counts, args.tauten, env))
                    sink.write(json.dumps(row, sort_keys=True) + "\n")
                    sink.flush()
                    print(
                        f"{task_id} {arm} #{trial}: defect={row['escaped_defect']} "
                        f"oracle={row['oracle_passed']}/{row['oracle_passed'] + row['oracle_failed']} "
                        f"tauten_errors={row['tauten']['errors']}",
                        flush=True,
                    )


if __name__ == "__main__":
    main()
