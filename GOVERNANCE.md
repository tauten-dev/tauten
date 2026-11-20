# Governance

tauten has one maintainer, the owner. The owner decides what is accepted,
what ships and what is published. Standing decisions are recorded in
[AGENTS.md](AGENTS.md) and [docs/OWNER-DECISIONS.md](docs/OWNER-DECISIONS.md).
This process is shared with the owner's other projects (adapted from
writfold's).

## Who does what

- **The owner** accepts or declines issues, accepts or rejects
  [RFCs](docs/rfcs/README.md), sets milestones and approves every crates.io
  publish.
- **Agents** (coding sessions working for the owner) may file issues, draft
  RFCs, implement accepted work, run the gates, push gated work to `main`
  and close the issues their commits resolve. Planning agents explore and
  write draft RFCs only. An agent never accepts its own RFC.
- **Contributors** open issues. A pull request needs an `accepted` issue or
  RFC behind it, unless it is a trivial fix ([CONTRIBUTING.md](CONTRIBUTING.md)).

## Where decisions live

| Kind | Where |
|---|---|
| Direction, and the acceptance gate for proposals | [AGENTS.md](AGENTS.md) |
| Design of a change | an RFC in [docs/rfcs/](docs/rfcs/README.md) |
| Work to do, in order of return | GitHub issues in milestones; [ROADMAP.md](docs/ROADMAP.md) |
| Parked and rejected ideas | [issue #30](https://github.com/tauten-dev/tauten/issues/30) |
| What changed for users | [CHANGELOG.md](CHANGELOG.md) |
| What is proved, and what the proofs trust | `proofs/claims.json`, [proofs/TRUST.md](proofs/TRUST.md) |

## Milestones

`next`, `0.1.0`, `0.2` and `later`. An issue with no milestone has not been
triaged. A milestone closes when its release is tagged; anything still open
moves to the next one, with a comment.

## Labels

- **Triage:** `needs-triage`, `accepted`, `blocked`, `needs-owner`,
  `declined`.
- **Design:** `rfc`, `needs-rfc`, `needs-proof` (decision logic: a Lean
  proof is due before wiring).
