//! `cargo tauten proofs draft` end to end (#49), on `fixtures/proofs-draft`:
//! a kernel crate with hand-written claims and new functions, whose gate is a
//! stand-in printing recorded audit lines (`proofs/audit-stub.sh`, no Lean).
//! The printed patch applies with `git apply`, preserves hand-written text,
//! is empty when drafted again, writes nothing, and refuses (no patch) on an
//! unknown package, missing tools, a stale translation or an ambiguity.

#[cfg(test)]
mod draft {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/proofs-draft")
    }

    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("mkdir");
        for entry in fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            if name == "target" {
                continue;
            }
            if entry.file_type().expect("type").is_dir() {
                copy(&entry.path(), &to.join(&name));
            } else {
                fs::copy(entry.path(), to.join(&name)).expect("copy");
            }
        }
    }

    fn scratch(name: &str) -> PathBuf {
        scratch_of(&fixture(), name)
    }

    fn scratch_of(from: &Path, name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("draft-{name}"));
        let _ = fs::remove_dir_all(&dir);
        copy(from, &dir);
        dir
    }

    fn tauten(args: &[&str], root: &Path, stub: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-tauten"));
        command
            .arg("tauten")
            .args(args)
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .env_remove("DRAFT_STUB");
        if let Some(stub) = stub {
            command.env("DRAFT_STUB", stub);
        }
        command.output().expect("run cargo-tauten")
    }

    fn draft(root: &Path, extra: &[&str], stub: Option<&str>) -> Output {
        let mut args = vec!["proofs", "draft"];
        args.extend_from_slice(extra);
        tauten(&args, root, stub)
    }

    fn git_apply(root: &Path, patch: &[u8], check: bool) -> Output {
        let file = root.join("draft.patch");
        fs::write(&file, patch).expect("write patch");
        let mut command = Command::new("git");
        command.arg("apply");
        if check {
            command.arg("--check");
        }
        let out = command
            .arg("draft.patch")
            .current_dir(root)
            .output()
            .expect("run git apply");
        fs::remove_file(&file).expect("remove patch");
        out
    }

    fn claims_of(root: &Path) -> toml::Table {
        let text = fs::read_to_string(root.join("proofs/claims.toml")).expect("claims");
        text.parse().expect("toml")
    }

    fn claim<'a>(table: &'a toml::Table, file: &str, function: &str) -> Option<&'a toml::Table> {
        table.get("claim")?.as_array()?.iter().find_map(|c| {
            let c = c.as_table()?;
            (c.get("file")?.as_str()? == file && c.get("function")?.as_str()? == function)
                .then_some(c)
        })
    }

    fn strings(table: &toml::Table, key: &str) -> Vec<String> {
        table
            .get(key)
            .and_then(toml::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }

    const LIB: &str = "draft-kernel/src/lib.rs";

    #[test]
    fn the_printed_patch_applies_keeps_hand_written_text_and_drafts_once() {
        let root = scratch("apply");
        let before = fs::read_to_string(root.join("proofs/claims.toml")).expect("claims");
        let out = draft(&root, &[], None);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(0), "{stderr}");
        let patch = out.stdout;
        assert!(patch.starts_with(b"--- a/proofs/claims.toml\n"), "{stderr}");
        // Writes nothing: no manifest change, no record.
        assert_eq!(
            fs::read_to_string(root.join("proofs/claims.toml")).expect("claims"),
            before
        );
        assert!(!root.join("proofs/claims.json").exists());
        // The same patch on a second run.
        assert_eq!(draft(&root, &[], None).stdout, patch);
        assert!(stderr.contains("skipped draft_kernel::quarter"), "{stderr}");
        assert!(stderr.contains("not a gate result"), "{stderr}");

        let check = git_apply(&root, &patch, true);
        assert!(
            check.status.success(),
            "{}",
            String::from_utf8_lossy(&check.stderr)
        );
        let applied = git_apply(&root, &patch, false);
        assert!(applied.status.success());
        let after = fs::read_to_string(root.join("proofs/claims.toml")).expect("claims");
        for kept in [
            "# Hand-written: this comment and the amendment below must survive a draft\n# byte for byte.\n",
            "[[amend]]\nfile = \"draft-kernel/src/lib.rs\"\nfunction = \"add_capped\"\nkind = \"theorem\"\nname = \"Proofs.DraftKernel.add_capped_old\"\nreason = \"Replaced by add_capped_spec, which states the cap exactly.\"\n",
            "theorems = [\"Proofs.DraftKernel.add_capped_spec\", \"Proofs.DraftKernel.add_capped_comm\"]  # the headline\n",
        ] {
            assert!(after.contains(kept), "{after}");
        }

        let table = claims_of(&root);
        let k = |name: &str| format!("Proofs.DraftKernel.{name}");
        // The two functions named `step`: no cross-assignment.
        let alpha = claim(&table, "draft-kernel/src/alpha.rs", "step").expect("alpha");
        assert_eq!(strings(alpha, "theorems"), [k("alpha_step_spec")]);
        assert_eq!(strings(alpha, "mutants"), ["alpha-adds-two"]);
        assert_eq!(strings(alpha, "subsumes"), ["TAU-N001", "TAU-P003"]);
        let beta = claim(&table, "draft-kernel/src/beta.rs", "step").expect("beta");
        assert_eq!(strings(beta, "theorems"), [k("beta_step_spec")]);
        assert!(beta.get("total").is_none() && beta.get("subsumes").is_none());
        // The helper keeps its total theorem; its caller gets no claim.
        let halve = claim(&table, LIB, "halve").expect("halve");
        assert_eq!(
            halve.get("total").and_then(toml::Value::as_str),
            Some(k("halve_spec").as_str())
        );
        assert!(claim(&table, LIB, "quarter").is_none());
        // The loop's total theorem joins `count_up`, never as its `total`.
        let count = claim(&table, LIB, "count_up").expect("count_up");
        assert_eq!(
            strings(count, "theorems"),
            [k("count_up_loop_spec"), k("count_up_spec")]
        );
        assert!(count.get("total").is_none());
        // A conditional theorem: no total.
        let double = claim(&table, LIB, "double").expect("double");
        assert!(double.get("total").is_none());
        // The claim with assumptions: a total, but no subsumes, and no
        // theorem resting on an assumption it does not declare.
        let latest = claim(&table, LIB, "latest").expect("latest");
        assert_eq!(
            latest.get("total").and_then(toml::Value::as_str),
            Some(k("latest_total").as_str())
        );
        assert!(latest.get("subsumes").is_none());
        assert!(!strings(latest, "theorems").contains(&k("latest_with_clock")));
        assert!(strings(latest, "theorems").contains(&k("latest_idempotent")));
        assert_eq!(strings(latest, "assumes"), ["FIXTURE.counter-monotone"]);
        // The existing claim gains the unassigned mutant inside it.
        let add = claim(&table, LIB, "add_capped").expect("add_capped");
        assert_eq!(strings(add, "mutants"), ["guard-dropped", "capped-at-zero"]);
        assert_eq!(strings(add, "subsumes"), ["TAU-N001"]);

        // Drafting again changes nothing.
        let again = draft(&root, &[], None);
        assert_eq!(again.status.code(), Some(0));
        assert!(
            again.stdout.is_empty(),
            "{}",
            String::from_utf8_lossy(&again.stdout)
        );

        // The gate still judges the applied entries: the stand-in reports
        // no mutant result, so nothing drafted is proved.
        let gate = tauten(&["proofs", "--live", "--no-write"], &root, None);
        assert_eq!(gate.status.code(), Some(1));
        let outcomes = String::from_utf8_lossy(&gate.stdout);
        assert_eq!(outcomes.lines().count(), 8, "{outcomes}");
        assert!(!outcomes.contains("\"outcome\":\"proved\""), "{outcomes}");
        assert!(!root.join("proofs/claims.json").exists());
    }

    #[test]
    fn one_package_or_none() {
        let root = scratch("crate");
        let one = draft(&root, &["--crate", "draft-kernel"], None);
        assert_eq!(one.status.code(), Some(0));
        assert_eq!(one.stdout, draft(&root, &[], None).stdout);
        let unknown = draft(&root, &["--crate", "no-such-package"], None);
        assert_ne!(unknown.status.code(), Some(0));
        assert!(unknown.stdout.is_empty());
        assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown package"));
    }

    #[test]
    fn missing_tools_a_stale_translation_or_an_ambiguity_print_no_patch() {
        let root = scratch("refused");
        let before = fs::read_to_string(root.join("proofs/claims.toml")).expect("claims");
        for (stub, needle) in [
            ("missing", "docker is not installed"),
            ("drift", "stale"),
            ("ambiguous", "several declarations"),
        ] {
            let out = draft(&root, &[], Some(stub));
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{stub}: {stderr}");
            assert!(out.stdout.is_empty(), "{stub}: partial patch");
            assert!(stderr.contains(needle), "{stub}: {stderr}");
            assert!(stderr.contains("no patch printed"), "{stub}: {stderr}");
        }
        assert_eq!(
            fs::read_to_string(root.join("proofs/claims.toml")).expect("claims"),
            before
        );
        assert!(!root.join("proofs/claims.json").exists());
    }

    #[test]
    fn an_inventory_names_what_to_draft_and_is_checked() {
        let root = scratch("inventory");
        let inventory = root.join("inventory.json");
        let k = |n: &str| format!("Proofs.DraftKernel.{n}");
        let json = |role: &str| {
            format!(
                r#"{{"version": 1,
  "functions": [{{"package": "draft-kernel", "file": "{LIB}", "item": "draft_kernel::clamp", "declaration": "draft_kernel.clamp"}}],
  "theorems": [{{"name": "{}", "subject": "draft_kernel.clamp", "role": "{role}"}}],
  "mutants": [{{"name": "clamp-to-x", "item": "draft_kernel::clamp", "theorems": ["{}"]}}]}}"#,
                k("clamp_spec"),
                k("clamp_spec")
            )
        };
        fs::write(&inventory, json("total")).expect("write");
        let path = inventory.to_string_lossy().into_owned();
        let out = draft(&root, &["--inventory", &path], None);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(0), "{stderr}");
        let patch = String::from_utf8_lossy(&out.stdout);
        assert_eq!(patch.matches("+[[claim]]").count(), 1, "{patch}");
        assert!(patch.contains("+function = \"clamp\""), "{patch}");
        assert!(patch.contains(&format!("+total = \"{}\"", k("clamp_spec"))));
        // A role the audit contradicts refuses the draft.
        fs::write(&inventory, json("loop")).expect("write");
        let out = draft(&root, &["--inventory", &path], None);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        // An unsupported version is refused before the gate runs.
        fs::write(
            &inventory,
            json("total").replace("\"version\": 1", "\"version\": 2"),
        )
        .expect("write");
        let out = draft(&root, &["--inventory", &path], None);
        assert_ne!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("unsupported inventory version"));
    }

    /// A real Aeneas translation and the recorded lines of a real gate run
    /// (`fixtures/proofs-draft-real`, from `cargo tauten new demo49` with its
    /// claim removed): the draft re-adds the scaffold's claim.
    #[test]
    fn a_real_translation_and_gate_run_redraft_the_scaffold_claim() {
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/proofs-draft-real");
        let root = scratch_of(&real, "real");
        let out = draft(&root, &[], None);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(0), "{stderr}");
        let patch = String::from_utf8_lossy(&out.stdout).into_owned();
        assert_eq!(
            patch,
            "--- a/proofs/claims.toml
+++ b/proofs/claims.toml
@@ -54,3 +54,13 @@
 file = \"crates/demo49-kernel/src/lib.rs\"
 before = \"None => false\"
 after = \"None => true\"
+
+[[claim]]
+crate = \"demo49-kernel\"
+file = \"crates/demo49-kernel/src/lib.rs\"
+function = \"affordable\"
+lean = \"demo49_kernel.affordable\"
+theorems = [\"Proofs.Demo49Kernel.affordable_spec\"]
+total = \"Proofs.Demo49Kernel.affordable_spec\"
+subsumes = [\"TAU-N001\", \"TAU-P003\"]
+mutants = [\"affordable-overflow-fits\", \"affordable-strict\"]
",
            "{stderr}"
        );
        let applied = git_apply(&root, patch.as_bytes(), true);
        assert!(
            applied.status.success(),
            "{}",
            String::from_utf8_lossy(&applied.stderr)
        );
        assert!(git_apply(&root, patch.as_bytes(), false).status.success());
        let again = draft(&root, &[], None);
        assert_eq!(again.status.code(), Some(0));
        assert!(again.stdout.is_empty());
        assert!(!root.join("proofs/claims.json").exists());
    }
}
