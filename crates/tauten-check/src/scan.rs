//! Source-level scans: crate attributes, authority minting and placeholder
//! comments. Inputs are file contents; nothing here touches the filesystem.

use proc_macro2::Span;
use syn::visit::Visit;
pub use tauten_check_kernel::layout::NoStd;

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

/// How a crate root declares `no_std`: the fact TAU-K003 is decided on
/// ([`tauten_check_kernel::layout::k003`]). `Unparsed` when the file does
/// not parse.
#[must_use]
pub fn no_std(source: &str) -> NoStd {
    let Ok(file) = syn::parse_file(source) else {
        return NoStd::Unparsed;
    };
    let mut found = NoStd::Absent;
    for attr in &file.attrs {
        let here = if attr.path().is_ident("no_std") {
            NoStd::Unconditional
        } else if attr.path().is_ident("cfg_attr") {
            match &attr.meta {
                syn::Meta::List(list) => cfg_attr_no_std(list),
                syn::Meta::Path(_) | syn::Meta::NameValue(_) => NoStd::Absent,
            }
        } else {
            NoStd::Absent
        };
        found = stronger(found, here);
    }
    found
}

/// The better of two declarations: unconditional over `not(test)` over
/// any other condition over none.
const fn stronger(a: NoStd, b: NoStd) -> NoStd {
    const fn rank(f: NoStd) -> u8 {
        match f {
            NoStd::Unconditional => 3,
            NoStd::NotTest => 2,
            NoStd::Conditional => 1,
            NoStd::Absent | NoStd::Unparsed => 0,
        }
    }
    if rank(b) > rank(a) { b } else { a }
}

/// `cfg_attr(predicate, ...)` naming `no_std`: `NotTest` only for exactly
/// `not(test)`, `Conditional` for any other predicate or nesting.
fn cfg_attr_no_std(list: &syn::MetaList) -> NoStd {
    let Ok(metas) = list.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    ) else {
        return NoStd::Absent;
    };
    let mut iter = metas.iter();
    let Some(predicate) = iter.next() else {
        return NoStd::Absent;
    };
    let mut named = false;
    for meta in iter {
        match meta {
            syn::Meta::Path(path) if path.is_ident("no_std") => named = true,
            syn::Meta::List(inner) if inner.path.is_ident("cfg_attr") => {
                if cfg_attr_no_std(inner) != NoStd::Absent {
                    return NoStd::Conditional;
                }
            }
            syn::Meta::Path(_) | syn::Meta::List(_) | syn::Meta::NameValue(_) => {}
        }
    }
    if !named {
        return NoStd::Absent;
    }
    let not_test = matches!(predicate, syn::Meta::List(not) if not.path.is_ident("not")
        && not.tokens.to_string().trim() == "test");
    if not_test {
        NoStd::NotTest
    } else {
        NoStd::Conditional
    }
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
    /// Whether it is `allow` (rather than `expect`).
    pub allow: bool,
    /// Whether it is written inside `cfg_attr(..)`, so in force only under
    /// some configuration.
    pub conditional: bool,
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
        let at = hit(attr.pound_token.span);
        let path = attr.path();
        if path.is_ident("cfg_attr") {
            if let syn::Meta::List(list) = &attr.meta {
                self.conditional(list, at, crate_level);
            }
            return;
        }
        if let syn::Meta::List(list) = &attr.meta {
            self.level(list, at, crate_level, false);
        }
    }

    /// `cfg_attr(predicate, attr, ...)`: every `allow`/`expect` among the
    /// attributes, at any depth of nested `cfg_attr`, is conditional.
    fn conditional(&mut self, list: &syn::MetaList, at: Hit, crate_level: bool) {
        let Ok(metas) = list.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            return;
        };
        for meta in metas.iter().skip(1) {
            if let syn::Meta::List(inner) = meta {
                if inner.path.is_ident("cfg_attr") {
                    self.conditional(inner, at, crate_level);
                } else {
                    self.level(inner, at, crate_level, true);
                }
            }
        }
    }

    fn level(&mut self, list: &syn::MetaList, at: Hit, crate_level: bool, conditional: bool) {
        let allow = list.path.is_ident("allow");
        if !(allow || list.path.is_ident("expect")) {
            return;
        }
        for lint in list_lints(list) {
            self.found.push(LintException {
                lint,
                at,
                crate_level,
                allow,
                conditional,
            });
        }
    }
}

impl<'ast> Visit<'ast> for ExceptionVisitor {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        self.record(attr, false);
    }
}

/// Lint paths named in `allow(..)`/`expect(..)`, skipping `reason = ".."`.
/// Parsed as attribute syntax, so commas inside the reason string are not
/// separators.
fn list_lints(list: &syn::MetaList) -> Vec<String> {
    let Ok(metas) = list.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    ) else {
        return Vec::new();
    };
    metas
        .iter()
        .filter_map(|meta| match meta {
            syn::Meta::Path(path) => Some(
                path.segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::"),
            ),
            syn::Meta::List(_) | syn::Meta::NameValue(_) => None,
        })
        .collect()
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
        assert_eq!(no_std(src), NoStd::Unconditional);
        assert_eq!(no_std("#![cfg_attr(not(test), no_std)]\n"), NoStd::NotTest);
        assert_eq!(
            no_std("#![cfg_attr(feature = \"x\", no_std)]\n"),
            NoStd::Conditional
        );
        assert_eq!(
            no_std("#![cfg_attr(not(feature = \"std\"), no_std)]\n"),
            NoStd::Conditional
        );
        assert_eq!(
            no_std("#![cfg_attr(unix, cfg_attr(not(test), no_std))]\n"),
            NoStd::Conditional
        );
        assert_eq!(no_std("pub fn f() {}"), NoStd::Absent);
        assert_eq!(no_std("pub fn f( {"), NoStd::Unparsed);
    }

    #[test]
    fn exception_lints_ignore_commas_in_the_reason() {
        let src = "#![allow(clippy::a, dead_code, reason = \"x, y z\")]\n\
                   #[expect(clippy::b, reason = \"p, q\")]\nfn f() {}\n";
        let found = lint_exceptions(src).expect("parses");
        let lints: Vec<(&str, bool, bool)> = found
            .iter()
            .map(|e| (e.lint.as_str(), e.crate_level, e.allow))
            .collect();
        assert_eq!(
            lints,
            vec![
                ("clippy::a", true, true),
                ("dead_code", true, true),
                ("clippy::b", false, false)
            ]
        );
    }

    #[test]
    fn cfg_attr_without_no_std_is_not_no_std() {
        assert_eq!(
            no_std("#![cfg_attr(test, allow(dead_code))]\n"),
            NoStd::Absent
        );
        assert_eq!(
            no_std("#![cfg_attr(not(test), some_tool)]\n"),
            NoStd::Absent
        );
        assert_eq!(
            no_std("#![cfg_attr(unix, tool(x, no_std))]\n"),
            NoStd::Absent
        );
        assert_eq!(no_std("#![cfg_attr(not(test), no_std)]\n"), NoStd::NotTest);
    }

    #[test]
    fn inner_doc_comments_and_literal_boundaries() {
        let found = placeholder_comments(
            "//! a todo in crate docs\nlet s = \"x\";// todo right after a literal\n",
        );
        let lines: Vec<u32> = found.iter().map(|(h, _)| h.line).collect();
        assert_eq!(lines, vec![2]);
    }

    #[test]
    fn doc_comments_are_skipped_even_when_tokenizing_fails() {
        // Unbalanced delimiters: no literal ranges are available.
        let found =
            placeholder_comments("//! a todo in crate docs\n/// a stub\nfn f( {\n// for now\n");
        let lines: Vec<u32> = found.iter().map(|(h, _)| h.line).collect();
        assert_eq!(lines, vec![4]);
    }

    #[test]
    fn comment_starting_exactly_at_a_literal_end_is_a_comment() {
        let found = placeholder_comments("let s = (\"x\"// todo\n);\n");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn exceptions_inside_cfg_attr_are_conditional() {
        let src = "#![cfg_attr(not(test), allow(clippy::a))]\n\
                   #[cfg_attr(unix, cfg_attr(test, expect(clippy::b, reason = \"x, y\")))]\n\
                   fn f() {}\n";
        let found: Vec<_> = lint_exceptions(src)
            .expect("parses")
            .into_iter()
            .map(|e| (e.lint, e.crate_level, e.allow, e.conditional))
            .collect();
        assert_eq!(
            found,
            vec![
                ("clippy::a".to_owned(), true, true, true),
                ("clippy::b".to_owned(), false, false, true),
            ]
        );
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
