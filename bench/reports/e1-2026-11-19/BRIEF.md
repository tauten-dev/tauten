# E1 finding-validity audit: classifier brief

You classify static-analysis findings ("deny-level profile findings" from cargo-tauten) found in
Rust repos that an AI agent wrote while doing a benchmark task. The question for each finding:
is the flagged expression an actual defect reachable from a legal input?

## Input
A batch file (JSON list). Each item: `fid`, `repo` (path to the final repo; READ-ONLY, never
modify it), `task`, `id` (rule), `file`, `line`, `column`, `message`.
Rules: TAU-N001 unchecked arithmetic (+ - * / % that can overflow/panic), TAU-P003 panicking
index/slice, TAU-N002 lossy `as` cast, TAU-E001 unwrap/expect, TAU-E003 panic!/unreachable!/todo!.

The legal inputs are defined by the repo's `SPEC.md` (top level) and the task prompt
(`<repo>/../prompt-0.md`). The binary is a CLI; its README/SPEC say how it reads input.
Hidden oracle test files (`tests/oracle.rs`, `tests/support/oracle_support.rs`) may be present:
ignore them, and do not use their expectations as the definition of legal input or of
correct output. Use only SPEC.md + the prompt.

## Classes (registered definitions; apply exactly)
- **confirmed**: a legal input (per SPEC.md and the prompt, expected output derivable from
  them) makes the binary deviate (wrong output, panic, or hang) in the dev or release
  profile, AND repairing only the flagged expression (e.g. checked_* with the SPEC's error
  behaviour, or a saturating op if that is what SPEC requires) removes the deviation. You
  must demonstrate it: copy the repo to a scratch dir (`cp -r <repo> $WORK/<fid>`), build
  with `cargo +1.98.0 build` and `cargo +1.98.0 build --release` (set
  CARGO_TARGET_DIR inside your scratch dir; no network needed, no dependencies), run the
  input, show the observed vs expected output, apply the minimal repair, show it fixed.
- **refuted**: a written bound argument shows no legal input reaches a failing value
  (e.g. "both operands are u8-range-validated at parse time on line N so the u32 sum
  cannot exceed 510"; "index i < v.len() by the loop condition"). The argument must cite
  the lines that establish the bound. If SPEC itself admits values that reach the failure,
  it is NOT refuted.
- **undetermined**: neither of the above could be established. Say why.

Wrapping/overflow in release is a deviation only if the produced output differs from what
SPEC requires. A panic on legal input is always a deviation. A panic on input SPEC declares
illegal is a deviation only if SPEC prescribes a different error behaviour for it (e.g. a
specific error message/exit code) — then it counts as wrong output on an input whose
expected output is derivable; treat such inputs as legal for this purpose.

Findings in the same repo often share code; reuse your analysis but classify each finding.

## Output
Write JSON to the output path you are given: a list of
`{"fid", "verdict": "confirmed"|"refuted"|"undetermined", "argument": "...", "input": "...or null",
"observed": "...or null", "expected": "...or null", "profile": "dev"|"release"|"both"|null,
"repair": "...or null", "repaired_output": "...or null"}`.
Keep `argument` concise but self-contained (cite file:line). One entry per fid in the batch.

## Independence
Another classifier audits the same findings. Do not look at any file under the e1 directory
other than BRIEF.md, your batch file and your own output path. Do not modify any repo in place.
