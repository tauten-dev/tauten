// writfold's verification wiring today, and with tauten.
// Render: typst compile docs/diagrams/writfold-wiring.typ --format svg
#set page(width: 300mm, height: auto, margin: 10mm, fill: rgb("#fbf8f1"))
#set text(font: ("Libertinus Serif", "New Computer Modern"), size: 9pt, fill: rgb("#2b2621"))
#show raw: set text(font: ("DejaVu Sans Mono", "Liberation Mono"), size: 7.6pt)

#let ink = rgb("#2b2621")
#let muted = rgb("#6f665c")
#let paper = rgb("#fffdf8")
#let warn = rgb("#a4452c")
#let ok = rgb("#3f6b45")
#let blue = rgb("#2f5d86")

#let box-(title, body, stroke: ink, fill: paper, tag: none) = block(
  width: 100%, inset: 7pt, radius: 3pt, stroke: 0.8pt + stroke, fill: fill, breakable: false,
)[
  #text(weight: "bold", title)
  #if tag != none [ #h(1fr) #text(size: 7.5pt, fill: stroke, tag) ]
  #v(-2pt)
  #set text(size: 8.2pt)
  #body
]
#let arrow(label: none) = align(center, text(fill: muted, size: 8pt)[↓ #if label != none [#label]])
#let gone(x) = text(fill: warn)[#strike(x)]
#let pain(x) = text(fill: warn, size: 7.8pt)[▲ #x]
#let win(x) = text(fill: ok, size: 7.8pt)[● #x]

#align(center, text(size: 15pt, weight: "bold")[writfold: verification wiring, today and with tauten])
#align(center, text(fill: muted)[Line counts and timings from the writfold main checkout and the `tauten-adoption` worktree, 2026-11-19.])
#v(4pt)

#grid(columns: (1fr, 1fr), column-gutter: 12pt,
[
  #align(center, text(size: 12pt, weight: "bold")[Today])
  #box-([Entry: `scripts/gate <command>`],
    [Python, about 1,140 lines (`writfold_gate`, `_evidence`, `_quality`, `_preflight`, `_tools`). Admits the tool image by digest, runs one service, writes a receipt outside the checkout.])
  #arrow()
  #grid(columns: (1fr, 1fr), column-gutter: 6pt,
    box-([Image A: tools], [
      `docker/Dockerfile.tools`, pinned in `toolchains.lock.json`:\
      Rust 1.98 · Verus + Z3 · Lean 4.21 · TLC 1.7.4 · TLAPS\
      Services: `quality` (own 537-line lint script, knows `verus!` syntax) · `unit` · `build` (musl) · `reference`
      #pain[Verus, TLAPS and Lean 4.21 serve archived layers only]
    ], stroke: muted),
    box-([Image B: proofs], [
      `proofs/Dockerfile`, `compose.proofs.yaml`, about 20 GB:\
      Lean 4.35 · Aeneas · Charon (built from source)\
      `proofs/gate.sh`:\
      `extract.sh` → drift diff → `lake build` → `Audit.lean` (449) → `mutants.py` (1,332; 236 mutants)
      #pain[105 min; any kernel edit re-runs everything]
    ], stroke: muted),
  )
  #arrow()
  #grid(columns: (1fr, 1fr), column-gutter: 6pt,
    box-([TLA+ (`spec/`)], [
      `KbWrite.tla`, `RunGraph.tla`, 16 mutant and 8 probe configs
      #pain[no runner wired into the gate]
    ], stroke: muted),
    box-([Archived, still in the tree], [
      Verus in `writfold-domain` (39 files); the Lean calculus (E01–E12, T01–T22)
      #pain[unchecked code]
    ], stroke: muted),
  )
  #arrow(label: [adoption worktree adds])
  #box-([tauten (worktree, not merged)], [
    `cargo tauten check` + baseline (527) · `proofs/claims.toml` (151 claims, 142 findings subsumed)\
    Adapters: `tauten-gate.sh`, `tauten_gate.py`, `Total.lean`, `tauten_claims.py`
    #pain[a seventh copy of gate glue; two mutant semantics]
  ], stroke: blue)
  #v(4pt)
  #text(size: 8pt, fill: muted)[Same gate machinery is forked in gripsack, strop, weft, splicery and tauten: six copies.]
],
[
  #align(center, text(size: 12pt, weight: "bold")[With tauten])
  #box-([Every edit: `cargo tauten check`], [
    Stable Rust, seconds, no Docker. Profile lints · tiers (kernel / core / edge) · kernel proof-shape rules · declared effects vs signatures · claim freshness (stale claims show as *pending*, not errors)
    #win[the agent learns "this will not extract" in seconds]
  ], stroke: ok, tag: [every change])
  #arrow()
  #box-([End of feature: `cargo tauten ship`], [
    Re-extract · re-prove *stale claims only* (per-claim fingerprint) · mutants for changed functions · property tests generated from refinements and contracts · quick TLC configs
    #win[target under 15 min, down from 105]
  ], stroke: ok, tag: [before "done"])
  #arrow()
  #box-([Nightly / merge: `cargo tauten verify --level full`], [
    Every claim from clean · every mutant · all TLC configs: writfold's 16 mutant configs become calibrated `invariant` claims (labelled *bounded*) · receipt written
  ], stroke: ok, tag: [scheduled])
  #arrow(label: [all run in])
  #box-([One public image: `ghcr.io/tauten-dev/proofs`], [
    Lean · Aeneas · Charon · TLC · `tauten-gate` · audit library (axioms, required claims, total shape) · shared kernel lemmas. Pinned by digest in `tauten-toolchain.lock`.
  ], stroke: blue)
  #v(6pt)
  #grid(columns: (1fr, 1fr), column-gutter: 6pt,
    box-([writfold keeps], [
      Theorems (`proofs/Writfold/`) · `mutants.toml` · `claims.toml` · `spec/*.tla` + `tla.toml` · tiers in `Cargo.toml`\
      Code: `writfold-core` kernel (proved) · `writfold-kb` core with capabilities, effects, evidence · `writfold-app` edge
    ], stroke: ok),
    box-([writfold deletes], [
      #gone[`extract.sh`] #gone[`gate.sh`] #gone[`refresh.sh`] #gone[`Audit.lean`] #gone[`mutants.py`] #gone[`proofs/Dockerfile`] #gone[tauten adapters] #gone[Verus/TLAPS layers of `Dockerfile.tools`]\
      Decide separately: archived Verus code; `writfold_evidence.py` vs `verify --receipt`
    ], stroke: warn),
  )
])

#v(6pt)
#line(length: 100%, stroke: 0.5pt + muted)
#text(size: 8pt, fill: muted)[
  Labels: *proved* = Lean theorem over the Aeneas translation of the shipped code, axiom-audited, calibrated by mutants. *bounded* = TLC or exploration up to a bound; never counts as proved. *checked* = lint or type rule.
]
