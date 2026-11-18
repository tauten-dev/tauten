//! Every fixture crate under `fixtures/cases` produces exactly the rule IDs
//! listed in its `[package.metadata.fixture] expect`, and nothing else
//! blocking. Accept fixtures expect nothing.

#[cfg(test)]
mod fixtures {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;
    use std::process::Command;

    /// Rules that legitimately add noise to small fixtures (docs).
    const IGNORED: [&str; 1] = ["TAU-D001"];

    fn expectations(cases: &Path) -> BTreeMap<String, BTreeSet<String>> {
        let mut out = BTreeMap::new();
        for entry in std::fs::read_dir(cases).expect("fixtures dir") {
            let manifest = entry.expect("entry").path().join("Cargo.toml");
            if !manifest.is_file() {
                continue;
            }
            let table: toml::Table = std::fs::read_to_string(&manifest)
                .expect("read")
                .parse()
                .expect("toml");
            let name = table["package"]["name"].as_str().expect("name").to_owned();
            let expect = table["package"]["metadata"]["fixture"]["expect"]
                .as_array()
                .expect("expect list")
                .iter()
                .map(|v| v.as_str().expect("id").to_owned())
                .collect();
            out.insert(name, expect);
        }
        out
    }

    #[test]
    fn fixtures_produce_exactly_their_expected_rules() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cases");
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-tauten"))
            .args(["tauten", "check", "--format", "json", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .output()
            .expect("run cargo-tauten");
        assert_eq!(
            output.status.code(),
            Some(1),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut unexpected_errors = Vec::new();
        let mut tiers_seen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for line in String::from_utf8(output.stdout).expect("utf8").lines() {
            let d: serde_json::Value = serde_json::from_str(line).expect("json line");
            let id = d["id"].as_str().expect("id").to_owned();
            let krate = d["crate"].as_str().expect("crate").to_owned();
            tiers_seen
                .entry(krate.clone())
                .or_default()
                .insert(d["tier"].to_string());
            if !id.starts_with("TAU-") {
                if d["severity"] == "error" {
                    unexpected_errors.push(format!("{krate}: {id}: {}", d["message"]));
                }
                continue;
            }
            if !IGNORED.contains(&id.as_str()) {
                found.entry(krate).or_default().insert(id);
            }
        }
        assert!(
            unexpected_errors.is_empty(),
            "non-profile errors: {unexpected_errors:#?}"
        );

        let expected = expectations(&root);
        let mut mismatches = Vec::new();
        for (krate, want) in &expected {
            let got = found.remove(krate).unwrap_or_default();
            if &got != want {
                mismatches.push(format!("{krate}: expected {want:?}, got {got:?}"));
            }
        }
        assert!(found.is_empty(), "findings in unknown crates: {found:?}");
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
        assert!(
            expected.len() >= 25,
            "fixture set shrank to {}",
            expected.len()
        );
    }
}
