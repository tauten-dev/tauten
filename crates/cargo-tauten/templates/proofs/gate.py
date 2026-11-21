#!/usr/bin/env python3
"""The proof gate of the tauten proofs template (`cargo tauten init --proofs`).

Runs inside the pinned proofs image (proofs/container.sh), from the
repository copy in /work. Reads proofs/claims.toml and runs, in order:

  tools    charon, aeneas, lake and python3 are on PATH
  extract  every crate of `[gate] crates` translated (Charon, then Aeneas
           with -abort-on-error) by proofs/extract.sh
  drift    the fresh translation equals the committed proofs/Extracted/
  build    `lake build Extracted Proofs` (the theorems)
  audit    `lake build Audit`: every theorem of the Proofs library depends
           only on propext, Classical.choice and Quot.sound; the audit also
           lists each theorem that states a function never fails, for every
           input (`∀ args, f args ⦃ _ ⦄`, no hypotheses), and the named
           assumptions each theorem's hypotheses mention (`premises`) or,
           for a witness, its conclusion establishes (`establishes`)
  mutants  every `[[mutant]]` applied to a copy of its crate, translated
           and built against the theorems: it must break at least one

Every result is printed as a line `TAUTEN-PROOFS <json>`, which `cargo
tauten proofs` turns into one outcome per claim. The exit status is 0 only
if every stage passed. Nothing here decides an outcome: a missing line is
never read as a pass.

    python3 proofs/gate.py                 # the gate
    python3 proofs/gate.py --extract-only [--crate DIR]
                                           # tools and extract stages only
    python3 proofs/gate.py --only-mutants A,B
                                           # every stage, only these mutants (partial)
    python3 proofs/gate.py --extract-tar   # a fresh extraction as a tar on stdout
"""
from pathlib import Path
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parent.parent
PROOFS = ROOT / "proofs"
ALLOWED = {"propext", "Classical.choice", "Quot.sound"}
TOOLS = ("charon", "aeneas", "lake", "python3")
STARTED = time.time()


def emit(**fields) -> None:
    print("TAUTEN-PROOFS " + json.dumps(fields, sort_keys=True), flush=True)


def log(text: str) -> None:
    print(f"[{int(time.time() - STARTED)}s] {text}", file=sys.stderr, flush=True)


def run(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, timeout=3600, env={**os.environ, **(env or {})})


def claims() -> dict:
    return tomllib.loads((PROOFS / "claims.toml").read_text())


def extract(crate: str, dest: Path, as_path: str | None = None, source: Path | None = None) -> subprocess.CompletedProcess:
    env = {"EXTRACT_AS": as_path} if as_path else {}
    return run(["sh", str(PROOFS / "extract.sh"), str(source or ROOT / crate), str(dest),
                "--", "-abort-on-error"], ROOT, env)


def extract_all(crates: list[str], dest: Path) -> str | None:
    """Translate every crate into `dest`; the error output on failure."""
    dest.mkdir(parents=True, exist_ok=True)
    for crate in crates:
        with tempfile.TemporaryDirectory() as part:
            result = extract(crate, Path(part) / "out")
            if result.returncode != 0:
                return result.stdout
            for lean in (Path(part) / "out").glob("*.lean"):
                shutil.copy2(lean, dest / lean.name)
    return None


ANSI = re.compile(r"\x1b\[[0-9;]*m")
AXIOM = re.compile(r"^axiom\s+(\S+)", re.M)
LOCATED = (
    # Charon (rustc-style): "error: message" then "  --> file:line:col"
    re.compile(r"^error[^\n]*?:\s*(?P<message>[^\n]+)\n\s*-->\s*(?P<where>\S+:\d+:\d+)", re.M),
    # Aeneas: "[Error] message" then "Source: 'file', lines a:b-c:d"
    re.compile(r"\[Error\]\s*(?P<message>[^\n]+)\nSource: '(?P<file>[^']+)', lines (?P<lines>[\d:-]+)"),
)


def first_located(output: str) -> str:
    """The first translation error with its source location, or the first
    error line when none is located."""
    output = ANSI.sub("", output)
    found = []
    for pattern in LOCATED:
        if match := pattern.search(output):
            where = match.groupdict().get("where") or f"{match.group('file')}:{match.group('lines')}"
            found.append((match.start(), f"{where}: {match.group('message').strip()}"))
    if found:
        return min(found)[1]
    for line in output.splitlines():
        if "error" in line.lower():
            return line.strip()
    return "the translation failed (no error line found; see the log)"


def extract_only(only: str | None) -> int:
    crates = claims().get("gate", {}).get("crates", [])
    if only is not None:
        if only not in crates:
            emit(stage="extract", status="fail", detail=f"{only} is not in [gate] crates")
            return 1
        crates = [only]
    missing = [tool for tool in ("charon", "aeneas", "lake") if shutil.which(tool) is None]
    if missing:
        emit(stage="tools", status="missing", detail=f"not on PATH: {', '.join(missing)}")
        return 3
    emit(stage="tools", status="ok")
    failed = 0
    for crate in crates:
        log(f"extract: {crate}")
        with tempfile.TemporaryDirectory() as part:
            out = Path(part) / "out"
            result = extract(crate, out)
            opaque = sorted({m.group(1) for lean in out.glob("*.lean")
                             for m in AXIOM.finditer(lean.read_text())})
            if result.returncode != 0:
                print(result.stdout, file=sys.stderr)
                emit(stage="extract", status="fail", crate=crate, detail=first_located(result.stdout))
                failed += 1
            elif opaque:
                # Translated, but calls library functions Aeneas models as
                # axioms: the axiom audit refuses every theorem that uses them.
                emit(stage="extract", status="fail", crate=crate,
                     detail=f"{crate} translates only with opaque library functions (axioms), "
                            f"so no theorem over it passes the audit: {', '.join(opaque)}")
                failed += 1
            elif (located := lean_failure(out)) is not None:
                # Translated, but the Lean it produced does not compile (a
                # name Lean reserves, a clash between generated names).
                emit(stage="extract", status="fail", crate=crate, detail=located)
                failed += 1
            else:
                emit(stage="extract", status="ok", crate=crate)
    return 1 if failed else 0


SOURCE = re.compile(r"Source: '(?P<file>[^']+)', lines (?P<line>\d+):\d+")


def lean_failure(out: Path) -> str | None:
    """Build the fresh translation in Lean (without the proofs). `None` when
    it compiles; otherwise the first Lean error, located at the Rust source
    the failing definition was translated from. Lean decides, not a list of
    identifiers that might clash."""
    with tempfile.TemporaryDirectory(prefix="extract-build-") as directory:
        project = Path(directory) / "proofs"
        project.mkdir()
        for name in ("lakefile.toml", "lean-toolchain", "lake-manifest.json"):
            shutil.copy2(PROOFS / name, project / name)
        shutil.copytree(out, project / "Extracted")
        (project / ".lake").mkdir()
        os.symlink(PROOFS / ".lake" / "packages", project / ".lake" / "packages")
        modules = sorted(f"Extracted.{p.stem}" for p in out.glob("*.lean"))
        build = run(["lake", "build", *modules], project)
        if build.returncode == 0:
            return None
        print(build.stdout, file=sys.stderr)
        text = ANSI.sub("", build.stdout)
        match = ERROR.search(text)
        if match is None:
            return "the translation does not compile in Lean (no error line found; see the log)"
        message = text[match.end():].splitlines()[0].strip() if text[match.end():] else ""
        lean = project / match.group("file")
        where = f"{match.group('file')}:{match.group('line')}"
        if lean.is_file():
            lines = lean.read_text().splitlines()
            for i in range(min(int(match.group("line")), len(lines)) - 1, -1, -1):
                if source := SOURCE.search(lines[i]):
                    where = f"{source.group('file')}:{source.group('line')}"
                    break
        return f"{where}: the translation does not compile in Lean: {message}"


def same_tree(a: Path, b: Path) -> list[str]:
    """Files that differ between two directories of .lean files."""
    names = {p.name for p in a.glob("*.lean")} | {p.name for p in b.glob("*.lean")}
    return sorted(n for n in names
                  if not (a / n).is_file() or not (b / n).is_file()
                  or (a / n).read_bytes() != (b / n).read_bytes())


ERROR = re.compile(r"^error:\s*(?P<file>[^\s:]+\.lean):(?P<line>\d+):(?P<col>\d+):", re.M)
DECLARATION = re.compile(
    r"^(?:@\[[^\]]*\]\s*)?(?:private\s+|protected\s+)?(?:theorem|lemma|def|abbrev|instance)\s+(?P<name>[^\s:(]+)",
    re.M)
NAMESPACE = re.compile(r"^namespace\s+(?P<name>\S+)", re.M)
AXIOMS = re.compile(r"AXIOMS (?P<name>\S+): \[(?P<axioms>[^\]]*)\]", re.S)
TOTAL = re.compile(r"TOTAL (?P<theorem>\S+) (?P<function>\S+)")
SUBJECT = re.compile(r"SUBJECT (?P<theorem>\S+) (?P<function>\S+)")
PREMISES = re.compile(r"(?P<kind>PREMISES|ESTABLISHES) (?P<name>\S+): \[(?P<ids>[^\]]*)\]", re.S)
SORRY_DEPENDENT = re.compile(r"SORRY_DEPENDENT (?P<name>\S+)")
REPORTER = """

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  for entry in env.constants.map₂.toList do
    if (match entry.2 with | .thmInfo _ => true | _ => false) then
      let axioms ← liftCoreM (Lean.collectAxioms entry.1)
      if axioms.contains ``sorryAx then
        logInfo m!"SORRY_DEPENDENT {entry.1}"
"""


def declarations(path: Path) -> list[tuple[int, str]]:
    namespace, found = [], []
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if match := NAMESPACE.match(line):
            namespace.append(match.group("name"))
            continue
        if line.startswith("end ") and namespace and line.split()[1] == namespace[-1]:
            namespace.pop()
            continue
        if match := DECLARATION.match(line):
            found.append((number, ".".join(namespace + [match.group("name")])))
    return found


def failing(output: str, project: Path) -> tuple[set[str], list[str]]:
    """Theorems whose proofs fail, and errors outside Proofs/."""
    failures, elsewhere = set(), []
    for match in ERROR.finditer(output):
        file = (project / match.group("file")).resolve()
        try:
            relative = file.relative_to(project)
        except ValueError:
            elsewhere.append(match.group(0))
            continue
        if relative.parts[0] != "Proofs" or not file.exists():
            elsewhere.append(str(relative))
            continue
        enclosing = None
        for start, name in declarations(file):
            if start <= int(match.group("line")):
                enclosing = name
        if enclosing is None:
            elsewhere.append(str(relative))
        else:
            failures.add(enclosing)
    return failures, elsewhere


def scratch_project(scratch: Path, extracted: Path) -> Path:
    project = scratch / "proofs"
    project.mkdir()
    for name in ("lakefile.toml", "lean-toolchain", "lake-manifest.json", "Proofs.lean"):
        shutil.copy2(PROOFS / name, project / name)
    shutil.copytree(PROOFS / "Proofs", project / "Proofs")
    for theorems in (project / "Proofs").rglob("*.lean"):
        theorems.write_text(theorems.read_text() + REPORTER)
    shutil.copytree(extracted, project / "Extracted")
    (project / ".lake").mkdir()
    os.symlink(PROOFS / ".lake" / "packages", project / ".lake" / "packages")
    return project


def mutant(spec: dict, crates: list[str], theorems: set[str]) -> dict:
    name = spec["name"]
    crate = next((c for c in crates if spec["file"].startswith(c.rstrip("/") + "/")), None)
    if crate is None:
        return {"status": "error", "detail": f"{spec['file']} is in no crate of [gate] crates"}
    with tempfile.TemporaryDirectory(prefix=f"mutant-{name}-") as directory:
        scratch = Path(directory)
        copy = scratch / "crate"
        shutil.copytree(ROOT / crate, copy, ignore=shutil.ignore_patterns("target"))
        target = copy / Path(spec["file"]).relative_to(crate)
        text = target.read_text()
        if text.count(spec["before"]) != 1:
            return {"status": "error", "detail": "the mutation site does not match exactly once"}
        target.write_text(text.replace(spec["before"], spec["after"]))
        result = extract(crate, scratch / "fresh", as_path=crate, source=copy)
        if result.returncode != 0:
            print(result.stdout, file=sys.stderr)
            return {"status": "error", "detail": "the mutated crate did not translate"}
        extracted = scratch / "Extracted"
        shutil.copytree(PROOFS / "Extracted", extracted)
        changed = False
        for lean in (scratch / "fresh").glob("*.lean"):
            if (extracted / lean.name).read_bytes() != lean.read_bytes():
                changed = True
            shutil.copy2(lean, extracted / lean.name)
        if not changed:
            return {"status": "survived", "detail": "the mutation does not change the translation"}
        project = scratch_project(scratch, extracted)
        build = run(["lake", "build", "Proofs"], project)
        if build.returncode == 0:
            return {"status": "survived", "broken": []}
        failures, elsewhere = failing(build.stdout, project)
        dependents = set(SORRY_DEPENDENT.findall(build.stdout))
        broken = sorted((failures | dependents) & theorems)
        # Reported apart (#39): declarations whose own proof fails, and
        # those that only depend on one that does.
        direct = sorted(failures)
        dependent = sorted(dependents - failures)
        if elsewhere:
            print(build.stdout, file=sys.stderr)
            return {"status": "error", "broken": broken, "direct": direct, "dependent": dependent,
                    "detail": f"errors outside the theorems: {elsewhere[:3]}"}
        return {"status": "killed" if broken or direct else "survived", "broken": broken,
                "direct": direct, "dependent": dependent}


def gate(only: set[str] | None = None) -> int:
    config = claims()
    crates = config.get("gate", {}).get("crates", [])
    missing = [tool for tool in TOOLS if shutil.which(tool) is None]
    if missing:
        emit(stage="tools", status="missing", detail=f"not on PATH: {', '.join(missing)}")
        return 3
    emit(stage="tools", status="ok")

    log(f"extract: {', '.join(crates)}")
    fresh = Path(tempfile.mkdtemp(prefix="gate-extract-"))
    if error := extract_all(crates, fresh):
        print(error, file=sys.stderr)
        emit(stage="extract", status="fail")
        return 1
    emit(stage="extract", status="ok")

    log("drift check against proofs/Extracted")
    if differ := same_tree(PROOFS / "Extracted", fresh):
        emit(stage="drift", status="fail", detail=f"differs: {', '.join(differ)}; run sh proofs/refresh.sh")
        return 1
    emit(stage="drift", status="ok")

    log("lake build Extracted Proofs")
    build = run(["lake", "build", "Extracted", "Proofs"], PROOFS)
    print(build.stdout, file=sys.stderr)
    if build.returncode != 0:
        failures, _ = failing(build.stdout, PROOFS)
        for name in sorted(failures):
            emit(theorem=name, status="fail")
        emit(stage="build", status="fail")
        return 1
    emit(stage="build", status="ok")

    log("lake build Audit (axioms, total theorems)")
    audit = run(["lake", "build", "Audit"], PROOFS)
    print(audit.stdout, file=sys.stderr)
    audited = set()
    for match in AXIOMS.finditer(audit.stdout):
        axioms = {a.strip() for a in match.group("axioms").split(",") if a.strip()}
        name = match.group("name")
        audited.add(name)
        emit(theorem=name, status="ok" if axioms <= ALLOWED else "fail")
    for match in TOTAL.finditer(audit.stdout):
        emit(total=match.group("theorem"), function=match.group("function"))
    for match in SUBJECT.finditer(audit.stdout):
        # Read by `cargo tauten proofs draft` only (#49).
        emit(theorem=match.group("theorem"), subject=match.group("function"))
    for match in PREMISES.finditer(audit.stdout):
        ids = [i.strip() for i in match.group("ids").split(",") if i.strip()]
        key = "premises" if match.group("kind") == "PREMISES" else "establishes"
        emit(theorem=match.group("name"), **{key: ids})
    if audit.returncode != 0 or not audited:
        emit(stage="audit", status="fail")
        return 1
    emit(stage="audit", status="ok")

    named = {t for claim in config.get("claim", []) for t in claim.get("theorems", [])}
    specs = [m for m in config.get("mutant", []) if only is None or m["name"] in only]
    if only is not None and len(specs) != len(only):
        unknown = sorted(only - {m["name"] for m in specs})
        emit(stage="mutants", status="fail", detail=f"unknown mutant(s): {', '.join(unknown)}")
        return 1
    survivors = 0

    def one(spec: dict) -> dict:
        log(f"mutant {spec['name']}")
        return mutant(spec, crates, named)

    if only is None:
        results = [one(spec) for spec in specs]
    else:
        # `--only-mutants` (cargo tauten proofs try): in parallel.
        from concurrent.futures import ThreadPoolExecutor
        with ThreadPoolExecutor(max_workers=max(1, min(len(specs), (os.cpu_count() or 2) // 2))) as pool:
            results = list(pool.map(one, specs))
    for spec, result in zip(specs, results):
        if result["status"] != "killed":
            survivors += 1
        emit(mutant=spec["name"], **result)
    emit(stage="mutants", status="ok" if survivors == 0 else "fail")
    log("proofs gate: " + ("OK" if survivors == 0 else f"{survivors} mutant(s) not killed"))
    return 0 if survivors == 0 else 1


def extract_tar() -> int:
    crates = claims().get("gate", {}).get("crates", [])
    fresh = Path(tempfile.mkdtemp(prefix="refresh-"))
    if error := extract_all(crates, fresh):
        print(error, file=sys.stderr)
        return 1
    with tarfile.open(fileobj=sys.stdout.buffer, mode="w|") as tar:
        for lean in sorted(fresh.glob("*.lean")):
            tar.add(lean, arcname=lean.name)
    return 0


def main(args: list[str]) -> int:
    if args == ["--extract-tar"]:
        return extract_tar()
    if args[:1] == ["--extract-only"]:
        rest = args[1:]
        if rest and (len(rest) != 2 or rest[0] != "--crate"):
            print("usage: gate.py --extract-only [--crate DIR]", file=sys.stderr)
            return 2
        return extract_only(rest[1] if rest else None)
    if args[:1] == ["--only-mutants"]:
        if len(args) != 2:
            print("usage: gate.py --only-mutants NAME[,NAME...]", file=sys.stderr)
            return 2
        return gate({n for n in args[1].split(",") if n})
    return gate()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
