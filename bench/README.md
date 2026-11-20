# Benchmark: does the profile reduce escaped defects?

**Question.** When an AI coding agent reports a task as done, does working
under the tauten profile reduce the defects that escape, and at what cost in
success rate and time?

**Status (final, 2026-11-19): no defect-reduction benefit established.**
- Pilot on the dev split (2026-11-18, Claude Sonnet 5.5, 36 trials): no
  escaped defects in any arm. Report:
  [reports/2026-11-18-pilot](reports/2026-11-18-pilot/README.md).
- Calibration, three rounds (2026-11-19, arm A, Claude Sonnet 5.5 and
  Claude Haiku 5.5): **2 escaped defects in 200 trials**. Round 3 was
  no-go by the preregistered rule, so the held-out comparison was **not
  run**. Rounds and decision: [TASK-STYLE.md](TASK-STYLE.md); every trial
  row: [results/calibration-2026-11-19.jsonl](results/calibration-2026-11-19.jsonl).
- E1 finding-validity audit: **0 of 120** sampled deny-level findings on
  the agents' code were confirmed defects (exact 95% interval 0-3.0%).
  Report: [reports/e1-2026-11-19](reports/e1-2026-11-19/README.md).
- Published: the harness, the three hosts, the dev tasks, the seven
  calibration tasks (K01-K03 and the probes H02, H07, H15, H19), the
  results and E1. The 16 held-out tasks that no model ran are withheld
  so they stay usable; see [WITHHELD.md](WITHHELD.md).
- [PREREGISTRATION.md](PREREGISTRATION.md) and
  [proposals/PREREGISTRATION-heldout.diff](proposals/PREREGISTRATION-heldout.diff)
  are the drafts the run would have frozen.

## Layout

| Path | Contents |
|---|---|
| `hosts/<host>/` | Host repos (kernel/core/edge crates, `SPEC.md` with ground rules, visible tests). `bench.toml` says where oracles go, the binary name and each crate's tier |
| `tasks/<id>/` | `task.toml` (split, stratum, expected rules, steps), `prompt.md` or `step1.md`/`step2.md`, `oracle/oracle.rs` (hidden, black-box), `reference/`, `sloppy/`, `evasive/` |
| `tasks/support/oracle_support.rs` | Runs the host binary on a prepared case (files, env, cwd, stdin, stdout to `/dev/full`) with a 10 s limit; a crash (panic, abort, stack overflow, signal) is `ORACLE-PANIC`, a timeout `ORACLE-HANG`; an expected error also requires empty standard output |
| `arms/{A,B,C}.md` | Agent instructions per arm; `B.md` is a template filled from the host's tiers |
| `agents/claude-code.sh` | The benchmark agent: one headless Claude Code session |
| `container/` | Image, proxy and entry point for isolated trials ([README](container/README.md)) |
| `run.py` | Runs trials (interleaved, seeded), writes one JSON line per trial; `--resume` skips trials already scored |
| `work/` | Default trial work root (git-ignored): one `.tar.gz` per finished trial (repo, transcripts, oracle logs); build output and the per-trial `CARGO_HOME` are deleted |
| `admit.py` | Admission gate for tasks |
| `analyze.py` | Per-arm metrics, paired bootstrap CIs, strata, class tags, permutation test, cost |
| `lock.py`, `heldout.lock` | SHA-256 freeze manifest of everything that defines a held-out trial |

## Arms

| Arm | The agent gets |
|---|---|
| **A** | The repo and "run `cargo test`" |
| **B** | A plus the same rules in prose (told, not enforced) |
| **C** | Tiers declared in the manifests, the tauten agent profile, and "`cargo tauten check` must pass" |

B separates the effect of *being told* from the effect of *being checked*.

## Tasks

| Split | Tasks | Purpose |
|---|---|---|
| dev | T01–T04 (ledger) | Harness and profile tuning; the pilot ran here |
| calibration | K01-discount (ledger), K02-lists (kvconf), K03-depends (jobq) | Arm A only: fix the task style and pick the primary model. Never part of a comparison |
| held-out | H01–H20 (ledger 5, kvconf 7, jobq 8) | The preregistered comparison |

Prompts do not mention SPEC.md; each host README does (TASK-STYLE.md S1). Edges are stated as rules, not worked examples, and
every oracle assertion cites the sentence that fixes it. Oracle test names
carry a class tag (`base__`, `overflow__`, `input__`, `unicode__`,
`rounding__`, `sequence__`, `authority__`, `variant__`, `ioerr__`,
`hang__`, `freshness__`, `origin__`); **task success** means every `base__`
test passes. `freshness__` (a check made on state from before a later
change: H03, H14, H16) and `origin__` (state that must come from the right
instance: H15 only, so it is reported as not measured) were added after
the review. Ground rules that every feature inherits include the coverage
rule (kvconf G7, jobq G9): every count and output format covers later
states, event kinds and value types, listed after the existing ones.

Strata (in `task.toml`): **T** targeted: a profile rule at deny level flags
the sloppy solution's defect; **A** adjacent: a rule fires near the defect
but the lint-quiet fix can still be wrong; **N** control: no rule flags the
sloppy solution.

| Id | Host | Stratum | Expected rules | Title | Steps |
|---|---|---|---|---|---|
| H01-interest | ledger | T | TAU-N001 | Interest in basis points | 1 |
| H02-statement | ledger | N | none | Statement with memos | 1 |
| H03-holds | ledger | N | none | Holds, captures and releases | 1 |
| H04-csv | ledger | T | TAU-P003 | CSV input | 1 |
| H05-summary | ledger | A | TAU-N001 | Deposit summary | 1 |
| H06-units | kvconf | T | TAU-N001 | Size and duration lookups | 1 |
| H07-include | kvconf | T | TAU-A001 | Include directive | 1 |
| H08-interpolate | kvconf | T | TAU-A001, TAU-P003 | Variable references in string values | 1 |
| H09-schema | kvconf | N | none | Schema checking | 1 |
| H10-keys | kvconf | N | none | Case-insensitive keys | 1 |
| H11-json-duration | kvconf | T | TAU-M001 | JSON dump, then a duration value type | 2 |
| H12-write-failure | kvconf | A | TAU-E003 | Output files and in-place set | 1 |
| H13-backoff | jobq | T | TAU-N001 | Exponential retry backoff | 1 |
| H14-timeout | jobq | T | TAU-N001, TAU-A001 | Job timeouts | 1 |
| H15-ratelimit | jobq | A | TAU-N001, TAU-N003 | Start rate limit | 1 |
| H16-cancel | jobq | N | none | Cancelling jobs | 1 |
| H17-stats-pause | jobq | T | TAU-M001 | Statistics, then pausing jobs (two sessions) | 2 |
| H18-aging | jobq | T | TAU-N002, TAU-N001 | Priority aging | 1 |
| H19-quoted-names | jobq | T | TAU-P003 | Quoted job names | 1 |
| H20-recurring | jobq | A | TAU-N001 | Recurring jobs | 1 |

Held-out strata: T 9, A 3, N 4 (16 tasks; H02, H07, H15 and H19 demoted to calibration, deviation D1 in TASK-STYLE.md). Calibration: K01-discount (ledger, T), K02-lists (kvconf, T), K03-depends (jobq, N).

## Measurements (every arm)

- **Escaped defect (primary):** a hidden-oracle failure on the finished repo,
  in the dev **or** the release profile, including an oracle that does not
  build. The oracle is black-box (it runs the binary), so it judges
  behaviour, never lint counts or architecture. Both profiles are recorded
  (`oracle_tests_dev`, `oracle_tests_release`, `escaped_defect_dev`,
  `escaped_defect_release`). A test that hits the time limit is re-run once
  after every other trial has finished; both runs are recorded
  (`oracle_hung`, `oracle_hang_rerun`) and the re-run decides.
- **Task success:** all `base__` oracle tests pass in both profiles.
- Per-test oracle results and the failing class tags; reachable panics
  (including aborts and stack overflows) and hangs.
- Test tampering (visible `#[test]` functions removed, counted by name and
  separately from tests added, or `#[ignore]` added); stubs.
- **Infra errors** are not scored: a harness exception, an agent session
  that ends on an API, authentication or quota error or starts no session,
  or a transcript whose model is not `--model` writes a row with
  `infra_error` and no outcome. Re-run them with `--resume`.
- **Profile findings:** `cargo tauten check` in every arm, as a measurement.
- **Cost:** wall time, turns, tokens and USD from the agent's transcript.
- **Provenance:** harness commit, lock and preregistration hashes, image id
  and its harness label, rustc, cargo-tauten and Claude Code versions,
  model id, base commit of the trial repo, in every row.

## Validation (no model budget)

```console
$ docker compose run --rm bench-validate      # in the tools image: every task, every built-in agent, then admit.py
$ docker compose run --rm -e BENCH_TASKS=H03-holds,K02-lists bench-validate   # a subset
```

Required before a task is admitted (`admit.py`):

| Agent | Must produce |
|---|---|
| reference | oracle 100% in the dev and the release profile, 0 profile errors, visible tests pass, in arms A and C |
| null | an escaped defect |
| sloppy | an escaped defect; for T and A tasks at least one profile error from `expected_rules`; for N tasks **zero** profile errors |
| evasive | (A tasks, and T tasks that have one) zero profile errors and still an escaped defect |

Result on 2026-11-19 after the review (`docker compose run --rm bench-validate`, 27 tasks, 121 trials, rustc 1.98.0, oracle in dev and release): **27 admitted, 0 problems**. Oracle tests passed per agent (a test passes only if it passes in both profiles; the only difference between the profiles: H15's sloppy solution fails `overflow__largest_rate_period_and_burst` in dev but passes it in release, where the wrapped arithmetic happens to give the right answer); profile errors raised by sloppy and evasive solutions:

| Task | Stratum | reference A / C | null | sloppy | evasive |
|---|---|---|---|---|---|
| H01-interest | T | 9/9 / 9/9 | 1/9 | 5/9; TAU-N001 | 6/9; 0 errors |
| H02-statement | N | 10/10 / 10/10 | 0/10 | 9/10; 0 errors | - |
| H03-holds | N | 11/11 / 11/11 | 0/11 | 6/11; 0 errors | - |
| H04-csv | T | 14/14 / 14/14 | 1/14 | 7/14; TAU-P003 | 7/14; 0 errors |
| H05-summary | A | 8/8 / 8/8 | 0/8 | 6/8; TAU-N001 | 6/8; 0 errors |
| H06-units | T | 8/8 / 8/8 | 0/8 | 4/8; TAU-N001 | 4/8; 0 errors |
| H07-include | T | 13/13 / 13/13 | 0/13 | 4/13; TAU-A001 | - |
| H08-interpolate | T | 11/11 / 11/11 | 1/11 | 8/11; TAU-A001,TAU-N001,TAU-P003 | - |
| H09-schema | N | 7/7 / 7/7 | 0/7 | 6/7; 0 errors | - |
| H10-keys | N | 7/7 / 7/7 | 0/7 | 6/7; 0 errors | - |
| H11-json-duration | T | 9/9 / 9/9 | 0/9 | 7/9; TAU-M001 | 7/9; 0 errors |
| H12-write-failure | A | 8/8 / 8/8 | 0/8 | 6/8; TAU-E003 | 6/8; 0 errors |
| H13-backoff | T | 11/11 / 11/11 | 1/11 | 8/11; TAU-N001,TAU-N002 | 9/11; 0 errors |
| H14-timeout | T | 13/13 / 13/13 | 0/13 | 11/13; TAU-N001 | 11/13; 0 errors |
| H15-ratelimit | A | 13/13 / 13/13 | 1/13 | 8/13; TAU-N001 | 9/13; 0 errors |
| H16-cancel | N | 9/9 / 9/9 | 0/9 | 3/9; 0 errors | - |
| H17-stats-pause | T | 11/11 / 11/11 | 1/11 | 8/11; TAU-M001 | - |
| H18-aging | T | 10/10 / 10/10 | 1/10 | 5/10; TAU-N001,TAU-N002 | 6/10; 0 errors |
| H19-quoted-names | T | 10/10 / 10/10 | 1/10 | 8/10; TAU-N001,TAU-P003 | 8/10; 0 errors |
| H20-recurring | A | 10/10 / 10/10 | 1/10 | 7/10; TAU-N001 | 9/10; 0 errors |
| K01-discount | T | 9/9 / 9/9 | 2/9 | 8/9; TAU-N001 | 8/9; 0 errors |
| K02-lists | T | 8/8 / 8/8 | 1/8 | 5/8; TAU-N001,TAU-N002,TAU-P003 | - |
| K03-depends | N | 10/10 / 10/10 | 0/10 | 8/10; 0 errors | - |
| T01-fee | T | 7/7 / 7/7 | 1/7 | 4/7; TAU-N001 | - |
| T02-stats | T | 6/6 / 6/6 | 0/6 | 4/6; TAU-P001 | - |
| T03-cents | T | 6/6 / 6/6 | 1/6 | 4/6; TAU-N002,TAU-X001 | - |
| T04-undo | T | 8/8 / 8/8 | 0/8 | 7/8; TAU-P001 | - |

The container path is checked without a model by the dry run in
[container/README.md](container/README.md#dry-run-no-model).

## Running with a model (needs the owner's approval and budget)

Costs: calibration round 1 measured **$0.209 and 64 s** mean per trial for
Claude Sonnet 5.5 and **$0.020 and 82 s** for Claude Haiku 5.5 (arm A, 15
trials each, 2026-11-19). Held-out estimates below scale those by 1.5–2×
for the larger tasks and arm C's extra turns; they are estimates, not
measurements. Evaluation now builds and runs the oracle twice (dev and
release), which adds roughly 20–40 s of wall time per trial but no model
cost.

**0. Build the image and give it a credential** (see
[container/README.md](container/README.md#credentials)):

```console
$ bench/container/build.sh
$ export CLAUDE_CODE_OAUTH_TOKEN=...          # from `claude setup-token`, or ANTHROPIC_API_KEY=...
```

**1. Calibration** (arm A only; `run.py` refuses other arms here). 3 tasks ×
5 trials per model per round, **about $3–6 (Sonnet) and under $1 (Haiku)
per round**, about 15 minutes per model per round with 4 workers. Use a
new round label each round; every round's rows are kept in the same file:

```console
$ python3 -I bench/run.py --agent claude --isolation docker --model claude-sonnet-5-5 \
    --tasks calibration --arms A --trials 5 --workers 4 \
    --label "calibration r2 model=claude-sonnet-5-5 date=$(date -I)" --out bench/results/calibration.jsonl
$ python3 -I bench/run.py --agent claude --isolation docker --model claude-haiku-5-5 \
    --tasks calibration --arms A --trials 5 --workers 4 \
    --label "calibration r2 model=claude-haiku-5-5 date=$(date -I)" --out bench/results/calibration.jsonl
$ python3 -I bench/analyze.py bench/results/calibration.jsonl --split calibration --label "calibration r2 model=claude-sonnet-5-5 date=..."
```

Target: arm A escaped-defect rate 30–60%. Outside that, change only the
style knobs that apply to every task (edge wording in SPEC vs prompt,
requirements per task, host size, turn limit), re-admit, and repeat; write
the knobs into `bench/TASK-STYLE.md`; pick the primary model (rate nearer
40%). Every round's rows are kept.

**Interrupted runs and infra errors.** Re-run the same command with
`--resume`: trials that already have a scored row with the same task, arm,
trial, model, agent and label are skipped; `infra_error` rows are run
again. `analyze.py` prints the infra rows and a balance and duplicate
check before any table.

Trial work goes to `bench/work/<run>/` (git-ignored) unless `--work` says
otherwise; each finished trial is kept there as a `.tar.gz` (repo,
transcripts, oracle logs, about 100 KB–1 MB each) and its build output is
deleted.

**2. Freeze** (irreversible for this suite; the owner's step): apply and
complete the preregistration diff, then

```console
$ docker compose run --rm bench-validate && python3 -I bench/lock.py --write
$ git add bench/PREREGISTRATION.md bench/heldout.lock && git commit -m "bench: freeze held-out preregistration"
$ bench/container/build.sh                    # the image label must be the freeze commit
```

**3. Held-out**, primary model then replication. 20 tasks × 3 arms × 10
trials = 600 trials per model (+60 second steps): **about $190–250 for
Sonnet 5.5** and **about $20–25 for Haiku 5.5** at calibration prices
scaled 1.5–2×, about 3–5 h with 4 workers. `run.py` refuses unless the
preregistration is frozen, the lock matches and the image was built from
the current commit.

```console
$ python3 -I bench/run.py --agent claude --isolation docker --model <primary-id> \
    --tasks heldout --arms A,B,C --trials 10 --workers 4 \
    --label "heldout primary model=<primary-id> date=$(date -I)" --out bench/results/heldout-primary.jsonl
$ python3 -I bench/run.py ... --resume        # same arguments, after an interruption or infra errors
$ python3 -I bench/analyze.py bench/results/heldout-primary.jsonl --agent claude --split heldout
```

`--max-budget-usd` (default 2) caps each agent session; `--max-turns` (60)
and `--agent-timeout` (900 s) are the same in every arm.
