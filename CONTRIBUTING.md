# Contributing

The rules and who decides are in [GOVERNANCE.md](GOVERNANCE.md). This page is
the how-to.

## Start in the right place

| I want to… | Go to |
|---|---|
| Report something broken or wrong, a false positive or a regression | a [bug report](../../issues/new?template=bug_report.yml) |
| Propose a rule, a kernel primitive or a change in behaviour | a [feature or design proposal](../../issues/new?template=feature_request.yml) |
| Implement something | a pull request that links an `accepted` issue or an accepted [RFC](docs/rfcs/README.md) |

New issues get `needs-triage`. The owner accepts them, labels them
`needs-rfc` when the design must be written down first
([when an RFC is required](docs/rfcs/README.md#when-an-rfc-is-required)),
and puts them in a milestone. A trivial fix (a typo, a doc correction, a
dependency bump) can go straight to a pull request. If you cannot tell
whether it is trivial, it is not.

## Building and checking

Read [AGENTS.md](AGENTS.md) first. In short:

- `docker compose run --rm gate` is the CI gate; run it before pushing
  `main`.
- If you change `crates/tauten-check-kernel` or `proofs/`, run the proofs
  gate on a quiet tree: `cargo tauten proofs` (the tree must not change
  while it runs).
- Decision logic is proved in Lean before it is wired into the checker, and
  every claim lists mutants that break it.

## Commits

- Name the issue: `Refs #N` while work is under way, `Closes #N` on the
  commit that finishes it.
- If users can notice the change, add a line to [CHANGELOG.md](CHANGELOG.md)
  and regenerate `docs/RULES.md` (`cargo tauten explain --markdown`) when a
  rule changes.
- If the work belongs to an RFC, update its `implementation` field and its
  registry row.
