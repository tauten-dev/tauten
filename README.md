<p align="center"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/brand/tauten-icon-dark.svg"><img src="assets/brand/tauten-icon.svg" width="112" alt="tauten logo"></picture></p>

# tauten

**A strictness profile for Rust written by AI agents.**

Agents rarely get memory safety wrong in Rust: the compiler stops them. What
gets through is everything else: `unwrap` on input, unchecked arithmetic,
`todo!()` left behind, errors silently dropped, a `_ =>` arm that swallows a
new variant, library code that quietly reads the clock or the filesystem.
tauten turns those into compiler-checked rules, sorted by how much authority
a crate holds, and reports each one with a stable ID and a repair hint an
agent can act on.

> **Status: pre-release.** Not published yet, no website yet, and no
> benchmark numbers yet. What exists and what has been exercised is listed in
> [docs/STATUS.md](docs/STATUS.md). The plan is [docs/PLAN.md](docs/PLAN.md).

## How it works

Each crate declares a **tier** in its manifest:

```toml
[package.metadata.tauten]
tier = "core"   # "kernel" | "core" | "edge"
```

| Tier | Meaning | Examples of what is rejected |
|---|---|---|
| `kernel` | Pure, `no_std`, deterministic decisions; eligible for proofs | any exception to a profile rule, floats, `HashMap`, `Rc`/`RefCell`, outward dependencies |
| `core` | Library code; effects arrive as parameters | `std::fs::read`, `SystemTime::now`, `println!`, detached threads, globals, `unwrap`, indexing, unchecked `+` |
| `edge` | Startup and IO; the only tier that may create authority | `unwrap`, ignored results, stubs (most other rules become warnings) |

Then:

```console
$ cargo tauten check                 # human output
$ cargo tauten check --format json   # one diagnostic per line, stable schema
$ cargo tauten explain TAU-A001      # why the rule exists and how to fix it
```

```text
error[TAU-A001]: use of a disallowed method `std::fs::read_to_string`
  --> crates/config/src/lib.rs:12:5 (core crate `config`)
  = rule: No ambient authority (enforced by Clippy)
  = fix: Take a capability parameter (for example a `&cap_std::fs::Dir` from `Authority::dir`, `&dyn tauten::cap::Clock`, or `&tauten::cap::Env`) and let an edge crate construct it.
```

The CLI owns the profile: it runs one `cargo check` with itself as the
workspace rustc wrapper, adding each crate's tier lints and a generated clippy
configuration. Deleting or loosening a project's own `clippy.toml` or `[lints]`
table does not weaken the check. Exceptions are possible only as
`#[expect(lint, reason = "...")]` on the smallest item, and never for stubs,
`unsafe` outside the edge, or anything in a kernel crate.

Every rule says what enforces it (rustc, clippy, a tauten structural check,
or convention). The full table, including what Rust **cannot** enforce, is in
[docs/MAPPING.md](docs/MAPPING.md).

## Proofs

tauten's own decisions (tier order, severity, lint exceptions, the
baseline ratchet, rule-table validation, proof subsumption) live in the
`no_std` crate `tauten-check-kernel`. Lean 4 theorems are stated over its
Charon/Aeneas translation, audited to Lean's three standard axioms and
calibrated with source mutants that must break them; the code the macros
generate is proved for a list of instances. What is proved and what is
trusted: [docs/proofs/README.md](docs/proofs/README.md),
[proofs/TRUST.md](proofs/TRUST.md). This is not "formally verified
tauten": scanning, diagnostic mapping and rendering are tested, not proved.

The same setup is a template for your own kernel crates:

```console
$ cargo tauten init --proofs my-kernel --write   # writes proofs/ (claims.toml, gate, Lean project)
$ sh proofs/refresh.sh                           # translate the crate (Docker, pinned image)
$ cargo tauten proofs                            # run the gate; one JSON outcome per claim
{"crate":"my-kernel","function":"apply","theorems":["Proofs.MyKernel.apply_spec"],…,"outcome":"proved",…}
```

Outcomes: `proved`, `drift`, `build-failed`, `audit-failed`,
`mutant-survived`, `tool-missing` (without Docker or the image nothing is
checked and nothing passes). The results are recorded with a fingerprint of
the sources in `proofs/claims.json`. In a kernel crate, a function proved
never to fail (`∀ args, f args ⦃ _ ⦄`, no hypotheses) whose claim lists
`subsumes = ["TAU-N001", "TAU-P003"]` has its arithmetic and indexing
findings reported as "subsumed by proof <theorem>" instead of as errors,
until the crate or the proofs change. Worked example:
[bench/hosts/ledger/proofs](bench/hosts/ledger/proofs).

## Develop

Everything that gates a commit runs in Docker:

```console
$ docker compose --profile build build tools
$ docker compose run --rm gate      # fmt, build, tests, fixture suite, self-check, cargo-deny
$ docker compose run --rm mutants   # mutation testing of the pure analysis crate
```

Layout:

| Path | Contents |
|---|---|
| `crates/tauten-check` | Pure analysis (tier `core`): the rule table (`rules.toml`), the agent profile (`agents.md`), lint flags, clippy config generation, diagnostic mapping, structural checks |
| `crates/tauten-check-kernel` | The checker's decisions in plain `no_std` Rust (tier `kernel`), proved in `proofs/` |
| `crates/cargo-tauten` | The CLI (tier `edge`): workspace loading, the rustc wrapper, output, `cargo tauten proofs` |
| `crates/cargo-tauten/templates/proofs` | The proof-tier template `cargo tauten init --proofs` writes |
| `proofs/` | Lean proofs over the kernel and the macros' instances; `claims.toml`/`claims.json` (badge data); `TRUST.md` |
| `fixtures/cases` | One crate per rule, each listing the exact rule IDs it must produce; three accept crates must produce none |
| `docs/` | Plan, guidelines, mapping, status, dry-run reports |
| `crates/tauten-check/agents.md` | The agent instructions adopting projects include (written by `cargo tauten init`) |

tauten checks itself with its own profile on every gate run.

## License

MIT. See [LICENSE](LICENSE).
