# tauten agent instructions

tauten is a strict Rust discipline plus proofs: lints, a checker and a
proved kernel, so that an agent told "use tauten" writes code inside a known
architecture and every verdict tauten gives is either decided by the Lean-proved
kernel or the compiler, or labelled honestly as weaker. Read `README.md`,
`docs/ROADMAP.md`, `docs/rfcs/README.md` and `proofs/TRUST.md` before changing
the project.

## Work, issues and RFCs

- All remaining and future work is a GitHub issue in a milestone
  ([roadmap](docs/ROADMAP.md)). A commit that does it names the issue
  (`Refs #N`, `Closes #N` on the last). Close an issue only when the work is
  on `main` and gated; never close one to make a count.
- Designs that are costly to reverse, and any new or changed decision logic,
  are RFCs in `docs/rfcs/` ([process](docs/rfcs/README.md)) with their proof
  obligations first. Agents draft RFCs; only the owner accepts or rejects one.
- Research and planning agents (Fable) write their proposals as `draft` RFCs,
  not as loose plan files. The main agent implements accepted RFCs; planning
  agents do not implement.
- The acceptance gate for any proposal: it is general across projects, it is
  decided by the proved kernel or the compiler (honest labels otherwise), and
  it fits the vision. No narrow checks that cannot be proved.
- Rejected and parked ideas stay recorded with their reasons and what would
  reopen them (issue #30).

## Proofs first

- New decision logic goes into `crates/tauten-check-kernel` first, is
  extracted with Charon/Aeneas and proved in `proofs/Tauten/`, and only then
  is wired into the shell.
- Every audited claim lists mutants in `proofs/mutants.py`; the expected
  broken set must equal the claims that actually break. A claim with no
  mutant is never "proved".
- Release proofs use no axioms beyond `propext`, `Classical.choice` and
  `Quot.sound`; no `sorry`, `native_decide`, `bv_decide` or `ofReduceBool`.
- Run the full gate (`cargo tauten proofs`) on a quiet tree: edits during
  the run are reported as drift. Use a separate worktree for parallel work.
- Assumptions about the outside world (filesystem, hashes, tools) are named
  conditions on the claims that use them, never Lean axioms; such claims are
  "proved under assumptions", never "proved".

## Releases and pushing

- Push gated work to `main` regularly.
- Publishing to crates.io is irreversible: confirm with the owner first.
- Never print tokens or put personal details in public issues or external
  requests.
