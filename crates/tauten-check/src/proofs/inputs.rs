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

/// The local modules a Lean file imports (`import A.B` -> `A/B.lean`, also
/// `public import`, `private import`, `meta import` and `import all`),
/// except the translation itself.
fn imports(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace().peekable();
            while words
                .peek()
                .is_some_and(|w| matches!(*w, "public" | "private" | "meta"))
            {
                words.next();
            }
            (words.next() == Some("import")).then_some(words)
        })
        .flatten()
        .filter(|m| *m != "all" && !m.starts_with("Extracted"))
        .map(|m| format!("{}.lean", m.replace('.', "/")))
        .collect()
}

/// What one claim depends on.
pub struct Sources<'a> {
    /// `proofs/Extracted/**/*.lean`: path below `Extracted/` -> text.
    pub extracted: &'a BTreeMap<String, String>,
    /// The project's other Lean files, relative to `proofs/` -> text.
    pub lean: &'a BTreeMap<String, String>,
    /// Mutant name -> its definition text.
    pub mutants: &'a BTreeMap<String, String>,
    /// The mutants whose kill depends only on the claim's own theorems
    /// breaking (`[[mutant]]` entries without `direct`). Every other mutant
    /// (an exact `direct` set, or a custom gate's "no other theorem may
    /// fail") is also bound to every Lean file of the project.
    pub monotone: &'a BTreeSet<String>,
    /// The toolchain pins and gate configuration, one text.
    pub toolchain: &'a str,
}

/// The prefix of an input that is never recorded: binding was incomplete,
/// so the claim (or mutant) it belongs to is re-calibrated on every
/// ship-level run.
pub const UNBOUND: &str = "unbound:";

/// The inputs of a claim as recorded: without the [`UNBOUND`] ones, so the
/// proved `binding::fresh` finds a current input the record does not
/// mention, and the claim stale, on every later run.
#[must_use]
pub fn recordable(inputs: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    inputs
        .iter()
        .filter(|(k, _)| !k.starts_with(UNBOUND))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// The mutant an input is about (`mutant:<m>`, `exact:<m>`,
/// `unbound:mutant:<m>`); `None` for an input of the whole claim.
#[must_use]
pub fn mutant_of(key: &str) -> Option<&str> {
    key.strip_prefix("mutant:")
        .or_else(|| key.strip_prefix("exact:"))
        .or_else(|| key.strip_prefix("unbound:mutant:"))
}

/// The translation as one set of declarations, whatever its file layout
/// (one file, Aeneas `-split-files`, a per-module split, or the same
/// declaration kept in several files): name -> every distinct text it has.
pub struct Index {
    decls: BTreeMap<String, BTreeSet<String>>,
    /// Every dotted suffix of a declared name -> the names with it.
    suffixes: BTreeMap<String, BTreeSet<String>>,
    /// Extracted files whose namespaces nest: names there may be referred
    /// to by qualifications this scanner does not follow.
    nested: Vec<String>,
    /// The fingerprint of every Lean file of the project, paths included.
    world: String,
}

/// The deepest `namespace` nesting of a Lean file (`section`s do not count).
fn namespace_depth(text: &str) -> usize {
    let mut stack: Vec<bool> = Vec::new();
    let mut deepest = 0;
    for line in text.lines() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("namespace") => {
                stack.push(true);
                deepest = deepest.max(stack.iter().filter(|n| **n).count());
            }
            Some("section") => stack.push(false),
            Some("end") => {
                stack.pop();
            }
            _ => {}
        }
    }
    deepest
}

impl Index {
    /// Index the extracted declarations and fingerprint the Lean files.
    #[must_use]
    pub fn new(sources: &Sources<'_>) -> Self {
        let mut decls: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut nested = Vec::new();
        for (path, text) in sources.extracted {
            if namespace_depth(text) > 1 {
                nested.push(path.clone());
            }
            for (name, body) in declarations(text) {
                decls.entry(name).or_default().insert(body);
            }
        }
        let mut suffixes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for name in decls.keys() {
            let mut rest = name.as_str();
            loop {
                suffixes
                    .entry(rest.to_owned())
                    .or_default()
                    .insert(name.clone());
                match rest.split_once('.') {
                    Some((_, tail)) => rest = tail,
                    None => break,
                }
            }
        }
        let files: Vec<(String, Vec<u8>)> = sources
            .extracted
            .iter()
            .map(|(p, t)| (format!("Extracted/{p}"), t.as_bytes().to_vec()))
            .chain(
                sources
                    .lean
                    .iter()
                    .map(|(p, t)| (p.clone(), t.as_bytes().to_vec())),
            )
            .collect();
        Self {
            decls,
            suffixes,
            nested,
            world: fingerprint(&files),
        }
    }

    /// The declared names a reference in extracted code may mean: the
    /// reference itself, or a suffix of it at a dot (another crate's
    /// namespace is written out, the declaring file's is not).
    fn meant(&self, reference: &str) -> Vec<&String> {
        let mut out = Vec::new();
        let mut rest = reference;
        loop {
            if let Some((name, _)) = self.decls.get_key_value(rest) {
                out.push(name);
            }
            match rest.split_once('.') {
                Some((_, tail)) => rest = tail,
                None => break,
            }
        }
        out
    }

    /// The declared names a token of a proof file may mean: as in
    /// extracted code, plus every name it is a dotted suffix of (after
    /// `open`, a proof refers to `ns.f` as `f`).
    fn mentioned(&self, token: &str) -> Vec<&String> {
        let mut out = self.meant(token);
        if let Some(names) = self.suffixes.get(token) {
            out.extend(names);
        }
        out
    }

    /// Every declaration reachable from `roots` through references.
    fn reach<'a>(&'a self, roots: impl IntoIterator<Item = &'a String>) -> BTreeSet<&'a String> {
        let mut seen = BTreeSet::new();
        let mut todo: Vec<&String> = roots.into_iter().collect();
        while let Some(name) = todo.pop() {
            if !seen.insert(name) {
                continue;
            }
            for text in self.decls.get(name).into_iter().flatten() {
                for r in references(text) {
                    for m in self.meant(&r) {
                        if !seen.contains(m) {
                            todo.push(m);
                        }
                    }
                }
            }
        }
        seen
    }

    /// The fingerprint of one declaration: its text, or every distinct
    /// text it has across files, order-independently.
    fn print(&self, name: &str) -> String {
        let texts: Vec<(String, Vec<u8>)> = self
            .decls
            .get(name)
            .into_iter()
            .flatten()
            .map(|t| (name.to_owned(), t.as_bytes().to_vec()))
            .collect();
        fingerprint(&texts)
    }
}

/// The named inputs of a claim and their fingerprints:
/// - `lean:<decl>` for each extracted declaration reachable from its
///   function, from the extracted instances, and from every extracted name
///   its bound proof files mention (their tactics may unfold any of them);
/// - `proof:<file>` for each Lean file declaring one of its theorems and
///   the local files those import;
/// - `mutant:<name>` for each of its mutants' definitions, and
///   `exact:<name>` (every Lean file of the project) for each mutant whose
///   kill also requires that no other theorem fails;
/// - `toolchain`;
/// - [`UNBOUND`] inputs where binding is incomplete: a function that names
///   no extracted declaration, nested namespaces in the translation, a
///   mutant whose definition is not known.
#[must_use]
pub fn claim_inputs(
    claim: &super::Claim,
    sources: &Sources<'_>,
    index: &Index,
) -> BTreeMap<String, String> {
    let print = |key: &str, text: &str| fingerprint(&[(key.to_owned(), text.as_bytes().to_vec())]);
    let mut out = BTreeMap::new();

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
    let mut bound = BTreeSet::new();
    while let Some(path) = files.pop() {
        if !bound.insert(path.clone()) {
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

    let function = {
        let mut best: Option<&String> = None;
        for name in index.decls.keys() {
            let fits = claim.lean == *name || claim.lean.ends_with(&format!(".{name}"));
            if fits && best.is_none_or(|b| name.len() > b.len()) {
                best = Some(name);
            }
        }
        best
    };
    if function.is_none() {
        out.insert(
            format!("{UNBOUND}lean-root"),
            format!("`{}` names no extracted declaration", claim.lean),
        );
    }
    for path in &index.nested {
        out.insert(
            format!("{UNBOUND}layout:{path}"),
            "nested namespaces".to_owned(),
        );
    }
    let mut roots: Vec<&String> = function.into_iter().collect();
    roots.extend(index.decls.keys().filter(|n| n.starts_with("instance@")));
    for path in &bound {
        if let Some(text) = sources.lean.get(path) {
            for token in references(text) {
                roots.extend(index.mentioned(&token));
            }
        }
    }
    for name in index.reach(roots) {
        out.insert(format!("lean:{name}"), index.print(name));
    }

    for mutant in &claim.mutants {
        match sources.mutants.get(mutant) {
            Some(text) => {
                out.insert(format!("mutant:{mutant}"), print(mutant, text));
            }
            None => {
                out.insert(
                    format!("{UNBOUND}mutant:{mutant}"),
                    "definition not found".to_owned(),
                );
            }
        }
        if !sources.monotone.contains(mutant) {
            out.insert(format!("exact:{mutant}"), index.world.clone());
        }
    }
    out.insert(
        "toolchain".to_owned(),
        print("toolchain", sources.toolchain),
    );
    out
}

/// One scanned `Mutant(...)` call in `proofs/mutants.py`: its byte span and
/// its name, when its first argument is a plain string literal.
struct Call {
    start: usize,
    end: usize,
    name: Option<String>,
}

/// A byte span of a text.
type Span = (usize, usize);

/// Whether `b[at..]` starts with `prefix`.
fn starts(b: &[u8], at: usize, prefix: &[u8]) -> bool {
    b.get(at..).is_some_and(|rest| rest.starts_with(prefix))
}

/// A Python string literal whose opening quote is at `at`: the index after
/// it and, for a plain single-line literal without escapes, its contents.
/// `None` if it is unterminated.
fn python_string(text: &str, at: usize, raw: bool) -> Option<(usize, Option<String>)> {
    let b = text.as_bytes();
    let q = *b.get(at)?;
    let close = [q, q, q];
    let triple = starts(b, at, &close);
    let open = if triple { 3 } else { 1 };
    let body = at.checked_add(open)?;
    let mut j = body;
    let mut plain = !triple;
    while let Some(&c) = b.get(j) {
        if c == b'\\' && !raw {
            plain = false;
            j = j.checked_add(2)?;
            continue;
        }
        if c == b'\n' && !triple {
            return None;
        }
        if c == q && (!triple || starts(b, j, &close)) {
            let contents = text.get(body..j).map(str::to_owned).filter(|_| plain);
            return Some((j.checked_add(open)?, contents));
        }
        j = j.checked_add(1)?;
    }
    None
}

/// The first byte at or after `at` that is not whitespace.
fn next_visible(b: &[u8], at: usize) -> Option<u8> {
    b.get(at..)?
        .iter()
        .copied()
        .find(|c| !c.is_ascii_whitespace())
}

/// A `Mutant(` call being scanned.
struct Open {
    start: usize,
    depth: usize,
    name: Option<String>,
    /// Whether anything after `(` has been read.
    seen: bool,
}

/// Scan `text` as Python: the `Mutant(...)` calls, and the comment spans
/// outside them and outside strings. `None` if a string or a call is
/// unterminated.
fn python_scan(text: &str) -> Option<(Vec<Call>, Vec<Span>)> {
    let b = text.as_bytes();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80;
    let mut calls = Vec::new();
    let mut comments = Vec::new();
    let mut call: Option<Open> = None;
    let mut i = 0usize;
    while let Some(&c) = b.get(i) {
        if c == b'#' {
            let end = text
                .get(i..)
                .and_then(|rest| rest.find('\n'))
                .and_then(|n| i.checked_add(n))
                .unwrap_or(b.len());
            if call.is_none() {
                comments.push((i, end));
            }
            i = end;
            continue;
        }
        if c == b'"' || c == b'\'' {
            let (next, contents) = python_string(text, i, false)?;
            if let Some(open) = call.as_mut()
                && !open.seen
            {
                open.seen = true;
                let alone = matches!(next_visible(b, next), Some(b',' | b')'));
                open.name = contents.filter(|_| alone);
            }
            i = next;
            continue;
        }
        if ident(c) {
            let len = b
                .get(i..)
                .map_or(0, |rest| rest.iter().take_while(|c| ident(**c)).count());
            let j = i.checked_add(len)?;
            let word = text.get(i..j).unwrap_or("");
            let quoted = matches!(b.get(j), Some(b'"' | b'\''));
            if quoted && word.len() <= 2 && word.chars().all(|c| "rRbBuUfF".contains(c)) {
                let (next, _) = python_string(text, j, word.contains(['r', 'R']))?;
                if let Some(open) = call.as_mut() {
                    open.seen = true;
                }
                i = next;
                continue;
            }
            if let Some(open) = call.as_mut() {
                open.seen = true;
            } else if word == "Mutant" {
                let spaces = b
                    .get(j..)
                    .map_or(0, |rest| rest.iter().take_while(|c| **c == b' ').count());
                let k = j.checked_add(spaces)?;
                if b.get(k) == Some(&b'(') {
                    call = Some(Open {
                        start: i,
                        depth: 1,
                        name: None,
                        seen: false,
                    });
                    i = k.checked_add(1)?;
                    continue;
                }
            }
            i = j;
            continue;
        }
        if let Some(open) = call.as_mut() {
            if matches!(c, b'(' | b'[' | b'{') {
                open.seen = true;
                open.depth = open.depth.checked_add(1)?;
            } else if matches!(c, b')' | b']' | b'}') {
                open.depth = open.depth.checked_sub(1)?;
                if open.depth == 0 {
                    calls.push(Call {
                        start: open.start,
                        end: i.checked_add(1)?,
                        name: open.name.take(),
                    });
                    call = None;
                }
            } else if !c.is_ascii_whitespace() {
                open.seen = true;
            }
        }
        i = i.checked_add(1)?;
    }
    call.is_none().then_some((calls, comments))
}

/// Whitespace or a comma: what separates the entries of a tuple or list.
fn separator(c: char) -> bool {
    c.is_whitespace() || c == ','
}

/// The text of `proofs/mutants.py` that is not a named `Mutant(...)` call,
/// without comments, trailing spaces and blank lines, each removed call a
/// NUL, runs of calls (with the commas between them) one NUL: what every
/// definition also depends on (module constants, the runner), and unchanged
/// when an entry is added or removed.
fn python_shared(text: &str, calls: &[Call], comments: &[Span]) -> String {
    let mut cuts: Vec<(usize, usize, bool)> = calls
        .iter()
        .filter(|c| c.name.is_some())
        .map(|c| (c.start, c.end, true))
        .chain(comments.iter().map(|&(s, e)| (s, e, false)))
        .collect();
    cuts.sort_unstable();
    let mut raw = String::new();
    let mut at = 0;
    for (s, e, call) in cuts {
        if s < at {
            continue;
        }
        raw.push_str(text.get(at..s).unwrap_or(""));
        if call {
            raw.push('\0');
        }
        at = e;
    }
    raw.push_str(text.get(at..).unwrap_or(""));
    let pieces: Vec<&str> = raw.split('\0').collect();
    let mut kept: Vec<&str> = Vec::new();
    for (n, piece) in pieces.iter().enumerate() {
        let last = n > 0 && n.checked_add(1) == Some(pieces.len());
        if last {
            kept.push(piece.trim_start_matches(separator));
        } else if n == 0 || !piece.trim_matches(separator).is_empty() {
            kept.push(piece);
        }
    }
    kept.join("\0")
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each mutant's definition in `proofs/mutants.py` (tauten's own gate and
/// those adapted from it): the text of its `Mutant("name", ...)` call, plus
/// the rest of the file (module constants such as expected-failure sets,
/// and the runner), so an edit to anything but another entry changes it.
/// A name defined twice, an entry whose name is not a plain string
/// literal, and a file the scanner cannot read give no definition, so
/// those mutants are unbound.
#[must_use]
pub fn python_mutants(text: &str) -> BTreeMap<String, String> {
    let Some((calls, comments)) = python_scan(text) else {
        return BTreeMap::new();
    };
    let shared = python_shared(text, &calls, &comments);
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for name in calls.iter().filter_map(|c| c.name.as_deref()) {
        let n = count.entry(name).or_default();
        *n = n.saturating_add(1);
    }
    calls
        .iter()
        .filter_map(|c| {
            let name = c.name.as_deref()?;
            let entry = text.get(c.start..c.end)?;
            (count.get(name) == Some(&1)).then(|| (name.to_owned(), format!("{entry}\n\0{shared}")))
        })
        .collect()
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

    #[test]
    fn python_mutants_read_whole_calls_and_the_shared_rest() {
        let text = "K = (\"x\",)\nM = (\n    # first\n    Mutant(\"a\", \"f.rs\", \"(\", ')', K),\n    Mutant(\"b\", r\"g\\.rs\", \"\"\"t\n)\"\"\", K),\n)\n";
        let m = python_mutants(text);
        assert_eq!(m.keys().collect::<Vec<_>>(), vec!["a", "b"]);
        assert!(m["a"].starts_with("Mutant(\"a\", \"f.rs\", \"(\", ')', K)\n"));
        assert!(m["b"].contains("\"\"\"t\n)\"\"\""), "{}", m["b"]);
        // The shared rest: constants and the list, without entries or comments.
        let shared = m["a"].split_once('\0').map(|(_, s)| s).unwrap_or("");
        assert_eq!(shared, "K = (\"x\",)\nM = (\n    \0)");
        // A comment, another entry's edit and a new entry change nothing.
        let edited = text
            .replace("# first", "# reworded\n    # more")
            .replace("\"g\\\\.rs\"", "\"h.rs\"")
            .replace(
                "K),\n)",
                "K),\n    Mutant(\"c\", \"z\", \"\", \"\", ()),\n)",
            );
        assert_eq!(python_mutants(&edited)["a"], m["a"]);
        // A constant does.
        assert_ne!(
            python_mutants(&text.replace("(\"x\",)", "(\"y\",)"))["a"],
            m["a"]
        );
    }

    #[test]
    fn python_mutants_leave_unreadable_entries_unbound() {
        // Defined twice: neither is known.
        let twice = "Mutant(\"a\", 1)\nMutant(\"a\", 2)\nMutant(\"b\", 3)\n";
        assert_eq!(python_mutants(twice).keys().collect::<Vec<_>>(), vec!["b"]);
        // A keyword or computed name is not read, and stays in the shared rest.
        let keyword = "Mutant(name=\"a\")\nMutant(\"b\" + s, 1)\nMutant(\"c\", 1)\n";
        let m = python_mutants(keyword);
        assert_eq!(m.keys().collect::<Vec<_>>(), vec!["c"]);
        assert!(m["c"].contains("name=\"a\""));
        // Unterminated: nothing is known.
        assert!(python_mutants("Mutant(\"a\", \"oops)\n").is_empty());
        assert!(python_mutants("Mutant(\"a\", 1\n").is_empty());
        // Not a call, or another name.
        let other = "class Mutant:\n    pass\nx = SubMutant(\"a\")\ndef f(m: Mutant): pass\n";
        assert!(python_mutants(other).is_empty());
    }

    #[test]
    fn module_system_imports_are_local_modules() {
        let text = "import Extracted.K\npublic import A.B\nmeta import C\nimport all D\nprivate import E F\n";
        assert_eq!(
            imports(text),
            vec!["A/B.lean", "C.lean", "D.lean", "E.lean", "F.lean"]
        );
    }

    #[test]
    fn namespace_depth_ignores_sections() {
        assert_eq!(namespace_depth("namespace a\nsection\nend\nend a\n"), 1);
        assert_eq!(
            namespace_depth("namespace a\nnamespace b\nend b\nend a\n"),
            2
        );
    }
}
