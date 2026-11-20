# jobq specification

`jobq run <script> [--now <ms>]` simulates a job queue: it reads an event
script, applies the events in order and prints the final job table.

## Time

Times and durations are whole milliseconds. A run starts at the time read
from the clock: `--now <ms>` gives a fixed clock reading `<ms>`; without it
the system clock (milliseconds since the Unix epoch) is used. After that,
time changes only through the script's `tick` and `at` events.

## Script

The script is UTF-8 text, read as lines (a line ends at `\n`; a `\r` before
it is ignored). Lines are numbered from 1. Leading and trailing whitespace on
a line is ignored. A line that is empty, or whose first non-space character
is `#`, is a comment: it is not an event, but it still counts for line
numbers. Every other line is one **event**: an event word and its arguments,
separated by whitespace.

| Event | Meaning |
|---|---|
| `submit <name> [priority <p>] [retry <ms>]` | add a pending job, submitted now |
| `start <name>` | start a job |
| `next` | start the first startable job in start order; nothing if none |
| `finish <name>` | a running job finishes: `done` |
| `fail <name>` | a running job fails |
| `tick <ms>` | time moves forward by `<ms>` |
| `at <ms>` | time is set to `<ms>` (earlier or later than now) |

- **Names.** A job name is 1 to 32 ASCII letters, digits, `_` or `-`. Names
  are unique within a script.
- **Submit options** may come in any order, each at most once. `priority` is
  0 to 255 (default 0). `retry <ms>` gives the job a retry delay.
- **Starting.** A job can start when it is `pending` and its ready time has
  come (ready time ≤ now). A submitted job is ready at once.
- **Failing.** A job without `retry` becomes `failed`. A job with `retry d`
  becomes `pending` again and is ready at now + d.
- **Start order** (used by `next`): higher priority first; equal priority,
  earlier submit time first; then names in byte order.

## Output

On success, standard output gets the job table, status 0:

- one line per job in submission order: `<name> <state>`, where state is
  `pending`, `running`, `done` or `failed`; a pending job whose ready time is
  later than the final time gets ` (ready at <t>)` appended;
- then `time: <t>`, the final time;
- then `pending: <n>`, the number of jobs that can still be started (neither
  running nor finished).

## Errors

The first invalid line stops the run. Nothing is printed on standard output;
standard error gets exactly one line `error: line N: <reason>`. The exit
status depends on the kind of the reason.

| Reason | Kind | Status |
|---|---|---|
| ``unknown event `<word>` `` | syntax | 3 |
| `missing job name`, `missing number` | syntax | 3 |
| ``bad name `<text>` ``, ``bad number `<text>` ``, ``bad priority `<text>` `` | syntax | 3 |
| ``unknown option `<word>` ``, ``missing value for `<word>` ``, ``duplicate option `<word>` `` | syntax | 3 |
| ``unexpected `<word>` `` (words after a complete event) | syntax | 3 |
| ``unknown job `<name>` ``, ``duplicate job `<name>` `` | state | 4 |
| ``job `<name>` cannot start``, ``job `<name>` is not running`` | state | 4 |
| `time overflow` | time | 5 |

An unreadable script gives `error: cannot read input: <detail>`, status 1.
A wrong command line (unknown command or option, missing script, a `--now`
that is not a number) prints `usage: jobq run <script> [--now <ms>]` on
standard error, status 2.

## Ground rules

These apply to every feature, present and future.

- **G1. Time arithmetic.** Times and durations are whole numbers of
  milliseconds from 0 to 18446744073709551615 (the largest `u64`). Every
  calculation gives the exact result. When a time the script asks for is
  above that limit the run stops with `time overflow`, unless a rule for that
  feature says otherwise. Nothing saturates, wraps or rounds unless its rule
  says so. No event fails because an intermediate value of a calculation is
  large: only the final result counts.
- **G2. Number syntax.** A number is one or more ASCII digits `0`–`9` and
  nothing else: no sign, no spaces, no decimal point, no unit. Leading zeros
  are allowed.
- **G3. One clock.** All time comes from the clock the run starts with and
  the script's `tick` and `at` events. Nothing reads the system time except
  the program's start-up, and only when `--now` is absent.
- **G4. Determinism.** The same script with the same `--now` always gives
  the same output and status.
- **G5. Bounded work.** The work a run does is bounded by the length of the
  script, never by the size of a number in it.
- **G6. Text.** Input is UTF-8. Where a rule allows free text, it may contain
  any Unicode; lengths are counted in Unicode scalar values (Rust `char`s),
  not bytes, unless the rule says bytes.
- **G7. No crashes.** The program never crashes, panics or hangs, whatever
  the input. Every failure is reported as described under Errors.
- **G8. Output failures.** If standard output cannot be written, the program
  prints `error: cannot write output` on standard error and exits with
  status 1.
- **G9. Complete counts and formats.** Every count and every output format
  covers every state, event kind or value type, including later ones; new
  ones are listed after the existing ones, in the order the feature's
  description introduces them.
