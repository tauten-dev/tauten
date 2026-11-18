# Pilot, 2026-11-18 (dev split)

**Model:** `claude-sonnet-5-5` through Claude Code (`bench/agents/claude-code.sh`),
same tools and limits in every arm. **Harness:** `c4de73b`. **Tasks:** the 4 dev
tasks (T01-fee, T02-stats, T03-cents, T04-undo) on the `ledger` host.
**Design:** 3 arms × 3 trials × 4 tasks = 36 trials. Total model cost $6.29.

This is the dev split. It was used to validate the harness and calibrate task
difficulty; it is not a held-out result and must not be presented as one.

## Result

| | A: plain | B: rules in AGENTS.md | C: enforced |
|---|---|---|---|
| Escaped defects (primary) | 0/12 | 0/12 | 0/12 |
| Trials left with unchecked arithmetic (TAU-N001) | 6/12 | 5/12 | 0/12 |
| Visible tests pass | 12/12 | 12/12 | 12/12 |
| Test tampering / stubs | 0 / 0 | 0 / 0 | 0 / 0 |
| Model cost | $1.95 | $2.14 | $2.20 |
| Mean turns | 15.6 | 17.3 | 18.2 |
| Mean agent time | 41 s | 46 s | 46 s |

Full table with paired bootstrap CIs: [analysis.md](analysis.md). Raw rows:
[results.jsonl](results.jsonl).

## Reading

- **Primary outcome: no measured difference, because nothing escaped.** The
  model solved every dev task correctly in every arm. The dev tasks are too
  easy for this model to separate the arms. That is a property of the tasks,
  not evidence for or against the profile.
- **Secondary: stating rules barely changed code; enforcing them did.** Half
  of the arm A trials and 5 of 12 arm B trials left unchecked arithmetic in a
  core crate (`+`/`*` that can overflow). Arm C left none. The hidden oracles
  never drove those operations to overflow, so they are not counted as
  defects. With 4 tasks, the bootstrap CI for this difference still includes 0.
- **Cost:** arm C used about 13% more model cost and 17% more turns than arm A,
  and took about 5 s longer per trial (CI excludes 0).

## Threats to validity

- **Oracle visibility.** Trials ran on the host, not in a container. The
  agent's tools were limited to cargo, git status/diff and file read/edit,
  but nothing stopped a file read outside the trial repo, including
  `bench/tasks/*/oracle` in the tauten checkout. Only the final JSON summary
  of each session was kept, not the transcript, so this cannot be ruled out.
  The held-out run requires per-trial containers and saved transcripts.
- **Prompts gave the edges away.** Several prompts state the hidden cases as
  worked examples (see `docs/bench/HELDOUT-DESIGN.md`), which by itself
  explains much of the ceiling.

## Next

1. Harder held-out tasks whose hidden oracles probe the edges agents skip:
   overflow near the type limits, malformed and empty input, rounding, error
   propagation, invariants across operations. The pre-registration is frozen
   before they run.
2. Harness fix found here: build output (`target/`) was counted in
   `diff_added_lines`; trial repos now ignore it.
