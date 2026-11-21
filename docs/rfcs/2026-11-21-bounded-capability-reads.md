---
rfc: "2026-11-21-bounded-capability-reads"
title: "Require a byte limit when reading through ReadDir"
status: draft
implementation: not-started
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/tauten-dev/tauten/issues/64
milestone: "0.2"
supersedes: []
superseded_by: []
blocked_on:
  - "Owner decision on the breaking ReadDir signatures and Env attenuation"
---

# RFC: Require a byte limit when reading through ReadDir

## Decision

In 0.1, [`ReadDir::read` and `read_to_string`](https://github.com/tauten-dev/tauten/blob/v0.1.0/crates/tauten/src/cap.rs#L233-L263) read to EOF. They constrain *where* code may read, not *how much*. [#64](https://github.com/tauten-dev/tauten/issues/64) split the API from the budget arithmetic shipped in 0.1. Use the minor-version break to require a per-call limit, rather than add a lint or an optional bounded method that leaves the unbounded method in scope. Also let a caller pass on a subset of an already captured `Env`.

## Proof obligations and limits

No new kernel verdict. Use the shipped `tauten_check_kernel::budget::charge(left: u64, cost: u64) -> Option<u64>` for every chunk appended. [`charge_spec` and `charges_never_exceed_initial`](../../proofs/Tauten/Helpers.lean) establish that an accepted cost fits and that the spent amount plus the remainder equals the original budget; the existing `charge-saturates` mutant must still break the calibrated claims. Do not fork the arithmetic into the IO layer. A change to that decision would require extraction, audit and calibrated mutants before wiring.

Rust type-checking enforces the presence and unit of the argument. Reading the file, probing EOF, charging every append, UTF-8 decoding and filtering the snapshot are **tested**, not proved. The limit bounds bytes returned per call, not process memory, elapsed time, physical IO, all reads by the process, or authority obtained through a different handle. No new badge, assumption, or claim about an adopter.

## User-visible contract

```rust
// 0.1: let manifest = dir.read_to_string("manifest.toml")?;
let manifest = dir.read_to_string("manifest.toml", Limit::<Bytes>::new(65_536))?;
let config_env = env.only(&["MANIFEST_MODE"]); // consumes this snapshot
```

- Under `fs`, both `ReadDir` read methods require a `Limit<Bytes>` (explicit `u64`, no default). Success is the **whole** `Vec<u8>` or `String`, never a truncated prefix. Distinguish a typed `ReadError::TooLarge` from `ReadError::Io(std::io::Error)`; malformed in-limit UTF-8 remains `InvalidData`. A zero-byte file fits a zero limit. An oversize byte stream is refused before UTF-8 validation. `sub` and `exists` keep their existing semantics; a subdirectory does not restore an unbounded read.
- Under `std`, `Env::only(self, names: &[&str]) -> Env` intersects the existing snapshot with the supplied names, without re-reading the process environment. Missing names stay missing; duplicates do not matter. Repeated attenuation cannot restore a removed name. This does not erase the caller's other copies or avoid the original snapshot. `Authority::env` and the writable directory capability are unchanged.

## Implementation boundary and alternatives

Open once, then read in fixed-size chunks no larger than the remaining allowance. Charge the bytes actually read before appending. Once the allowance reaches zero, probe for **one** further byte on the same handle: EOF succeeds, a byte means `TooLarge`, an IO error stays an error. Do not compute `limit + 1`, allocate from a file's reported length, or first read an unbounded buffer and check its size afterwards. A metadata-only check is not sufficient for streams or a growing file. Convert to `String` only after the bounded read. Keep the existing kernel dependency optional under `fs` if added to `tauten`; `no_std` users should not pay for the reader.

An additive `read_bounded` keeps callers compiling but cannot give the advertised `ReadDir` guarantee. A global byte budget, path-specific policy, a process-wide allocator cap or a timeout is a different contract. For example, [Writfold's row-limited search](https://github.com/writfold-dev/writfold/issues/70) is not a file-byte limit, and [Gripsack's destination-read limitation](https://github.com/gripsack-dev/gripsack/issues/313) is not fixed by publishing this API. Neither adopter must take a dependency to ship Tauten.

## Compatibility, evidence and rollout

1. Owner reviews the signatures, error distinction and per-call (not cumulative) limit. The new calls are not available in 0.1; the removed unbounded calls must fail to compile in 0.2. No change to on-disk files or gate output.
2. Exercise both a real `ReadDir` and a short/chunked reader: empty/exact/one-over, a read error after a prefix, malformed UTF-8 under the cap, an oversized invalid byte stream, `u64::MAX` with a small file, and a path outside the capability. Check that an oversize stream consumes at most one probe byte past the cap and never returns partial data. Exercise `Env::only` on present, missing, duplicate and previously removed keys.
3. Show one before/after call and typed error handling in rustdoc and the minor-release migration notes. Run Tauten's normal gates; keep the old release docs versioned. If the break is declined, leave 0.1's guarantee explicit; do not describe an optional helper as removing unbounded reads.

## Decision log

- 2026-11-21: drafted for #64; the existing roadmap names the capability, but not the boundary cases or migration contract. No implementation or acceptance implied.
