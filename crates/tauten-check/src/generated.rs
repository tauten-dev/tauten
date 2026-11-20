//! The generated-file stage: committed files that a command regenerates
//! (translations, rule documentation, registries) are compared byte for
//! byte with what the command produces now.
//!
//! The shell runs each command (`cargo tauten generated`); this module
//! compares, with the proved [`tauten_check_kernel::binding::same_bytes`],
//! and decides the stage with the proved
//! [`tauten_check_kernel::control::admit`]: one `Pass` control per entry, so
//! a command that failed or an output it did not produce is never `ok`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use tauten_check_kernel::binding::same_bytes;
use tauten_check_kernel::control::{self as kernel_control, Control, Expect, Observation, Seen};

/// One `[[workspace.metadata.tauten.generated]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Short name, for `--only` and the report.
    pub name: String,
    /// The command and its arguments, run from the workspace root.
    pub command: Vec<String>,
    /// The committed file the command's standard output must equal.
    #[serde(default)]
    pub stdout: Option<String>,
    /// Files or directories the command writes (run in a scratch copy of
    /// the workspace), compared with the committed ones.
    #[serde(default)]
    pub outputs: Vec<String>,
}

/// What one entry came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every output equals the committed file.
    Same,
    /// These paths differ from, or are absent from, the committed files.
    Drift(Vec<String>),
    /// The command ran but did not produce these outputs.
    Missing(Vec<String>),
    /// The command could not be run or exited unsuccessfully.
    Failed(String),
}

/// Compare what a command produced with what is committed. Both maps are
/// keyed by path relative to the workspace root; `produced` holds every
/// file found under the declared outputs, `declared` the outputs as
/// written (a declared output with nothing under it is missing).
#[must_use]
pub fn compare(
    declared: &[String],
    committed: &BTreeMap<String, Vec<u8>>,
    produced: &BTreeMap<String, Vec<u8>>,
) -> Outcome {
    let under = |path: &str, root: &str| path == root || path.starts_with(&format!("{root}/"));
    let missing: Vec<String> = declared
        .iter()
        .filter(|d| !produced.keys().any(|p| under(p, d)))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Outcome::Missing(missing);
    }
    let paths: BTreeSet<&String> = committed.keys().chain(produced.keys()).collect();
    let drift: Vec<String> = paths
        .into_iter()
        .filter(|p| match (committed.get(*p), produced.get(*p)) {
            (Some(a), Some(b)) => !same_bytes(a, b),
            (Some(_), None) | (None, Some(_)) | (None, None) => true,
        })
        .cloned()
        .collect();
    if drift.is_empty() {
        Outcome::Same
    } else {
        Outcome::Drift(drift)
    }
}

/// Whether the stage passes: every entry ran and produced exactly the
/// committed files. Decided by the proved `control::admit`.
#[must_use]
pub fn passes(outcomes: &[Outcome]) -> bool {
    let controls: Vec<Control> = (0..outcomes.len())
        .map(|id| Control {
            id,
            expect: Expect::Pass,
        })
        .collect();
    let observed: Vec<Observation> = outcomes
        .iter()
        .enumerate()
        .map(|(id, outcome)| Observation {
            id,
            seen: match outcome {
                Outcome::Same => Seen::Pass,
                Outcome::Drift(_) => Seen::FailOther,
                Outcome::Missing(_) => Seen::Empty,
                Outcome::Failed(_) => Seen::ToolMissing,
            },
        })
        .collect();
    kernel_control::admit(&controls, &observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
        entries
            .iter()
            .map(|(p, t)| ((*p).to_owned(), t.as_bytes().to_vec()))
            .collect()
    }

    #[test]
    fn one_byte_is_drift_and_a_missing_output_is_never_ok() {
        let declared = vec!["out".to_owned()];
        let committed = files(&[("out/a.txt", "hello\n"), ("out/b.txt", "x")]);
        assert_eq!(compare(&declared, &committed, &committed), Outcome::Same);
        let changed = files(&[("out/a.txt", "hellO\n"), ("out/b.txt", "x")]);
        assert_eq!(
            compare(&declared, &committed, &changed),
            Outcome::Drift(vec!["out/a.txt".to_owned()])
        );
        let extra = files(&[
            ("out/a.txt", "hello\n"),
            ("out/b.txt", "x"),
            ("out/c.txt", ""),
        ]);
        assert_eq!(
            compare(&declared, &committed, &extra),
            Outcome::Drift(vec!["out/c.txt".to_owned()])
        );
        let none = BTreeMap::new();
        assert_eq!(
            compare(&declared, &committed, &none),
            Outcome::Missing(declared.clone())
        );
        assert!(passes(&[Outcome::Same, Outcome::Same]));
        assert!(!passes(&[Outcome::Same, Outcome::Drift(vec![])]));
        assert!(!passes(&[Outcome::Missing(vec![])]));
        assert!(!passes(&[Outcome::Failed("exit 1".into())]));
        // Nothing checked is not a pass.
        assert!(!passes(&[]));
    }
}
