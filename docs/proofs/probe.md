# P0 probe: Charon/Aeneas on tauten-shaped code

2026-11-18. Milestone P0 of `docs/PROOF-PLAN.md`. Everything below was run.

> **Retired in P1.** The probe crate (`proofs/probe`), its theorems
> (`proofs/Probe`), `proofs/probe.sh` and `Extracted/TautenProbe.lean` were
> removed when the gate moved to `crates/tauten-check-kernel`
> (`docs/proofs/README.md`). The probe needs crates.io for
> `tauten-macros`'s dependencies and would have doubled the gate for no
> new claim. The files remain at commit `2885ee1`. Its constructor
> enumeration returns with the macro instances in P3.
The only machine was a local 32-CPU WSL2 host. No GitHub runner was used.

## Toolchain (writfold's pins, unchanged)

I reused the local image `writfold-proofs-deps:dev`
(`sha256:a1b9264f1ef0…`, built 2026-11-09 from writfold's `proofs/Dockerfile`,
`deps` stage). I did not build a new image. I checked the image against
writfold's current Dockerfile pins by running the tools inside it:

| Tool | Reported | Pin |
|---|---|---|
| Charon | `0.1.313 (8cae8fd1905946de61147a65ecdd7122cc6b491b)` | `CHARON_COMMIT=8cae8fd…` |
| Aeneas | `aeneas nightly-2026.11.07-c5eb4a2` | `AENEAS_RELEASE` |
| Lean | `4.35.0` (commit `eb6cafb7…`) | `proofs/lean-toolchain` |
| rustc (Charon) | `1.100.0-nightly (923c95cdf 2026-09-16)` | `nightly-2026-09-17` |
| lake manifest | byte-identical to writfold's `proofs/lake-manifest.json`, apart from the package name | mathlib `56a00585…` |

## Files

- `proofs/probe/`: the probe crate, `tauten-probe`, with `publish = false`.
  It is excluded from the workspace (root `exclude = ["fixtures", "proofs"]`)
  and inherits `[workspace.package]` fields only inside the shadow workspace.
- `proofs/extract.sh`: writfold's script, generalised. It copies the crate
  and every crate it reaches through `path` dependencies (here
  `crates/tauten` and `crates/tauten-macros`) into the shadow workspace. It
  takes the resolver from the real root and copies `Cargo.lock`.
- `proofs/probe.sh`: translates each item on its own. Aeneas runs without
  `-abort-on-error`. The script then type-checks whatever Aeneas wrote.
- `proofs/gate.sh`: extract, drift check against `proofs/Extracted/`, then
  `lake build`. There are no mutants yet; they come in P1.
- `proofs/Extracted/TautenProbe.lean`: the committed translation, made with
  the manifest's `start_from` and `include`.
- `proofs/Probe/*.lean`: the theorems. `proofs/Audit.lean` holds the axiom
  audit and the constructor enumeration.

## Results per item

There are two configurations.

- **Default**: the item's own `--start-from` only, so tauten runtime items
  stay opaque.
- **`include`**: also `--include tauten::contract --include tauten::error`.

"Axioms" lists the Lean `axiom`s that Aeneas emitted for items it had no body
for. The audit rejects any such axiom.

| Item | Charon | Aeneas | Lean builds | Axioms, default | Axioms, `include` | Theorem (audited) |
|---|---|---|---|---|---|---|
| `tier::rank`, `may_depend_on` (rank match) | yes | yes | yes | none | none | `rank_spec`, `may_depend_on_spec`, `kernel_depends_only_on_kernel` |
| `tier::may_depend_on_derived` (derived `PartialOrd`) | yes | yes | yes | none | none | `may_depend_on_derived_agrees` |
| `ratchet::apply` (while loop over `Vec<u32>`, `&[Finding]`) | yes | yes | yes | none | none | none (P2) |
| `Percent(u8)` through the real `#[refined]`, module in `start_from` | yes | yes | yes | `tauten.contract.holds`, `tauten.error.RefineError`, `RefineError.new` | none | `percent_try_new_ok_iff` |
| same, only `Percent::try_new` in `start_from` | yes | yes | yes | same three | none | (same) |
| `PercentHand`: closure-free hand expansion | yes | yes | yes | `RefineError`, `RefineError.new` | none | `percent_hand_try_new_ok_iff` |
| `deposit` through the real `#[requires]`/`#[ensures]`/`old` | yes | yes | yes | `holds`, `snapshot`, `ContractViolation.requires`, `.ensures` | none | `deposit_zero_refused`, `deposit_ok` |
| `deposit_hand`: closure-free hand expansion (body in a nested `fn`) | yes | yes | yes | `ContractViolation.requires`, `.ensures` | none | none |
| `ledger::apply` (`checked_add`, `checked_sub`, `ok_or`) and `shrink` (`saturating_sub`) | yes | yes | yes | none | none | `apply_deposit_spec` |
| `control::Plain` with `derive(Default)` | yes | yes | yes | none | none | constructor audit (below) |

In the default configuration, Aeneas warns: `The crate contains extracted
external, unknown definitions: we advise using the option -split-files to
allow manually providing these definitions in separate files.` The file
still builds, but the axiom audit would reject it.

The combined gate (`sh proofs/gate.sh`) passed. `lake build` ends with:

```
AXIOM_AUDIT ok: 14 theorems, 9 required claims, axioms ⊆ [propext, Classical.choice, Quot.sound]
CONSTRUCTORS tauten_probe.percent.__tauten_percent.Percent: [….Insts.CoreCloneClone.clone, ….Insts.CoreConvertTryFromU8RefineError.try_from, ….try_new]
CONSTRUCTORS control.Plain: [tauten_probe.control.Plain.Insts.CoreDefaultDefault.default, tauten_probe.control.fresh]
CONSTRUCTOR_AUDIT ok
```

### Failures met along the way (exact text)

- **`--extract-opaque-bodies`** (tried in order to see `holds`'s body). It
  pulls in bodies from `core`, and Aeneas fails on them:
  `Breaks to outer loops are not supported yet` (`core/src/str/count.rs`),
  `Continue to outer loops are not supported yet`
  (`core/src/str/pattern.rs`), and `Detected groups of mixed mutually
  recursive definitions` (`core_simd`, `core::fmt::builders`). The contract
  items then also fail in Lean:
  `Type mismatch  i.checked_add i1 has type Option U64 but is expected to
  have type Result ?m.2`. Do not use this flag. Use `--include` on the tauten
  modules instead.
- **A parameter with the same name as its module** (`mod tier { fn rank(tier:
  Tier) }`). Aeneas output that does not elaborate:
  `Invalid pattern variable: Variable name must be atomic, but
  tier.Tier.Kernel has multiple components`. Renaming the parameter fixed
  it. Kernel code must not name a parameter after its module.
- **Re-exported path in `start_from`.** `tauten_probe::percent::Percent::try_new`
  gives `failed to resolve item path … does not correspond to any item` and
  then `ERROR Code failed to compile`. Only the path through the generated
  module works: `percent::__tauten_percent::Percent::try_new`.
- **CLI and manifest are merged.** Charon combines `[package.metadata.charon]
  start_from`/`include` with the command-line flags; the command line does not
  replace the manifest. `EXTRACT_NO_METADATA=1` in `extract.sh` strips the
  manifest table for the isolated runs.
- **`--network none`.** A cold run cannot fetch `syn`, `quote` or
  `proc-macro2`: `download of config.json failed … Could not resolve host:
  index.crates.io`. Writfold can run offline because its kernel has no
  dependencies. A tauten instances crate that uses the real macros needs
  vendored crates in the image.

## Section 6 open questions

1. **Does Aeneas translate `holds(&value, |v| ...)` and the
   immediately-invoked body closure? Yes, both.**
   - The predicate closure becomes a closure type with an `FnOnce` instance
     whose `call_once` is `ok (tupled_args <= 100#u8)`.
   - The body closure becomes an `Fn` instance whose `call` holds the body.
   - `holds` and `snapshot` translate with their bodies once `tauten::contract`
     is included.
   - Proofs need only `unfold`/`simp` with the closure's `call_once`/`call`.
   - The prediction ("unreliable") was wrong for these shapes.
   - What fails without configuration is not the closure. It is that items
     of the external `tauten` crate are opaque by default, and Aeneas then
     emits `axiom`s for them.
2. **Are derived impls visible? Yes, if the generated module is in
   `start_from`.**
   - All of these translated, and Lean builds them: `Debug` (through the
     Aeneas `core.fmt` model), `Clone`, `Copy`, `PartialEq`, `Eq` on
     `Percent`; `PartialOrd`/`Ord` on `Tier`; `Default` on `Plain`.
   - With only `try_new` in `start_from`, no derive is translated (8
     declarations instead of 24).
   - `Plain::default` appears as `Plain.Insts.CoreDefaultDefault.default :
     Result control.Plain`, and the constructor enumeration finds it. So the
     check is not vacuous.
   - The derived `<=` is provable: unfold `PartialOrd.le.trait_default`,
     `le_body`, `partial_cmp`, `cmp` and `Tier.read_discriminant`, then
     `decide`. It is clumsier than the `rank` match, which needs one
     `simp` plus `decide`.
3. **`checked_add`, `checked_sub`, `saturating_sub`, `Option::ok_or`: all
   modelled, none opaque.**
   - In the pinned library, `U64.checked_add` is a `def` with
     `checked_add_bv_spec`, `ok_or` is an `@[expose] def` with `simp`
     lemmas, and `UScalar.saturating_sub` is a `def`.
   - `apply_deposit_spec` and `deposit_ok` use them and audit to the
     standard three axioms.
4. **Does the shadow workspace work with `resolver = "3"` and inherited
   `rust-version`? Yes, after two changes to writfold's script.**
   - Copy path dependencies at their repository-relative paths.
   - Take the resolver from the root instead of hard-coding `"2"`.
   - The shadow root carries `edition`, `license`, `rust-version`,
     `version` (the union of what the three crates inherit). The 2026-09-17
     nightly accepts `rust-version = "1.98"` and edition 2024.
5. **Gate time.** Measured locally on the 32-CPU host, using the reused
   image, whose mathlib and Aeneas `.lake` are prebuilt.
   - Cold container (empty cargo registry, cold Charon sysroot): 35 s in
     total. Extraction, including the crates.io download, took 23 s;
     `lake build` with the audit took 11 s.
   - Warm, same container: 7 s.
   - Each isolated probe item took 5 to 9 s.
   - Not measured:
     - Building the image from scratch. It is about 19.8 GB uncompressed,
       Charon is compiled from source and the mathlib cache is fetched.
     - **The time on a 2-CPU GitHub runner, cold or warm. It is unknown**
       and is measured in P1.

## Decisions

- **Closure-free macro expansion is not needed for extraction or proof.**
  The real `#[refined]` and `#[requires]`/`#[ensures]`/`old` expansions
  translate, build and prove, given
  `include = ["tauten::contract", "tauten::error"]` in
  `[package.metadata.charon]`.
  - The hand-written closure-free versions translate too, but they still
    need the same `include`, because of `RefineError` and
    `ContractViolation`. So the expansion strategy does not decide
    extractability; the `include` does.
  - Plan section 4's in-fragment closure-free expansion can still be done
    for its own reasons (the `provable` classification), but P3 does not
    depend on it.
- **Derived impls are visible**, as long as the generated module (not only
  `try_new`) is in `start_from`. So F01 target 5 (constructor enumeration)
  is feasible as planned. P0 has a working prototype in `Audit.lean`.
  - Pitfall: Lean's `Name.isInternal` treats the macro's `__tauten_percent`
    module as internal. A filter on `isInternal` hides every refined
    constructor. The first version of the audit reported an empty list
    because of it.

## Other findings

- **Macro bug (product code, not fixed here).** In `#[ensures]`, the
  condition text is quoted *after* `old(...)` has been rewritten. The
  violation message therefore reads `| after : & u64 | * after >
  __tauten_old_0` instead of `old(balance)` (see the string in
  `Extracted/TautenProbe.lean`). The fix is to compute `text` before
  `snapshots.visit_expr_mut`.
- **Refined types are transparent in Lean.** Aeneas emits `Percent` as
  `@[reducible] def … := Std.U8`. The constructor audit therefore reads the
  unreduced signature. Theorems must state privacy-dependent facts (for
  example "every `Percent` is ≤ 100") over the constructors. A type
  invariant would not work.
- **Not probed, left for P3:**
  - contracts on functions with `&mut` parameters (the body closure would
    capture a mutable borrow);
  - bodies that use `?` or `return`;
  - predicates on `String`/`Vec`;
  - async.

## Commands run

```sh
git -C ~/workspace/tauten worktree add ../tauten-p0 -b proofs-p0
docker images | grep proofs
docker run --rm writfold-proofs-deps:dev sh -c 'charon version; aeneas -version; lean --version; rustc --version'
docker run -d --name tauten-p0-dev -v <worktree>:/repo:ro writfold-proofs-deps:dev sleep infinity
docker exec tauten-p0-dev sh proofs/probe.sh /tmp/pf                      # per item (in a copy of /repo)
docker exec tauten-p0-dev sh proofs/extract.sh proofs/probe /tmp/fresh -- -abort-on-error
docker exec tauten-p0-dev sh -c 'cd /work/proofs && lake build'           # theorems + Audit
docker run --rm -v <worktree>:/repo:ro writfold-proofs-deps:dev \
  bash -c 'cp -r /repo/. /work/ && cd /work && sh proofs/gate.sh && sh proofs/gate.sh'   # cold, then warm
docker run --rm --network none … sh proofs/gate.sh                        # fails: crates.io unreachable
rustfmt --edition 2024 --check proofs/probe/src/lib.rs
```

## Next: P1

1. Create `crates/tauten-check-kernel` (`no_std`, no dependencies) with
   `rank`/`may_depend_on`, `severity` and `exception`, and make
   `tauten-check` call it.
2. Write a tauten `proofs/Dockerfile`: writfold's stages unchanged, plus a
   `gate` stage that copies the tauten crates. For the macro-instance crate
   in P3, vendor the crates so `--network none` works.
3. Add `mutants.py` with at least 4 mutants and `.github/workflows/proofs.yml`.
   Measure the 2-CPU runner time there.
4. Keep `extract.sh` as generalised here. Drop or keep `proofs/probe` once
   the kernel exists.
