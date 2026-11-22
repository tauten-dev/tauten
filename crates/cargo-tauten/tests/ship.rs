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

    #[test]
    fn harness_reruns_only_the_claim_whose_extracted_function_changed() {
        let p = Project::new("one-changed");
        p.tauten(&["proofs"]);
        p.edit(
            "proofs/Extracted/SubKernel.lean",
            "then ok 0#u32",
            "then ok 1#u32",
        );
        assert_eq!(ship_agrees_with_full(&p), set(&["second-value"]));
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
