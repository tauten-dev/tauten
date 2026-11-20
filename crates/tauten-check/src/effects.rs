//! Facts for TAU-F001 (declared effects match the signature).
//!
//! This module only extracts facts: for each function, the row its
//! `#[effects(..)]` declares, one [`Source`] per capability its parameters
//! carry, whether it returns a `Result`, whether it is `async`, and whether
//! every parameter type could be classified. The decision is
//! [`tauten_check_kernel::effects::verdict`], which is proved.
//!
//! Classification is by type name, so it is a heuristic, and it errs toward
//! "unresolved" rather than "plain": a type it does not recognise (a type
//! from another crate, a generic with an unfamiliar bound) makes the
//! function unresolved, and the proved verdict then only warns.
//!
//! * `Clock`, `SystemClock`, `ManualClock`, or a `Clock` bound: `clock`.
//! * `Env`: `env`. `Dir`: `fs_read`, `fs_write`. `ReadDir`: `fs_read`.
//!   `Net`: `net`.
//! * A struct, enum or alias declared in the same crate: its fields,
//!   recursively, with generic arguments substituted.
//! * Primitives, `String`, paths, durations and standard containers: their
//!   type arguments.
//! * `Authority` and everything else: unresolved.

use std::collections::{BTreeMap, BTreeSet};

use syn::visit::Visit;
use tauten_check_kernel::effects::{Row, Source};

use crate::scan::Hit;

/// The labels `#[effects(..)]` accepts, in `Row` field order. Equal to
/// `tauten::effects::LABELS` (tested).
pub const LABELS: [&str; 7] = [
    "clock", "env", "fs_read", "fs_write", "net", "fail", "suspend",
];

/// The facts about one function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnFacts {
    /// Index of the file it is in.
    pub file: usize,
    /// Its name, qualified by the impl's self type for methods.
    pub name: String,
    /// Location of the name.
    pub at: Hit,
    /// `pub` (only `pub` functions must declare effects).
    pub public: bool,
    /// The declared row, or `None` without `#[effects]`.
    pub declared: Option<Row>,
    /// One source per capability reachable from the parameters.
    pub sources: Vec<Source>,
    /// Returns a `Result`.
    pub fallible: bool,
    /// `async fn`.
    pub asynchronous: bool,
    /// Parameter types that could not be classified (empty when resolved).
    pub unresolved: Vec<String>,
}

/// Row with the given labels set; `None` if a label is unknown.
#[must_use]
pub fn row_of(labels: &[&str]) -> Option<Row> {
    let mut row = tauten_check_kernel::effects::empty();
    for label in labels {
        match *label {
            "clock" => row.clock = true,
            "env" => row.env = true,
            "fs_read" => row.fs_read = true,
            "fs_write" => row.fs_write = true,
            "net" => row.net = true,
            "fail" => row.fail = true,
            "suspend" => row.suspend = true,
            _ => return None,
        }
    }
    Some(row)
}

/// The labels set in `row`, in `LABELS` order.
#[must_use]
pub fn labels_of(row: Row) -> Vec<&'static str> {
    let flags = [
        row.clock,
        row.env,
        row.fs_read,
        row.fs_write,
        row.net,
        row.fail,
        row.suspend,
    ];
    LABELS
        .iter()
        .zip(flags)
        .filter_map(|(l, f)| f.then_some(*l))
        .collect()
}

/// Extract the facts for every function that declares effects, and every
/// `pub` function, across the files of one crate. Files that do not parse
/// contribute nothing (the compiler reports them).
#[must_use]
pub fn facts(files: &[&str]) -> Vec<FnFacts> {
    let parsed: Vec<Option<syn::File>> = files.iter().map(|t| syn::parse_file(t).ok()).collect();
    let mut table = Table::default();
    for file in parsed.iter().flatten() {
        table.collect(&file.items);
    }
    let mut out = Vec::new();
    for (index, file) in parsed.iter().enumerate() {
        if let Some(file) = file {
            let mut walker = Walker {
                table: &table,
                file: index,
                out: &mut out,
            };
            walker.visit_file(file);
        }
    }
    out
}

/// A type declared in the crate: its generic parameter names (with their
/// trait bounds) and the types of all its fields.
#[derive(Debug, Clone)]
struct Decl {
    params: Vec<(String, Vec<String>)>,
    fields: Vec<syn::Type>,
}

#[derive(Default)]
struct Table {
    decls: BTreeMap<String, Vec<Decl>>,
}

impl Table {
    fn collect(&mut self, items: &[syn::Item]) {
        for item in items {
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "syn::Item is non_exhaustive; other items declare no fields"
            )]
            match item {
                syn::Item::Struct(s) => {
                    let fields = s.fields.iter().map(|f| f.ty.clone()).collect();
                    self.add(&s.ident, &s.generics, fields);
                }
                syn::Item::Enum(e) => {
                    let fields = e
                        .variants
                        .iter()
                        .flat_map(|v| v.fields.iter().map(|f| f.ty.clone()))
                        .collect();
                    self.add(&e.ident, &e.generics, fields);
                }
                syn::Item::Type(t) => self.add(&t.ident, &t.generics, vec![(*t.ty).clone()]),
                syn::Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        self.collect(inner);
                    }
                }
                _ => {}
            }
        }
    }

    fn add(&mut self, ident: &syn::Ident, generics: &syn::Generics, fields: Vec<syn::Type>) {
        self.decls.entry(ident.to_string()).or_default().push(Decl {
            params: type_params(generics),
            fields,
        });
    }
}

/// Generic type parameters with their trait bounds (inline and `where`).
fn type_params(generics: &syn::Generics) -> Vec<(String, Vec<String>)> {
    let mut params: Vec<(String, Vec<String>)> = generics
        .type_params()
        .map(|p| (p.ident.to_string(), bound_names(p.bounds.iter())))
        .collect();
    if let Some(clause) = &generics.where_clause {
        for pred in &clause.predicates {
            if let syn::WherePredicate::Type(pt) = pred
                && let syn::Type::Path(tp) = &pt.bounded_ty
                && tp.qself.is_none()
                && let Some(ident) = tp.path.get_ident()
                && let Some(entry) = params.iter_mut().find(|(n, _)| ident == n)
            {
                entry.1.extend(bound_names(pt.bounds.iter()));
            }
        }
    }
    params
}

fn bound_names<'a>(bounds: impl Iterator<Item = &'a syn::TypeParamBound>) -> Vec<String> {
    bounds
        .filter_map(|b| {
            let syn::TypeParamBound::Trait(t) = b else {
                return None;
            };
            t.path.segments.last().map(|s| s.ident.to_string())
        })
        .collect()
}

/// What a type was classified as.
#[derive(Debug, Clone, Default)]
struct Class {
    sources: Vec<Source>,
    unresolved: Vec<String>,
}

impl Class {
    fn one(source: Source) -> Self {
        Self {
            sources: vec![source],
            unresolved: Vec::new(),
        }
    }

    fn unknown(what: String) -> Self {
        Self {
            sources: Vec::new(),
            unresolved: vec![what],
        }
    }

    fn join(&mut self, other: Self) {
        self.sources.extend(other.sources);
        self.unresolved.extend(other.unresolved);
    }
}

/// Traits that confer no capability: a value bounded only by these is data
/// or a caller-supplied sink.
const PLAIN_TRAITS: [&str; 28] = [
    "Fn",
    "FnMut",
    "FnOnce",
    "Iterator",
    "IntoIterator",
    "DoubleEndedIterator",
    "ExactSizeIterator",
    "AsRef",
    "AsMut",
    "Borrow",
    "Into",
    "From",
    "TryInto",
    "ToString",
    "Display",
    "Debug",
    "Clone",
    "Copy",
    "Send",
    "Sync",
    "Sized",
    "Hash",
    "Eq",
    "PartialEq",
    "Ord",
    "PartialOrd",
    "Default",
    "Error",
];

/// Caller-supplied IO sinks and sources (`impl Write`, `impl Read`): the
/// caller chose them, so they grant nothing ambient.
const SINK_TRAITS: [&str; 4] = ["Read", "Write", "BufRead", "Seek"];

/// Types that are data; their type arguments are classified.
const PLAIN_TYPES: [&str; 54] = [
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "f32",
    "f64",
    "bool",
    "char",
    "str",
    "String",
    "Path",
    "PathBuf",
    "OsStr",
    "OsString",
    "Duration",
    "Instant",
    "SystemTime",
    "Vec",
    "VecDeque",
    "Option",
    "Box",
    "Arc",
    "Rc",
    "Cow",
    "Result",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "BinaryHeap",
    "RefCell",
    "Cell",
    "Mutex",
    "RwLock",
    "PhantomData",
    "Range",
    "RangeInclusive",
    "Ordering",
    "NonZeroU32",
    "NonZeroU64",
    "NonZeroUsize",
    "Weak",
    "Formatter",
    "Arguments",
    "Error",
    "Infallible",
];

struct Env<'a> {
    table: &'a Table,
    /// Generic parameter name -> its classification in this context.
    params: BTreeMap<String, Class>,
    /// `Self`, when inside an impl.
    self_ty: Option<&'a syn::Type>,
    /// Crate types being expanded (cycle guard).
    expanding: BTreeSet<String>,
}

impl Env<'_> {
    /// The capabilities a value bounded by `bounds` can carry. An
    /// unbounded generic carries none.
    fn bounds(&self, name: &str, bounds: &[String]) -> Class {
        let mut class = Class::default();
        for bound in bounds {
            match bound.as_str() {
                "Clock" => class.join(Class::one(Source::Clock)),
                b if PLAIN_TRAITS.contains(&b) || SINK_TRAITS.contains(&b) => {}
                b => class.join(Class::unknown(format!("{name}: {b}"))),
            }
        }
        class
    }

    fn classify(&mut self, ty: &syn::Type) -> Class {
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "syn::Type is non_exhaustive; a future kind is unclassified"
        )]
        match ty {
            syn::Type::Reference(r) => self.classify(&r.elem),
            syn::Type::Ptr(p) => self.classify(&p.elem),
            syn::Type::Paren(p) => self.classify(&p.elem),
            syn::Type::Slice(s) => self.classify(&s.elem),
            syn::Type::Array(a) => self.classify(&a.elem),
            syn::Type::Tuple(t) => {
                let mut class = Class::default();
                for elem in &t.elems {
                    class.join(self.classify(elem));
                }
                class
            }
            syn::Type::Never(_) | syn::Type::FnPtr(_) => Class::default(),
            syn::Type::ImplTrait(i) => {
                let names = bound_names(i.bounds.iter());
                self.bounds("impl", &names)
            }
            syn::Type::TraitObject(o) => {
                let names = bound_names(o.bounds.iter());
                self.bounds("dyn", &names)
            }
            syn::Type::Path(p) if p.qself.is_none() => self.path(&p.path),
            // A qualified path, `_`, a macro, or a future kind.
            _ => Class::unknown(quote_type(ty)),
        }
    }

    fn path(&mut self, path: &syn::Path) -> Class {
        let Some(last) = path.segments.last() else {
            return Class::default();
        };
        let name = last.ident.to_string();
        let args: Vec<&syn::Type> = match &last.arguments {
            syn::PathArguments::AngleBracketed(a) => a
                .args
                .iter()
                .filter_map(|g| {
                    let syn::GenericArgument::Type(t) = g else {
                        return None;
                    };
                    Some(t)
                })
                .collect(),
            syn::PathArguments::None | syn::PathArguments::Parenthesized(_) => Vec::new(),
        };
        if path.segments.len() == 1
            && let Some(class) = self.params.get(&name)
        {
            return class.clone();
        }
        match name.as_str() {
            "Clock" | "SystemClock" | "ManualClock" => return Class::one(Source::Clock),
            "Env" => return Class::one(Source::Env),
            "Dir" => return Class::one(Source::Dir),
            "ReadDir" => return Class::one(Source::ReadDir),
            "Net" => return Class::one(Source::Net),
            "Self" => {
                return match self.self_ty {
                    Some(ty) => {
                        let ty = ty.clone();
                        self.classify(&ty)
                    }
                    None => Class::unknown("Self".to_owned()),
                };
            }
            _ => {}
        }
        if let Some(decls) = self.table.decls.get(&name).cloned() {
            if !self.expanding.insert(name.clone()) {
                return Class::default();
            }
            let mut class = Class::default();
            for decl in &decls {
                let mut params = BTreeMap::new();
                for (i, (pname, pbounds)) in decl.params.iter().enumerate() {
                    let c = match args.get(i) {
                        Some(arg) => self.classify(arg),
                        None => self.bounds(pname, pbounds),
                    };
                    params.insert(pname.clone(), c);
                }
                let saved = std::mem::replace(&mut self.params, params);
                for field in &decl.fields {
                    let c = self.classify(field);
                    class.join(c);
                }
                self.params = saved;
            }
            self.expanding.remove(&name);
            return class;
        }
        if PLAIN_TYPES.contains(&name.as_str()) {
            let mut class = Class::default();
            for arg in args {
                class.join(self.classify(arg));
            }
            return class;
        }
        Class::unknown(path_text(path))
    }
}

fn path_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn quote_type(ty: &syn::Type) -> String {
    use quote::ToTokens;
    ty.to_token_stream().to_string()
}

struct Walker<'a, 'o> {
    table: &'a Table,
    file: usize,
    out: &'o mut Vec<FnFacts>,
}

/// The labels of an `#[effects(..)]` attribute: `None` if absent,
/// `Some(None)` if present but not a list of known labels.
fn declared(attrs: &[syn::Attribute]) -> Option<Option<Row>> {
    let attr = attrs.iter().find(|a| {
        a.path()
            .segments
            .last()
            .is_some_and(|s| s.ident == "effects")
    })?;
    let labels: Vec<String> = match &attr.meta {
        syn::Meta::Path(_) => Vec::new(),
        syn::Meta::List(_) => match attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated,
        ) {
            Ok(idents) => idents.iter().map(ToString::to_string).collect(),
            Err(_) => return Some(None),
        },
        syn::Meta::NameValue(_) => return Some(None),
    };
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    Some(row_of(&refs))
}

fn returns_result(output: &syn::ReturnType) -> bool {
    match output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ty) => {
            let syn::Type::Path(p) = &**ty else {
                return false;
            };
            p.path.segments.last().is_some_and(|s| s.ident == "Result")
        }
    }
}

impl Walker<'_, '_> {
    fn function(
        &mut self,
        attrs: &[syn::Attribute],
        sig: &syn::Signature,
        public: bool,
        outer: Option<(&syn::Type, &syn::Generics)>,
    ) {
        let declared = match declared(attrs) {
            Some(Some(row)) => Some(row),
            // Unknown labels: `#[effects]` itself reports them.
            Some(None) => return,
            None if public => None,
            None => return,
        };
        let mut env = Env {
            table: self.table,
            params: BTreeMap::new(),
            self_ty: outer.map(|(ty, _)| ty),
            expanding: BTreeSet::new(),
        };
        let mut generics = Vec::new();
        if let Some((_, g)) = outer {
            generics.extend(type_params(g));
        }
        generics.extend(type_params(&sig.generics));
        for (name, bounds) in &generics {
            let class = env.bounds(name, bounds);
            env.params.insert(name.clone(), class);
        }
        let mut class = Class::default();
        for input in &sig.inputs {
            let c = match input {
                syn::FnArg::Receiver(_) => match outer {
                    Some((ty, _)) => env.classify(ty),
                    None => Class::default(),
                },
                syn::FnArg::Typed(pt) => env.classify(&pt.ty),
            };
            class.join(c);
        }
        let name = match outer {
            Some((ty, _)) => format!("{}::{}", quote_type(ty).replace(' ', ""), sig.ident),
            None => sig.ident.to_string(),
        };
        let start = sig.ident.span().start();
        self.out.push(FnFacts {
            file: self.file,
            name,
            at: Hit {
                line: u32::try_from(start.line).unwrap_or(u32::MAX),
                column: u32::try_from(start.column).map_or(u32::MAX, |c| c.saturating_add(1)),
            },
            public,
            declared,
            sources: class.sources,
            fallible: returns_result(&sig.output),
            asynchronous: sig.asyncness.is_some(),
            unresolved: class.unresolved,
        });
    }
}

impl<'ast> Visit<'ast> for Walker<'_, '_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let public = matches!(f.vis, syn::Visibility::Public(_));
        self.function(&f.attrs, &f.sig, public, None);
    }

    fn visit_item_impl(&mut self, imp: &'ast syn::ItemImpl) {
        for item in &imp.items {
            if let syn::ImplItem::Fn(f) = item {
                // Trait methods are as public as the trait: only checked
                // when they declare effects.
                let public = imp.trait_.is_none() && matches!(f.vis, syn::Visibility::Public(_));
                self.function(
                    &f.attrs,
                    &f.sig,
                    public,
                    Some((&imp.self_ty, &imp.generics)),
                );
            }
        }
    }

    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        // `#[cfg(test)]` modules are not API.
        let is_test = m.attrs.iter().any(|a| {
            a.path().is_ident("cfg") && a.parse_args::<syn::Ident>().is_ok_and(|i| i == "test")
        });
        if !is_test {
            syn::visit::visit_item_mod(self, m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauten_check_kernel::effects::{Verdict, derive, verdict};

    fn one(src: &str) -> FnFacts {
        let mut all = facts(&[src]);
        assert_eq!(all.len(), 1, "{all:?}");
        all.remove(0)
    }

    fn decide(f: &FnFacts, required: bool) -> Verdict {
        let derived = derive(&f.sources, f.fallible, f.asynchronous);
        verdict(f.declared, derived, required, f.unresolved.is_empty())
    }

    #[test]
    fn labels_match_the_library() {
        assert_eq!(LABELS.as_slice(), tauten::effects::LABELS);
        for label in LABELS {
            let row = row_of(&[label]).expect("known");
            assert_eq!(labels_of(row), vec![label]);
        }
        assert!(row_of(&["time"]).is_none());
    }

    #[test]
    fn matching_declaration_is_silent() {
        let f = one("#[effects(clock, fs_read, fail)]\n\
             pub fn f(c: &impl Clock, d: &ReadDir, n: u64) -> Result<u64, E> { todo!() }");
        assert!(f.unresolved.is_empty(), "{:?}", f.unresolved);
        assert_eq!(decide(&f, true), Verdict::Silent);
    }

    #[test]
    fn dir_grants_writing_and_readdir_does_not() {
        let f = one("#[effects(fs_read)]\npub fn f(d: &Dir) {}");
        assert_eq!(decide(&f, true), Verdict::Error);
        let f = one("#[effects(fs_read, fs_write)]\npub fn f(d: &ReadDir) {}");
        assert_eq!(decide(&f, true), Verdict::Error);
        assert_eq!(decide(&f, false), Verdict::Warning);
    }

    #[test]
    fn bundles_generics_and_self_are_expanded() {
        let src = "#[derive(Cap)] pub struct Caps<C> { clock: C, dir: Option<ReadDir> }\n\
                   pub struct Svc<C: Clock> { caps: Caps<C>, n: u32 }\n\
                   impl<C: Clock> Svc<C> {\n\
                     #[effects(clock, fs_read, suspend)]\n\
                     pub async fn run(&self) {}\n\
                   }";
        let f = one(src);
        assert_eq!(f.name, "Svc<C>::run");
        assert!(f.unresolved.is_empty(), "{:?}", f.unresolved);
        assert_eq!(decide(&f, true), Verdict::Silent);
    }

    #[test]
    fn unknown_types_only_warn() {
        let f = one("#[effects()]\npub fn f(db: &other::Db) {}");
        assert_eq!(f.unresolved, vec!["other::Db".to_owned()]);
        assert_eq!(decide(&f, true), Verdict::Silent);
        let f = one("#[effects(net)]\npub fn f(a: &Authority) {}");
        assert_eq!(decide(&f, true), Verdict::Warning);
    }

    #[test]
    fn undeclared_pub_fns_are_required_only_when_asked() {
        let all =
            facts(&["pub fn f(n: u32) {}\nfn g() {}\n#[cfg(test)] mod tests { pub fn h() {} }"]);
        assert_eq!(all.len(), 1);
        assert_eq!(decide(&all[0], true), Verdict::Error);
        assert_eq!(decide(&all[0], false), Verdict::Silent);
    }

    /// The capabilities and unresolved types of `x: <ty>` in a function
    /// whose surrounding source is `extra`.
    fn param(ty: &str, extra: &str) -> (Vec<Source>, bool) {
        let all = facts(&[&format!("{extra}\npub fn f(x: {ty}) {{}}")]);
        let f = all.iter().find(|f| f.name == "f").expect("f");
        let mut sources = f.sources.clone();
        sources.sort_by_key(|s| *s as u8);
        sources.dedup();
        (sources, !f.unresolved.is_empty())
    }

    #[test]
    fn every_type_form_is_classified() {
        use Source::{Clock, Dir, Env, Net, ReadDir};
        let cases: &[(&str, &[Source], bool)] = &[
            ("Env", &[Env], false),
            ("Dir", &[Dir], false),
            ("ReadDir", &[ReadDir], false),
            ("Net", &[Net], false),
            ("SystemClock", &[Clock], false),
            ("&Env", &[Env], false),
            ("*const Dir", &[Dir], false),
            ("(Net)", &[Net], false),
            ("&[ReadDir]", &[ReadDir], false),
            ("[Dir; 2]", &[Dir], false),
            ("(SystemClock, Net)", &[Clock, Net], false),
            ("fn() -> u8", &[], false),
            ("impl Clock", &[Clock], false),
            ("&dyn Clock", &[Clock], false),
            ("impl Iterator<Item = u8>", &[], false),
            ("impl std::io::Write", &[], false),
            ("impl Frobnicate", &[], true),
            ("<u8 as Add>::Output", &[], true),
            ("u64", &[], false),
        ];
        for (ty, sources, unresolved) in cases {
            assert_eq!(param(ty, ""), (sources.to_vec(), *unresolved), "x: {ty}");
        }
    }

    #[test]
    fn crate_types_are_expanded() {
        use Source::{Dir, Env};
        let extra = "pub enum Mode { Plain, Scoped(Env) }\n\
                     pub type Root = Dir;\n\
                     pub mod inner { pub struct Wrapped { pub e: Env } }";
        assert_eq!(param("Mode", extra), (vec![Env], false));
        assert_eq!(param("Root", extra), (vec![Dir], false));
        assert_eq!(param("Wrapped", extra), (vec![Env], false));
        assert_eq!(param("<u8 as Tr>::Env", ""), (vec![], true));
    }

    #[test]
    fn where_clauses_bound_the_named_parameter() {
        let all = facts(&["pub fn f<T, U>(x: T) where T: Clock {}"]);
        assert_eq!(all[0].sources, vec![Source::Clock]);
        assert!(all[0].unresolved.is_empty());
    }

    #[test]
    fn modules_and_impls_are_walked() {
        let all = facts(&["#[cfg(unix)] mod m { pub fn g(e: &Env) {} }\n\
             pub trait Tr { fn t(&self, e: &Env); }\n\
             pub struct S;\n\
             impl Tr for S { #[effects(env)] fn t(&self, e: &Env) {} }\n\
             impl S { pub fn own(&self) {} #[effects(env)] fn private(&self, e: &Env) {} }"]);
        let g = all
            .iter()
            .find(|f| f.name == "g")
            .expect("g in a cfg(unix) module");
        assert!(g.public);
        let t = all
            .iter()
            .find(|f| f.name.ends_with("t"))
            .expect("trait method");
        assert!(
            !t.public,
            "trait methods are checked only when they declare effects"
        );
        let own = all
            .iter()
            .find(|f| f.name.ends_with("own"))
            .expect("inherent method");
        assert!(own.public);
        let private = all
            .iter()
            .find(|f| f.name.ends_with("private"))
            .expect("private method");
        assert!(!private.public);
    }

    #[test]
    fn generic_parameters_and_self_resolve_only_by_their_own_name() {
        use Source::Clock;
        let all = facts(&["pub fn f<T: Clock>(x: T, y: other::T) {}"]);
        assert_eq!(all[0].sources, vec![Clock]);
        assert!(
            !all[0].unresolved.is_empty(),
            "other::T is not the parameter T"
        );
        let all =
            facts(&["pub struct Holder { c: SystemClock }\nimpl Holder { pub fn f(h: Self) {} }"]);
        let f = all.iter().find(|f| f.name.ends_with("f")).expect("f");
        assert_eq!(f.sources, vec![Clock], "{f:?}");
    }

    #[test]
    fn recursive_types_terminate() {
        let f = one(
            "pub struct Node { next: Option<Box<Node>>, d: Dir }\n#[effects(fs_read, fs_write)]\npub fn f(n: &Node) {}",
        );
        assert_eq!(decide(&f, true), Verdict::Silent);
    }

    #[test]
    fn unknown_labels_are_left_to_the_macro() {
        assert!(facts(&["#[effects(time)]\npub fn f() {}"]).is_empty());
    }
}
