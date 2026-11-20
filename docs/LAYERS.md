# Declared layers inside a tier (B6, TAU-T004)

Tiers (kernel, core, edge) are the trust boundary and fixed. Inside them a
workspace may declare an order of named layers; a crate names its layer and
may depend on a crate of the same or an earlier layer. The decision is the
proved `tauten_check_kernel::layer::finding` (`proofs/Tauten/Layer.lean`:
the order over positions is reflexive, transitive, antisymmetric and total,
and an unknown or missing declaration is never clean).

```toml
# workspace Cargo.toml
[workspace.metadata.tauten]
layers = ["foundation", "pure", "io", "kb", "front", "bin", "tools"]

# a member's Cargo.toml
[package.metadata.tauten]
tier = "core"
layer = "io"
```

## One existing project as configuration: writfold

Writfold enforces its crate graph with a hand-written allowlist
(`crates/writfold-xtask/src/graph.rs`, `allowed`). As layers:

| Layer | Crates | Writfold's allowlist says they may use |
|---|---|---|
| `foundation` | `writfold-domain`, `writfold-core` | nothing |
| `pure` | `tuft` | `writfold-domain` |
| `io` | `writfold-sources`, `writfold-runtime` | `writfold-domain` |
| `kb` | `writfold-kb` | domain, sources, runtime, core, tuft |
| `front` | `writfold-app`, `writfold-tui` | `writfold-kb` only |
| `bin` | `writfold` | `writfold-app`, `writfold-tui` |
| `tools` | `writfold-xtask` | nothing |

Every edge writfold allows points to an earlier layer, so TAU-T004 accepts
writfold's graph and rejects every upward or sideways-into-a-later-layer
edge (a cycle cannot be written). What layers do **not** express: the
allowlist is stricter than the order. `writfold-app` may use only
`writfold-kb`, but the layer order would also let it use `writfold-domain`;
`writfold-sources` may not use `tuft`, but `io` comes after `pure`. Those
are edge rules, and the roadmap's no-fit exit for B6 is explicit: keep
layers a linear order and do not add edge lists. A project that needs them
keeps its own check for the remaining edges.
