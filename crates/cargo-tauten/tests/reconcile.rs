//! `cargo tauten proofs reconcile` (#50) on an adopter-shaped two-parent
//! merge of `fixtures/proofs-subsume`: two branches edit the same source
//! file (so the generated translation conflicts) and each adds a theorem to
//! a different claim. Reconcile keeps both parents' obligations, regenerates
//! the translation, and writes a pending record that is never evidence;
//! only a gate on the merge commit replaces it.

#[cfg(test)]
mod reconcile {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/proofs-subsume")
    }

    fn tauten(args: &[&str], root: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .arg("tauten")
            .args(args)
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .output()
            .expect("run cargo-tauten")
    }

    fn git(root: &Path, args: &[&str]) -> Output {
        Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
            .args(["-c", "merge.conflictStyle=merge"])
            .args(args)
            .current_dir(root)
            .output()
            .expect("git")
    }

    fn ok(root: &Path, args: &[&str]) -> String {
        let out = git(root, args);
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
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

    /// A stand-in gate reporting every claim of the snapshot proved.
    const FAKE_GATE: &str = r#"import json, tomllib
c = tomllib.load(open("proofs/claims.toml", "rb"))
def e(**f): print("TAUTEN-PROOFS " + json.dumps(f))
for s in ("tools", "extract", "drift", "build"): e(stage=s, status="ok")
for k in c["claim"]:
    for t in k["theorems"] + ([k["witness"]] if "witness" in k else []):
        e(theorem=t, status="ok")
        e(theorem=t, premises=k.get("assumes", []))
    if "witness" in k: e(theorem=k["witness"], establishes=k["assumes"])
    if "total" in k: e(total=k["total"], function=k["lean"])
e(stage="audit", status="ok")
for k in c["claim"]:
    for m in k["mutants"]: e(mutant=m, status="killed", broken=[k["theorems"][0]])
e(stage="mutants", status="ok")
"#;

    /// The "translation": numbered source lines, so an edit near the top
    /// renumbers everything after it (as a real extraction's `Source:` lines
    /// move).
    const GEN: &str = "mkdir -p proofs/Extracted\nrm -f proofs/Extracted/*\nawk '{print NR \": \" $0}' sub-kernel/src/lib.rs > proofs/Extracted/SubKernel.lean\n";

    /// Add `theorem` to the claim of `function`, in claims.toml and in the
    /// recorded claims.json (as that branch's gate would have).
    fn add_theorem(root: &Path, function: &str, theorem: &str) {
        let path = root.join("proofs/claims.toml");
        let text = fs::read_to_string(&path).expect("toml");
        let marker = format!("function = \"{function}\"\n");
        let at = text.find(&marker).expect("claim");
        let (head, tail) = text.split_at(at);
        let tail = tail.replacen("theorems = [", &format!("theorems = [\"{theorem}\", "), 1);
        fs::write(&path, format!("{head}{tail}")).expect("write toml");
        let path = root.join("proofs/claims.json");
        let mut record: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("json")).expect("parse");
        for claim in record["claims"].as_array_mut().expect("claims") {
            if claim["function"] == function {
                claim["theorems"]
                    .as_array_mut()
                    .expect("theorems")
                    .insert(0, theorem.into());
            }
        }
        let mut text = serde_json::to_string_pretty(&record).expect("json");
        text.push('\n');
        fs::write(&path, text).expect("write json");
    }

    fn edit_source(root: &Path, top: bool, line: &str) {
        let path = root.join("sub-kernel/src/lib.rs");
        let text = fs::read_to_string(&path).expect("lib");
        let text = if top {
            format!("{line}\n{text}")
        } else {
            format!("{text}{line}\n")
        };
        fs::write(&path, text).expect("write lib");
    }

    /// A repository on `main` with a branch `a` and a branch `b`, each one
    /// commit ahead, each having regenerated the translation; `main` is
    /// checked out at `a` and a merge of `b` is left in progress.
    fn conflicted_merge(name: &str, b_function: &str) -> PathBuf {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("reconcile-{name}"));
        let _ = fs::remove_dir_all(&root);
        copy(&fixture(), &root);
        let manifest = root.join("Cargo.toml");
        let text = fs::read_to_string(&manifest).expect("manifest");
        fs::write(
            &manifest,
            format!(
                "{text}\n[[workspace.metadata.tauten.generated]]\nname = \"translation\"\ncommand = [\"sh\", \"gen.sh\"]\noutputs = [\"proofs/Extracted\"]\n"
            ),
        )
        .expect("write manifest");
        fs::write(root.join("gen.sh"), GEN).expect("gen");
        fs::write(root.join("fake_gate.py"), FAKE_GATE).expect("gate");
        let claims = root.join("proofs/claims.toml");
        let text = fs::read_to_string(&claims).expect("claims").replace(
            r#"command = ["sh", "proofs/run.sh"]"#,
            r#"command = ["python3", "fake_gate.py"]"#,
        );
        fs::write(&claims, text).expect("write claims");
        let regen = |root: &Path| {
            let out = Command::new("sh")
                .arg("gen.sh")
                .current_dir(root)
                .output()
                .expect("gen");
            assert!(out.status.success());
        };
        regen(&root);
        ok(&root, &["init", "-q", "-b", "main"]);
        ok(&root, &["add", "-A"]);
        ok(&root, &["commit", "-q", "-m", "base"]);

        ok(&root, &["checkout", "-q", "-b", "b"]);
        edit_source(&root, false, "// branch b, at the end");
        add_theorem(&root, b_function, "Proofs.SubKernel.extra_b");
        regen(&root);
        ok(&root, &["commit", "-q", "-am", "b"]);

        ok(&root, &["checkout", "-q", "main"]);
        edit_source(&root, true, "// branch a, at the top");
        add_theorem(&root, "add_capped", "Proofs.SubKernel.extra_a");
        regen(&root);
        ok(&root, &["commit", "-q", "-am", "a"]);

        let merge = git(&root, &["merge", "--no-ff", "-q", "b"]);
        assert!(!merge.status.success(), "the translation must conflict");
        let unmerged = ok(&root, &["diff", "--name-only", "--diff-filter=U"]);
        assert!(
            unmerged.contains("proofs/Extracted/SubKernel.lean"),
            "{unmerged}"
        );
        assert!(!unmerged.contains("lib.rs"), "{unmerged}");
        root
    }

    #[test]
    fn a_merge_is_pending_until_the_merge_commit_is_proved() {
        let root = conflicted_merge("ok", "first_or_zero");
        let first = ok(&root, &["rev-parse", "HEAD"]);
        let second = ok(&root, &["rev-parse", "MERGE_HEAD"]);

        let out = tauten(&["proofs", "reconcile"], &root);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{err}");
        assert!(err.contains("pending, not proved"), "{err}");
        let listed = String::from_utf8_lossy(&out.stdout);
        assert!(
            listed.contains("proofs/Extracted/SubKernel.lean"),
            "{listed}"
        );

        // The record: pending, both parents, both new theorems, no results.
        let record: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("proofs/claims.json")).expect("record"),
        )
        .expect("json");
        assert_eq!(record["state"], "pending-merge");
        assert_eq!(record["parents"], serde_json::json!([first, second]));
        assert!(record.get("claims").is_none());
        let text = record.to_string();
        assert!(
            text.contains("extra_a") && text.contains("extra_b"),
            "{text}"
        );

        // The translation equals a fresh one of the merged source.
        let generated = tauten(&["generated", "--only", "translation"], &root);
        assert!(
            generated.status.success(),
            "{}",
            String::from_utf8_lossy(&generated.stdout)
        );
        // Pending is not evidence: nothing subsumed, nothing to export.
        let check = tauten(&["check", "--format", "json"], &root);
        assert!(
            !String::from_utf8_lossy(&check.stdout).contains("subsumed_by\":\"Proofs"),
            "a pending record subsumed a finding"
        );
        assert!(
            !tauten(&["proofs", "export", "--crate", "sub-kernel"], &root)
                .status
                .success()
        );
        // Rerunning on the same inputs finishes the same way.
        assert!(tauten(&["proofs", "reconcile"], &root).status.success());

        // Commit the merge; the merge commit's gate replaces the record.
        ok(&root, &["add", "-A"]);
        ok(&root, &["commit", "-q", "--no-edit"]);
        let gate = tauten(&["proofs", "--level", "ship"], &root);
        let err = String::from_utf8_lossy(&gate.stderr);
        assert!(gate.status.success(), "{err}");
        assert!(err.contains("pending merge record"), "{err}");
        let record: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("proofs/claims.json")).expect("record"),
        )
        .expect("json");
        assert!(record.get("state").is_none());
        assert_eq!(record["claims"].as_array().map(Vec::len), Some(4));
    }

    #[test]
    fn dropping_a_parents_obligation_is_refused() {
        let root = conflicted_merge("weakened", "first_or_zero");
        let path = root.join("proofs/claims.toml");
        let text = fs::read_to_string(&path).expect("toml");
        fs::write(&path, text.replace("\"Proofs.SubKernel.extra_a\", ", "")).expect("write");
        ok(&root, &["add", "proofs/claims.toml"]);
        let before = fs::read(root.join("proofs/claims.json")).expect("record");
        let out = tauten(&["proofs", "reconcile"], &root);
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{err}");
        assert!(err.contains("TAU-R002") && err.contains("extra_a"), "{err}");
        assert_eq!(
            fs::read(root.join("proofs/claims.json")).expect("kept"),
            before
        );
    }

    #[test]
    fn unresolved_human_conflicts_and_non_merges_are_refused() {
        // Both branches add a theorem to the same claim: claims.toml, a
        // human-authored input, conflicts.
        let root = conflicted_merge("human", "add_capped");
        let unmerged = ok(&root, &["diff", "--name-only", "--diff-filter=U"]);
        assert!(unmerged.contains("proofs/claims.toml"), "{unmerged}");
        let out = tauten(&["proofs", "reconcile"], &root);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{err}");
        assert!(
            err.contains("resolve and stage") && err.contains("proofs/claims.toml"),
            "{err}"
        );
        assert!(
            !fs::read_to_string(root.join("proofs/claims.json"))
                .expect("record")
                .contains("pending-merge")
        );

        ok(&root, &["merge", "--abort"]);
        let out = tauten(&["proofs", "reconcile"], &root);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("no merge in progress"));
    }

    /// On unchanged source, where the v1 record subsumes findings, a pending
    /// record over the same claims subsumes none, yet still guards TAU-R002.
    #[test]
    fn a_pending_record_subsumes_nothing_but_keeps_the_registry() {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("reconcile-registry");
        let _ = fs::remove_dir_all(&root);
        copy(&fixture(), &root);
        let subsumed = |root: &Path| {
            String::from_utf8_lossy(&tauten(&["check", "--format", "json"], root).stdout)
                .matches("subsumed_by\":\"Proofs")
                .count()
        };
        assert_eq!(subsumed(&root), 3);
        let path = root.join("proofs/claims.json");
        let record: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("record")).expect("json");
        let registry: Vec<serde_json::Value> = record["claims"]
            .as_array()
            .expect("claims")
            .iter()
            .map(|c| {
                let mut c = c.clone();
                let o = c.as_object_mut().expect("object");
                for key in [
                    "outcome",
                    "badge",
                    "detail",
                    "fingerprint",
                    "inputs",
                    "mutant_results",
                ] {
                    o.remove(key);
                }
                c
            })
            .collect();
        let pending = serde_json::json!({
            "version": 2, "state": "pending-merge",
            "parents": ["1111111", "2222222"], "registry": registry,
        });
        fs::write(&path, pending.to_string()).expect("write");
        assert_eq!(subsumed(&root), 0, "a pending record is not evidence");
        let toml = root.join("proofs/claims.toml");
        let text = fs::read_to_string(&toml).expect("toml");
        fs::write(
            &toml,
            text.replace(
                "mutants = [\"guard-dropped\"]",
                "mutants = [\"second-value\"]",
            ),
        )
        .expect("write");
        let check = tauten(&["check", "--format", "json"], &root);
        assert!(
            String::from_utf8_lossy(&check.stdout).contains("TAU-R002"),
            "the pending registry is still the TAU-R002 baseline"
        );
    }

    #[test]
    fn a_pending_record_for_other_parents_is_not_promoted() {
        let root = conflicted_merge("forged", "first_or_zero");
        assert!(tauten(&["proofs", "reconcile"], &root).status.success());
        // Swap the parents: the merge commit no longer matches.
        let path = root.join("proofs/claims.json");
        let mut record: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("record")).expect("json");
        let parents = record["parents"].as_array_mut().expect("parents");
        parents.reverse();
        fs::write(&path, record.to_string()).expect("write");
        ok(&root, &["add", "-A"]);
        ok(&root, &["commit", "-q", "--no-edit"]);
        let gate = tauten(&["proofs"], &root);
        let err = String::from_utf8_lossy(&gate.stderr);
        assert_eq!(gate.status.code(), Some(1), "{err}");
        assert!(err.contains("is not the merge of"), "{err}");
        let kept = fs::read_to_string(&path).expect("record");
        assert!(kept.contains("pending-merge"), "{kept}");
    }
}
