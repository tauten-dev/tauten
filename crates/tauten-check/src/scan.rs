//! Source-level scans: crate attributes, authority minting and placeholder
//! comments. Inputs are file contents; nothing here touches the filesystem.

use proc_macro2::Span;
use syn::visit::Visit;

/// A location found by a scan (1-based line and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32,
}

fn hit(span: Span) -> Hit {
    let start = span.start();
    Hit {
        line: u32::try_from(start.line).unwrap_or(u32::MAX),
        column: u32::try_from(start.column).map_or(u32::MAX, |c| c.saturating_add(1)),
    }
}

/// Crate-level facts read from a crate root (`lib.rs` / `main.rs`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrateAttrs {
    /// `#![no_std]` (possibly under `cfg_attr`) is present.
    pub no_std: bool,
}

/// Read crate-level attributes. Returns `None` if the file does not parse.
#[must_use]
pub fn crate_attrs(source: &str) -> Option<CrateAttrs> {
    let file = syn::parse_file(source).ok()?;
    let no_std = file.attrs.iter().any(|attr| {
        let path = attr.path();
        path.is_ident("no_std")
            || (path.is_ident("cfg_attr") && attr_tokens(attr).contains("no_std"))
    });
    Some(CrateAttrs { no_std })
}

/// A lint named in an `#[allow]`/`#[expect]` attribute written in source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintException {
    /// Lint name as written, e.g. `clippy::unwrap_used`.
    pub lint: String,
    /// Attribute location.
    pub at: Hit,
    /// Whether the attribute is crate-level (`#![...]` at the file root).
    pub crate_level: bool,
}

/// Every lint exception written in the file. Returns `None` if the file
/// does not parse.
#[must_use]
pub fn lint_exceptions(source: &str) -> Option<Vec<LintException>> {
    let file = syn::parse_file(source).ok()?;
    let mut visitor = ExceptionVisitor::default();
    for attr in &file.attrs {
        visitor.record(attr, true);
    }
    for item in &file.items {
        visitor.visit_item(item);
    }
    Some(visitor.found)
}

#[derive(Default)]
struct ExceptionVisitor {
    found: Vec<LintException>,
}

impl ExceptionVisitor {
    fn record(&mut self, attr: &syn::Attribute, crate_level: bool) {
        let path = attr.path();
        if !(path.is_ident("allow") || path.is_ident("expect")) {
            return;
        }
        for lint in attr_tokens(attr).split(',') {
            let lint = lint.trim();
            if !lint.is_empty() && !lint.starts_with("reason") {
                self.found.push(LintException {
                    lint: lint.to_owned(),
                    at: hit(attr.pound_token.span),
                    crate_level,
                });
            }
        }
    }
}

impl<'ast> Visit<'ast> for ExceptionVisitor {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        self.record(attr, false);
    }
}

fn attr_tokens(attr: &syn::Attribute) -> String {
    match &attr.meta {
        syn::Meta::List(list) => list.tokens.to_string().replace(' ', ""),
        syn::Meta::Path(_) | syn::Meta::NameValue(_) => String::new(),
    }
}

/// Find calls that mint authority (`Authority::acquire`). Returns `None` if
/// the file does not parse.
#[must_use]
pub fn authority_mints(source: &str) -> Option<Vec<Hit>> {
    let file = syn::parse_file(source).ok()?;
    let mut visitor = MintVisitor::default();
    visitor.visit_file(&file);
    Some(visitor.hits)
}

#[derive(Default)]
struct MintVisitor {
    hits: Vec<Hit>,
}

impl<'ast> Visit<'ast> for MintVisitor {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let mut segments = path.segments.iter().rev();
        if let (Some(last), Some(prev)) = (segments.next(), segments.next())
            && last.ident == "acquire"
            && prev.ident == "Authority"
        {
            self.hits.push(hit(last.ident.span()));
        }
        syn::visit::visit_path(self, path);
    }
}

const PLACEHOLDER_MARKERS: [&str; 7] = [
    "todo",
    "fixme",
    "xxx",
    "hack",
    "for now",
    "placeholder",
    "stub",
];

/// Ordinary `//` comments (not doc comments, not inside string literals)
/// containing placeholder markers, with the marker found.
#[must_use]
pub fn placeholder_comments(source: &str) -> Vec<(Hit, &'static str)> {
    let literals = literal_ranges(source);
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let line_no = u32::try_from(index).map_or(u32::MAX, |i| i.saturating_add(1));
        for (start, _) in line.match_indices("//") {
            let Some(comment) = line.get(start..) else {
                continue;
            };
            let column = line.get(..start).map_or(0, |p| p.chars().count());
            let column = u32::try_from(column).unwrap_or(u32::MAX);
            if literals.iter().any(|r| r.contains(line_no, column)) {
                continue;
            }
            let is_doc = comment.starts_with("///") || comment.starts_with("//!");
            let lower = comment.to_ascii_lowercase();
            if !is_doc
                && let Some(marker) = PLACEHOLDER_MARKERS
                    .iter()
                    .find(|m| contains_word(&lower, m))
            {
                found.push((
                    Hit {
                        line: line_no,
                        column: column.saturating_add(1),
                    },
                    *marker,
                ));
            }
            break;
        }
    }
    found
}

/// Half-open source range of a literal token: (line, 0-based column).
#[derive(Debug, Clone, Copy)]
struct Range {
    start: (u32, u32),
    end: (u32, u32),
}

impl Range {
    fn contains(self, line: u32, column: u32) -> bool {
        self.start <= (line, column) && (line, column) < self.end
    }
}

fn literal_ranges(source: &str) -> Vec<Range> {
    let Ok(tokens) = source.parse::<proc_macro2::TokenStream>() else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    collect_literals(tokens, &mut ranges);
    ranges
}

fn collect_literals(tokens: proc_macro2::TokenStream, out: &mut Vec<Range>) {
    for token in tokens {
        match token {
            proc_macro2::TokenTree::Literal(lit) => {
                let span = lit.span();
                let pos = |p: proc_macro2::LineColumn| {
                    (
                        u32::try_from(p.line).unwrap_or(u32::MAX),
                        u32::try_from(p.column).unwrap_or(u32::MAX),
                    )
                };
                out.push(Range {
                    start: pos(span.start()),
                    end: pos(span.end()),
                });
            }
            proc_macro2::TokenTree::Group(group) => collect_literals(group.stream(), out),
            proc_macro2::TokenTree::Ident(_) | proc_macro2::TokenTree::Punct(_) => {}
        }
    }
}

fn contains_word(haystack: &str, word: &str) -> bool {
    haystack.match_indices(word).any(|(at, _)| {
        let before = haystack.get(..at).and_then(|s| s.chars().next_back());
        let after = at
            .checked_add(word.len())
            .and_then(|end| haystack.get(end..))
            .and_then(|s| s.chars().next());
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_crate_attrs() {
        let src = "#![no_std]\n#![expect(clippy::unwrap_used, reason = \"x\")]\npub fn f() {}\n";
        let attrs = crate_attrs(src).expect("parses");
        assert!(attrs.no_std);
        let cfg = crate_attrs("#![cfg_attr(not(test), no_std)]\n").expect("parses");
        assert!(cfg.no_std);
        assert!(!crate_attrs("pub fn f() {}").expect("parses").no_std);
    }

    #[test]
    fn finds_lint_exceptions() {
        let src = "#![expect(clippy::unwrap_used, reason = \"x\")]\nmod m {\n    #[allow(clippy::panic, dead_code)]\n    fn f() {}\n}\n";
        let found = lint_exceptions(src).expect("parses");
        let summary: Vec<(&str, bool, u32)> = found
            .iter()
            .map(|e| (e.lint.as_str(), e.crate_level, e.at.line))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("clippy::unwrap_used", true, 1),
                ("clippy::panic", false, 3),
                ("dead_code", false, 3)
            ]
        );
    }

    #[test]
    fn finds_authority_mints() {
        let src = "fn main() { let a = tauten::cap::Authority::acquire(); let b = Authority::acquire(); }";
        let hits = authority_mints(src).expect("parses");
        assert_eq!(hits.len(), 2);
        assert!(
            authority_mints("fn f(a: &Authority) {}")
                .expect("parses")
                .is_empty()
        );
    }

    #[test]
    fn finds_placeholder_comments() {
        let src = "let x = 1; // for now\n// TODO: later\nlet stubborn = 2; // stubborn is fine\n/// a placeholder doc\nlet s = \"// todo in a string\";\nlet u = \"http://x\"; // hack\n";
        let found = placeholder_comments(src);
        let lines: Vec<(u32, &str)> = found.iter().map(|(h, m)| (h.line, *m)).collect();
        assert_eq!(lines, vec![(1, "for now"), (2, "todo"), (6, "hack")]);
    }
}
