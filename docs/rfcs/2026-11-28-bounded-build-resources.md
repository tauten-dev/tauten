---
rfc: "2026-11-28-bounded-build-resources"
title: "Bounded resources for the proof gate"
status: accepted
implementation: in-progress
authors:
  - fable
created: 2026-11-28
updated: 2026-11-28
issue: https://github.com/tauten-dev/tauten/issues/82
milestone: later
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Bounded resources for the proof gate

## Summary

The proof gate runs in a container with a CPU and memory allowance, and sizes
its own concurrency from that allowance: mutant jobs × Lean threads never
exceed the CPUs the container was given. A build that the allowance cuts
short is reported as a failed calibration, never as a kill. This applies to
tauten's own gate (`proofs/`) and to the template `cargo tauten init --proofs`
writes.

Nothing changes in what is proved, in `claims.toml`, `claims.json`, the
result-line protocol or any kernel function. The Rust build, the Cargo
profiles, the linker and the commit gate (`compose.yaml`) stay as they are:
they were measured and are not the problem.

## Motivation

The owner reports that building tauten, and above all `cargo tauten proofs`,
tends to hang the development host: WSL2 with 32 logical CPUs, 30 GiB of RAM
and 8 GiB of swap. Issue #98 records two host restarts during one
qualification run. The measurements below were taken on that host on
2026-11-28. No hang was reproduced, and the full gate was not run: the
conclusions come from the code and from single builds inside containers with
a hard allowance.

### What the code does today

| # | Fact | Where |
|---|---|---|
| 1 | The proofs container has no CPU, memory or task limit. The tools container of the commit gate has all three. | `compose.proofs.yaml:13-28`, template `compose.yaml:7-20`; compare `compose.yaml:17-19` |
| 2 | The partial path (`proofs try`, the ship level) runs `nproc / 2` mutants at once, each with its own `lake build`. On this host that is 16. The template uses `os.cpu_count() // 2`. | `proofs/gate.sh:76`; template `gate.py:396` |
| 3 | The full gate runs one mutant at a time, and never sets a thread count for Lean. | `proofs/gate.sh:80`, `proofs/mutants.py:954-956`, `:880` |
| 4 | A timed-out build has only its direct child killed. | `proofs/mutants.py:801-803`; template `gate.py:60-62` |
| 5 | A mutant is read as killed from the `error: <file>.lean:<line>:<col>` lines of a failed build. A Lean process that was killed prints no such line. | `proofs/mutants.py:751`, `:780-798`, `:880-912` |
| 6 | `cargo tauten proofs` starts the gate with the caller's environment and adds no limit of its own. | `crates/cargo-tauten/src/proofs.rs:475-494` |

### What was measured

**A CPU quota does not change what the gate counts.** In a container started
with `--cpus 2`, `nproc` printed 32 and Python's `os.cpu_count()` returned 32;
`/sys/fs/cgroup/cpu.max` read `200000 100000`. So capping the container
alone would leave row 2 at 16 jobs. Lake 5.0.0 lists no job-count option in
`lake --help` or `lake build --help`; `LEAN_NUM_THREADS` is the control, and
it works (below).

**One `lake build` of the theorems** (Extracted, Tauten and Audit: what the
build stage does once and every mutant repeats). Each run in a fresh
container with the CPU allowance shown, 12 GiB of memory and no swap. Other
work on the host raised the load average from 33 to 175 during these runs, so
the wall times are inflated and only their ratios are meaningful.

| `LEAN_NUM_THREADS` | CPUs | Wall | CPU time | Anonymous memory, peak | Tasks, peak | Throttled |
|---|---|---|---|---|---|---|
| unset | 8 | 40.3 s | 155.5 s | 1907 MiB | 171 | 21.7 s |
| 8 | 8 | 42.7 s | 143.4 s | 1447 MiB | 111 | 0.8 s |
| 4 | 4 | 49.0 s | 138.7 s | 1003 MiB | 82 | 31.3 s |
| 2 | 2 | 86.6 s | 154.3 s | 798 MiB | 63 | 64.2 s |

An earlier run with the variable unset and 8 CPUs, on a quieter host, took
32 s, 118 s of CPU time and peaked at 199 tasks and 4448 MiB including page
cache. Three things follow.

- Left alone, one build creates about 170 to 200 tasks whatever its
  allowance, as if sized to the host's 32 CPUs.
- A build is mostly sequential: about 140 s of CPU time whatever the thread
  count, and doubling from four to eight CPUs shortens it by 13%. The
  likely reason is that the theorem files import the translation, directly
  or through one another, so they wait for it.
- Memory grows with threads: about 1 GiB of anonymous memory at four
  threads, 1.9 GiB with the default.

**The partial path multiplies that.** Sixteen builds at once (row 2) is
about 2,700 to 3,200 tasks on 32 CPUs and about 30 GiB of anonymous memory,
on a host with 30 GiB of RAM and 8 GiB of swap. This is arithmetic from the
first row of the table, which was measured with 8 CPUs. It was not run.

**Memory pressure on this host ends in swapping.** Since the last boot four
days earlier the kernel had swapped out about 21 GiB and back in 11 GiB, and
tasks had been stalled on memory for 633 s (`/proc/pressure/memory`), with
no out-of-memory kill at all before this investigation. With no limit on
the container, a build that needs more memory than the host has makes the
whole VM swap. That is the likely hang; the counters show it happened, not
which run caused it.

**Uncapped Lean containers were observed saturating the host.** During the
measurements three Lean containers from other work on the host, each with no
limit (`docker stats` showed a limit of 30.18 GiB, the whole VM), ran at 730%
to 1050% CPU with up to 533 tasks each. The load average reached 281 on 32
CPUs and CPU pressure 70%. The capped measurement container next to them
stayed at 195% and 19 tasks. tauten's proofs container is configured the same
way as those three.

**A memory limit produces a failure the gate does not recognise.** With a
400 MiB limit, `lake build` printed `error: Lean exited with code 137`, with
no file position, and lake itself exited with 137. A genuine proof failure
exits with 1 and prints a position. If a Lean process is killed after the
expected theorems have already reported their errors, today's attribution
(row 5) reports a kill from a build that never finished, so a break in the
killed module goes unseen. A stand-in confirmed that the line is ignored:
with it appended to the output of a real build of the mutant `warn-blocks`,
today's `mutants.py` still reported `killed`. This is possible today
whenever the host kills a process; a memory limit makes it more likely
unless the rule changes.

**A timeout leaves the build running.** With the same call shape as row 4, a
one-second timeout on `sh -c 'sleep & sleep'` left both `sleep` processes
alive. Under overload this is the worst case: the gate moves on to the next
mutant while the previous build's Lean processes keep the CPUs.

**The Rust build is not the cause.** A clean `cargo build --workspace
--all-targets -j 8` took 20.6 s and 72 s of CPU time; the largest process
used 657 MiB; the target directory was 1.1 GB. `cargo test --workspace` in a
cgroup with 8 CPUs took 25 s and peaked at 1085 MiB and 115 tasks. The
slowest units are `syn` (built twice: 9.3 s and 6.1 s), the tests of
`tauten-check` (7.0 s), `clap_builder` (6.7 s) and `rustix` (6.3 s).

## Behaviour

`cargo tauten proofs`, `proofs try`, `proofs extract` and `sh proofs/run.sh`
work as before, inside an allowance:

| Setting (environment) | Default | Meaning |
|---|---|---|
| `TAUTEN_PROOFS_CPUS` | 8; through `proofs/run.sh`, the Docker daemon's CPU count when that is lower | CPUs for the proofs container |
| `TAUTEN_PROOFS_MEMORY` | `12g` | Memory for the proofs container; it gets no swap |
| `TAUTEN_MUTANT_JOBS` | CPUs ÷ threads | Mutants calibrated at once. Exists today; now honoured only up to the CPUs allowed |
| `LEAN_NUM_THREADS` | 4 per mutant build; all the CPUs for a build that runs alone | Lean's own variable; honoured only up to the CPUs allowed |

The gate prints its budget once, and what it used at the end. These lines
are from the patched gate on two mutants in a container with 8 CPUs:

```text
[0s] budget: 8 CPU(s); mutants: 2 job(s) x 4 Lean thread(s)
...
[134s] used: 652s CPU, 3048 MiB memory peak, 178 tasks peak
[134s] proofs gate: OK
```

A build that the allowance cuts short fails its mutant with a reason and a
remedy (output of the patched `mutants.py` in the stand-in experiment):

```text
FAIL warn-blocks: the build was cut short (lake exited with 1); raise TAUTEN_PROOFS_MEMORY or lower TAUTEN_MUTANT_JOBS
TAUTEN-PROOFS {"broken": [], "mutant": "warn-blocks", "status": "other"}
FAIL: 1 mutant(s) not calibrated: ['warn-blocks']
```

What an existing setup can notice (the CHANGELOG entry):

- The full gate calibrates more than one mutant at a time when its allowance
  is eight CPUs or more. Result lines for mutants may arrive in a different
  order; the set of lines and the record are the same.
- A proofs container that needed more than 12 GiB or 8 CPUs is now limited.
  The limit shows as slower builds or as a failed mutant, never as a pass.
- Running Compose directly on a host with fewer than 8 CPUs needs
  `TAUTEN_PROOFS_CPUS` set: Docker refuses a value above the daemon's CPU
  count (asking for 64 on this host: "range of CPUs is from 0.01 to 32.00, as
  there are only 32 CPUs available"). `proofs/run.sh` sets it;
  `proofs/refresh.sh` now goes through `run.sh` for that reason.
- Projects that already ran `init --proofs` keep their old files until they
  run it again.

## Proof obligations

None: no decision logic changes.

- **Kernel:** nothing is added or changed in `crates/tauten-check-kernel`.
  The two outcomes the gate can newly report already exist and are already
  proved not to admit a claim: mutant status `other` becomes
  `Seen::FailOther` and `timeout` becomes `Seen::Timeout`
  (`crates/tauten-check/src/proofs.rs:827-832`), and neither satisfies a
  control (`infrastructure_never_passes` and `admit_ok_iff` in
  `proofs/Tauten/Control.lean`, calibrated by `timeout-counts-as-kill` among
  others).
- **Theorems, mutants:** none new.
- **Tested and unproved:** `proofs/mutants.py` and the template's `gate.py`
  are the trusted shell steps `proofs/TRUST.md` already lists. This RFC
  changes three things in them: how many builds run at once, how a timeout is
  enforced, and that a cut-short build is reported as `other` (template:
  `error`) where it could have been reported as `killed`. The last one
  narrows when the script says `killed`, so it can only make the gate
  stricter, which is the property TRUST.md states for this script. Tests are
  under "Evidence and tests". TRUST.md gains one sentence saying so.

## Design

Ranked by effect on the reported problem. Each item states whether it is a
pure resource change. The full patch is in the appendix.

### 1. Limit the proofs container (refs #82)

- **Evidence:** rows 1 and 6; the swap figures; the three uncapped containers.
- **Change:** `cpus`, `mem_limit`, `memswap_limit` (equal to the memory
  limit, so no swap) and `pids_limit: 4096` on the `proofs` service of
  `compose.proofs.yaml` and of the template's `compose.yaml`.
  `proofs/run.sh` cuts the CPU default down to the daemon's count.
  `.github/workflows/proofs.yml` passes the runner's count.
- **Effect:** one gate can no longer take the host's memory or all its CPUs.
  Exceeding the memory limit becomes a kill inside the container, in
  seconds, where it used to be host-wide swapping.
- **Decision logic:** none. Configuration only.

### 2. One CPU budget: mutant jobs × Lean threads ≤ CPUs allowed (closes #82)

- **Evidence:** rows 2 and 3; `nproc` under a quota; the thread table.
- **Change:** the gate reads its allowance from what it already has, as #82
  asks: its affinity (`nproc`, `os.sched_getaffinity`) cut down to the
  cgroup's quota (`/sys/fs/cgroup/cpu.max`). From that it derives the two
  numbers:

  ```text
  threads = LEAN_NUM_THREADS, or CPUs ÷ jobs when only the jobs are set, or min(4, CPUs)
  jobs    = TAUTEN_MUTANT_JOBS, at most CPUs ÷ threads
  ```

  Both are at least 1 and their product is at most the CPUs allowed, so an
  explicit setting is honoured only up to the budget. A build that runs
  alone (the theorems, the extract-only check) gets all the CPUs. A shortage
  lowers the parallelism; the mutant set is never touched. `nproc / 2` and
  `os.cpu_count() // 2` are removed.
- **Effect:** the partial path on this host goes from 16 builds of about 170
  tasks each to 2 builds of 4 threads under the default allowance.
- **Decision logic:** none. Scheduling only; every mutant still runs.

### 3. The full gate uses the same budget (refs #82, #76)

- **Evidence:** row 3; a build gains 13% from four more CPUs, so CPUs beyond
  four per build are better spent on a second mutant.
- **Change:** the full path passes the same `jobs` and `threads` to
  `mutants.py` (it already supports `TAUTEN_MUTANT_JOBS`); the template uses
  its pool for both paths and keeps emitting results in manifest order.
- **Effect:** with 8 CPUs, two 4-thread builds side by side instead of one
  build with every thread it can create. No time is promised (#82): the
  qualification runs below record it. It should also offset what item 1
  costs a serial gate that used to have 32 CPUs to itself.
- **Decision logic:** none. Same mutants, same per-mutant verdict.

### 4. A cut-short build is never a calibration; a timeout kills the whole build (refs #82, #98)

- **Evidence:** row 5, the 400 MiB run and the stand-in; row 4 and the
  `sleep` test.
- **Change:** in both gates, a failed `lake build` whose exit status is not
  1, or whose output has `error: Lean exited with code N` with N other than
  1, is reported as `other` (template: `error`) with the remedy, before any
  attribution. `run()` starts each command in its own session and kills the
  process group on timeout.
- **Effect:** item 1's memory limit cannot turn into a kill verdict, which
  is #82's "shortage cannot skip a mutant". A timed-out build stops using
  CPUs. For #98, a result kept from an interrupted run is safe to reuse only
  if a cut-short build was never recorded as a kill.
- **Decision logic:** not in the kernel. It changes a fact the trusted
  script reports; see "Proof obligations".

### 5. The gate prints what it used (refs #82)

- **Evidence:** #82's pickup trigger asks for the allocation, the effective
  outer jobs and inner threads, CPU and wall time of a run. Today none of
  them is printed.
- **Change:** the budget line at the start and, in `proofs/gate.sh`, a
  closing line from the container's cgroup (`cpu.stat`, `memory.peak`,
  `pids.peak`). Progress output only: not a `TAUTEN-PROOFS` result line.
- **Decision logic:** none.

### Not changed, with the measurement that says why

- **Cargo profiles, debug info, linker.** The whole workspace builds in 21 s
  at eight jobs and tests in 25 s within 1.1 GiB. `Cargo.toml` is also part
  of every claim's fingerprint (`proofs.rs:905`), so a profile edit would
  cost a re-record to save seconds.
- **The commit gate** (`compose.yaml`, `scripts/gate`). It is already
  limited to 8 CPUs, 8 GiB and 1024 tasks. Its two fan-out steps (4
  `cargo-mutants` jobs and 4 benchmark workers, each with 8 build jobs) do
  oversubscribe inside that allowance, which costs throttling and cannot
  reach the host.
- **A committed `.cargo/config.toml` with `jobs`.** It would bound each
  host-side build, but the build is too small to matter and it would also
  apply to every contributor. Left to the owner's shell (below).
- **Selective Lean rebuilds (#76)** and **keeping results of an interrupted
  run (#98).** Separate designs. This RFC gives #76 one more number (each
  mutant costs about 140 s of CPU because every theorem file is rebuilt) and
  gives #98 the cut-short rule it needs.
- **`check --package` (#87).** It is about adopters' large workspaces;
  tauten's own Clippy pass is part of the 25 s above.

## Compatibility and reversibility

- **The record.** Every file under `proofs/` is part of each claim's
  fingerprint (`proofs.rs:912-940`), so editing `gate.sh`, `mutants.py`,
  `run.sh` or `refresh.sh` makes the committed `claims.json` stale until the
  gate runs again. The mutant definitions and the `[gate]` table, which the
  ship level binds (`proofs.rs:110-145`), are not edited, so recorded kills
  should stay reusable; the first run after the change will show it. Land
  the script edits in one commit and re-record once.
  `compose.proofs.yaml` and the workflow are outside the fingerprint.
- **Older and newer.** No file format or protocol changes, so any
  `cargo-tauten` runs either version of the scripts. A project with the old
  template keeps today's behaviour.
- **cgroup v1 or no cgroup.** `/sys/fs/cgroup/cpu.max` is absent, and the
  budget falls back to the affinity count, which is today's input. The gate
  may then plan more builds than its quota covers; the container limit still
  holds.
- **Reverting** is deleting the added lines; nothing is migrated.

## Alternatives

- **Do nothing; fix the host only** (`.wslconfig`). It helps this one
  machine and leaves every adopter with a gate that sizes itself to the host.
  By the arithmetic above, the partial path alone can need about as much
  memory as this VM has.
- **Limit the container and nothing else.** The gate would still start 16
  builds, now inside 8 CPUs and 12 GiB, where they would probably be killed
  for memory.
- **A cpuset instead of a quota.** `nproc` would then report the right
  count without any code. But a cpuset pins a gate to fixed cores, so two
  gates collide on the same cores unless something assigns them, and whether
  Lean's default thread count follows a cpuset was not measured.
- **A host-wide lock** so that only one gate runs at a time. It bounds the
  total, which per-container limits do not, but it is a new cross-project
  convention and makes agents wait on each other silently. Worth revisiting
  if several limited gates at once are still too much.
- **A `--jobs` flag on `cargo tauten proofs`.** The gate command is the
  project's own (`[gate] command`); an environment variable reaches it
  without a new flag, and #82 asks for no public knob before the allowance
  already available has been used.
- **Reuse `TAUTEN_CPUS` and `TAUTEN_MEMORY`** from `compose.yaml`. One fewer
  pair of names, but CI sets those to 2 CPUs and 6 GiB for the commit gate,
  which is too little for Lean.

## Evidence and tests

- **Budget rule.** A table test of the template's `budget()` (a pure
  function), including one CPU, a quota below the affinity count, settings
  above the budget and malformed settings. In this investigation the shell
  and Python versions agreed on 3,584 combinations, with jobs × threads ≤
  CPUs in all of them.
- **Cut-short rule.** A table test of `cut_short()` in both gates with the
  two outputs recorded above (a real proof failure; exit 137), and the mixed
  case: expected errors followed by `Lean exited with code 137` must not be
  `killed`.
- **Timeout.** A test that a command which starts a child and sleeps leaves
  no process after `run()` times out.
- **Qualification, which closes #82.** Two full runs of tauten's own gate at
  the same commit and a recorded allowance: one with `TAUTEN_MUTANT_JOBS=1`,
  one with the default budget. Their records must be equal under the #83
  projection. On a fixture project, a serial and a budgeted run must also
  report a mutant that is not killed in the same way (#82's unsuccessful
  mutant). Record the budget line and the closing line of each run. Repeat
  one partial run with `TAUTEN_PROOFS_CPUS=1`.
- **Already run for this draft,** with the patch applied to copies of the
  scripts: `gate.sh --extract-only` in a 4-CPU container (budget "4 CPU(s);
  mutants: 1 job(s) x 4 Lean thread(s)", both crates extracted, 71 tasks and
  1243 MiB at peak), and `gate.sh --only-mutants
  warn-blocks,no-level-is-error` in an 8-CPU container (every stage `ok`,
  both mutants `killed` with the theorems their claims name, two at a time,
  134 s in all; the lines under "Behaviour"). The stand-in experiment: the
  same real build of `warn-blocks` with `error: Lean exited with code 137`
  appended is `killed` under today's `mutants.py` and `other` under the
  patched one.
- **Gates.** The commit gate, then the full proofs gate on the commit that
  lands the script edits.

## Rollout

1. **tauten's own gate.** Items 1 to 5 for `compose.proofs.yaml`,
   `proofs/run.sh`, `refresh.sh`, `gate.sh`, `mutants.py` and
   `.github/workflows/proofs.yml`, in one commit. Run the qualification pair
   and commit the record. Safe alone: the template is untouched.
2. **The template.** The same for `templates/proofs/` (`compose.yaml`,
   `run.sh`, `gate.py`), its README, and the copies under `fixtures/` and
   `bench/hosts/ledger/proofs`. CHANGELOG entry. Ships in the next release.
3. **Documentation.** `docs/proofs/README.md` (the settings table, the
   thread measurements, new timings from step 1), one sentence in
   `proofs/TRUST.md`, `docs/STATUS.md`.

Host settings are the owner's and are not part of the rollout (next section).

## Host advice (owner only; not a repository change)

The host has no `%UserProfile%\.wslconfig`, so WSL uses its defaults: every
logical processor, half the RAM, and swap of a quarter of that. Docker
Desktop runs in the same VM and reports the VM's 32 CPUs and 30 GiB as its
own, so a container without limits is limited only by the VM.

```ini
[wsl2]
processors=24
memory=30GB
swap=8GB
```

- `processors=24` is the one that matters: with every logical processor
  given to the VM, a saturated VM leaves Windows nothing to run the terminal,
  the WSL service and Docker Desktop's backend with.
- `memory` and `swap` restate today's values so they stop depending on a
  default. Lower `swap` (for example `4GB`) if the VM still stalls: less swap
  means an out-of-memory kill sooner and less time swapping.
- Apply with `wsl --shutdown`, which stops every distribution and container.
- Long-lived Lean containers started outside Compose have no limit either;
  `docker update --cpus 8 --memory 12g --memory-swap 12g <name>` limits one
  without restarting it.
- For builds run directly on the host, `CARGO_BUILD_JOBS=8` in the shell
  profile bounds each one. A transient cgroup also works on this host; the
  test measurement above ran as `systemd-run --user --wait --collect --pipe
  -p CPUQuota=800% -p MemoryMax=12G -p MemorySwapMax=0 …`.
- A limit per container does not bound several gates at once. With the
  defaults, three gates side by side may use 24 CPUs.

## Unresolved questions

- **Owner:** are 8 CPUs and 12 GiB the right defaults? Lower defaults are
  safer on small machines; higher ones make an idle 32-CPU host faster.
- **Owner:** should the full gate run mutants side by side by default
  (item 3), or only when `TAUTEN_MUTANT_JOBS` is set, until the qualification
  pair has shown equal records?
- **Owner:** the names `TAUTEN_PROOFS_CPUS` and `TAUTEN_PROOFS_MEMORY`.
- **Owner:** #82 is in `later`. Its pickup trigger asked for a pinned run
  showing oversubscription under a known allowance; the thread table is that
  for the native gate's build, not for a whole run. Is that enough to pick it
  up now?
- Charon's and Aeneas's memory use and the cost of several extractions at
  once were not measured. The qualification run's closing line will show
  them.

## Decision log

- 2026-11-28: drafted from measurements on the owner's host. The patch in
  the appendix was applied to copies of the scripts and exercised as listed
  under "Evidence and tests"; nothing is implemented in the repository and
  there is no owner decision.
- 2026-11-28: accepted by the owner, who asked for the proposal to be landed
  before milestone work. The unresolved questions take the RFC's defaults
  (8 CPUs, 12 GiB, the full gate runs mutants side by side within the
  budget, the names as written) until the owner revisits them. The appendix
  patch is applied as is; #82 stays open until the qualification pair
  (`TAUTEN_MUTANT_JOBS=1` and the default budget at one commit) shows equal
  records.

## Appendix: proposed patch

Not applied. It was built against `bac9fdd`, where `patch -p1 --dry-run`
accepts it. The implementer owns the final form.

```diff
--- a/.github/workflows/proofs.yml
+++ b/.github/workflows/proofs.yml
@@ -73,4 +73,6 @@
           docker logout ghcr.io
           df -h /
       - name: Proofs gate (extract, drift, lake build, axiom audit, mutants)
-        run: time docker compose -f compose.proofs.yaml run --rm proofs
+        run: |
+          export TAUTEN_PROOFS_CPUS="$(nproc)"
+          time docker compose -f compose.proofs.yaml run --rm proofs
--- a/compose.proofs.yaml
+++ b/compose.proofs.yaml
@@ -15,6 +15,21 @@
     platform: linux/amd64
     pull_policy: missing
     init: true
+    # Bounded: the gate sizes its own concurrency from this allocation
+    # (mutant jobs x Lean threads <= CPUs), so it never depends on how many
+    # CPUs the host has. No swap: a build that outgrows the memory limit is
+    # killed and reported as a failure, where swapping would stall the whole
+    # host. Docker refuses `cpus` above the daemon's CPU count; proofs/run.sh
+    # cuts the default down to it.
+    cpus: ${TAUTEN_PROOFS_CPUS:-8}
+    mem_limit: ${TAUTEN_PROOFS_MEMORY:-12g}
+    memswap_limit: ${TAUTEN_PROOFS_MEMORY:-12g}
+    pids_limit: 4096
+    # Passed through only when set on the host; both are honoured up to
+    # the CPU allocation.
+    environment:
+      - TAUTEN_MUTANT_JOBS
+      - LEAN_NUM_THREADS
     # Network stays on: in a fresh container Charon builds its full-MIR
     # sysroot (`miri setup`), which fetches crates from crates.io. The
     # kernel itself has no dependencies; the instance crate's macros need
--- a/crates/cargo-tauten/templates/proofs/compose.yaml
+++ b/crates/cargo-tauten/templates/proofs/compose.yaml
@@ -9,6 +9,21 @@
     platform: linux/amd64
     pull_policy: never
     init: true
+    # Bounded: the gate sizes its own concurrency from this allocation
+    # (mutant jobs x Lean threads <= CPUs), so it never depends on how many
+    # CPUs the host has. No swap: a build that outgrows the memory limit is
+    # killed and reported as a failure, where swapping would stall the whole
+    # host. Docker refuses `cpus` above the daemon's CPU count; proofs/run.sh
+    # cuts the default down to it.
+    cpus: ${TAUTEN_PROOFS_CPUS:-8}
+    mem_limit: ${TAUTEN_PROOFS_MEMORY:-12g}
+    memswap_limit: ${TAUTEN_PROOFS_MEMORY:-12g}
+    pids_limit: 4096
+    # Passed through only when set on the host; both are honoured up to
+    # the CPU allocation.
+    environment:
+      - TAUTEN_MUTANT_JOBS
+      - LEAN_NUM_THREADS
     # Network stays on: in a fresh container Charon builds its full-MIR
     # sysroot (`miri setup`), which fetches crates from crates.io.
     volumes:
--- a/crates/cargo-tauten/templates/proofs/gate.py
+++ b/crates/cargo-tauten/templates/proofs/gate.py
@@ -35,6 +35,7 @@
 import os
 import re
 import shutil
+import signal
 import subprocess
 import sys
 import tarfile
@@ -58,8 +59,62 @@
 
 
 def run(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
-    return subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
-                          text=True, timeout=3600, env={**os.environ, **(env or {})})
+    # Its own session: a timeout kills the command and everything it
+    # started (lake's lean processes, cargo's rustc), not the child alone.
+    with subprocess.Popen(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
+                          text=True, env={**os.environ, **(env or {})},
+                          start_new_session=True) as process:
+        try:
+            output, _ = process.communicate(timeout=3600)
+        except subprocess.TimeoutExpired:
+            os.killpg(process.pid, signal.SIGKILL)
+            process.wait()
+            raise
+    return subprocess.CompletedProcess(command, process.returncode, output)
+
+
+def allocated_cpus() -> int:
+    """The CPUs this process may use: its affinity, cut down to the cgroup's
+    CPU quota when there is one. `os.cpu_count()` counts every host CPU
+    under a quota (`docker run --cpus`, compose `cpus`)."""
+    cpus = len(os.sched_getaffinity(0))
+    try:
+        quota, period = Path("/sys/fs/cgroup/cpu.max").read_text().split()
+        if quota != "max":
+            cpus = min(cpus, -(-int(quota) // int(period)))
+    except (OSError, ValueError):
+        pass
+    return max(1, cpus)
+
+
+def budget(cpus: int, jobs: str | None, threads: str | None) -> tuple[int, int, int]:
+    """(Lean threads of a build that runs alone, mutant jobs, Lean threads
+    per mutant build), with jobs x threads <= cpus (#82). The settings
+    (TAUTEN_MUTANT_JOBS, LEAN_NUM_THREADS) are honoured up to the budget; a
+    shortage lowers the parallelism, never the mutant set. One build gains
+    little beyond four threads, so the rest of the budget runs mutants side
+    by side."""
+    def whole(text: str | None) -> int | None:
+        return int(text) if text and re.fullmatch(r"[1-9][0-9]*", text) else None
+    wanted_jobs, wanted_threads = whole(jobs), whole(threads)
+    alone = min(wanted_threads or cpus, cpus)
+    if wanted_threads is None:
+        wanted_threads = min(4, cpus) if wanted_jobs is None else cpus // min(wanted_jobs, cpus)
+    wanted_threads = min(wanted_threads, cpus)
+    most = cpus // wanted_threads
+    return alone, (most if wanted_jobs is None else min(wanted_jobs, most)), wanted_threads
+
+
+def plan() -> tuple[int, int]:
+    """Size this run to its allocation: every `lake build` from here on
+    runs alone with the whole budget. Returns (mutant jobs, Lean threads per
+    mutant build) for the mutant stage."""
+    cpus = allocated_cpus()
+    alone, jobs, threads = budget(cpus, os.environ.get("TAUTEN_MUTANT_JOBS"),
+                                  os.environ.get("LEAN_NUM_THREADS"))
+    os.environ["LEAN_NUM_THREADS"] = str(alone)
+    log(f"budget: {cpus} CPU(s); mutants: {jobs} job(s) x {threads} Lean thread(s)")
+    return jobs, threads
 
 
 def claims() -> dict:
@@ -124,6 +179,7 @@
         emit(stage="tools", status="missing", detail=f"not on PATH: {', '.join(missing)}")
         return 3
     emit(stage="tools", status="ok")
+    plan()
     failed = 0
     for crate in crates:
         log(f"extract: {crate}")
@@ -199,6 +255,17 @@
 
 
 ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):(?P<line>\d+):(?P<col>\d+):", re.M)
+CUT_SHORT = re.compile(r"^error: Lean exited with code (?!1$)\d+$", re.M)
+
+
+def cut_short(returncode: int, output: str) -> bool:
+    """Whether a failed `lake build` ended early instead of failing its
+    proofs. Lean and lake exit with 1 when a proof fails; any other status
+    (137: killed, for one by the container's memory limit) means the errors
+    reported are not the whole set, so the build is not a calibration."""
+    return returncode != 1 or CUT_SHORT.search(output) is not None
+
+
 DECLARATION = re.compile(
     r"^(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def|abbrev|instance)\s+(?P<name>[^\s:(]+)",
     re.M)
@@ -304,6 +371,10 @@
         build = run(["lake", "build", "Proofs"], project)
         if build.returncode == 0:
             return {"status": "survived", "broken": []}
+        if cut_short(build.returncode, build.stdout):
+            print(build.stdout, file=sys.stderr)
+            return {"status": "error", "detail": f"the build was cut short (lake exited with "
+                    f"{build.returncode}); raise TAUTEN_PROOFS_MEMORY or lower TAUTEN_MUTANT_JOBS"}
         failures, elsewhere = failing(build.stdout, project)
         dependents = set(SORRY_DEPENDENT.findall(build.stdout))
         broken = sorted((failures | dependents) & theorems)
@@ -327,6 +398,7 @@
         emit(stage="tools", status="missing", detail=f"not on PATH: {', '.join(missing)}")
         return 3
     emit(stage="tools", status="ok")
+    jobs, threads = plan()
 
     log(f"extract: {', '.join(crates)}")
     fresh = Path(tempfile.mkdtemp(prefix="gate-extract-"))
@@ -388,13 +460,11 @@
         log(f"mutant {spec['name']}")
         return mutant(spec, crates, named)
 
-    if only is None:
-        results = [one(spec) for spec in specs]
-    else:
-        # `--only-mutants` (cargo tauten proofs try): in parallel.
-        from concurrent.futures import ThreadPoolExecutor
-        with ThreadPoolExecutor(max_workers=max(1, min(len(specs), (os.cpu_count() or 2) // 2))) as pool:
-            results = list(pool.map(one, specs))
+    # Every mutant runs, `jobs` at a time; results keep the manifest's order.
+    os.environ["LEAN_NUM_THREADS"] = str(threads)
+    from concurrent.futures import ThreadPoolExecutor
+    with ThreadPoolExecutor(max_workers=jobs) as pool:
+        results = list(pool.map(one, specs))
     for spec, result in zip(specs, results):
         if result["status"] != "killed":
             survivors += 1
--- a/crates/cargo-tauten/templates/proofs/run.sh
+++ b/crates/cargo-tauten/templates/proofs/run.sh
@@ -20,5 +20,13 @@
 docker compose version >/dev/null 2>&1 || missing "docker compose is not available"
 docker image inspect "$image" >/dev/null 2>&1 \
     || missing "the proofs image $image is not present; pull it first (about 20 GB)"
+# The container's CPU allowance (compose `cpus`). Docker refuses one above
+# the daemon's CPU count, so the default of 8 is cut down to it.
+if [ -z "${TAUTEN_PROOFS_CPUS:-}" ]; then
+    host_cpus="$(docker info --format '{{.NCPU}}' 2>/dev/null)"
+    case "$host_cpus" in '' | *[!0-9]* | 0*) host_cpus=1 ;; esac
+    TAUTEN_PROOFS_CPUS=$(( host_cpus < 8 ? host_cpus : 8 ))
+    export TAUTEN_PROOFS_CPUS
+fi
 PROOFS_IMAGE="$image" exec docker compose -f proofs/compose.yaml run --rm -T proofs \
     sh /repo/proofs/container.sh "$@"
--- a/proofs/gate.sh
+++ b/proofs/gate.sh
@@ -25,6 +25,43 @@
 done
 report tools ok
 
+# The CPU budget (#82). `nproc` is this process's affinity; under a CPU
+# quota (`docker run --cpus`, compose `cpus`) it still counts every host
+# CPU, so the cgroup's quota cuts it down. Mutant jobs x Lean threads stay
+# within the budget: TAUTEN_MUTANT_JOBS and LEAN_NUM_THREADS are honoured
+# up to it, and a shortage lowers the parallelism, never the mutant set.
+# One build gains little beyond four threads (docs/proofs/README.md), so
+# the rest of the budget runs mutants side by side.
+cpus="$(nproc)"
+if [ -r /sys/fs/cgroup/cpu.max ]; then
+    read -r quota period < /sys/fs/cgroup/cpu.max
+    if [ "$quota" != max ]; then
+        limit=$(( (quota + period - 1) / period ))
+        [ "$limit" -ge "$cpus" ] || cpus="$limit"
+    fi
+fi
+[ "$cpus" -ge 1 ] || cpus=1
+whole() { case "$1" in '' | *[!0-9]* | 0*) return 1 ;; esac; }
+jobs="${TAUTEN_MUTANT_JOBS:-}"
+threads="${LEAN_NUM_THREADS:-}"
+whole "$jobs" || jobs=""
+whole "$threads" || threads=""
+# A build that runs alone (the theorems, the extract-only check) takes the
+# whole budget, or LEAN_NUM_THREADS when that is lower.
+alone="${threads:-$cpus}"
+[ "$alone" -le "$cpus" ] || alone="$cpus"
+if [ -z "$threads" ] && [ -z "$jobs" ]; then
+    threads=$(( cpus < 4 ? cpus : 4 ))
+elif [ -z "$threads" ]; then
+    [ "$jobs" -le "$cpus" ] || jobs="$cpus"
+    threads=$(( cpus / jobs ))
+fi
+[ "$threads" -le "$cpus" ] || threads="$cpus"
+most=$(( cpus / threads ))
+if [ -z "$jobs" ] || [ "$jobs" -gt "$most" ]; then jobs="$most"; fi
+stamp "budget: $cpus CPU(s); mutants: $jobs job(s) x $threads Lean thread(s)"
+export LEAN_NUM_THREADS="$alone"
+
 stage=extract
 stamp "extract: charon $(charon version) / aeneas $(aeneas -version)"
 sh proofs/extract-all.sh /tmp/gate-extract
@@ -70,15 +107,19 @@
     # reuses the recorded kills.
     stamp "mutant calibration: none selected"
 elif [ "${1:-}" = "--only-mutants" ]; then
-    # `cargo tauten proofs try` and the ship level: only the named mutants,
-    # in parallel.
+    # `cargo tauten proofs try` and the ship level: only the named mutants.
     stamp "mutant calibration (partial: ${2:-})"
-    TAUTEN_MUTANT_JOBS="${TAUTEN_MUTANT_JOBS:-$(( $(nproc) / 2 > 0 ? $(nproc) / 2 : 1 ))}" \
+    TAUTEN_MUTANT_JOBS="$jobs" LEAN_NUM_THREADS="$threads" \
         python3 proofs/mutants.py $(printf '%s' "${2:-}" | tr ',' ' ')
 else
     stamp "mutant calibration"
-    python3 proofs/mutants.py
+    TAUTEN_MUTANT_JOBS="$jobs" LEAN_NUM_THREADS="$threads" python3 proofs/mutants.py
 fi
 report mutants ok
 
+# What the run used, from the container's cgroup (absent outside one).
+if [ -r /sys/fs/cgroup/cpu.stat ] && [ -r /sys/fs/cgroup/memory.peak ] && [ -r /sys/fs/cgroup/pids.peak ]; then
+    used="$(awk '$1 == "usage_usec" { print int($2 / 1000000) }' /sys/fs/cgroup/cpu.stat)"
+    stamp "used: ${used}s CPU, $(( $(cat /sys/fs/cgroup/memory.peak) / 1048576 )) MiB memory peak, $(cat /sys/fs/cgroup/pids.peak) tasks peak"
+fi
 stamp "proofs gate: OK"
--- a/proofs/mutants.py
+++ b/proofs/mutants.py
@@ -36,6 +36,7 @@
 import os
 import re
 import shutil
+import signal
 import subprocess
 import sys
 import tempfile
@@ -749,6 +750,17 @@
         logInfo m!"SORRY_DEPENDENT {entry.1}"
 """
 ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):(?P<line>\d+):(?P<col>\d+):", re.M)
+CUT_SHORT = re.compile(r"^error: Lean exited with code (?!1$)\d+$", re.M)
+
+
+def cut_short(returncode: int, output: str) -> bool:
+    """Whether a failed `lake build` ended early instead of failing its
+    proofs. Lean and lake exit with 1 when a proof fails; any other status
+    (137: killed, for one by the container's memory limit) means the errors
+    reported are not the whole set, so the build is not a calibration."""
+    return returncode != 1 or CUT_SHORT.search(output) is not None
+
+
 DECLARATION = re.compile(r"^(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def|abbrev|instance)\s+(?P<name>[^\s:(]+)", re.M)
 NAMESPACE = re.compile(r"^namespace\s+(?P<name>\S+)", re.M)
 
@@ -799,8 +811,18 @@
 
 
 def run(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
-    return subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
-                          text=True, timeout=1800, env={**os.environ, **(env or {})})
+    # Its own session: a timeout kills the command and everything it
+    # started (lake's lean processes, cargo's rustc), not the child alone.
+    with subprocess.Popen(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
+                          text=True, env={**os.environ, **(env or {})},
+                          start_new_session=True) as process:
+        try:
+            output, _ = process.communicate(timeout=1800)
+        except subprocess.TimeoutExpired:
+            os.killpg(process.pid, signal.SIGKILL)
+            process.wait()
+            raise
+    return subprocess.CompletedProcess(command, process.returncode, output)
 
 
 def mutate(source: Path, mutant: Mutant) -> None:
@@ -880,6 +902,12 @@
     build = run(["lake", "build"], project)
     if build.returncode == 0:
         raise SystemExit(f"FAIL {mutant.name}: the proofs still build against the mutated code")
+    if cut_short(build.returncode, build.stdout):
+        # Neither `killed` nor `survived` can be read off it.
+        print(build.stdout)
+        raise SystemExit(f"FAIL {mutant.name}: the build was cut short "
+                         f"(lake exited with {build.returncode}); raise TAUTEN_PROOFS_MEMORY "
+                         "or lower TAUTEN_MUTANT_JOBS")
     failures, unattributed = failing_theorems(build.stdout, project)
     if mutant.audit is not None:
         # Only the audit may fail, with the expected text, and no claim may break.
--- a/proofs/refresh.sh
+++ b/proofs/refresh.sh
@@ -8,7 +8,7 @@
 cd "$(dirname "$0")/.."
 fresh="$(mktemp)"
 trap 'rm -f "$fresh"' EXIT
-docker compose -f compose.proofs.yaml run --rm -T proofs \
+sh proofs/run.sh \
     sh /repo/proofs/container.sh sh -c \
     'sh proofs/extract-all.sh /tmp/fresh >&2 && tar -C /tmp/fresh -cf - .' \
     > "$fresh"
--- a/proofs/run.sh
+++ b/proofs/run.sh
@@ -16,4 +16,12 @@
 docker compose version >/dev/null 2>&1 || missing "docker compose is not available"
 docker image inspect "$image" >/dev/null 2>&1 \
     || missing "the proofs image $image is not present; pull it first (about 20 GB)"
+# The container's CPU allowance (compose `cpus`). Docker refuses one above
+# the daemon's CPU count, so the default of 8 is cut down to it.
+if [ -z "${TAUTEN_PROOFS_CPUS:-}" ]; then
+    host_cpus="$(docker info --format '{{.NCPU}}' 2>/dev/null)"
+    case "$host_cpus" in '' | *[!0-9]* | 0*) host_cpus=1 ;; esac
+    TAUTEN_PROOFS_CPUS=$(( host_cpus < 8 ? host_cpus : 8 ))
+    export TAUTEN_PROOFS_CPUS
+fi
 exec docker compose -f compose.proofs.yaml run --rm -T proofs "$@"
```
