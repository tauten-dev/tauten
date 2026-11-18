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
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
PROOFS = ROOT / "proofs"
KERNEL = ROOT / "crates" / "tauten-check-kernel"


@dataclass(frozen=True)
class Mutant:
    name: str
    file: str
    before: str
    after: str
    fails: tuple[str, ...]


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
) + RATCHET_MUTANTS

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


def calibrate(mutant: Mutant, scratch: Path) -> None:
    crate = scratch / "crate"
    shutil.copytree(KERNEL, crate, ignore=shutil.ignore_patterns("target"))
    source = crate / "src" / mutant.file
    text = source.read_text()
    if text.count(mutant.before) != 1:
        raise SystemExit(f"FAIL {mutant.name}: mutation site does not match exactly once")
    source.write_text(text.replace(mutant.before, mutant.after))

    extraction = run(["sh", str(PROOFS / "extract.sh"), str(crate), str(scratch / "Extracted"),
                      "--", "-abort-on-error"], ROOT,
                     {"EXTRACT_AS": str(KERNEL.relative_to(ROOT))})
    if extraction.returncode != 0:
        print(extraction.stdout)
        raise SystemExit(f"FAIL {mutant.name}: the mutated kernel did not translate")

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
        raise SystemExit(f"FAIL {mutant.name}: the proofs still build against the mutated kernel")
    failures, unattributed = failing_theorems(build.stdout, project)
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
            try:
                calibrate(mutant, Path(directory))
            except SystemExit as error:
                print(error, flush=True)
                failed.append(mutant.name)
    if failed:
        raise SystemExit(f"FAIL: {len(failed)} mutant(s) not calibrated: {failed}")
    print(f"PROOF_CALIBRATIONS={len(mutants)}", flush=True)


if __name__ == "__main__":
    main()
