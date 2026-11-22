# Adopting tauten

Three routes. Each stands on its own: an advisory trial is a complete
outcome, not an unfinished migration, and reusing one Lean module does not
require the CLI or any Rust crate. None of them replaces a check a project
already runs.

| Route | What changes in the repository | Exit code used by |
|---|---|---|
| [Advisory trial](#advisory-trial) | nothing | a person reading the result |
| [Opt-in CI check](#opt-in-ci-check) | tiers and a reviewed baseline, committed | CI, as a required check |
| [Lean module only](#lean-module-only) | one vendored `.lean` file, its license and a checksum file | the project's own proof gate |

## Advisory trial

Run the structural checks once on a disposable checkout, with the tiers
given on the command line. Use the exact released CLI and the Rust
toolchain it is built for (`rust-version` in the workspace `Cargo.toml`).

```console
$ rustup toolchain install 1.98.0 --profile minimal --component clippy
$ cargo +1.98.0 install --locked --version 0.2.0 cargo-tauten
$ git clone --quiet . ../tauten-trial && cd ../tauten-trial   # disposable checkout
$ cargo +1.98.0 tauten init                                   # preview the tier plan; writes nothing
$ cargo +1.98.0 tauten check --structure-only --no-baseline \
    --assume-tier '*=core' --assume-tier app=edge --format json > ../tauten-trial.jsonl
$ echo $?
1
$ git status --short                                          # prints nothing
```

- **Trial tiers.** `init` without `--write` prints the tier it would give
  each crate and writes nothing. `--assume-tier` takes that plan without
  editing a manifest: `*=core` for every crate, then `<crate>=edge` for
  binaries and startup crates (`app` above is an example name).
- **Output.** `--format json` prints one finding per line. Keep the file
  outside the checkout.
- **Exit code.** 0: no error-level finding from the checks that ran. 1: at
  least one error-level finding under the assumed tiers; the code still
  builds and nothing is broken. 2: tauten itself could not run (for
  example, no `Cargo.toml`).
- **What did not run.** `--structure-only` skips the Clippy and rustc pass,
  so most rules decided by Clippy or rustc (`docs/RULES.md`, column
  "Decided by"), such as TAU-P001 (`unwrap`), TAU-N001 (arithmetic) and
  TAU-A001 (ambient authority), report nothing. `--no-baseline` ignores any
  baseline, so TAU-R001 does not run. No proof gate runs and nothing is
  translated (TAU-K004). Drop `--structure-only` to include Clippy; it then
  builds the workspace.
- **What stays the same.** No manifest, baseline, agent profile or proof
  record is written; `git status` stays empty.
- **Reading the result.** A finding is a place where the code departs from
  the profile for the assumed tier, not a defect, and the number of
  findings is not a defect count. On tauten's own benchmark, 0 of 120
  sampled findings were reachable defects (`docs/DISCIPLINE-PROPOSAL.md`).
- **TAU-H001.** Most workspaces report that release builds do not check
  integer overflow. Turning `overflow-checks` on makes an overflow in a
  release build panic instead of wrap: a decision about release behaviour
  for the project's owners, not a step of the trial.

## Opt-in CI check

Commit the tiers and a reviewed baseline, then make the full check a
required CI step. The baseline records today's error-level findings; CI
reports only new ones, and the baseline may only shrink.

```console
$ cargo +1.98.0 tauten init --write      # tiers, AGENTS.tauten.md, [profile.release] overflow-checks
$ git diff                               # review every tier; keep overflow-checks only as a decision
$ cargo +1.98.0 tauten baseline          # writes tauten-baseline.json; review it
$ git add -u && git add AGENTS.tauten.md tauten-baseline.json
$ git commit -m "tauten: tiers and reviewed baseline"
$ cargo +1.98.0 tauten check             # in CI: structure and Clippy, against the committed baseline
```

- **Tiers.** `init --write` adds `[package.metadata.tauten] tier = ...` to
  each crate. Review each one; change any by hand.
- **Overflow checks.** `init --write` also adds `overflow-checks = true`
  under `[profile.release]`. If the project has not decided that, remove
  the line before recording the baseline; TAU-H001 is then recorded in the
  baseline like any other finding.
- **In CI.** Install the same CLI version and toolchain as above and run
  `cargo tauten check` (the same as `cargo tauten verify --level fast`).
  It exits 1 for a finding the baseline does not accept, and for a finding
  that was fixed while the baseline still accepts it (TAU-R001; shrink the
  baseline in the same change).
- **Never in CI.** Do not run `cargo tauten baseline` in CI, and never
  regenerate the baseline to make a check pass. Growing it needs
  `--allow-increase`, a reviewed local change.
- **A lint check, not a proof gate.** `cargo tauten check` runs no proof.
  A project with a proof gate keeps it as a separate step
  (`cargo tauten proofs`, or its own); passing the check says nothing about
  proofs.

## Lean module only

A Lean project that already builds against Aeneas can reuse
`Tauten.Machine` (reachability, invariants, worlds and discharge for step
machines) without the CLI or any Rust crate. Vendor one file at a pinned
commit, with tauten's license, and check both against a committed checksum
file. Fetching happens once, when the pin changes; the proof build itself
needs no network.

```console
$ TAUTEN=370e7050f2adaba1e0a1c7351ab686d5f1238c47   # tauten 0.2.0
$ RAW=https://raw.githubusercontent.com/tauten-dev/tauten/$TAUTEN
$ curl -fsSLo Proofs/TautenMachine.lean $RAW/crates/cargo-tauten/templates/proofs/Proofs/TautenMachine.lean
$ curl -fsSLo Proofs/TautenMachine.LICENSE $RAW/LICENSE
$ sha256sum -c TAUTEN.sha256
Proofs/TautenMachine.lean: OK
Proofs/TautenMachine.LICENSE: OK
```

`TAUTEN.sha256`, committed with the two files:

```text
73a648f123876f2c3ae43610ddeac683061906d26b8ccc3b6cd297743bbe07f3  Proofs/TautenMachine.lean
bb23cea06c1a4f205b5f6e2d4ecc5ff1c1ebf15829706f1fe366424d0919e9be  Proofs/TautenMachine.LICENSE
```

- **Requirements.** The module imports `Aeneas` and nothing else: Lean
  v4.35.0 with the Aeneas Lean library at nightly-2026.11.07-c5eb4a2 (and
  its mathlib), the versions in the public proofs image
  `ghcr.io/tauten-dev/proofs-toolchain:lean-4.35.0-aeneas-c5eb4a2-charon-8cae8fd`.
- **Placement.** `Proofs/` stands for the project's own library directory;
  import the file as `<Library>.TautenMachine`. Its declarations live in
  the namespace `Tauten.Machine`.
- **Keep the existing gate.** The vendored file is source the project's own
  build checks; keep the project's axiom audit and gate unchanged.
  tauten's own gate audits the same file (`proofs/Tauten/Machine.lean`)
  to Lean's three standard axioms.
- **Updating.** Change `TAUTEN` and the checksums together, in one reviewed
  change; run `sha256sum -c TAUTEN.sha256` in the gate if it should refuse
  an edited copy.
