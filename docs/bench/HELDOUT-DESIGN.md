# Held-out suite design for the tauten benchmark

Planner's design, 2026-11-18. Nothing here is implemented. Based on reading
`/home/tarek/workspace/tauten/bench` at `6b8fb8c` (README, PREREGISTRATION,
`run.py`, `analyze.py`, `admit.py`, host `ledger`, tasks T01–T04, arms, agent
script), `docs/PLAN.md` §5/§9, `docs/RULES.md`, and the pilot report and rows.

Difficulty figures and expected effects below are my guesses, not measurements.
The only measured inputs are the pilot's 36 rows.

---

## 1. Why the dev tasks hit the ceiling

Six causes, in the order I think they matter.

1. **The prompt hands over the oracle.** Every hidden edge case is spelled out
   in the prompt as a worked example or an exact error string:
   - T01: "Fees must work for every balance the ledger can hold" points
     straight at the `u64::MAX * 50` case.
   - T03: lists the bad amounts (`12.`, `.5`, `12.345`, `1e3`, `-1`) and says
     "overflow is an error".
   - T02 and T04: give the empty case with its exact output.

   The agent turns the prompt into tests and passes. The oracle measures
   reading comprehension of one page.
2. **The host is tiny and already written in profile style.** 226 lines, read
   in full in a few turns. `ledger-kernel` already uses `checked_add` /
   `checked_sub` and exhaustive matches, so the arm A agent copies the profile
   from the surrounding code. This shrinks any A-vs-C gap, and it is partly
   unavoidable: arm C needs a host that passes the check.
3. **One feature, one site.** About 150 added lines, one arithmetic or parse
   site, no interaction with existing state. Nothing is easy to forget.
4. **The sloppy solutions are strawmen for this model.** `pop().unwrap()`,
   `f64` money and a bare `*` on a balance are not mistakes Sonnet 5.5 makes
   when the prompt names the case. Admission proved the oracle catches the
   sloppy solution. It did not show that any agent writes it.
5. **The arithmetic the profile flagged was not reachable.** T01 and T03 in
   arms A and B each carry exactly one TAU-N001 error, yet pass the overflow
   oracle. So the flagged operation is somewhere the black-box input cannot
   drive to overflow (I have not seen the trial repos; a line counter or a
   bounded intermediate is likely). Until an oracle drives it, "6/12 left
   unchecked arithmetic" is a style count, not a defect count.
6. **Scale.** 4 tasks × 3 trials cannot show a difference smaller than about
   30 points even off the ceiling.

Two things the pilot could not rule out, which the held-out run must:

- **Oracle leakage.** The agent has `Read` and `Bash(cargo:*)` on the machine
  that holds `bench/tasks/*/oracle`, `reference/` and `sloppy/`. No transcript
  was kept (`--output-format json` records only the final message), so the
  pilot cannot show the agent stayed inside the trial repo.
- **`diff_added_lines` in arm C is ~1,570 against ~150 in A and B.** The pilot
  note blames `target/`; confirm that the fix covers whatever arm C writes.

---

## 2. Principles for held-out tasks

**Fair, not a gotcha.** Every hidden case must be decidable from text the
agent is given: the prompt plus the host's `SPEC.md`. Three mechanisms:

- Each host has a `SPEC.md` with a "Ground rules" section (see §3). Each
  prompt ends with one fixed sentence: "The ground rules in SPEC.md apply to
  everything you add." Edges are stated as rules, not as worked examples.
- Each oracle test carries a comment citing the sentence that fixes its
  expected output (`// SPEC G3; prompt ¶2`).
- **Determinacy review** before hashing: a reviewer who sees only the prompt
  and `SPEC.md` (no host code, no oracle outputs) is given each oracle input
  and must predict the expected output. Any case the reviewer cannot derive
  is rewritten or dropped. The reviewer may be a model, but not run through
  the benchmark agent script and not in any arm. This is the one place a
  model sees held-out inputs before the freeze; the owner can require a human
  instead.

**Harder by realism, not by obscurity.**

- Hosts of 800–1,200 lines, so the agent cannot hold everything in view.
- Features touch two or three crates and several arithmetic or parse sites.
- Each task has 6–10 requirements, of which 2–4 are edges.
- Where a rule is about plumbing (clock, filesystem, environment), the place
  that needs the value is two or three calls away from where it is available.

**Three strata, declared in `task.toml` before the run.**

| Stratum | Meaning | Count |
|---|---|---|
| **T** targeted | The sloppy solution's defect is flagged by a profile rule at deny level | 11 |
| **A** adjacent | A rule fires near the defect, but the lint-quiet fix can still be wrong | 4 |
| **N** control | No rule flags the sloppy solution | 5 |

The headline is over all 20 tasks. The mix is the designer's choice and it
sets the headline size, so per-stratum results are always printed beside it.

**Tagged oracle tests.** Test names carry a class prefix: `base__`,
`overflow__`, `input__`, `unicode__`, `rounding__`, `sequence__`,
`authority__`, `variant__`, `ioerr__`, `hang__`. Each test runs the binary
with a 10 s limit. This allows a per-class breakdown and a task-success
metric (all `base__` tests pass) separate from the escaped-defect metric.

**Evasive variant (T and A tasks).** Besides `reference/` and `sloppy/`, add
`evasive/`: the sloppy solution edited the cheapest way that silences the
lint without fixing behaviour (`wrapping_mul`, `unwrap_or_default`,
`checked_add` mapped to the wrong error). It must pass `cargo tauten check`
and still fail the oracle. This proves the oracle is not the lint in disguise.

---

## 3. Hosts

All hosts: three crates (kernel / core / edge), no external dependencies,
pinned toolchain, clean under `cargo tauten check` with the declared tiers,
their own `SPEC.md`, 15–25 visible tests. Capabilities are host-local traits,
not the `tauten` crate, so arm A does not get profile vocabulary for free.

### 3.1 `ledger` (kept unchanged, 226 lines)

Add only a `SPEC.md` (new file; dev tasks are unaffected). Ground rules:

- G1. Amounts are `u64`. Every operation gives the exact result or the
  `balance overflow` error. No entry fails because an intermediate is large.
- G2. Amounts are ASCII digits only. No sign, no spaces inside.
- G3. Input is UTF-8 text. Free text may contain any Unicode. Lengths are
  counted in Unicode scalar values unless a rule says bytes.
- G4. The program never crashes or hangs. Every failure is
  `error: line N: <reason>` or a usage error with status 2.
- G5. If output cannot be written, status 1 and `error: cannot write output`.

### 3.2 `kvconf` (new): layered configuration reader

`kvconf get|check|dump <file>` over a line format (`key = value`, sections,
comments).

| Crate | Tier | Contents |
|---|---|---|
| `kvconf-kernel` | kernel | `Value` enum (Str, Int, Bool, List), key paths, merge rules, `BTreeMap` store |
| `kvconf-core` | core | Parser, typed lookup, `trait Source` (read a file relative to the config root), `trait Env` (variable lookup); both passed in |
| `kvconf-cli` | edge | Argument parsing; builds `Source` rooted at the config file's directory and `Env` from `--env-file` or the process; `write_out` helper that returns `Result` |

Ground rules: integers are `i64`, exact or `value out of range`; digits only
with optional leading `-`; UTF-8 with any Unicode in values, keys ASCII per
the key grammar; files are found only through `Source`, variables only
through `Env`; never crashes or hangs; output-write failure as in ledger G5.

### 3.3 `jobq` (new): job queue simulator

`jobq run <script> [--now <ms>]` reads an event script (`submit`, `start`,
`finish`, `fail`, `tick <ms>`, `at <ms>`) and prints the final job table.

| Crate | Tier | Contents |
|---|---|---|
| `jobq-kernel` | kernel | `State` enum and transition function, priority order, backoff arithmetic. Has `is_terminal` / `is_runnable` helpers written with `matches!` |
| `jobq-core` | core | Script parser, scheduler, `trait Clock { fn now_ms(&self) -> u64 }` passed in |
| `jobq-cli` | edge | Builds the clock (`--now` gives a fixed clock, else system time); maps error kinds to exit codes |

Ground rules: times and durations are `u64` milliseconds, exact or a named
error, and time arithmetic saturates only where a rule says so; all time
comes from the `Clock`; the same script and `--now` always give the same
output; work is bounded by script size, never by the size of a number in it;
never crashes or hangs.

---

## 4. Held-out tasks (20)

"Difficulty" is my guess at the arm A escaped-defect rate for Sonnet 5.5:
low < 15%, medium 15–40%, high > 40%.

### ledger (5)

**H01-interest** · T · TAU-N001 · medium
- Prompt: add `interest <bp>` (basis points, 0 to 100000). Adds
  balance × bp / 10000, rounded half to even. Bad values give `bad rate`.
- Oracle: `base__` simple cases; `rounding__` ties (25 at 200 bp adds 0, 75
  at 200 bp adds 2); `overflow__` balance × bp exceeds `u64` while the result
  fits; result exceeds `u64` gives `balance overflow`; `input__` `100001`,
  `-1`, `1.5`, `+5`.
- Sloppy: `balance * bp / 10_000` with `+ 5_000` half-up.
- Note: the overflow is rule-flagged; the tie rule is not. Tags separate them.

**H02-statement** · N · no rule · medium
- Prompt: `--statement` prints one row per entry with a running balance.
  Entries may carry a memo after `;`. Memos longer than 16 characters are cut
  to 15 plus `…`. Columns are padded to fixed widths.
- Oracle: `unicode__` 16 CJK characters (48 bytes) is not cut; 17 emoji is
  cut to 15 plus `…`; combining marks count per scalar; `base__` ASCII;
  `input__` empty memo, memo of only spaces, `u64::MAX` amount width.
- Sloppy: tests `memo.len() > 16` (bytes), then cuts with `chars().take(15)`.
  Never panics; nothing for a rule to flag.

**H03-holds** · N · no rule · high
- Prompt: `hold <id> <amount>`, `capture <id> [amount]`, `release <id>`.
  Available = balance − held. Withdrawals may not go below held funds. A
  partial capture releases the remainder. Ids are reusable after release or
  capture. Named errors for unknown id, duplicate id, capture above the hold.
- Oracle: `sequence__` hold, withdraw to the limit, release, withdraw;
  capture twice; partial capture then reuse of the id; hold above available;
  holds summing above `u64` while each is valid; ten interleaved holds.
- Sloppy: tracks total held in one counter and forgets to reduce it on
  partial capture; allows id reuse while live.

**H04-csv** · T · TAU-P003 · medium
- Prompt: `--csv` reads `kind,amount,memo` rows. Memos may be quoted and
  contain commas and doubled quotes. Accepts CRLF and a leading byte-order
  mark. Wrong field count gives `bad row`.
- Oracle: `input__` one-field row, empty line, trailing comma, unterminated
  quote, quote in the middle of a field, BOM, CRLF, final row without
  newline; `base__` plain rows.
- Sloppy: `line.split(',')` then `fields[1]` and `fields[2]` (panics on short
  rows); the BOM makes the first kind unknown.
- Note: the index panic is rule-flagged; BOM and quoting are not.

**H05-summary** · A · TAU-N001 fires, the easy fix is wrong · medium-high
- Prompt: `--summary` prints the count, smallest, largest and mean deposit
  (rounded half up). "The summary never fails for a ledger that is itself
  valid." No deposits gives `none`.
- Oracle: `overflow__` deposit `u64::MAX`, withdraw it, deposit `u64::MAX`
  again: the ledger is valid, the deposit total exceeds `u64`, and the mean
  must print exactly; `rounding__` mean ending in .5; `base__`; empty.
- Sloppy: `total += amount` in `u64` (panics in a debug build).
- Evasive: `checked_add` mapped to `balance overflow`. Lint-clean, and wrong
  by the prompt. The correct fix is a wider accumulator.
- Purpose: turns the pilot's secondary finding into a behavioural test, and
  asks whether a lint leads to the right repair or only to a quiet one.

### kvconf (7)

**H06-units** · T · TAU-N001 · medium-high
- Prompt: typed lookups `get --bytes` and `get --millis` accept `10KiB`,
  `5MB`, `1h30m`, `250ms`. The unit table is given. Results are exact `u64`
  or `value out of range`.
- Oracle: `overflow__` `16EiB`, `18446744073709551615B` (valid),
  `17179869184GiB`, `1h` + huge `ms` sum; `input__` `+5MB`, `5 MB`, `5mb`
  (per the table), `MB`, `1h1h`, empty, `1.5h`; `base__`.
- Sloppy: `n * multiplier` and a running `+`; `parse::<u64>` accepts `+5`.

**H07-include** · T · TAU-A001 · medium-high
- Prompt: `include "<path>"` reads another file relative to the including
  file's directory, through `Source`. Cycles give `include cycle`; depth
  above 8 gives `include too deep`; paths that leave the config root are
  rejected; a missing file is a line error.
- Oracle: `authority__` binary run from a different working directory with a
  nested include in a subdirectory; `../outside.conf`; absolute path;
  `sequence__` a→b→a, self-include, depth 9; `base__`.
- Sloppy: `std::fs::read_to_string(path)` in `kvconf-core` (relative to the
  process directory, no confinement) and no cycle check.

**H08-interpolate** · T · TAU-A001, TAU-P003 · medium
- Prompt: `${NAME}` and `${NAME:-default}` in values, looked up through
  `Env`. `$$` is a literal dollar. Unknown names and malformed references are
  line errors. Substituted text is not expanded again.
- Oracle: `authority__` `--env-file` sets `X=1` while the process has `X=2`
  (must print 1); a name present only in the process with `--env-file` given
  is unknown; `input__` `${`, `${}`, `${A:-`, `$`, `${é}`, a value ending in
  `$`; `unicode__` multi-byte text around a reference; a value containing
  `${B}` is not re-expanded.
- Sloppy: `std::env::var` in core; byte offsets with `&s[i + 2..end]`.

**H09-schema** · N · no rule · medium
- Prompt: `check --schema <file>` with `key: int[min..max]`,
  `key: enum(a|b)`, `key: list<int>[1..4]`, `required`. Reports every
  violation, sorted by key, then exits 1.
- Oracle: `sequence__` three violations reported in key order;
  `input__` range `i64::MIN..i64::MAX`, reversed range is a schema error,
  empty enum, unknown type; a key that is both missing and required;
  duplicate schema keys; `base__`.
- Sloppy: stops at the first violation; reports in file order.

**H10-keys** · N · no rule · medium
- Prompt: `set <key> <value>` validates keys against the grammar in SPEC
  (ASCII lower-case letters, digits, `_`, dot-separated, 64 bytes at most).
  Lookups are ASCII case-insensitive.
- Oracle: `unicode__` `é`, `ß`, Arabic-Indic digits, `İ`, non-breaking space
  inside a key; `input__` `a..b`, `.a`, `a.`, 65 bytes, empty, leading digit;
  `base__`.
- Sloppy: `c.is_alphanumeric()` and `to_lowercase()`; both accept or alter
  non-ASCII text. No panic and no rule.

**H11-json-then-duration** · T · TAU-M001 · two steps · low-medium
- Step 1 prompt: `dump --format json` with correct string escaping.
- Step 2 prompt (fresh session, same repo): add a `Duration` variant to
  `Value`; `dump` prints it as integer milliseconds in JSON and in canonical
  form in the default format.
- Oracle (after step 2): `variant__` durations in JSON at top level and
  inside lists; `input__` escaping of quotes, backslash, control characters,
  non-ASCII; `base__`.
- Sloppy: step 1 renders with `other => format!("\"{other}\"")`, so step 2's
  variant comes out as a string with no compile error.
- Purpose: the only honest test of TAU-M001's claim, which is about variants
  added later. In a single session rustc already forces every exhaustive
  site in all arms.

**H12-write-failure** · A · TAU-E001/E003, TAU-O001 · low-medium
- Prompt: `dump --out <path>` (`-` is stdout) and `set --in-place`. In-place
  writes are all-or-nothing. SPEC's output-failure rule applies.
- Oracle: `ioerr__` stdout redirected to `/dev/full`; `--out` under a path
  whose parent is a regular file; in-place target is a directory; after each
  failure the original file is byte-identical; `base__` comments and blank
  lines preserved.
- Sloppy: `println!` (panics on a failed write) or `let _ = writeln!(…)`
  (status 0 after a failed write); truncates before writing.
- Note: the rules reach this only when rendering sits in core or the result
  is dropped with `let _`. `println!` in the edge is allowed, so the outcome
  depends on where the agent puts the code. Uses failures that still occur
  for root.

### jobq (8)

**H13-backoff** · T · TAU-N001 · medium
- Prompt: `retry base <ms> cap <ms>`; the delay before attempt n is
  min(cap, base × 2^(n−1)). Attempts are unbounded.
- Oracle: `overflow__` attempt 64, 65 and 4,000,000,000; base `u64::MAX`;
  base 0; cap below base; `base__` first five attempts.
- Sloppy: `base * 2u64.pow(n - 1)` or `base << (n - 1)`.

**H14-timeout** · T · TAU-A001, TAU-N001 · medium-high
- Prompt: `submit … timeout <ms>`. A running job whose elapsed time reaches
  its timeout fails with `timed out`. If the clock reads earlier than the
  start, elapsed is 0. A deadline beyond `u64` never expires.
- Oracle: `authority__` two runs with different `--now` give the stated,
  different results; `overflow__` start near `u64::MAX` plus timeout;
  `sequence__` `at` moving time backwards; timeout 0; `base__`.
- Sloppy: `SystemTime::now()` inside core because the clock is two calls up;
  `now - started`.

**H15-ratelimit** · A · TAU-N003 fires on the float route only · high
- Prompt: `limit <n> per <ms> burst <b>`; starts consume a token; tokens
  refill continuously and exactly. "Splitting a wait into smaller ticks never
  changes the outcome."
- Oracle: `sequence__` 1,000 ticks of 1 ms against one tick of 1,000 ms with
  3 per 1,000; `rounding__` 1 per 3 ms over 10 ms; `overflow__` elapsed × n
  beyond `u64`; `input__` per 0, burst 0; `base__`.
- Sloppy: float token count (flagged in the kernel), or integer
  `elapsed * n / per` that drops the remainder each tick (not flagged).

**H16-cancel** · N · no rule (limit of TAU-M001) · high
- Prompt: `cancel <id>` and a `Cancelled` state. Cancelled is terminal: no
  retry, not pending, cannot be started; jobs waiting on it fail with
  `dependency cancelled`; cancelling a terminal job is an error.
- Oracle: `variant__` cancel then `start`; cancel then `retry`; the pending
  count; a dependant of a cancelled job; exit code for the new error kind;
  `sequence__` cancel twice, cancel after finish; `base__`.
- Sloppy: adds the variant and fixes what rustc reports. `is_terminal` and
  `is_runnable` use `matches!`, which neither rustc nor clippy reports, so
  cancelled jobs can still start.
- Purpose: shows where enforcement stops.

**H17-stats-then-pause** · T · TAU-M001 · two steps · low-medium
- Step 1 prompt: `--stats` prints job counts per state and event counts per
  kind.
- Step 2 prompt (fresh session): `pause <id>` / `resume <id>` and a `Paused`
  state; paused jobs are not started and keep their queue position.
- Oracle: `variant__` the paused count appears in `--stats`; totals equal
  the job count; `sequence__` pause, tick, resume ordering; pause of a
  running job is an error; `base__`.
- Sloppy: step 1 counts with `_ => other += 1` or leaves the state out.

**H18-aging** · T · TAU-N002, TAU-N001 · medium-high
- Prompt: priority is 0–255. Effective priority = priority + waited / age
  (`age <ms>` in the script), capped at 255. Order is effective priority
  descending, then submit time, then id.
- Oracle: `overflow__` priority 250 after six age periods (255, not 0);
  waited / age above 2^32; `input__` age 0 is an error, priority 256;
  `sequence__` three-way ties; `base__`.
- Sloppy: `(prio as u64 + waited / age) as u8` (wraps); divides by age 0.

**H19-quoted-names** · T · TAU-P003 · medium
- Prompt: job names may be quoted, with `\"`, `\\`, `\n` escapes; 1–32
  characters; no control characters. Lines above 4,096 bytes give
  `line too long`.
- Oracle: `input__` unterminated quote, backslash as the last byte, unknown
  escape, empty name, tab inside quotes, CRLF, a 5,000-byte line;
  `unicode__` a 32-character CJK name (96 bytes) is accepted, 33 rejected;
  `base__`.
- Sloppy: byte scanner with `bytes[i + 1]` and `&line[a..b]`; counts bytes.

**H20-recurring** · A · TAU-N001 fires, the hang is unflagged · high
- Prompt: `every <ms>` jobs run on a fixed grid from their first start. After
  a clock jump a job runs once and continues on the grid. When the next run
  time is not representable the job becomes `Expired`.
- Oracle: `hang__` interval 1 with a jump of 10^15 ms must finish inside
  10 s; `overflow__` first start near `u64::MAX`; `sequence__` jump across
  3.5 intervals lands on the grid; `input__` interval 0; `base__`.
- Sloppy: `while next <= now { next += interval }`.
- Evasive: `checked_add` inside the same loop. Lint-clean and still hangs.

### Coverage

| Probe from the brief | Tasks |
|---|---|
| Overflow at type limits | H01, H05, H06, H13, H14, H18, H20 |
| Malformed, empty, Unicode input | H02, H04, H08, H10, H19 |
| Rounding | H01, H05, H15 |
| Errors instead of panics | H04, H08, H12, H19 |
| Invariants across sequences | H03, H09, H15, H16, H20 |
| Injected clock, files, environment | H07, H08, H14 |
| Wildcard swallows a new variant | H11, H17 (flagged), H16 (not flagged) |
| Ignored results | H12 |
| Controls (no rule) | H02, H03, H09, H10, H16 |

---

## 5. Calibration without contaminating held-out

**Calibration split:** three tasks, one per host, `split = "calibration"`,
written in the same style and never part of any reported comparison.

| Id | Host | Feature | Classes |
|---|---|---|---|
| K01-discount | ledger | Tiered percentage discount entries | overflow, rounding, input |
| K02-lists | kvconf | List values and `key[i]` lookup | input, index panic, unicode |
| K03-depends | jobq | `after <id>` dependencies | sequence, cycle, hang |

**Procedure.**

1. Build the hosts and K01–K03. Run **arm A only**, 5 trials each, on both
   candidate models.
2. Target: arm A escaped-defect rate between 30% and 60% over the
   calibration tasks. If outside, change only *style knobs* that apply to
   every task: how much of the edge wording sits in `SPEC.md` rather than the
   prompt, requirements per task, host size, turn limit. At most three
   rounds; every round's rows are kept and published.
3. Fix the style knobs in a short `bench/TASK-STYLE.md`. Choose the primary
   model: the one whose calibration rate is nearer 40%. The other is the
   replication.
4. Only then write H01–H20 to that style. They are admitted with the built-in
   agents, pass the determinacy review, and are hashed. No benchmark agent
   runs on them before the freeze.
5. Plumbing for arms B and C on the new hosts is checked with the built-in
   agents plus the already-exposed dev task T01 in all arms inside the
   container. No model runs arm B or C on a calibration or held-out task
   before the freeze.

**Admission gate.** As the brief states it: reference passes 100% with 0
profile errors; null fails; sloppy fails the oracle. One thing must change in
`admit.py`, which today also demands a profile error from every sloppy
solution and would reject every control:

- T: sloppy must raise at least one rule listed in `expected_rules`.
- A: same, and `evasive/` must be lint-clean and fail the oracle.
- N: sloppy must raise **zero** profile errors, so a control is a real one.

**If held-out still lands at the ceiling or floor:** no retuning. The result
is published as inconclusive for that model, and the other model's result
stands on its own.

---

## 6. Statistical design

**What the pilot gives.** The primary outcome had zero variance, so the pilot
supplies no estimate of between-task spread. It does supply cost: mean
$0.175 per trial (range $0.147–$0.266), 44 s, 16–18 turns. Power therefore
rests on stated assumptions, simulated with the analysis the preregistration
names (bootstrap over tasks, 95% interval excludes 0). Script:
`/tmp/claude-1000/-home-tarek-workspace-sovel-sovel-handover/fa26b365-cd1b-428b-bad7-464890bc8476/scratchpad/power.py`.
Task baselines are drawn from a wide Beta distribution around the stated mean.

Power to detect the C − A difference (1,500 simulations per cell):

| Arm A rate | Effect by stratum (relative cut T / A / N) | Mean difference | 16 tasks × 5 | 20 × 5 | 20 × 8 | 20 × 10 |
|---|---|---|---|---|---|---|
| 40% | 75% / 40% / 0 | −20 points | 0.78 | 0.87 | 0.94 | 0.97 |
| 40% | 50% / 25% / 0 | −13 points | 0.47 | 0.57 | 0.69 | 0.80 |
| 40% | 30% / 10% / 0 | −7 points | 0.22 | 0.24 | 0.32 | 0.44 |
| 25% | 75% / 40% / 0 | −12 points | 0.54 | 0.61 | 0.75 | 0.85 |
| 12% | 75% / 40% / 0 | −6 points | 0.29 | 0.32 | 0.45 | 0.54 |
| any | none | 0 | 0.04 | 0.04 | 0.03 | 0.04 |

Reading:

- **Recommendation: 20 tasks × 3 arms × 10 trials = 600 trials per model.**
  A 20-point reduction is detected with about 97% power, a 13-point one with
  80%. The 95% interval half-width is about ±9 to ±11 points.
- The preregistered minimum (16 tasks × 5 trials) has 78% power for 20 points
  and under 50% for 13.
- With controls in the mix, a 20-point headline needs roughly a 75% cut on
  targeted tasks. That is a strong effect; the per-stratum table is where a
  moderate effect will show.
- If arm A's rate is near 12%, no design of this size is adequately powered.
  That is why calibration targets 30–60%.
- False-positive rate under no effect is 3–4%.

**Cost.** Larger hosts and two two-step tasks should raise the per-trial
cost; I assume 1.5–2× the pilot.

| Item | Trials | At pilot rate ($0.18, 45 s) | At 2× |
|---|---|---|---|
| Calibration, arm A, 2 models, up to 3 rounds | up to 90 | $16 | $32 |
| Primary model, held-out | 600 (+60 second steps) | $119 | $238 |
| Replication model, held-out | 600 (+60) | below the primary; price from its calibration trials | |
| Wall time per model, 4 parallel workers | | about 2 h | about 4 h |

A budget of about $400 covers both models with margin. If it must be cut,
drop to 8 trials before dropping tasks: tasks carry more power than trials.

**Second model.** Haiku 5.5 through the same agent script, same tasks, arms
and limits, reported separately and never pooled. It is cheaper and more
likely to sit off the ceiling. Which of the two is primary is fixed from
calibration data before the freeze (§5).

**Changes to `PREREGISTRATION.md` before freezing.**

1. Splits: dev (4), calibration (3, arm A only, never reported as a result),
   held-out (20, listed by id with host and stratum).
2. Design row: 10 trials per task per arm; primary and replication models by
   exact id; agent script hash; Claude Code version; tauten version.
3. Primary outcome unchanged. For two-step tasks, one value after the last
   step. A timeout or agent crash is scored by the oracle on whatever the
   repo contains.
4. New secondary outcomes: task success (all `base__` tests pass); defects by
   class tag; C − B; per-stratum differences (T, A, N), labelled exploratory;
   arm C compliance (trials that end with profile errors); timeouts; tokens
   and cost.
5. Analysis: keep the bootstrap over tasks as primary. Add a sign-flip
   permutation test on per-task differences as a sensitivity check. One
   primary comparison (C − A on all held-out tasks), so no multiplicity
   correction; everything else is secondary.
6. Go/no-go: define "task success" as above so the "C lowers task success by
   more than 10 points" rule is computable; it is not measured today. Use
   cost in dollars, since tokens are not yet split out.
7. Exclusions: none after the freeze. A task found broken after the run is
   reported both with and without, with the reason.
8. Freeze record: `bench/heldout.lock` holds SHA-256 of every prompt, oracle,
   `task.toml`, host tree, support file, arm file, agent script, `run.py`,
   `analyze.py`, `TASK-STYLE.md` and the profile's `rules.toml`. The lock
   hash and the preregistration commit go into every result row.
9. Threats, stated up front:
   - the hosts are profile-clean, so this measures new work in an adopted
     repo, not adoption of a legacy repo with a baseline;
   - tasks and sloppy solutions were written by a model of the same family
     as the agent under test;
   - the stratum mix is a design choice;
   - debug builds turn overflow into panics.

**Harness changes.**

| # | Change | Why |
|---|---|---|
| 1 | Per-trial container: only the trial repo, toolchain, `cargo-tauten` and the model key mounted; no network except the model API; `bench/tasks` not visible | Closes the oracle leak path |
| 2 | Evaluate in a second container after the agent exits | The oracle runs agent-written code |
| 3 | `--output-format stream-json --verbose`, saved per trial; a post-run scan flags any path outside the repo | Transcripts are promised for publication; proves no leak |
| 4 | Same `--max-turns` (60) and wall limit (900 s per step) in every arm; record timeouts | Preregistered, not yet in the agent script |
| 5 | Randomised, seeded order: each trial index is a shuffled block of (task, arm); 4 workers | Today's loop runs task → arm → trial in sequence, which is not the interleaving the preregistration promises |
| 6 | Multi-host support: per-host oracle support file with stdin, working directory, environment, stdout redirection and a per-case time limit | Today's support file is ledger-only |
| 7 | Arm B text generated from each host's `bench.toml` | `arms/B.md` names ledger crates |
| 8 | Two-step tasks: `steps = ["step1.md", "step2.md"]`, a fresh session per step on the same repo | H11, H17 |
| 9 | Per-test oracle results by name (drop `--quiet`); task-success field | Class breakdown, go/no-go |
| 10 | `admit.py`: strata, `expected_rules`, `evasive/`, zero-error rule for controls | §5 |
| 11 | `run.py` refuses `--tasks heldout` unless the lock matches; rows carry model id, tool versions, lock and preregistration hashes, input/output/cache tokens | Freeze enforcement |
| 12 | `analyze.py`: strata, class tags, C − B, permutation test, cost | §6 |
| 13 | Confirm `diff_added_lines` in arm C after the `target/` fix | Pilot shows ~1,570 against ~150 |

---

## 7. Build checklist for the implementer

Estimates are agent-days of focused work. Order matters: nothing in steps
7–9 may start before the step above it is complete.

| # | Step | Estimate |
|---|---|---|
| 1 | Harness changes 4–9, 11–13; unit-check with built-in agents on dev tasks | 1.0 |
| 2 | Container images and runner (changes 1–3); canary: T01 in all arms inside the container, transcripts saved, leak scan clean | 0.5 |
| 3 | `admit.py` strata and evasive checks (change 10); `SPEC.md` for ledger | 0.25 |
| 4 | Host `kvconf` (code, SPEC, visible tests, profile-clean) | 0.75 |
| 5 | Host `jobq` (same) | 0.75 |
| 6 | Calibration tasks K01–K03 with reference, sloppy, oracle; admit | 0.5 |
| 7 | Calibration runs, arm A, both models, up to three rounds; write `TASK-STYLE.md`; pick the primary model | 0.5 + up to $32 |
| 8 | Write H01–H20: prompt, oracle with spec citations and class tags, reference, sloppy, evasive where required; admit all | 3.0 |
| 9 | Determinacy review of every oracle case; fix or drop; re-admit | 0.5 |
| 10 | Update `PREREGISTRATION.md` (§6 list), generate `heldout.lock`, commit both; owner sign-off on models and budget | 0.25 |
| 11 | Primary model run, 600 trials; then replication run | 0.5 + about $250–400 |
| 12 | Report: headline, strata, classes, cost, transcripts, every trial listed | 0.5 |
| | **Total** | **about 9 agent-days, $300–450** |

Stop points for the owner: after step 7 (is any model off the ceiling?) and
at step 10 (the freeze is irreversible for this suite).

## 8. Risks I could not resolve by reading

- Whether Sonnet 5.5 makes these mistakes at a useful rate is unknown until
  step 7. The suite may again sit near the ceiling for it; the replication
  model is the hedge.
- Several clippy behaviours are assumed, not checked: that `matches!` is not
  reported by `wildcard_enum_match_arm` (H16), that shifts and `pow` are
  reported by `arithmetic_side_effects` (H13), and that `.ok()` on a dropped
  `Result` is not reported (H12). Admission will settle each; a task whose
  stratum turns out wrong is relabelled before the freeze, not after.
- Two-step tasks double the cost of two tasks and add harness surface. If
  change 8 is cut, H11 and H17 have no single-session equivalent and
  TAU-M001 goes untested; say so in the report.
