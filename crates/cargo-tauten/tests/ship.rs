//! Ship-level reuse against full runs (0.2.1, #83): a stand-in gate driven
//! by a table records which mutants it was asked to run, and a ship run
//! must agree with a full run on the same tree, per claim and mutant, about
//! status and the claim's theorems each mutant broke.

#[cfg(test)]
mod ship {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/proofs-subsume")
    }

    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("mkdir");
        for entry in fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            if name == "target" || name == ".lake" {
                continue;
            }
            if entry.file_type().expect("type").is_dir() {
                copy(&entry.path(), &to.join(&name));
            } else {
                fs::copy(entry.path(), to.join(&name)).expect("copy");
            }
        }
    }

    /// The stand-in gate. It reports every stage and every claim's theorems
    /// as passing, then one result per selected mutant: all of them for the
    /// full command, the comma list in its last argument for `try`. Each
    /// mutant's result comes from `proofs/kills.json` (mutant -> {status,
    /// broken, direct}); absent, it is killed breaking the first theorem of
    /// the claim listing it. Every run appends its selection to `ran.log`.
    const GATE: &str = r#"import json, sys, tomllib, os
c = tomllib.load(open("proofs/claims.toml", "rb"))
table = json.load(open("proofs/kills.json")) if os.path.exists("proofs/kills.json") else {}
def e(**f): print("TAUTEN-PROOFS " + json.dumps(f))
for s in ("tools", "extract", "drift", "build"): e(stage=s, status="ok")
for k in c["claim"]:
    for t in k["theorems"] + ([k["witness"]] if "witness" in k else []):
        e(theorem=t, status="ok")
        e(theorem=t, premises=k.get("assumes", []))
    if "witness" in k: e(theorem=k["witness"], establishes=k["assumes"])
    if "total" in k: e(total=k["total"], function=k["lean"])
e(stage="audit", status="ok")
owner = {m: k for k in c["claim"] for m in k["mutants"]}
if len(sys.argv) > 1 and sys.argv[1] == "only":
    chosen = [m for m in sys.argv[2].split(",") if m]
else:
    chosen = list(owner)
with open(LOG, "a") as log:
    log.write(",".join(sorted(chosen)) + "\n")
for m in chosen:
    r = table.get(m, {"status": "killed", "broken": [owner[m]["theorems"][0]]})
    e(mutant=m, status=r["status"], broken=r.get("broken", []), direct=r.get("direct", []))
e(stage="mutants", status="ok")
"#;

    /// A project: the proofs fixture with the stand-in gate as both the full
    /// and the `try` command.
    pub struct Project {
        pub root: PathBuf,
    }

    impl Project {
        pub fn new(name: &str) -> Self {
            let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ship-{name}"));
            let _ = fs::remove_dir_all(&root);
            copy(&fixture(), &root);
            let log = root.join("ran.log");
            fs::write(
                root.join("fake_gate.py"),
                GATE.replace("LOG", &format!("{:?}", log.display().to_string())),
            )
            .expect("write gate");
            let claims = root.join("proofs/claims.toml");
            let text = fs::read_to_string(&claims).expect("read").replace(
                r#"command = ["sh", "proofs/run.sh"]"#,
                "command = [\"python3\", \"fake_gate.py\"]\ntry = [\"python3\", \"fake_gate.py\", \"only\"]",
            );
            fs::write(&claims, text).expect("write");
            let _ = fs::remove_file(root.join("proofs/claims.json"));
            Self { root }
        }

        pub fn path(&self, rel: &str) -> PathBuf {
            self.root.join(rel)
        }

        pub fn read(&self, rel: &str) -> String {
            fs::read_to_string(self.path(rel)).expect("read")
        }

        pub fn write(&self, rel: &str, text: &str) {
            let p = self.path(rel);
            if let Some(dir) = p.parent() {
                fs::create_dir_all(dir).expect("mkdir");
            }
            fs::write(p, text).expect("write");
        }

        pub fn edit(&self, rel: &str, from: &str, to: &str) {
            let text = self.read(rel);
            assert!(text.contains(from), "{rel} lacks {from:?}");
            self.write(rel, &text.replacen(from, to, 1));
        }

        pub fn tauten(&self, args: &[&str]) -> Output {
            Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
                .arg("tauten")
                .args(args)
                .arg("--live")
                .arg("--manifest-path")
                .arg(self.root.join("Cargo.toml"))
                .output()
                .expect("run cargo-tauten")
        }

        /// The mutant selections of every gate run so far, then clears them.
        pub fn ran(&self) -> Vec<BTreeSet<String>> {
            let log = self.path("ran.log");
            let text = fs::read_to_string(&log).unwrap_or_default();
            let _ = fs::remove_file(&log);
            text.lines()
                .map(|l| {
                    l.split(',')
                        .filter(|m| !m.is_empty())
                        .map(str::to_owned)
                        .collect()
                })
                .collect()
        }

        pub fn record(&self) -> String {
            self.read("proofs/claims.json")
        }

        pub fn restore_record(&self, text: &str) {
            self.write("proofs/claims.json", text);
        }
    }

    /// Per claim: outcome and badge; per (claim, mutant): status and the
    /// claim's theorems it broke (#83). Raw `broken`/`direct` lists of a
    /// carried result are historical and not compared.
    pub type Projection =
        BTreeMap<String, (String, String, BTreeMap<String, (String, Vec<String>)>)>;

    pub fn project(record: &str) -> Projection {
        let v: serde_json::Value = serde_json::from_str(record).expect("record json");
        let mut out = BTreeMap::new();
        for c in v["claims"].as_array().expect("claims") {
            let theorems: BTreeSet<&str> = c["theorems"]
                .as_array()
                .map(|a| a.iter().filter_map(|t| t.as_str()).collect())
                .unwrap_or_default();
            let mut mutants = BTreeMap::new();
            for m in c["mutant_results"].as_array().into_iter().flatten() {
                let broken: Vec<String> = m["broken"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| t.as_str())
                    .filter(|t| theorems.contains(t))
                    .map(str::to_owned)
                    .collect();
                mutants.insert(
                    m["mutant"].as_str().unwrap_or("").to_owned(),
                    (m["status"].as_str().unwrap_or("").to_owned(), broken),
                );
            }
            out.insert(
                c["function"].as_str().unwrap_or("").to_owned(),
                (
                    c["outcome"].as_str().unwrap_or("").to_owned(),
                    c["badge"].as_str().unwrap_or("").to_owned(),
                    mutants,
                ),
            );
        }
        out
    }

    /// Run ship, then full on the same tree from the same starting record;
    /// assert they agree and return the mutants ship ran. Leaves the full
    /// run's record in place.
    pub fn ship_agrees_with_full(p: &Project) -> BTreeSet<String> {
        let start = p.record();
        p.ran();
        let ship = p.tauten(&["proofs", "--level", "ship"]);
        let ship_err = String::from_utf8_lossy(&ship.stderr).into_owned();
        assert!(
            ship_err.contains("ship level:"),
            "ship did not plan: {ship_err}"
        );
        let ran = p.ran();
        assert_eq!(ran.len(), 1, "one gate run: {ran:?}");
        let shipped = project(&p.record());
        p.restore_record(&start);
        p.tauten(&["proofs"]);
        p.ran();
        let full = project(&p.record());
        assert_eq!(shipped, full, "ship and full disagree\n{ship_err}");
        ran.into_iter().next().unwrap_or_default()
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| (*s).to_owned()).collect()
    }

    const ALL: [&str; 4] = ["guard-dropped", "quadruple", "second-value", "stored-wins"];

    #[test]
    fn harness_records_full_then_reuses_everything_unchanged() {
        let p = Project::new("unchanged");
        let first = p.tauten(&["proofs"]);
        assert!(first.status.code().is_some(), "{first:?}");
        assert_eq!(p.ran(), vec![set(&ALL)]);
        assert_eq!(ship_agrees_with_full(&p), set(&[]));
    }

    /// One proof file per claim, so each claim's proof files mention only
    /// its own function (the fixture keeps every theorem in one file, which
    /// mentions them all).
    fn split_proofs(p: &Project) {
        fs::remove_file(p.path("proofs/Proofs/SubKernel.lean")).expect("rm");
        for (file, theorem, function) in [
            ("Add", "add_capped_spec", "add_capped"),
            ("First", "first_or_zero_spec", "first_or_zero"),
            ("Double", "double_spec", "double"),
            ("Latest", "latest_is_later_reading", "latest"),
        ] {
            p.write(
                &format!("proofs/Proofs/{file}.lean"),
                &format!(
                    "import Extracted.SubKernel\nimport Proofs.TautenAssumption\nopen sub_kernel\nnamespace Proofs.SubKernel\ntheorem {theorem} : {function} = {function} := by\n  unfold {function}\n  rfl\nend Proofs.SubKernel\n"
                ),
            );
        }
        p.write(
            "proofs/Proofs/Witness.lean",
            "namespace Proofs.SubKernel\ntheorem constant_counter_is_monotone : True := trivial\nend Proofs.SubKernel\n",
        );
    }

    /// Record a full run, with the proofs split per claim.
    fn recorded(name: &str) -> Project {
        let p = Project::new(name);
        split_proofs(&p);
        p.tauten(&["proofs"]);
        p.ran();
        p
    }

    #[test]
    fn harness_reruns_only_the_claim_whose_extracted_function_changed() {
        let p = recorded("one-changed");
        p.edit(
            "proofs/Extracted/SubKernel.lean",
            "then ok 0#u32",
            "then ok 1#u32",
        );
        assert_eq!(ship_agrees_with_full(&p), set(&["second-value"]));
    }

    /// The translation split in two files: `add_capped` in one calls a
    /// helper declared in the other.
    fn split_translation(p: &Project) {
        let text = p.read("proofs/Extracted/SubKernel.lean");
        let (head, _) = text.split_once("namespace sub_kernel").expect("namespace");
        p.write(
            "proofs/Extracted/SubKernel.lean",
            &text.replace(
                "  if a > i\n",
                "  let c ← sub_kernel.helpers.cap i\n  if a > c\n",
            ),
        );
        p.write(
            "proofs/Extracted/SubKernel/Helpers.lean",
            &format!(
                "{head}namespace sub_kernel\n\ndef helpers.cap (x : Std.U32) : Result Std.U32 := do\n  ok x\n\nend sub_kernel\n"
            ),
        );
    }

    /// #79: a claim's inputs follow its function into every extracted file.
    #[test]
    fn a_callee_in_another_extracted_file_is_an_input() {
        let p = Project::new("cross-file");
        split_proofs(&p);
        split_translation(&p);
        p.tauten(&["proofs"]);
        p.ran();
        p.edit(
            "proofs/Extracted/SubKernel/Helpers.lean",
            "  ok x\n",
            "  ok (x - 1#u32)\n",
        );
        assert_eq!(ship_agrees_with_full(&p), set(&["guard-dropped"]));
    }

    fn inputs_of(p: &Project) -> serde_json::Value {
        let v: serde_json::Value = serde_json::from_str(&p.record()).expect("json");
        v["claims"]
            .as_array()
            .expect("claims")
            .iter()
            .map(|c| {
                (
                    c["function"].as_str().unwrap_or("").to_owned(),
                    c["inputs"].clone(),
                )
            })
            .collect::<serde_json::Map<_, _>>()
            .into()
    }

    /// #79: the same declaration kept in two files (a retained single-file
    /// translation next to a split one) is one input, fingerprinted as in
    /// one file; a differing copy is bound too.
    #[test]
    fn a_duplicated_declaration_is_one_input() {
        let p = recorded("duplicate");
        let single = inputs_of(&p);
        let text = p.read("proofs/Extracted/SubKernel.lean");
        p.write("proofs/Extracted/Aeneas/SubKernel.lean", &text);
        p.tauten(&["proofs"]);
        assert_eq!(inputs_of(&p), single, "identical copies change nothing");
        p.ran();
        p.write(
            "proofs/Extracted/Aeneas/SubKernel.lean",
            &text.replace("a * 2#u32", "a * 2#u32 + 0#u32"),
        );
        assert_eq!(ship_agrees_with_full(&p), set(&["quadruple"]));
    }

    fn drop_mutant_entry(p: &Project, name: &str) {
        let text = p.read("proofs/claims.toml");
        let at = text
            .find(&format!("[[mutant]]\nname = \"{name}\""))
            .expect("entry");
        let end = text[at + 1..]
            .find("[[mutant]]")
            .map_or(text.len(), |n| at + 1 + n);
        p.write(
            "proofs/claims.toml",
            &format!("{}{}", &text[..at], &text[end..]),
        );
    }

    /// #80: a mutant whose definition is in neither `[[mutant]]` nor
    /// `proofs/mutants.py` is run on every ship-level run.
    #[test]
    fn a_mutant_with_no_known_definition_is_never_reused() {
        let p = Project::new("unknown-mutant");
        split_proofs(&p);
        drop_mutant_entry(&p, "stored-wins");
        p.tauten(&["proofs"]);
        p.ran();
        assert!(
            !p.record().contains("unbound:"),
            "unbound inputs are never recorded"
        );
        let ship = p.tauten(&["proofs", "--level", "ship"]);
        let err = String::from_utf8_lossy(&ship.stderr);
        assert!(err.contains("unbound:mutant:stored-wins"), "{err}");
        assert_eq!(p.ran(), vec![set(&["stored-wins"])]);
        assert_eq!(ship_agrees_with_full(&p), set(&["stored-wins"]));
    }

    const MUTANTS_PY: &str = r#"from dataclasses import dataclass

@dataclass(frozen=True)
class Mutant:
    name: str
    fails: tuple

T = "Proofs.SubKernel."
LATEST = (T + "latest_is_later_reading",)

MUTANTS = (
    # latest: the stored value always wins
    Mutant("stored-wins", "sub-kernel/src/lib.rs",
           "if seen >= stored { seen } else { stored }", "stored",
           LATEST),
    Mutant("other", "sub-kernel/src/lib.rs", "a * 3", "a * 9", ()),
)

def main():
    print("Mutant(\"fake\", in a string)")
"#;

    /// #80: a `proofs/mutants.py` entry is bound with everything outside
    /// the entries (constants it uses, the runner); another entry, a
    /// comment or a new entry is not part of it.
    #[test]
    fn a_python_mutant_is_bound_with_the_constants_it_uses() {
        let p = Project::new("python-mutant");
        split_proofs(&p);
        drop_mutant_entry(&p, "stored-wins");
        p.write("proofs/mutants.py", MUTANTS_PY);
        p.tauten(&["proofs"]);
        p.ran();
        p.edit("proofs/mutants.py", "\"a * 9\"", "\"a * 8\"");
        p.edit(
            "proofs/mutants.py",
            "    # latest:",
            "    # new comment\n    # latest:",
        );
        p.edit(
            "proofs/mutants.py",
            "\n)\n\ndef",
            "\n    Mutant(\"added\", \"x.rs\", \"a\", \"b\", ()),\n)\n\ndef",
        );
        assert_eq!(ship_agrees_with_full(&p), set(&[]));
        p.edit("proofs/mutants.py", "LATEST = (T", "LATEST = (T + \"x\", T");
        assert_eq!(ship_agrees_with_full(&p), set(&["stored-wins"]));
    }

    fn set_direct(p: &Project, name: &str, direct: &str) {
        p.edit(
            "proofs/claims.toml",
            &format!("[[mutant]]\nname = \"{name}\"\n"),
            &format!("[[mutant]]\nname = \"{name}\"\ndirect = [{direct}]\n"),
        );
    }

    fn kills(p: &Project, json: &str) {
        p.write("proofs/kills.json", json);
    }

    /// Gap A: a mutant with an exact `direct` set is killed only if no other
    /// declaration fails directly, so an edit to any Lean file of the
    /// project, even one no claim of it mentions, re-runs it; a mutant
    /// without `direct` is not affected.
    #[test]
    fn an_exact_mutant_is_bound_to_every_lean_file() {
        let p = Project::new("exact");
        split_proofs(&p);
        set_direct(&p, "guard-dropped", "\"Proofs.SubKernel.add_capped_spec\"");
        kills(
            &p,
            r#"{"guard-dropped": {"status": "killed", "broken": ["Proofs.SubKernel.add_capped_spec"], "direct": ["Proofs.SubKernel.add_capped_spec"]}}"#,
        );
        p.tauten(&["proofs"]);
        p.ran();
        // A new theorem elsewhere also fails directly under the mutant.
        p.write(
            "proofs/Proofs/Elsewhere.lean",
            "namespace Proofs.Elsewhere\ntheorem other : True := trivial\nend Proofs.Elsewhere\n",
        );
        kills(
            &p,
            r#"{"guard-dropped": {"status": "killed", "broken": ["Proofs.SubKernel.add_capped_spec", "Proofs.Elsewhere.other"], "direct": ["Proofs.SubKernel.add_capped_spec", "Proofs.Elsewhere.other"]}}"#,
        );
        assert_eq!(ship_agrees_with_full(&p), set(&["guard-dropped"]));
    }

    /// Gap B: a theorem may unfold any extracted declaration its file
    /// mentions, not only those its function reaches.
    #[test]
    fn a_declaration_a_proof_file_mentions_is_an_input() {
        let p = Project::new("mentioned");
        split_proofs(&p);
        p.edit("proofs/Proofs/Latest.lean", "  rfl\n", "  simp [triple]\n");
        p.tauten(&["proofs"]);
        p.ran();
        p.edit(
            "proofs/Extracted/SubKernel.lean",
            "a * 3#u32",
            "a * 3#u32 + 0#u32",
        );
        assert_eq!(ship_agrees_with_full(&p), set(&["stored-wins"]));
    }

    /// #79: a claim whose function names no extracted declaration, or a
    /// translation with nested namespaces, is never reused.
    #[test]
    fn incomplete_binding_is_never_reused() {
        let p = recorded("unresolved");
        p.edit(
            "proofs/claims.toml",
            "lean = \"sub_kernel.double\"",
            "lean = \"sub_kernel.doubled\"",
        );
        p.tauten(&["proofs"]);
        p.ran();
        assert_eq!(ship_agrees_with_full(&p), set(&["quadruple"]));

        let p = recorded("nested");
        p.edit(
            "proofs/Extracted/SubKernel.lean",
            "def triple",
            "namespace inner\ndef t := 0\nend inner\ndef triple",
        );
        p.tauten(&["proofs"]);
        p.ran();
        assert_eq!(ship_agrees_with_full(&p), set(&ALL));
    }

    /// #79: Lean module-system imports are followed.
    #[test]
    fn module_system_imports_are_followed() {
        let p = Project::new("public-import");
        split_proofs(&p);
        p.write(
            "proofs/Proofs/Lemma.lean",
            "theorem helper_lemma : True := trivial\n",
        );
        p.edit(
            "proofs/Proofs/First.lean",
            "import Proofs.TautenAssumption\n",
            "import Proofs.TautenAssumption\npublic import Proofs.Lemma\n",
        );
        p.tauten(&["proofs"]);
        p.ran();
        p.edit("proofs/Proofs/Lemma.lean", "trivial", "by trivial");
        assert_eq!(ship_agrees_with_full(&p), set(&["second-value"]));
    }

    /// #77: a result that could not be recorded is refused before the gate
    /// starts, with the record untouched; a run that records nothing warns.
    #[test]
    fn a_known_weakening_is_refused_before_the_gate_runs() {
        let p = recorded("weakened");
        let before = p.record();
        p.edit(
            "proofs/claims.toml",
            r#"mutants = ["guard-dropped"]"#,
            r#"mutants = ["guard-weakened"]"#,
        );
        for level in ["full", "ship"] {
            let out = p.tauten(&["proofs", "--level", level]);
            let err = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{level}: {err}");
            assert!(
                err.contains("TAU-R002") && err.contains("not running the gate"),
                "{err}"
            );
            assert!(p.ran().is_empty(), "{level}: the gate ran");
            assert_eq!(p.record(), before);
        }
        let dry = p.tauten(&["proofs", "--no-write"]);
        assert!(String::from_utf8_lossy(&dry.stderr).contains("could not be recorded"));
        assert_eq!(p.ran().len(), 1, "--no-write still runs the gate");
        assert_eq!(p.record(), before);
        // Amended, it runs and records.
        let text = p.read("proofs/claims.toml");
        p.write(
            "proofs/claims.toml",
            &format!("{text}\n[[amend]]\nfile = \"sub-kernel/src/lib.rs\"\nfunction = \"add_capped\"\nkind = \"mutant\"\nname = \"guard-dropped\"\nreason = \"replaced\"\n"),
        );
        p.tauten(&["proofs"]);
        assert_eq!(p.ran().len(), 1);
        assert_ne!(p.record(), before);
    }

    /// #77: a pending merge record is replaced only by a gate on a snapshot
    /// of the merge commit: on the live tree that is known before the run.
    #[test]
    fn a_pending_record_is_not_replaced_from_the_live_tree() {
        let p = recorded("pending");
        let pending =
            r#"{"version": 2, "state": "pending-merge", "parents": ["a", "b"], "registry": []}"#;
        p.restore_record(pending);
        let out = p.tauten(&["proofs"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("snapshot of the merge commit"));
        assert!(p.ran().is_empty(), "the gate ran");
        assert_eq!(p.record(), pending);
    }

    /// Every claim's recorded inputs on the unchanged fixture, pinned: a
    /// change here recalibrates every adopter once, so it must be deliberate
    /// (`TAUTEN_BLESS=1` rewrites the golden file).
    #[test]
    fn inputs_are_pinned() {
        let p = Project::new("golden");
        p.tauten(&["proofs"]);
        let v: serde_json::Value = serde_json::from_str(&p.record()).expect("json");
        let mut got = BTreeMap::new();
        for c in v["claims"].as_array().expect("claims") {
            got.insert(
                c["function"].as_str().unwrap_or("").to_owned(),
                c["inputs"].clone(),
            );
        }
        let got = serde_json::to_string_pretty(&got).expect("json") + "\n";
        let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/ship-inputs.json");
        if std::env::var_os("TAUTEN_BLESS").is_some() {
            fs::write(&golden, &got).expect("bless");
        }
        assert_eq!(got, fs::read_to_string(&golden).unwrap_or_default());
    }
}
