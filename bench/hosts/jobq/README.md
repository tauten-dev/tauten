# jobq

A job queue simulator.

```console
$ jobq run jobs.txt --now 1000
build done
test pending
time: 1007
pending: 1
```

with `jobs.txt`:

```text
# comments and blank lines are ignored
submit build priority 5
submit test retry 100
start build
tick 7
finish build
```

Crates:

- `jobq-kernel`: job states, transitions, start order, time arithmetic (pure);
- `jobq-core`: script parsing, the scheduler and the job table (no direct
  IO; the clock is a `Clock` passed in, output any `Write`);
- `jobq-cli`: the `jobq` binary (clock choice, file reading, exit codes).

The full behaviour, including the ground rules every feature follows, is in
[SPEC.md](SPEC.md).

Run the tests with `cargo test`.
