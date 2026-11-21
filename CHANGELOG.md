# Changelog

## Unreleased

- `cargo tauten init --write` keeps valid manifests valid (#54). An existing
  `[package.metadata.tauten]` table gets the proposed `tier` in whatever form
  it has (explicit header, dotted keys, inline table), instead of a second,
  duplicate header that Cargo refuses. Comments and other keys are kept, a
  second run changes nothing, and a `tier` that is present but not a string
  is refused before anything is written. TAU-H001's `overflow-checks` is
  likewise added to an existing `[profile.release]` table. Manifests are
  edited with `toml_edit`, and every edit is re-parsed before it is written.
- Library and binary roots come from Cargo's declared targets (#55). A
  library at a non-default path or file name (`[lib] path =
  "code/kernel.rs"`) now gets the same structural findings (TAU-K003 and the
  other source rules) and the same `init` tier proposal as `src/lib.rs`.
  Before, such a crate was checked as if it had no library and was proposed
  as `edge`. Files outside `src/`, `tests/`, `benches/` and `examples/` are
  collected from each declared target's directory. Library targets of any
  crate type (`cdylib`, `staticlib`, ...) count as crate roots.
  `tauten_check::SourceFile` gains `is_lib_root`.

## 0.0.6 (2026-11-21)

- TAU-X001 advice for modules shared by several targets (#59): an inner
  `#![expect]` in a module compiled into several targets (helpers under
  `tests/` declared with `mod support;` by several test files) is unfulfilled
  wherever the lint does not fire, so the old advice could not work. In a
  module file the finding now names the form that does: a reasoned
  `#[expect]` on the `mod` declaration in each target where the lint fires.
  The rule's repair text also names the structural repairs (one
  integration-test target, or a dev-dependency support crate). New fixtures
  `acc-shared-test-module` and `rej-shared-test-allow`.
- TAU-X001 accepts a reasoned file-level `#![allow(dead_code, reason =
  "...")]` in a module under `tests/` or `benches/` that is not the root of a
  Cargo target (#59, RFC `2026-11-21-shared-test-module-allow`). An
  unreasoned one, another lint or group, `cfg_attr`, a target root, or any
  file in `src/`, `examples/` or the build script is still reported. Target
  roots come from `cargo metadata`. `tauten_check::SourceFile` gains
  `is_target_root`, and `scan::LintException` gains `reasoned`.

- TAU-N002 no longer promises a proof discharge (#57): its repair text said a
  recorded never-fails proof discharges a lossy cast, but only TAU-N001 and
  TAU-P003 are subsumable (a cast never fails, it changes the value). The text
  now points to a lossless or checked conversion, `cargo tauten obligations
  --list` labels TAU-N002 entries as not dischargeable by a proof, and a test
  ties every repair text that promises a discharge to the subsumable rules. No
  verdict changes.

- `cargo tauten proofs reconcile` (#50): in a two-parent merge whose
  human-authored conflicts are resolved, checks the resolved `claims.toml`
  against both parents' recorded claims (TAU-R002 twice, decided together by
  the proved `control::admit`), regenerates the declared translation, and
  writes a `pending-merge` record (`version: 2`, `parents`, `registry`; no
  results). A pending record is never evidence: it subsumes nothing, `proofs
  export` refuses it, the ship level reuses nothing, and a v1 reader refuses
  it. It stays the TAU-R002 baseline until a gate on a snapshot of the merge
  commit (its parents checked against the record) proves every claim.

- A missing proof tool no longer overwrites the record (#52): when the gate
  cannot start, prints nothing, or reports only `tools` missing, `cargo tauten
  proofs` still prints a `tool-missing` outcome per claim and exits 1, but
  leaves `proofs/claims.json` byte-identical (or absent), so the same command
  can be retried from the clean checkout. A gate that started and then failed
  (drift, build, audit, mutants) is recorded as before.
- Draft claim entries (#49, RFC `2026-11-21-draft-claim-entries`):
  `cargo tauten proofs draft [--crate <package>] [--inventory <file>]`
  prints a unified diff to `proofs/claims.toml` for new or changed
  functions, ready for `git apply` after review; empty stdout and exit 0
  when there is nothing to draft. It runs `[gate] try` with no mutant
  selected (tools, extract, drift, build, audit), joins each theorem to the
  fully qualified extracted declaration the audit names as its subject
  (never a short name; a loop's theorems join its function only through the
  translation's `loop` header), and locates each `[[mutant]]` no claim lists
  by its unique site. `total` and `subsumes` are suggested only for a
  premise-free total theorem about the function itself, and `subsumes`
  only on an unconditional claim. Existing claims are only extended in
  place; comments, other entries and `[[amend]]` are kept byte for byte,
  and a second draft is empty. Unknown packages, missing tools, a stale
  translation, ambiguous or contradictory associations and layouts it
  cannot edit refuse with no patch (exit 1). It writes no file, runs no
  mutant and grants no proof credit; exit 0 is never a gate result. The
  template's `Audit.lean` prints `SUBJECT <theorem> <function>` and
  `gate.py` emits it as `{"theorem", "subject"}` (read only by the draft;
  projects set up earlier copy both from `cargo tauten init --proofs` or use
  `--inventory`, a version-1 JSON of functions, theorems and mutants checked
  against the translation and the audit). Fixture `fixtures/proofs-draft`.

- Publishing waits for the full proofs gate on the tag commit (#53):
  `release.yml` calls `proofs.yml` (extract, drift, Lean build and audit,
  every mutant) in the same run as the CI gate, and `publish` needs both, so a
  failed, cancelled, timed-out or skipped proofs job blocks it. A
  `workflow_dispatch` of `release.yml` rehearses the same path without
  publishing. The hosted proofs timeout is 150 minutes (a full run takes about
  an hour; 60 minutes cancelled runs that would have passed).

- An empty proof inventory never passes (#51): `cargo tauten proofs` (any
  `--level`) and `cargo tauten verify` over a `proofs/claims.toml` with no
  `[[claim]]` exit 1, say that no theorem was checked, and write no record; a
  previous record still triggers TAU-R002. The run's verdict is now decided by
  the proved `control::admit` (one `Pass` control per claim), which refuses an
  empty required set. Projects without `proofs/claims.toml` are unaffected.

## 0.0.5 (2026-11-21)

- `cargo-tauten` packages: the `cargo tauten new` templates' manifests are
  `Cargo.toml.in` (cargo leaves any subdirectory with a `Cargo.toml` out of a
  package, so the first v0.0.5 publish failed its verification and uploaded
  nothing). The CI gate runs `cargo package` over every published crate.

- Inherit a dependency's claim (A7b, #48): `inherit =
  "crate@version:function"` in `proofs/claims.toml` reuses a claim the
  dependency ships in the `claims.json` next to its `Cargo.toml`
  (`cargo tauten proofs export --crate <name>` writes it; `tauten` ships
  its publish claims). The badge is the proved `badge::inherit`
  (`inherited_badge_le`: never stronger than the inherited one;
  `inherit_needs_version_and_record`): a version other than the locked one
  or a missing record establishes nothing. The record carries the
  dependency's theorems and assumptions. Mutants `inherit-ignores-version`,
  `inherit-strengthens`. Fixture `fixtures/inherit`.

- Cadence levels and incremental proofs (D2, #36). Each claim records its
  named inputs (`inputs` in `proofs/claims.json`): the declarations of the
  translation its function reaches, the Lean files stating and proving its
  theorems (with their local imports), its mutants' definitions and the
  toolchain pins. `cargo tauten proofs --level ship` runs every stage
  (extract, drift, build, audit) but re-runs only the mutants of claims
  whose inputs the proved `binding::fresh` finds changed (or that were not
  proved), and reuses the other kills from the record; an edit to code the
  translation does not reach re-runs no mutant. `cargo tauten ship` is
  `check` then the gate at ship level; `cargo tauten verify --level
  fast|ship|full` runs a cadence level. Needs `[gate] try`.

- Mutant verdicts report what broke directly (#39, with #40 and #47): the
  template gate emits, per mutant, the declarations whose own proof failed
  (`direct`) apart from those that failed only through them
  (`dependent`). A `[[mutant]]` may declare `direct = [...]`, the exact set
  of declarations (helpers included) that must fail directly; that kill is
  decided by the proved `control::admit`: for expected `{L}`, `{L}`
  calibrates, `{H}` and `{L, H}` do not, nothing observed or an
  infrastructure failure never does. Without `direct`, the claim-level
  policy is unchanged.

- The gate runs as the owner of the checkout, not root (#42):
  `docker/entry.sh` (the tools services' entry point) hands the cache
  volumes to that uid once and drops privileges with `setpriv`; root is an
  explicit opt-in (`TAUTEN_GATE_ROOT=1`). The gate prints the uid it ran
  as. `tests/host.rs` overwrites a 0444 file and expects
  `PermissionDenied`, so a root-run gate fails it exactly as a host would.

- The fast extract stage builds the translation in Lean (#46):
  `cargo tauten proofs extract` and `check --extract` now also compile the
  fresh Lean (without the proofs), so a name Lean reserves (a parameter
  `given`) or a generated-name clash (a parameter named like its module)
  fails at extract speed, located at the Rust line, instead of one full
  gate round later. Lean decides; there is no keyword list. About 10-15 s
  more per crate. Fixtures `rej-lean-keyword`, `rej-name-clash`.

- Declared layers inside a tier (B6, #21): `[workspace.metadata.tauten]
  layers = [...]` orders a workspace's crates; each member names its
  `layer` and may depend only on the same or an earlier one. New rule
  TAU-T004, decided by the proved `layer::finding` (`layer_finding_iff`,
  `unknown_layer_is_finding`) over an order proved reflexive, transitive,
  antisymmetric and total; mutants `layer-strict`, `layer-inverted`,
  `unknown-layer-clean`. A missing, unknown or malformed declaration is a
  finding. Fixture workspace `fixtures/layers`; `docs/LAYERS.md` writes
  writfold's crate graph as layers and states what layers cannot express
  (edge allowlists).

- Scanner mutation gate and a rewritten `proofs/TRUST.md` (T7, #34). The CI
  gate runs `cargo-mutants` over the fact extraction (`scan.rs`,
  `effects.rs`, `manifest.rs`) and requires zero survivors: 147 mutants,
  125 caught, 22 unviable. New unit tests cover every type form the effect
  classifier reads, crate type expansion, where clauses, walked modules
  and impls, and `cfg_attr` exceptions; two branches that no input could
  reach were removed, one equivalent mutant is excluded with its reason.
  `scripts/mutants` now builds each parallel job in its own target
  directory (they used to share the image's, and tested each other's
  binaries). `TRUST.md` lists what is proved, compiler-enforced and tested,
  and what each scanner does not see.

- Evidence that cannot go stale (A5, #9). `#[evidence(subject = T,
  exclusive)]` adds `mint_exclusive`, returning a `Permit` that holds the
  only `&mut T`: check and act are one chained call (`Permit::act`);
  mutation between them, minting outside the module and cloning do not
  compile (three new compile-fail tests). `#[evidence(subject = T,
  stamped)]` with `T: Versioned` adds `mint_stamped`, returning a
  `Stamped` fact that crosses an `await` or a queue without borrowing;
  `Stamped::verify` returns `Stale` once the version moved.
  `#[derive(Bindings)]` generates `diff` (every differing field, new fields
  included) and `revalidate` into `Evidence<'_, Fresh<T>>`. Proved for an
  example kernel action (`tauten_proof_instances::stamped::bump`):
  `stale_refused` (a stale stamp returns `Stale` and leaves the state
  unchanged) and `fresh_accepted`; mutants `stamp-ignored`,
  `stale-accepted`. `tauten::origin` documents each carrier's label and the
  three hand-written token shapes it replaces.

- Atomic publish, proved (A7, #11). `proofs/Tauten/Durable.lean` runs the
  publish machine against a filesystem model with crashes; each filesystem
  guarantee is a switchable flag and a named assumption
  (`proofs/assumptions.toml`): `FS.rename-atomic`, `FS.sync-file`,
  `FS.sync-dir`, `FS.exclusive-create`. Proved over the extracted steps:
  `publish_old_or_new` (a reader, and a reader after any crash, sees the old
  or the new content; under rename-atomic and sync-file),
  `publish_durable_on_done` (adding sync-dir, `tauten::publish::durable`
  phases survive any crash with the new content),
  `create_new_never_replaces` (under exclusive-create, for the new
  `tauten::publish::create_step`, which moves with `RenameNoReplace`),
  `publish_stops_on_failure` and `publish_frame` (no assumption),
  `profile_mono` and `crash_characterisation`. One profile satisfies every
  assumption (`posix_satisfies`) and reaches `Done`; three necessity
  theorems show each property failing with its guarantee switched off.
  Mutants `skip-sync`, `failure-ignored`, `durable-too-early`,
  `replace-under-create-new`, with `rename-before-sync` from A6. tauten's
  own proof gate now derives each theorem's assumptions (`PREMISES`,
  `ESTABLISHES`), as the template's does; the machine library gains
  `RSysReachable` and `rsys_of_model` for extracted steps.

- The agent profile (`AGENTS.tauten.md`) opens with the architecture on
  one page (A12, #16): decisions are total kernel functions with claims,
  protocols are machines, the world is reached only through capabilities
  in the effect row, a check travels as evidence or a binding, a result is
  shown with the badge its controls admit, registries are never weakened.
  Each line names the rule or type that enforces it and labels it proved,
  compiler-enforced, lint, tested or not checked.

- `cargo tauten new <path> [--tauten-path DIR]` (A12, #16) writes a fresh
  workspace: a kernel crate with one total function (`affordable`) and its
  claim, theorem and two mutants (through the `init --proofs` template), a
  core crate with a `Clock` capability, `#[effects]` (`effects =
  "required"`) and one `tauten::origin::Evidence` check (to be replaced by
  the A5 permit), and a thin edge binary calling `run(caps, args)`. It
  passes `cargo tauten check` with no findings (tested); after
  `sh proofs/refresh.sh`, `cargo tauten proofs` records the claim `proved`
  with both mutants killed (run once locally, 102 s). Acceptance trial: 3
  of 3 fresh Claude Sonnet 5.5 sessions given only the profile added a
  feature to the template and passed `check` and the proof gate
  (`docs/dry-runs/a12-profile-2026-11-20.md`; one request, $0.39).

- `cargo tauten obligations [--list]` (D3, #37): in kernel crates,
  arithmetic and indexing findings (TAU-N001, TAU-N002, TAU-P003) are
  listed as proof obligations: proved (subsumed by a recorded proof that the
  function never fails), open and baselined, or open. The counts are the
  ones `check` decides (proved = subsumed, open and baselined = accepted by
  the baseline); it fails when an obligation is open. The rules' fix texts
  say so.

- `cargo tauten generated [--only NAME…]`: the generated-file stage (A11,
  #15). Each `[[workspace.metadata.tauten.generated]]` entry names a
  command and either the committed file its standard output must equal
  (`stdout`) or the files it writes (`outputs`, run in a scratch copy so the
  checkout is never written). Files are compared with the proved
  `same_bytes` and the stage is decided by the proved `control::admit`: one
  byte is `drift`, and an output the command did not produce or a failing
  command is `failed`, never `ok`. tauten declares `rules`
  (`docs/RULES.md`, now checked this way by the CI gate) and `translation`
  (`proofs/Extracted`, needs Docker).

- Which compiler messages become findings, whether a rule applies to a
  target (test targets included) and whether `check` passes are decided by
  the proved kernel (`compiler::kept`, `applies`, `passes`; T5, #32):
  `kept_iff`, `applies_spec`, `check_passes_iff`, `warnings_never_fail`.
  The pinned toolchain's real compiler output on `fixtures/cases` is
  recorded in `crates/cargo-tauten/tests/golden/` and replayed by a test;
  `scripts/record-golden` re-records it for a new toolchain
  (`TAUTEN_RECORD_COMPILER_JSON` keeps the raw messages).

- TAU-T001, TAU-T003 and TAU-K003 are decided by the proved kernel
  (`layout::t001`, `t003`, `k003`; T6, #33). `no_std` is no longer found by
  substring: only `#![no_std]` or `#![cfg_attr(not(test), no_std)]` satisfies
  TAU-K003; `no_std` under any other condition (a feature, nesting) and a
  library root that does not parse are findings. New fixtures
  `rej-conditional-no-std` and `acc-kernel-not-test`.

- Lint exceptions are decided by the proved kernel in more places (T4,
  #31). `exception::decide` takes how the attribute is written and every
  rule it may cover: an exception inside `cfg_attr` is never accepted in a
  kernel crate (`conditional_never_allowed_in_kernel`), and an exception for
  a lint group (`warnings`, `clippy::all`, `unused`, …) is decided as the
  worst over every profile rule the group may contain
  (`group_never_weaker`). TAU-X003 is `exception::loosens` (`x003_iff`) and
  now also sees groups in `[lints]`. The scanner reads exceptions inside
  `cfg_attr` (reported under TAU-X001 when they `allow`) and in `tests/`,
  `benches/`, `examples/` and the build script; in test targets a rule that
  does not apply to tests covers nothing. New reject fixtures:
  `rej-cfg-attr-allow`, `rej-group-allow`, `rej-test-exception`.

## 0.0.4 (2026-11-20)

- `cargo tauten proofs try --theorems T… | --mutants M…`: the developer
  loop. Runs extract, drift, build and audit, then only the mutants of the
  claims stating those theorems (or the named mutants), in parallel; prints
  each affected claim as `PARTIAL`, never writes `proofs/claims.json`, and
  says it is not a gate result. Needs `[gate] try` (the template writes it).
  On tauten itself, one theorem's two mutants take 78 s against about 25
  minutes for the full gate.

- `cargo tauten proofs` gates a snapshot of the committed `HEAD` (`git
  archive` into a private directory) instead of the live working tree, so
  the checkout may be edited while a long gate runs; uncommitted changes in
  the workspace are refused, and `proofs/claims.json` records the `commit`
  it is about. `--live` keeps the old behaviour (any edit during the run is
  drift). Outside Git, the live tree is gated as before.

- `tauten::machine`: a protocol as a pure step function (`Machine`), with
  `run` and `replay` (compare a recorded tape of events and commands). The
  Lean library `Tauten.Machine` (shipped in the proofs template as
  `Proofs/TautenMachine.lean`) proves the induction once: `inv_of_step`,
  `inv_run`, `rinv_of_step` for extracted steps that return a `Result`,
  `sys_inv_of_step` against a world that interprets commands and takes
  adversary steps, and `discharged_of_inv`. Worked instance:
  `tauten::publish` (write, sync, rename, sync the directory; remove the
  temporary file on failure), proved over its translation to leave no
  temporary file behind in any reachable terminal phase, with no
  filesystem assumption.

- Every rule states its `family` (safety, obligation, style), exactly what it
  decides (`invariant`), what it reads (`sees`), what it does not see
  (`dominion`), whether it is `complete`, and its reject `fixtures`
  (`docs/RULES.md`, `cargo tauten explain`). Proved laws: an incomplete rule
  never blocks (only TAU-S002 is incomplete), an input a rule could not read
  is never clean (a source file `syn` cannot parse is now reported under each
  source-scanning rule instead of being skipped), and a complete rule that
  blocks must name a reject fixture or the rule table is invalid. Nine new
  reject fixtures cover the rules that had none.

- `cargo tauten proofs extract [--crate <dir>]` and `cargo tauten check
  --extract` (TAU-K004): translate the proved kernel crates without building
  the proofs and report the first error with its location; code that
  translates only through opaque library functions (axioms) is refused too.
  Without the toolchain the result is tool-missing, never clean. Needs
  `[gate] extract` in `proofs/claims.toml` (the template writes it). The
  agent profile's kernel section is generated from `shapes.toml`, whose
  every entry is a fixture re-checked against the pinned toolchain
  (`scripts/extract-fixtures`): integer casts and derived `PartialEq` do
  translate at this pin; trait objects, floats, slices of borrowing
  structs, early returns across two loops, `match` on `get` in a loop and
  iterator chains do not. The planned syntax lint (old TAU-K004 plan) and
  `#[kernel]` macro are dropped.

- TAU-H001: release builds must check integer overflow
  (`overflow-checks = true` under `[profile.release]` in the workspace
  Cargo.toml, with no per-package or `.cargo/config.toml` override turning it
  off). A value tauten cannot read is never clean. Decided by the proved
  `tauten_check_kernel::manifest::hardening`; `cargo tauten init --write`
  adds the setting.

- Named assumptions are derived, not just declared. Tag a Lean proposition
  `@[tauten_assumption "ID"]` (template file `Proofs/TautenAssumption.lean`);
  the audit derives which assumptions each theorem's hypotheses use, and a
  claim's `assumes` must be exactly that list. A claim with assumptions names
  a `witness` theorem showing they can hold together; optional `necessity`
  theorems must pass like theorems. Decided by the proved
  `tauten_check_kernel::assumption::assumptions_ok`. Existing proofs
  directories: copy the template's `Audit.lean`, `gate.py` and
  `Proofs/TautenAssumption.lean`.

- TAU-R002: a recorded proof claim may not be weakened. `proofs/claims.toml`
  may not drop a theorem or mutant recorded in the committed
  `proofs/claims.json`, nor add an assumption, unless an `[[amend]]` entry
  (`file`, `function`, `kind`, `name`, `reason`) names the change;
  `cargo tauten proofs` refuses to re-record otherwise. Decided by the proved
  `tauten_check_kernel::registry::conserves`.

## 0.0.3 (2026-11-19)

- `cargo tauten proofs`: runs a project's proof gate (`proofs/claims.toml`)
  and prints one JSON outcome per claim (`proved`, `drift`,
  `build-failed`, `audit-failed`, `mutant-survived`, `tool-missing`);
  records them in `proofs/claims.json`.
- `cargo tauten init --proofs <crate>` writes the Charon/Aeneas/Lean proof
  template for a kernel crate.
- In kernel crates, TAU-N001/TAU-P003 findings inside a function with a
  current, total, proved claim are reported as subsumed by the proof
  (JSON field `subsumed_by`, severity `warning`), not as errors.
- Rule-table validation and proof subsumption are decided by
  `tauten-check-kernel` and proved in Lean (`docs/proofs/README.md`).
- **Breaking:** `#[refined]` accepts only an inner type that is
  `tauten::refine::Frozen` (primitives, `String`, and `Box`, `Option`, `Vec`,
  arrays, slices, tuples and `&'static` references of those). A `Cell` could
  be changed through `get()`, and a user-written `Clone` could break the
  predicate, after `try_new` checked it. Every refined type implements
  `tauten::refine::Refined`. Reported by an external review.
- Proof fingerprints include the workspace `Cargo.toml` (without `version`
  lines) and the toolchain pin.

## 0.0.2 (2026-11-18)

- `#[refined]` rejects derives and attributes that could construct a value
  without checking the predicate (`Default`, `Deserialize`, ...). Allowed:
  `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`.
- TAU-A002: `Authority::acquire` is banned outside the edge by resolved path,
  so aliases and function pointers are caught; cap-std's ambient constructors
  are banned under TAU-A001.
- TAU-A001's repair hint no longer names a type that does not exist.
- `#[ensures]` violation messages show `old(..)` as written.
- The ratchet's TAU-R001 names the crate's real tier.
- Rule tables missing a rule the checker emits are rejected.
- Releases use crates.io trusted publishing.

## 0.0.1 (2026-11-18)

First release.
