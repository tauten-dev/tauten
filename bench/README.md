# Benchmark: does the profile reduce escaped defects?

**Question.** When an AI coding agent reports a task as done, does working
under the tauten profile reduce the defects that escape, and at what cost in
success rate and time?

**Status.**
- The harness, one host repo and 4 dev tasks exist and are validated with
  built-in agents.
- **Pilot on the dev split only (2026-11-18, Claude Sonnet 5.5, 36 trials): no held-out result yet.** Report: [reports/2026-11-18-pilot](reports/2026-11-18-pilot/README.md).
- The design is in [PREREGISTRATION.md](PREREGISTRATION.md) (draft, not yet
  frozen).

## Layout

| Path | Contents |
|---|---|
| `hosts/ledger/` | Host repo: a three-crate ledger CLI (kernel/core/edge shape). `bench.toml` says where oracles go and which tier each crate has |
| `tasks/<id>/` | `task.toml` (split, defect classes), `prompt.md` (what the agent sees), `oracle/oracle.rs` (hidden, black-box), `reference/` (known-good solution), `sloppy/` (plausible solution with the typical mistake) |
| `tasks/support/oracle_support.rs` | Runs the built binary; any panic fails the oracle with an `ORACLE-PANIC` marker |
| `arms/{A,B,C}.md` | Agent instructions per arm |
| `run.py` | Runs trials, writes one JSON line per trial |
| `analyze.py` | Per-arm metrics and paired bootstrap CIs vs arm A |

## Arms

| Arm | The agent gets |
|---|---|
| **A** | The repo and "run `cargo test`" |
| **B** | A plus the same rules in prose (told, not enforced) |
| **C** | Tiers declared in the manifests, the tauten agent profile, and "`cargo tauten check` must pass" |

B separates the effect of *being told* from the effect of *being checked*.

## Measurements (every arm)

- **Escaped defect (primary):** a hidden-oracle failure on the finished repo.
  The oracle is black-box (it runs the binary), so it judges behaviour, never
  lint counts or architecture.
- **Reachable panic:** the binary panicked on oracle input.
- **Test tampering:** visible `#[test]`s removed, or `#[ignore]` added.
- **Stubs:** `todo!`/`unimplemented!` added.
- **Profile findings:** `cargo tauten check` is run in every arm, as a
  measurement.
- **Cost:** agent wall time and exit status. Token usage is planned, taken
  from the agent's JSON output.

## Validation (no model budget)

```console
$ python3 -I bench/run.py --agent reference --tasks dev --out results/validate.jsonl
$ python3 -I bench/run.py --agent null      --tasks dev --out results/validate.jsonl
$ python3 -I bench/run.py --agent sloppy    --tasks dev --arms A --out results/validate.jsonl
```

Required before a task is admitted:

| Agent | Must produce |
|---|---|
| reference | oracle 100%, 0 profile errors, in arms A and C |
| null | an escaped defect |
| sloppy | an escaped defect **and** at least one profile error naming the mistake |

Result on 2026-11-18, all four dev tasks:

| Task | reference | null | sloppy oracle | sloppy profile finding |
|---|---|---|---|---|
| T01-fee | 5/5, 0 errors | defect | 3/5, binary panicked | TAU-N001 ×3 (overflowing `*` and `-`) |
| T02-stats | 5/5, 0 errors | defect | 4/5, binary panicked | TAU-P001 (`unwrap`) |
| T03-cents | 5/5, 0 errors | defect | 3/5 | TAU-N002 ×3 (`f64 as u64`), TAU-X001 ×2 |
| T04-undo | 6/6, 0 errors | defect | 5/6, binary panicked | TAU-P001 (`pop().unwrap()`) |

## Running a real agent (needs the owner's model budget; not done yet)

`--agent cmd` runs any command in the trial repo, with `{prompt_file}` and
`{repo}` substituted. For Claude Code in headless mode, something like the
following, run inside a disposable container with `cargo-tauten` on `PATH`:

```console
$ python3 -I bench/run.py --agent cmd --trials 3 --label "<model> <date>" \
    --agent-cmd 'claude -p "$(cat {prompt_file})" --output-format json --max-turns 60 --dangerously-skip-permissions' \
    --out results/pilot.jsonl
```

Check the current CLI flags before the pilot. Skipping permissions is only
acceptable inside a throwaway container with no credentials beyond the model
key.
