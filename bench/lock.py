#!/usr/bin/env python3
"""Freeze record for the held-out split (docs/bench/HELDOUT-DESIGN.md §6 item 8).

    python3 -I bench/lock.py --write    # regenerate bench/heldout.lock
    python3 -I bench/lock.py --check    # exit 1 if any locked file changed

bench/heldout.lock lists `sha256  path` (sha256sum format, paths relative to
the repository root) for every file that defines a held-out trial: each
held-out task (task.toml, prompts, oracle, reference, sloppy, evasive), the
hosts those tasks use (without build output), the oracle support file, the
arm texts, the agent profile and rule table, the agent script, the
container files, every file built into the trial image (the workspace
manifests and `crates/**`, from which cargo-tauten is built) and the harness
scripts. run.py refuses a model run on
held-out tasks unless this check passes, and stores the lock's own sha256
in every result row.
"""

import hashlib
import sys
import tomllib
from pathlib import Path

BENCH = Path(__file__).resolve().parent
ROOT = BENCH.parent
LOCK = BENCH / "heldout.lock"


def tree(path):
    return [p for p in sorted(path.rglob("*")) if p.is_file() and "target" not in p.relative_to(path).parts]


def locked_files():
    files, hosts = [], set()
    for task_dir in sorted((BENCH / "tasks").iterdir()):
        toml = task_dir / "task.toml"
        if not toml.is_file():
            continue
        task = tomllib.loads(toml.read_text())
        if task.get("split") != "heldout":
            continue
        files += tree(task_dir)
        hosts.add(task["host"])
    for host in sorted(hosts):
        files += tree(BENCH / "hosts" / host)
    files += [BENCH / "tasks" / "support" / "oracle_support.rs"]
    files += sorted((BENCH / "arms").glob("*.md"))
    files += [ROOT / "crates" / "tauten-check" / "agents.md", ROOT / "crates" / "tauten-check" / "rules.toml"]
    files += [BENCH / "agents" / "claude-code.sh"]
    files += tree(BENCH / "container")
    # Everything built into the trial image (bench/container/build.sh): cargo-tauten's workspace.
    files += [ROOT / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")]
    files += tree(ROOT / "crates")
    files += [BENCH / name for name in ("run.py", "analyze.py", "admit.py", "lock.py")]
    files += [ROOT / "scripts" / "bench-validate"]
    style = BENCH / "TASK-STYLE.md"
    if style.is_file():
        files.append(style)
    return list(dict.fromkeys(files))  # crates/** repeats the profile files listed above


def manifest():
    lines = []
    for path in locked_files():
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        lines.append(f"{digest}  {path.relative_to(ROOT).as_posix()}")
    return "\n".join(lines) + "\n"


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "--check"
    current = manifest()
    if mode == "--write":
        LOCK.write_text(current)
        print(f"wrote {LOCK.relative_to(ROOT)}: {current.count(chr(10))} files")
        return
    if mode != "--check":
        sys.exit(__doc__)
    if not LOCK.is_file():
        sys.exit("no bench/heldout.lock")
    parse = lambda text: {path: digest for digest, path in (line.split("  ", 1) for line in text.splitlines())}
    old, new = parse(LOCK.read_text()), parse(current)
    # Held-out tasks that were never run are kept private (bench/WITHHELD.md):
    # a locked file whose whole task directory is absent is withheld, not changed.
    def withheld(path):
        parts = path.split("/")
        return path not in new and parts[:2] == ["bench", "tasks"] and not (ROOT / "/".join(parts[:3])).exists()
    held = sorted(p for p in old if withheld(p))
    if held:
        print(f"withheld: {len(held)} file(s) of {len({'/'.join(p.split('/')[:3]) for p in held})} unrun task(s)")
    changed = sorted(p for p in old.keys() | new.keys() if old.get(p) != new.get(p) and p not in held)
    for path in changed:
        state = "added" if path not in old else ("removed" if path not in new else "changed")
        print(f"{state}: {path}")
    if changed:
        sys.exit(f"bench/heldout.lock does not match: {len(changed)} file(s)")
    print(f"bench/heldout.lock matches ({len(new)} files)")


if __name__ == "__main__":
    main()
