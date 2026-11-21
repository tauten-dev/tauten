//! What a claim's recorded result depends on, input by input (D2, #36).
//!
//! A claim's mutant calibration is about the translation of its function
//! and of everything that translation uses, about the Lean files that state
//! and prove its theorems, and about its mutants' definitions. Each becomes
//! one named input with a fingerprint; a ship-level gate re-runs a claim's
//! mutants only when the proved `binding::fresh` says an input moved
//! ([`super::unchanged`]). An edit to Rust code the translation does not
//! reach changes no input.
//!
//! Read from the committed `proofs/Extracted/`, which the same gate run
//! checks against a fresh translation (the drift stage), so the inputs are
//! about the current source whenever the run passes.
//!
//! Trusted, not proved: this scanner of Lean declarations (tested), which
//! over-approximates references (any identifier naming a declaration).

use std::collections::{BTreeMap, BTreeSet};

use super::fingerprint;

/// Keywords that open a declaration in the translation and in proof files.
const DECLARATION: &[&str] = &[
    "def",
    "structure",
    "inductive",
    "abbrev",
    "instance",
    "theorem",
    "lemma",
    "opaque",
    "axiom",
    "class",
];

/// Modifiers that may precede a declaration keyword.
const MODIFIERS: &[&str] = &[
    "private",
    "protected",
    "noncomputable",
    "partial",
    "unsafe",
    "nonrec",
];

/// Lines that end a declaration without starting another.
const SECTION: &[&str] = &[
    "namespace",
    "end",
    "open",
    "section",
    "import",
    "set_option",
];

/// The declaration a line opens, if any: its name (`instance` without a
/// name is keyed by its line).
pub(super) fn opens(line: &str, number: usize) -> Option<String> {
    let mut words = line.split_whitespace().peekable();
    while let Some(word) = words.peek() {
        if word.starts_with("@[") {
            // An attribute on the same line: skip to the word that closes it.
            for w in words.by_ref() {
                if w.ends_with(']') {
                    break;
                }
            }
            continue;
        }
        if MODIFIERS.contains(word) {
            words.next();
            continue;
        }
        break;
    }
    let keyword = words.next()?;
    if !DECLARATION.contains(&keyword) {
        return None;
    }
    let name: String = words
        .next()
        .unwrap_or("")
        .chars()
        .take_while(|c| !matches!(c, ':' | '(' | '{' | '[' | ' '))
        .collect();
    Some(
        if name.is_empty() || keyword == "instance" && name.starts_with(':') {
            format!("instance@{number}")
        } else {
            name
        },
    )
}

/// The declarations of a Lean file: name -> text, without doc comments,
/// comments and attribute-only lines (so a moved `Source:` line number or a
/// reworded doc comment changes nothing).
#[must_use]
pub fn declarations(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut current: Option<(String, String)> = None;
    let mut in_comment = 0usize;
    for (number, raw) in text.lines().enumerate() {
        let mut line = raw.to_owned();
        if in_comment > 0 || line.trim_start().starts_with("/-") {
            in_comment = in_comment
                .saturating_add(line.matches("/-").count())
                .saturating_sub(line.matches("-/").count());
            continue;
        }
        if let Some(i) = line.find("--") {
            line.truncate(i);
        }
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let first = trimmed.split_whitespace().next().unwrap_or("");
        let attribute_only = trimmed.starts_with("@[") && trimmed.ends_with(']');
        if let Some(name) = opens(trimmed, number) {
            if let Some((n, t)) = current.take() {
                out.insert(n, t);
            }
            current = Some((name, format!("{}\n", line.trim_end())));
        } else if SECTION.contains(&first) {
            if let Some((n, t)) = current.take() {
                out.insert(n, t);
            }
        } else if !attribute_only && let Some((_, t)) = current.as_mut() {
            t.push_str(line.trim_end());
            t.push('\n');
        }
    }
    if let Some((n, t)) = current {
        out.insert(n, t);
    }
    out
}

/// Identifiers in `text` (dotted names, with every dotted prefix).
fn references(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for token in text.split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.' | '\''))) {
        let token = token.trim_matches('.');
        if token.is_empty() {
            continue;
        }
        let mut name = token;
        loop {
            out.insert(name.to_owned());
            match name.rsplit_once('.') {
                Some((prefix, _)) => name = prefix,
                None => break,
            }
        }
    }
    out
}

/// The declaration `lean` names in `decls`: the one whose name is `lean` or
/// a suffix of it at a dot (the translation omits the crate namespace).
fn resolve<'a>(decls: &'a BTreeMap<String, String>, lean: &str) -> Option<&'a String> {
    decls
        .keys()
        .filter(|name| lean == name.as_str() || lean.ends_with(&format!(".{name}")))
        .max_by_key(|name| name.len())
}

/// Every declaration reachable from `root` through references, `root`
/// included; empty when `root` is not declared.
#[must_use]
pub fn closure(decls: &BTreeMap<String, String>, root: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let Some(start) = resolve(decls, root) else {
        return seen;
    };
    let mut todo = vec![start.clone()];
    while let Some(name) = todo.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(text) = decls.get(&name) {
            for r in references(text) {
                if r != name && decls.contains_key(&r) && !seen.contains(&r) {
                    todo.push(r);
                }
            }
        }
    }
    seen
}

/// The local modules a Lean file imports (`import A.B` -> `A/B.lean`),
/// except the translation itself.
fn imports(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("import "))
        .flat_map(str::split_whitespace)
        .filter(|m| !m.starts_with("Extracted"))
        .map(|m| format!("{}.lean", m.replace('.', "/")))
        .collect()
}

/// What one claim depends on.
pub struct Sources<'a> {
    /// `proofs/Extracted/*.lean`: file name -> text.
    pub extracted: &'a BTreeMap<String, String>,
    /// The project's other Lean files, relative to `proofs/` -> text.
    pub lean: &'a BTreeMap<String, String>,
    /// Mutant name -> its definition text.
    pub mutants: &'a BTreeMap<String, String>,
    /// The toolchain pins and gate configuration, one text.
    pub toolchain: &'a str,
}

/// The named inputs of a claim and their fingerprints: `lean:<decl>` for
/// each declaration of the translation its function reaches, `proof:<file>`
/// for each Lean file declaring one of its theorems and the local files
/// those import, `mutant:<name>` for each of its mutants, and `toolchain`.
#[must_use]
pub fn claim_inputs(claim: &super::Claim, sources: &Sources<'_>) -> BTreeMap<String, String> {
    let print = |key: &str, text: &str| fingerprint(&[(key.to_owned(), text.as_bytes().to_vec())]);
    let mut out = BTreeMap::new();
    for text in sources.extracted.values() {
        let decls = declarations(text);
        for name in closure(&decls, &claim.lean) {
            if let Some(body) = decls.get(&name) {
                out.insert(format!("lean:{name}"), print(&name, body));
            }
        }
    }
    let theorems: BTreeSet<&str> = super::required_theorems(claim)
        .into_iter()
        .map(|t| t.rsplit('.').next().unwrap_or(t))
        .collect();
    let mut files: Vec<String> = sources
        .lean
        .iter()
        .filter(|(_, text)| {
            declarations(text)
                .keys()
                .any(|d| theorems.contains(d.rsplit('.').next().unwrap_or(d)))
        })
        .map(|(path, _)| path.clone())
        .collect();
    let mut seen = BTreeSet::new();
    while let Some(path) = files.pop() {
        if !seen.insert(path.clone()) {
            continue;
        }
        if let Some(text) = sources.lean.get(&path) {
            out.insert(format!("proof:{path}"), print(&path, text));
            files.extend(
                imports(text)
                    .into_iter()
                    .filter(|p| sources.lean.contains_key(p)),
            );
        }
    }
    for mutant in &claim.mutants {
        let text = sources.mutants.get(mutant).map_or("", String::as_str);
        out.insert(format!("mutant:{mutant}"), print(mutant, text));
    }
    out.insert(
        "toolchain".to_owned(),
        print("toolchain", sources.toolchain),
    );
    out
}

/// Each mutant's definition in `proofs/mutants.py` (tauten's own gate): the
/// text from its `Mutant("name"` to the next `Mutant(`.
#[must_use]
pub fn python_mutants(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    // Each piece after a split is one mutant's definition, name first.
    for piece in text.split("Mutant(\"").skip(1) {
        if let Some((name, _)) = piece.split_once('"') {
            out.insert(name.to_owned(), format!("Mutant(\"{piece}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXTRACTED: &str = r#"
namespace k

/-- [k::rank]:
    Source: 'src/lib.rs', lines 10:0-14:1 -/
def tiers.rank (of : tiers.Tier) : Result U8 := do
  match of with
  | tiers.Tier.Kernel => ok 0#u8
  | tiers.Tier.Core => ok 1#u8

/-- [k::Tier] -/
inductive tiers.Tier where
| Kernel : tiers.Tier
| Core : tiers.Tier

@[reducible]
def tiers.may (of : tiers.Tier) (dep : tiers.Tier) : Result Bool := do
  let i ← tiers.rank dep
  let i1 ← tiers.rank of
  ok (i <= i1)

def other.f (x : U8) : Result U8 := ok x

end k
"#;

    #[test]
    fn declarations_and_their_closure() {
        let decls = declarations(EXTRACTED);
        assert_eq!(
            decls.keys().collect::<Vec<_>>(),
            vec!["other.f", "tiers.Tier", "tiers.may", "tiers.rank"]
        );
        let reach = closure(&decls, "k.tiers.may");
        assert_eq!(
            reach.into_iter().collect::<Vec<_>>(),
            vec!["tiers.Tier", "tiers.may", "tiers.rank"]
        );
        assert!(closure(&decls, "k.missing").is_empty());
    }

    #[test]
    fn source_lines_and_doc_comments_do_not_count() {
        let moved = EXTRACTED
            .replace("lines 10:0-14:1", "lines 40:0-44:1")
            .replace("[k::Tier]", "[k::Tier] reworded");
        assert_eq!(declarations(EXTRACTED), declarations(&moved));
        let edited = EXTRACTED.replace("ok 1#u8", "ok 2#u8");
        assert_ne!(
            declarations(EXTRACTED).get("tiers.rank"),
            declarations(&edited).get("tiers.rank")
        );
        assert_eq!(
            declarations(EXTRACTED).get("other.f"),
            declarations(&edited).get("other.f")
        );
    }

    #[test]
    fn python_mutants_are_split_by_name() {
        let text = "M = (\n    Mutant(\"a\", \"f.rs\",\n x),\n    Mutant(\"b\", \"g.rs\", y),\n)";
        let m = python_mutants(text);
        assert_eq!(m.keys().collect::<Vec<_>>(), vec!["a", "b"]);
        assert!(m["a"].contains("f.rs") && !m["a"].contains("g.rs"));
    }
}
