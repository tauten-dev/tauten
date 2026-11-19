#!/usr/bin/env python3
"""Mutant calibration for the proofs gate (docs/proofs/README.md).

Adapted from writfold's proofs/mutants.py. Every translated kernel function
gets at least one semantic mutation of its Rust source. For each: copy the
crate, apply the mutation, translate it exactly as the gate does
(proofs/extract.sh), build the theorems against the mutated translation in a
scratch copy of the lake project, and require the build to FAIL so that
exactly the named claims are broken: no other claim may break, and the
translation itself must still type-check. A proof that survives its mutant
proves nothing; a failure anywhere else is noise, not calibration.

Macro mutants (P3) mutate crates/tauten-macros/src/lib.rs instead: the
instance crate crates/tauten-proof-instances is translated through the
mutated macros (in a scratch copy of the crates it reaches), and the claims
about the macros' expansion must break. A mutant with `audit` set must
instead fail the constructor audit in Audit.lean, with that text, and break
no claim: a refined value built outside `try_new` is caught by the
enumeration, not by a theorem.

A claim is broken when its own proof fails, or when it depends (through
Lean's axiom closure: the failed proof becomes `sorryAx`) on a lemma whose
proof fails. The ratchet claims (P2) are proved from shared refinement
lemmas (`apply_spec`, `stale_spec`, ...), so a mutant of a ratchet function
breaks every claim proved through that function's lemma, and such helper
lemmas may fail as long as only the named claims break. To find the
dependents, a reporter is appended to every theorem file of the scratch
copy; it logs `SORRY_DEPENDENT <name>` for each theorem whose axioms
include `sorryAx`.

    python3 proofs/mutants.py [mutant-name ...]
"""
from dataclasses import dataclass
from pathlib import Path
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
PROOFS = ROOT / "proofs"
KERNEL = ROOT / "crates" / "tauten-check-kernel"
INSTANCES = ROOT / "crates" / "tauten-proof-instances"
MACROS = ROOT / "crates" / "tauten-macros"
# The crates the instance crate reaches through `path` dependencies.
INSTANCE_TREE = ("tauten-proof-instances", "tauten", "tauten-macros")


@dataclass(frozen=True)
class Mutant:
    name: str
    file: str
    before: str
    after: str
    fails: tuple[str, ...]
    # "kernel": `file` is in crates/tauten-check-kernel/src; "macros": in
    # crates/tauten-macros/src, and the instance crate is re-translated.
    crate: str = "kernel"
    # Expected text of an Audit.lean error, for a mutant only the audit catches.
    audit: str | None = None


T = "Tauten.Kernel."

# Every ratchet claim proved through `apply_spec`.
APPLY = (T + "apply_covered_iff", T + "apply_cover_count", T + "apply_suppressed_le_accepted",
         T + "apply_left_spec", T + "ratchet_stale_iff", T + "ratchet_passes_iff",
         T + "ratchet_headroom", T + "ratchet_example")
# Every ratchet claim proved through `stale_spec`.
STALE = (T + "ratchet_stale_iff", T + "ratchet_passes_iff", T + "ratchet_example")
# Every ratchet claim proved through `count_errors_spec`.
COUNT = (T + "count_errors_counts", T + "regenerated_baseline_never_grows")
# Every ratchet claim proved through `increases_spec`.
INCREASES = (T + "increases_flags", T + "increases_none_iff",
             T + "regenerated_baseline_never_grows", T + "increase_example")

RATCHET_MUTANTS = (
    # ratchet: warnings covered by the baseline
    Mutant("warnings-covered", "ratchet.rs",
           "    if !finding.error {\n        return false;\n    }\n    match left.get_mut",
           "    match left.get_mut",
           APPLY),
    # ratchet: an entry with nothing left still covers (and never runs out)
    Mutant("headroom-ignored", "ratchet.rs",
           "        Some(slot) if *slot > 0 => {",
           "        Some(slot) if *slot >= 0 => {",
           APPLY),
    # ratchet: covering a finding does not use up the entry
    Mutant("cover-not-consumed", "ratchet.rs",
           "            *slot = slot.saturating_sub(1);\n            true",
           "            true",
           APPLY),
    # ratchet: covered findings get no flag, so the flags no longer line up
    # with the findings (the only mutant that breaks R-conserve)
    Mutant("covered-flag-dropped", "ratchet.rs",
           "        let hit = take_at(&mut left, findings, i);\n        covered.push(hit);",
           "        let hit = take_at(&mut left, findings, i);\n        if !hit {\n            covered.push(hit);\n        }",
           (T + "apply_conserves",) + APPLY),
    # ratchet: an entry with one finding to spare is not stale
    Mutant("stale-needs-two", "ratchet.rs",
           "        if count_at(left, k) > 0 {\n            keys.push(k);",
           "        if count_at(left, k) > 1 {\n            keys.push(k);",
           STALE),
    # ratchet: a new baseline counts warnings too
    Mutant("warnings-counted", "ratchet.rs",
           "    if finding.error\n        && let Some(slot) = counts.get_mut(finding.key)",
           "    if let Some(slot) = counts.get_mut(finding.key)",
           COUNT),
    # ratchet: an entry that stays the same counts as growth
    Mutant("equal-is-growth", "ratchet.rs",
           "        Some(entry) => count_at(old, entry.key) < entry.count,",
           "        Some(entry) => count_at(old, entry.key) <= entry.count,",
           INCREASES),
    # ratchet: a key the old baseline lacks counts as accepting one finding,
    # so a new entry of count 1 is not reported as growth
    Mutant("missing-key-accepts-one", "ratchet.rs",
           "        Some(count) => *count,\n        None => 0,",
           "        Some(count) => *count,\n        None => 1,",
           STALE + INCREASES),
)

# Every rule-table claim: the examples are proved through `validate_ok_iff`.
TABLE = (T + "validate_ok_iff", T + "validate_examples")

TABLE_MUTANTS = (
    # table: two rules may share an ID
    Mutant("duplicate-ids-accepted", "table.rs",
           "    if let Some((_, rule)) = first_duplicate(ids) {",
           "    if let Some((_, rule)) = None::<(usize, usize)> {",
           TABLE),
    # table: a lint may be owned twice when no ID repeats
    Mutant("duplicate-lints-accepted", "table.rs",
           "    if let Some((first, second)) = lint {\n        return Some(Invalid::DuplicateLint(first, second));\n    }\n",
           "",
           TABLE),
    # table: an API ban may name a rule that does not exist
    Mutant("unknown-api-rule-accepted", "table.rs",
           "    if let Some(api) = first_missing(ids, api_rules) {\n        return Err(Invalid::UnknownRule(api));\n    }\n",
           "",
           TABLE),
    # table: a required ID may be missing (a table without TAU-R001 passes)
    Mutant("required-id-missing-accepted", "table.rs",
           "        Some(id) => Err(Invalid::MissingRule(id)),\n        None => Ok(()),",
           "        Some(_) | None => Ok(()),",
           TABLE),
    # table: the search stops one position short, so the last rule ID is
    # never found and an adjacent repeat is never seen
    Mutant("search-stops-short", "table.rs",
           "    while i < end {",
           "    while i.saturating_add(1) < end {",
           TABLE),
)

SUBSUME_MUTANTS = (
    # subsume: a proof recorded for another source still silences the lint
    Mutant("stale-proof-subsumes", "subsume.rs",
           "    kernel && kind && listed && proved && fresh",
           "    kernel && kind && listed && proved",
           (T + "subsumes_spec", T + "stale_proof_never_subsumes")),
    # subsume: core crates are subsumed too
    Mutant("core-subsumed", "subsume.rs",
           "        Tier::Core | Tier::Edge => false,\n    };\n    kernel",
           "        Tier::Core => true,\n        Tier::Edge => false,\n    };\n    kernel",
           (T + "subsumes_spec", T + "subsumes_only_in_kernel")),
    # subsume: every rule is subsumed, not only arithmetic and indexing
    Mutant("any-finding-subsumed", "subsume.rs",
           "        Finding::Other => false,",
           "        Finding::Other => true,",
           (T + "subsumes_spec", T + "subsumes_only_arithmetic_and_indexing")),
    # subsume: an unlisted rule is subsumed
    Mutant("unlisted-rule-subsumed", "subsume.rs",
           "    kernel && kind && listed && proved && fresh",
           "    kernel && kind && proved && fresh",
           (T + "subsumes_spec",)),
)

EFFECTS_MUTANTS = (
    # effects: a writable directory no longer grants writing
    Mutant("dir-not-write", "effects.rs",
           "        Source::Dir => (false, false, true, true, false),",
           "        Source::Dir => (false, false, true, false, false),",
           (T + "derive_spec", T + "f001_decision")),
    # effects: a read-only directory grants writing
    Mutant("readdir-writes", "effects.rs",
           "        Source::ReadDir => (false, false, true, false, false),",
           "        Source::ReadDir => (false, false, true, true, false),",
           (T + "derive_spec", T + "f001_decision")),
    # effects: a `Result` return no longer derives `fail`
    Mutant("fail-ignored", "effects.rs",
           "        fail: fallible,",
           "        fail: false,",
           (T + "derive_spec", T + "f001_decision")),
    # effects: a declared label nothing grants is accepted
    Mutant("agrees-ignores-extra", "effects.rs",
           "    is_empty(missing(declared, derived)) && is_empty(extra(declared, derived))",
           "    is_empty(missing(declared, derived))",
           (T + "agrees_spec", T + "f001_decision", T + "f001_examples", T + "verdict_spec", T + "f001_error_iff", T + "f001_silent_iff")),
    # effects: `missing` reports `net` the wrong way round
    Mutant("missing-net-swapped", "effects.rs",
           "        net: derived.net && !declared.net,",
           "        net: declared.net && !derived.net,",
           (T + "missing_spec", T + "extra_spec", T + "agrees_spec", T + "f001_decision",
            T + "f001_examples", T + "verdict_spec", T + "f001_error_iff", T + "f001_silent_iff")),
    # effects: an unclassified parameter type can block
    Mutant("unresolved-blocks", "effects.rs",
           "            } else if required && resolved {",
           "            } else if required {",
           (T + "verdict_spec", T + "f001_error_iff", T + "f001_silent_iff")),
    # effects: a missing declaration blocks even when effects are optional
    Mutant("undeclared-always-blocks", "effects.rs",
           """            if required {
                Verdict::Error
            } else {
                Verdict::Silent
            }""",
           """            Verdict::Error""",
           (T + "verdict_spec", T + "f001_error_iff", T + "f001_silent_iff")),
)

OUTCOME_MUTANTS = (
    # outcome: sources changing during the run no longer block "proved"
    Mutant("changed-ignored", "outcome.rs",
           "    } else if f.changed {\n        Outcome::Drift\n    } else {",
           "    } else if false {\n        Outcome::Drift\n    } else {",
           (T + "outcome_proved_iff", T + "outcome_examples")),
    # outcome: a surviving mutant is ignored
    Mutant("mutants-ignored", "outcome.rs",
           "    } else if !f.mutants {",
           "    } else if false {",
           (T + "outcome_proved_iff", T + "outcome_mutant_survived_iff")),
    # outcome: a stage that never reported counts as passed
    Mutant("absent-passes", "outcome.rs",
           "        Stage::Failed | Stage::Absent => false,",
           "        Stage::Failed => false,\n        Stage::Absent => true,",
           (T + "outcome_proved_iff", T + "outcome_tool_missing_iff",
            T + "outcome_mutant_survived_iff", T + "outcome_examples")),
)

SPAN_MUTANTS = (
    # span: a later span of equal width replaces the first
    Mutant("tie-takes-later", "span.rs",
           "        Some(current) if width(*current) <= width(*candidate) => best,",
           "        Some(current) if width(*current) < width(*candidate) => best,",
           (T + "innermost_spec",)),
    # span: the widest covering span wins
    Mutant("widest-wins", "span.rs",
           "        Some(current) if width(*current) <= width(*candidate) => best,",
           "        Some(current) if width(*current) >= width(*candidate) => best,",
           (T + "innermost_spec",)),
    # span: the first span is never considered
    Mutant("skips-first-span", "span.rs",
           "    let mut i: usize = 0;",
           "    let mut i: usize = 1;",
           (T + "innermost_spec",)),
    # span: a span ending before the line still covers it
    Mutant("covers-ignores-end", "span.rs",
           "    lines.0 <= line && line <= lines.1",
           "    lines.0 <= line",
           (T + "innermost_spec",)),
)

CONTROL_MUTANTS = (
    # outcome: a claim's mutants are not consulted from a run
    Mutant("of-run-mutants-ignored", "outcome.rs",
           "        mutants: admit(mutants, mutant_results),",
           "        mutants: true,",
           (T + "of_run_proved_iff",)),
    # control: any broken theorem counts as the named reason
    Mutant("named-any-broken", "control.rs",
           "            found = found || member(broken, *x);",
           "            found = found || !broken.is_empty();",
           (T + "named_iff",)),
    # control: a negative control that timed out counts as killed
    Mutant("timeout-counts-as-kill", "control.rs",
           "        (Expect::Pass, Seen::Pass) | (Expect::Fail, Seen::FailNamed) => true,",
           "        (Expect::Pass, Seen::Pass) | (Expect::Fail, Seen::FailNamed) => true,\n        (Expect::Fail, Seen::Timeout) => true,",
           (T + "control_ok_spec", T + "admit_spec", T + "admit_ok_iff", T + "admit_infrastructure_refused")),
    # control: a negative control that failed for another reason counts
    Mutant("unnamed-failure-accepted", "control.rs",
           "        (Expect::Pass, Seen::Pass) | (Expect::Fail, Seen::FailNamed) => true,",
           "        (Expect::Pass, Seen::Pass) | (Expect::Fail, Seen::FailNamed) => true,\n        (Expect::Fail, Seen::FailOther) => true,",
           (T + "control_ok_spec", T + "admit_spec", T + "admit_ok_iff", T + "admit_infrastructure_refused")),
    # control: a control observed twice is accepted
    Mutant("duplicate-accepted", "control.rs",
           "count_id(observed, o.id) == 1",
           "count_id(observed, o.id) >= 1",
           (T + "admit_spec", T + "admit_ok_iff", T + "admit_infrastructure_refused")),
    # control: an empty inventory is admitted
    Mutant("empty-inventory-accepted", "control.rs",
           "    !required.is_empty() && observations_ok(required, observed)",
           "    observations_ok(required, observed)",
           (T + "admit_spec", T + "admit_ok_iff", T + "admit_infrastructure_refused")),
    # control: a required control that was never observed is accepted
    Mutant("missing-observation-accepted", "control.rs",
           "            ok = ok && met(observed, c.id, c.expect);",
           "            ok = ok && (met(observed, c.id, c.expect) || count_id(observed, c.id) == 0);",
           (T + "admit_spec", T + "admit_ok_iff", T + "admit_infrastructure_refused")),
    # control: an observation of a control nobody required is accepted
    Mutant("unexpected-accepted", "control.rs",
           "            ok = ok && count_id(observed, o.id) == 1 && required_id(required, o.id);",
           "            ok = ok && count_id(observed, o.id) == 1;",
           (T + "admit_spec", T + "admit_ok_iff", T + "admit_infrastructure_refused")),
)

MUTANTS = (
    # tiers: a tier may depend on its own tier; `<` refuses same-tier edges
    Mutant("same-tier-refused", "tiers.rs",
           "    rank(dep) <= rank(of)",
           "    rank(dep) < rank(of)",
           (T + "may_depend_on_spec", T + "may_depend_on_refl", T + "may_depend_on_total",
            T + "kernel_depends_only_on_kernel")),
    # tiers: one step outward allowed, so kernel -> core -> edge chains but
    # kernel -/-> edge: the relation stops being transitive
    Mutant("one-step-outward", "tiers.rs",
           "    rank(dep) <= rank(of)",
           "    rank(dep) <= rank(of)\n        || (rank(of) == 0 && rank(dep) == 1)\n        || (rank(of) == 1 && rank(dep) == 2)",
           (T + "may_depend_on_spec", T + "may_depend_on_trans", T + "may_depend_on_antisymm",
            T + "kernel_depends_only_on_kernel")),
    # tiers: core and edge swapped. Still a total order, so only the
    # statements that pin the intended order notice.
    Mutant("core-edge-swapped", "tiers.rs",
           "        Tier::Core => 1,\n        Tier::Edge => 2,",
           "        Tier::Core => 2,\n        Tier::Edge => 1,",
           (T + "rank_spec", T + "may_depend_on_spec")),
    # tiers: kernel and core share a rank, so a kernel crate may depend on core
    Mutant("kernel-ranks-as-core", "tiers.rs",
           "        Tier::Kernel => 0,",
           "        Tier::Kernel => 1,",
           (T + "rank_spec", T + "may_depend_on_spec", T + "may_depend_on_antisymm",
            T + "kernel_depends_only_on_kernel")),
    # severity: warn blocks
    Mutant("warn-blocks", "severity.rs",
           "        Level::Warn => false,",
           "        Level::Warn => true,",
           (T + "is_blocking_spec", T + "severity_spec", T + "exception_spec")),
    # severity: no level means error, ignoring the compiler's own severity
    Mutant("no-level-is-error", "severity.rs",
           "        None => fallback,",
           "        None => Severity::Error,",
           (T + "severity_spec",)),
    # exceptions: the kernel clause dropped, so kernel crates excuse levelled rules
    Mutant("kernel-excuses", "exception.rs",
           "            Tier::Kernel => false,",
           "            Tier::Kernel => true,",
           (T + "permitted_spec", T + "exception_spec", T + "kernel_never_excuses_levelled_rule")),
    # exceptions: the rule's `exceptions = false` ignored
    Mutant("rule-flag-ignored", "exception.rs",
           "    if !rule_allows {\n        return false;\n    }\n",
           "",
           (T + "permitted_spec", T + "exception_spec", T + "no_exceptions_never_excused")),
    # exceptions: any crate-wide exception reported, blocking or not
    Mutant("crate-wide-any", "exception.rs",
           "    if crate_wide && blocking {",
           "    if crate_wide || blocking {",
           (T + "exception_spec",)),
) + RATCHET_MUTANTS + TABLE_MUTANTS + SUBSUME_MUTANTS + EFFECTS_MUTANTS + OUTCOME_MUTANTS + SPAN_MUTANTS + CONTROL_MUTANTS

I = "Tauten.Instances."
REFINED_TYPES = ("percent", "port", "offset", "count")
# Every refined claim proved through a `*_try_new_shape` theorem.
REFINED_SHAPE = tuple(I + f"{ty}_{claim}" for ty in REFINED_TYPES
                      for claim in ("try_new_shape", "try_new_ok_iff", "try_new_sound",
                                    "try_new_err_iff", "boundaries"))
TRY_FROM = tuple(I + f"{ty}_try_from_spec" for ty in REFINED_TYPES)
# Every contract claim proved through `deposit_shape`, `withdraw_shape`,
# `scale_shape` or `share_shape` (`contract_examples` uses the first three).
DEPOSIT = tuple(I + c for c in ("deposit_shape", "deposit_requires_iff", "deposit_never_ensures",
                                "deposit_ok", "deposit_overflow"))
WITHDRAW = tuple(I + c for c in ("withdraw_shape", "withdraw_requires_iff",
                                 "withdraw_requires_keeps_balance", "withdraw_never_ensures",
                                 "withdraw_ok", "withdraw_insufficient"))
SCALE = tuple(I + c for c in ("scale_shape", "scale_requires_iff", "scale_ensures_iff",
                              "scale_ok", "scale_overflow"))
SHARE = tuple(I + c for c in ("share_shape", "share_requires_skips_body", "share_requires_iff",
                              "share_ok"))
EXAMPLES = (I + "contract_examples",)
BODY = ('            #[allow(clippy::redundant_closure_call, reason = "the body runs in a closure '
        'so `return` stays local")]\n'
        '            let __tauten_result: #ret = (|| -> #ret #body)();\n')

MACRO_MUTANTS = (
    # refined: the check inverted, so try_new accepts exactly the invalid values
    Mutant("refined-check-inverted", "lib.rs",
           "if ::tauten::contract::holds(&value, #predicate) {",
           "if !::tauten::contract::holds(&value, #predicate) {",
           REFINED_SHAPE, crate="macros"),
    # refined: TryFrom wraps the raw value without the check
    Mutant("try-from-unchecked", "lib.rs",
           "                    Self::try_new(value)\n",
           "                    ::core::result::Result::Ok(Self(value))\n",
           TRY_FROM, crate="macros"),
    # refined: the generator adds a `Default` that builds a value without the
    # check (the `derive(Default)` bug class); only the constructor audit sees it
    Mutant("refined-default", "lib.rs",
           "    let conversions = if predicate.is_some() {\n        quote! {\n",
           "    let conversions = if predicate.is_some() {\n        quote! {\n"
           "            impl ::core::default::Default for #name {\n"
           "                fn default() -> Self { Self(::core::default::Default::default()) }\n"
           "            }\n",
           (), crate="macros", audit="constructor audit"),
    # newtype/refined: the user's attributes (derives among them) not forwarded
    # to the generated struct
    Mutant("attributes-dropped", "lib.rs",
           "            #(#attrs)*\n            pub struct #name(#inner);\n",
           "            pub struct #name(#inner);\n",
           tuple(I + f"{ty}_derived_spec" for ty in REFINED_TYPES) + (I + "tag_control_spec",),
           crate="macros"),
    # requires: the negation dropped, so exactly the valid calls are refused
    Mutant("requires-inverted", "lib.rs",
           "            if !(#cond) {",
           "            if (#cond) {",
           DEPOSIT + WITHDRAW + SCALE + SHARE + EXAMPLES, crate="macros"),
    # requires: no precondition check at all
    Mutant("requires-dropped", "lib.rs",
           "            #(#checks_pre)*\n            #(#takes)*\n",
           "            #(#takes)*\n",
           DEPOSIT + WITHDRAW + SCALE + SHARE + EXAMPLES, crate="macros"),
    # requires: checked only after the body has run
    Mutant("requires-after-body", "lib.rs",
           "            #(#checks_pre)*\n            #(#takes)*\n" + BODY,
           "            #(#takes)*\n" + BODY + "            #(#checks_pre)*\n",
           DEPOSIT + WITHDRAW + SCALE + SHARE + EXAMPLES, crate="macros"),
    # old(): the snapshot taken after the body, so `old(*balance)` sees its write
    Mutant("snapshot-after-body", "lib.rs",
           "            #(#takes)*\n" + BODY,
           BODY + "            #(#takes)*\n",
           WITHDRAW + EXAMPLES, crate="macros"),
    # ensures: no postcondition check at all
    Mutant("ensures-skipped", "lib.rs",
           "                #(#checks_post)*\n",
           "",
           DEPOSIT + WITHDRAW + SCALE + EXAMPLES, crate="macros"),
    # ensures: a postcondition violation reported as a precondition violation
    Mutant("ensures-reported-as-requires", "lib.rs",
           "                    ::tauten::ContractViolation::ensures(#function, #text),",
           "                    ::tauten::ContractViolation::requires(#function, #text),",
           DEPOSIT + WITHDRAW + SCALE + EXAMPLES, crate="macros"),
)
MUTANTS = MUTANTS + MACRO_MUTANTS

# lake reports `error: <path>.lean:<line>:<col>: <message>`
SORRY_DEPENDENT = re.compile(r"SORRY_DEPENDENT (?P<name>\S+)")
REPORTER = """

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  for entry in env.constants.map₂.toList do
    if (`Tauten).isPrefixOf entry.1 && entry.2.isThm then
      let axioms ← liftCoreM (Lean.collectAxioms entry.1)
      if axioms.contains ``sorryAx then
        logInfo m!"SORRY_DEPENDENT {entry.1}"
"""
ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):(?P<line>\d+):(?P<col>\d+):", re.M)
DECLARATION = re.compile(r"^(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def|abbrev|instance)\s+(?P<name>[^\s:(]+)", re.M)
NAMESPACE = re.compile(r"^namespace\s+(?P<name>\S+)", re.M)


def declarations(path: Path) -> list[tuple[int, str]]:
    """(line, fully qualified name) of every declaration in a theorem file."""
    namespace = ""
    found = []
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if match := NAMESPACE.match(line):
            namespace = match.group("name") + "."
            continue
        if line.startswith("end "):
            namespace = ""
            continue
        if match := DECLARATION.match(line):
            found.append((number, namespace + match.group("name")))
    return found


def attribute(file: Path, line: int) -> str | None:
    enclosing = None
    for start, name in declarations(file):
        if start <= line:
            enclosing = name
    return enclosing


def failing_theorems(output: str, project: Path) -> tuple[set[str], list[str]]:
    failures: set[str] = set()
    unattributed: list[str] = []
    for match in ERROR.finditer(output):
        file = (project / match.group("file")).resolve()
        try:
            relative = file.relative_to(project)
        except ValueError:
            unattributed.append(match.group(0))
            continue
        if relative.parts[0] != "Tauten" or not file.exists():
            unattributed.append(str(relative))
            continue
        name = attribute(file, int(match.group("line")))
        if name is None:
            unattributed.append(str(relative))
        else:
            failures.add(name)
    return failures, unattributed


def run(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, timeout=1800, env={**os.environ, **(env or {})})


def mutate(source: Path, mutant: Mutant) -> None:
    text = source.read_text()
    if text.count(mutant.before) != 1:
        raise SystemExit(f"FAIL {mutant.name}: mutation site does not match exactly once")
    source.write_text(text.replace(mutant.before, mutant.after))


def translate(mutant: Mutant, scratch: Path) -> Path:
    """Translate the crate the mutant affects; return the Lean file written."""
    if mutant.crate == "kernel":
        crate = scratch / "crate"
        shutil.copytree(KERNEL, crate, ignore=shutil.ignore_patterns("target"))
        mutate(crate / "src" / mutant.file, mutant)
        command = ["sh", str(PROOFS / "extract.sh"), str(crate), str(scratch / "fresh"),
                   "--", "-abort-on-error"]
        extraction = run(command, ROOT, {"EXTRACT_AS": str(KERNEL.relative_to(ROOT))})
        produced = scratch / "fresh" / "TautenCheckKernel.lean"
    elif mutant.crate == "macros":
        # A scratch copy of the crates the instance crate reaches, at their
        # repository paths, with proofs/extract.sh, so the translation of the
        # unchanged instance sources is placed exactly as the gate places it.
        tree = scratch / "tree"
        (tree / "proofs").mkdir(parents=True)
        for name in ("Cargo.toml", "Cargo.lock"):
            shutil.copy2(ROOT / name, tree / name)
        shutil.copy2(PROOFS / "extract.sh", tree / "proofs" / "extract.sh")
        for name in INSTANCE_TREE:
            shutil.copytree(ROOT / "crates" / name, tree / "crates" / name,
                            ignore=shutil.ignore_patterns("target"))
        mutate(tree / "crates" / "tauten-macros" / "src" / mutant.file, mutant)
        command = ["sh", str(tree / "proofs" / "extract.sh"),
                   str(tree / "crates" / "tauten-proof-instances"), str(scratch / "fresh"),
                   "--", "-abort-on-error"]
        extraction = run(command, tree)
        produced = scratch / "fresh" / "TautenProofInstances.lean"
    else:
        raise SystemExit(f"FAIL {mutant.name}: unknown crate {mutant.crate}")
    if extraction.returncode != 0 or not produced.exists():
        print(extraction.stdout)
        raise SystemExit(f"FAIL {mutant.name}: the mutated code did not translate")
    return produced


def emit(**fields) -> None:
    """A result line for `cargo tauten proofs` (proofs/report.py, P5)."""
    print("TAUTEN-PROOFS " + json.dumps(fields, sort_keys=True), flush=True)


def calibrate(mutant: Mutant, scratch: Path) -> None:
    produced = translate(mutant, scratch)
    # The committed translation of every crate, with the mutated one replaced.
    shutil.copytree(PROOFS / "Extracted", scratch / "Extracted")
    if not (scratch / "Extracted" / produced.name).exists():
        raise SystemExit(f"FAIL {mutant.name}: {produced.name} is not in proofs/Extracted")
    if (scratch / "Extracted" / produced.name).read_text() == produced.read_text():
        raise SystemExit(f"FAIL {mutant.name}: the mutation does not change the translation")
    shutil.copy2(produced, scratch / "Extracted" / produced.name)

    project = scratch / "proofs"
    project.mkdir()
    for name in ("lakefile.toml", "lean-toolchain", "lake-manifest.json", "Tauten.lean", "Audit.lean"):
        shutil.copy2(PROOFS / name, project / name)
    shutil.copytree(PROOFS / "Tauten", project / "Tauten")
    for theorems in (project / "Tauten").glob("*.lean"):
        theorems.write_text(theorems.read_text() + REPORTER)
    shutil.copytree(scratch / "Extracted", project / "Extracted")
    (project / ".lake").mkdir()
    os.symlink(PROOFS / ".lake" / "packages", project / ".lake" / "packages")

    build = run(["lake", "build"], project)
    if build.returncode == 0:
        raise SystemExit(f"FAIL {mutant.name}: the proofs still build against the mutated code")
    failures, unattributed = failing_theorems(build.stdout, project)
    if mutant.audit is not None:
        # Only the audit may fail, with the expected text, and no claim may break.
        if failures or any(entry != "Audit.lean" for entry in unattributed) or not unattributed:
            print(build.stdout)
            raise SystemExit(f"FAIL {mutant.name}: expected only Audit.lean to fail; "
                             f"theorems {sorted(failures)}, elsewhere {unattributed}")
        if mutant.audit not in build.stdout:
            print(build.stdout)
            raise SystemExit(f"FAIL {mutant.name}: Audit.lean failed without '{mutant.audit}'")
        print(f"calibration: {mutant.name} fails the audit ({mutant.audit})", flush=True)
        emit(mutant=mutant.name, status="killed", broken=[])
        return
    if unattributed:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: errors outside the theorem files: {unattributed}")
    dependent = set(SORRY_DEPENDENT.findall(build.stdout))
    broken = (failures | dependent) & claims()
    expected = set(mutant.fails)
    if not broken or not expected <= broken:
        print(build.stdout)
        print(f"failing: {sorted(failures)}; dependent: {sorted(dependent)}")
        raise SystemExit(f"FAIL {mutant.name}: expected {sorted(expected)} to break, got {sorted(broken)}")
    if broken - expected:
        print(build.stdout)
        raise SystemExit(f"FAIL {mutant.name}: unexpected broken claims {sorted(broken - expected)}")
    helpers = sorted(failures - claims())
    note = f" (failing lemmas: {', '.join(helpers)})" if helpers else ""
    print(f"calibration: {mutant.name} breaks {', '.join(sorted(broken))}{note}", flush=True)
    emit(mutant=mutant.name, status="killed", broken=sorted(broken))


def claims() -> set[str]:
    """The required claims of the axiom audit."""
    return set(re.findall(r"``(Tauten[\w.]+)", (PROOFS / "Audit.lean").read_text()))


def unclaimed() -> list[str]:
    """Theorems a mutant names that the axiom audit does not require: a
    rename would then pass the audit and be caught only here."""
    return sorted({name for m in MUTANTS for name in m.fails} - claims())


def main() -> None:
    if missing := unclaimed():
        raise SystemExit(f"FAIL: theorems named by mutants are not audited claims: {missing}")
    selected = set(sys.argv[1:])
    mutants = [m for m in MUTANTS if not selected or m.name in selected]
    if selected and len(mutants) != len(selected):
        raise SystemExit(f"unknown mutant(s): {sorted(selected - {m.name for m in mutants})}")
    # Every mutant runs even after a failure, so one gate run reports them all.
    failed: list[str] = []
    for mutant in mutants:
        with tempfile.TemporaryDirectory(prefix=f"mutant-{mutant.name}-") as directory:
            # Status: `survived` only when the proofs still build against the
            # mutant; `timeout` when a build ran out of time; `other` when
            # it failed for a reason other than the named theorems.
            try:
                calibrate(mutant, Path(directory))
            except subprocess.TimeoutExpired:
                print(f"FAIL {mutant.name}: timed out", flush=True)
                emit(mutant=mutant.name, status="timeout", broken=[])
                failed.append(mutant.name)
            except SystemExit as error:
                print(error, flush=True)
                status = "survived" if "still build" in str(error) else "other"
                emit(mutant=mutant.name, status=status, broken=[])
                failed.append(mutant.name)
    if failed:
        raise SystemExit(f"FAIL: {len(failed)} mutant(s) not calibrated: {failed}")
    print(f"PROOF_CALIBRATIONS={len(mutants)}", flush=True)


if __name__ == "__main__":
    main()
