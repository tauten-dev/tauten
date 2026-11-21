//! `cargo tauten proofs draft` (#49, RFC 2026-11-21-draft-claim-entries):
//! a reviewable patch to `proofs/claims.toml` for new or changed functions.
//!
//! A draft is a suggestion, never evidence. It joins three things that
//! already exist: the committed translation (each declaration's doc comment
//! names its Rust item and source file), what the Lean audit of a gate run
//! reported about each theorem (passed, its subject, total, its named
//! premises) and the declared mutants. It writes no record, runs no mutant
//! and decides nothing about proof, freshness, totality or subsumption: the
//! ordinary gate judges the applied entry like a hand-written one.
//!
//! Trusted, not proved (tested shell code): the doc-comment scanner, the
//! join, the TOML text edits and the diff. Nothing is joined on a short
//! name: a theorem belongs to the function whose fully qualified extracted
//! declaration the audit names as its subject, and an association that is
//! needed for an entry but is not unique refuses the whole draft.
//!
//! This module performs no IO: the caller reads the files and runs the gate.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::{Claim, ClaimsFile, Event, FunctionSpan, Report, SUBSUMABLE};

/// What a translated declaration is, by its doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// `[path]:` — the function itself.
    Spec,
    /// `[path]: loop N:` or `[path]: loop body N:` — a loop of the function.
    Loop,
    /// Inventory only: the theorem is claimed to be total. Checked against
    /// the audit, never trusted.
    Total,
}

/// One declaration of the translation whose doc comment names its Rust
/// item and source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    /// The Rust item as the translation names it, e.g. `k::m::f`.
    pub item: String,
    /// [`Role::Spec`] for the function, [`Role::Loop`] for one of its loops.
    pub role: Role,
    /// Workspace-relative source file.
    pub file: String,
    /// The fully qualified Lean declaration, e.g. `k.m.f`.
    pub lean: String,
}

/// The declarations of one translated Lean file that name a Rust item
/// (`/-- [item]:` with a `Source: 'file'` line), in order. Types and other
/// declarations without that header are left out. The declaration names
/// come from the same scanner as [`super::inputs::declarations`].
#[must_use]
pub fn translated(text: &str) -> Vec<Translated> {
    let mut out = Vec::new();
    let mut namespace: Vec<String> = Vec::new();
    let mut lines = text.lines().enumerate().peekable();
    while let Some((_, raw)) = lines.next() {
        let line = raw.trim();
        if let Some(name) = line.strip_prefix("namespace ") {
            namespace.push(name.trim().to_owned());
            continue;
        }
        if let Some(name) = line.strip_prefix("end ") {
            if namespace.last().is_some_and(|n| n == name.trim()) {
                namespace.pop();
            }
            continue;
        }
        let Some(header) = line.strip_prefix("/-- [") else {
            continue;
        };
        let mut doc = vec![raw.to_owned()];
        let mut closed = raw.contains("-/");
        while !closed {
            let Some((_, next)) = lines.next() else { break };
            closed = next.contains("-/");
            doc.push(next.to_owned());
        }
        let Some((item, rest)) = bracketed(header) else {
            continue;
        };
        let rest = rest.trim_end_matches("-/").trim();
        let role = if rest == ":" {
            Role::Spec
        } else if let Some(kind) = rest.strip_prefix(':') {
            let kind = kind.trim();
            if (kind.starts_with("loop ") || kind.starts_with("loop body ")) && kind.ends_with(':')
            {
                Role::Loop
            } else {
                continue;
            }
        } else {
            continue;
        };
        let Some(file) = doc.iter().find_map(|l| {
            let after = l.split_once("Source: '")?.1;
            Some(after.split_once('\'')?.0.to_owned())
        }) else {
            continue;
        };
        // The declaration the comment documents: the next line that is not
        // an attribute on its own.
        let mut name = None;
        while let Some((number, next)) = lines.peek().copied() {
            let t = next.trim();
            if t.is_empty() || (t.starts_with("@[") && t.ends_with(']')) {
                lines.next();
                continue;
            }
            name = super::inputs::opens(t, number);
            break;
        }
        let Some(name) = name else { continue };
        let mut lean = namespace.join(".");
        if !lean.is_empty() {
            lean.push('.');
        }
        lean.push_str(&name);
        out.push(Translated {
            item: item.to_owned(),
            role,
            file,
            lean,
        });
    }
    out
}

/// `text` up to its matching `]` (nested brackets allowed), and the rest.
fn bracketed(text: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '[' => depth = depth.saturating_add(1),
            ']' if depth == 0 => {
                return Some((text.get(..i)?, text.get(i.saturating_add(1)..)?));
            }
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

/// The `::` segments of a Rust path, not splitting inside `{…}`, `<…>`,
/// `(…)` or `[…]`.
fn segments(path: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut previous = ' ';
    let mut chars = path.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '{' | '<' | '(' | '[' => depth = depth.saturating_add(1),
            '>' if previous == '-' => {}
            '}' | '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ':' if depth == 0 && chars.peek().is_some_and(|(_, n)| *n == ':') => {
                out.push(path.get(start..i).unwrap_or(""));
                chars.next();
                start = i.saturating_add(2);
            }
            _ => {}
        }
        previous = c;
    }
    out.push(path.get(start..).unwrap_or(""));
    out
}

/// The last segment of a type path, without generic arguments.
fn type_name(path: &str) -> String {
    let path = path.trim();
    let path = path.split_once('<').map_or(path, |(head, _)| head);
    segments(path)
        .last()
        .copied()
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// A translated item segment as [`super::functions`] names it: an inherent
/// impl `{k::Type<..>}` is `Type`, a trait impl `{impl Trait<..> for
/// k::Type}` is `<Type as Trait>`. Closures and other generated items have
/// no claim form.
fn claim_segment(segment: &str) -> Option<String> {
    let Some(inner) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) else {
        let plain = !segment.is_empty() && segment.chars().all(|c| c.is_alphanumeric() || c == '_');
        return plain.then(|| segment.to_owned());
    };
    if inner.contains("closure") {
        return None;
    }
    if let Some(body) = inner.strip_prefix("impl ") {
        let (bound, ty) = split_for(body)?;
        let (trait_, ty) = (type_name(bound), type_name(ty));
        return (!trait_.is_empty() && !ty.is_empty()).then(|| format!("<{ty} as {trait_}>"));
    }
    let ty = type_name(inner);
    (!ty.is_empty()).then_some(ty)
}

/// `Trait<..> for Type` split at the ` for ` outside brackets.
fn split_for(body: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut previous = ' ';
    for (i, c) in body.char_indices() {
        match c {
            '{' | '<' | '(' | '[' => depth = depth.saturating_add(1),
            '>' if previous == '-' => {}
            '}' | '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ' ' if depth == 0 && body.get(i..).is_some_and(|r| r.starts_with(" for ")) => {
                return Some((body.get(..i)?, body.get(i.saturating_add(5)..)?));
            }
            _ => {}
        }
        previous = c;
    }
    None
}

/// The module path of a source file of a package in `dir`: `src/lib.rs`
/// and `src/main.rs` are the crate root, `src/a/mod.rs` and `src/a.rs` are
/// `a`. `None` for files outside `src/` or under `src/bin/`.
#[must_use]
pub fn module_of(file: &str, dir: &str) -> Option<Vec<String>> {
    let relative = if dir.is_empty() {
        file
    } else {
        file.strip_prefix(dir)?.strip_prefix('/')?
    };
    let rest = relative.strip_prefix("src/")?.strip_suffix(".rs")?;
    if rest == "lib" || rest == "main" {
        return Some(Vec::new());
    }
    let mut parts: Vec<String> = rest.split('/').map(str::to_owned).collect();
    if parts.first().is_some_and(|p| p == "bin") {
        return None;
    }
    if parts.last().is_some_and(|p| p == "mod") {
        parts.pop();
    }
    Some(parts)
}

/// The claim's `function` for a translated item of the crate `lib` (its
/// library name) defined in a file with module path `modules`: the item's
/// path inside that file, e.g. `Type::name`.
#[must_use]
pub fn claim_function(item: &str, lib: &str, modules: &[String]) -> Option<String> {
    let segs = segments(item);
    let (first, rest) = segs.split_first()?;
    if *first != lib {
        return None;
    }
    let local = rest.strip_prefix(
        modules
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
    )?;
    if local.is_empty() {
        return None;
    }
    let names: Option<Vec<String>> = local.iter().map(|s| claim_segment(s)).collect();
    Some(names?.join("::"))
}

/// A package whose functions may be drafted.
#[derive(Debug, Clone)]
pub struct Package {
    /// The Cargo package name (a claim's `crate`).
    pub name: String,
    /// Its directory, workspace-relative (`""` for the root package).
    pub dir: String,
    /// Its library name, the first segment of its translated items.
    pub lib: String,
}

/// `--inventory PATH`: an ephemeral adapter for custom gates. Suggestions
/// only: every reference is resolved against the current manifest,
/// translation and audit, and a contradiction refuses the draft.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    /// Must be 1.
    pub version: u32,
    /// The functions that may be claimed.
    #[serde(default)]
    pub functions: Vec<InventoryFunction>,
    /// Theorems and their subjects.
    #[serde(default)]
    pub theorems: Vec<InventoryTheorem>,
    /// Declared mutants and what they target.
    #[serde(default)]
    pub mutants: Vec<InventoryMutant>,
}

/// One function of an [`Inventory`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryFunction {
    /// The Cargo package.
    pub package: String,
    /// The workspace-relative source file.
    pub file: String,
    /// The qualified Rust item, as the translation names it (`k::m::f`).
    pub item: String,
    /// The fully qualified extracted declaration (`k.m.f`).
    pub declaration: String,
}

/// One theorem of an [`Inventory`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryTheorem {
    /// Fully qualified theorem name.
    pub name: String,
    /// The extracted declaration its triple is about.
    pub subject: String,
    /// `spec`, `loop` or `total`.
    pub role: Role,
}

/// One mutant of an [`Inventory`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryMutant {
    /// The declared mutant name.
    pub name: String,
    /// The qualified Rust item it changes.
    pub item: String,
    /// The theorems it is meant to break (a requirement, not a kill).
    pub theorems: Vec<String>,
}

impl Inventory {
    /// Parse an inventory file.
    ///
    /// # Errors
    /// Malformed JSON, an unknown field or an unsupported version.
    pub fn parse(text: &str) -> Result<Self, String> {
        let inventory: Self =
            serde_json::from_str(text).map_err(|e| format!("the inventory is not valid: {e}"))?;
        if inventory.version != 1 {
            return Err(format!(
                "unsupported inventory version {}; this tauten reads version 1",
                inventory.version
            ));
        }
        Ok(inventory)
    }
}

/// Everything a draft reads.
pub struct Inputs<'a> {
    /// `proofs/claims.toml`, parsed.
    pub claims: &'a ClaimsFile,
    /// `proofs/claims.toml`, as text (the patch is against it).
    pub text: &'a str,
    /// The selected packages.
    pub packages: &'a [Package],
    /// `proofs/Extracted/*.lean`: file name -> text.
    pub extracted: &'a BTreeMap<String, String>,
    /// The selected packages' Rust sources: workspace-relative path -> text.
    pub sources: &'a BTreeMap<String, String>,
    /// The result lines of a gate run up to the audit (no mutants).
    pub report: &'a Report,
    /// `--inventory`, when given.
    pub inventory: Option<&'a Inventory>,
}

/// A successful draft: the edited manifest, the patch and what was left out.
#[derive(Debug, Clone, Default)]
pub struct Drafted {
    /// The unified diff; empty when nothing changes.
    pub patch: String,
    /// `proofs/claims.toml` after the patch.
    pub text: String,
    /// New `[[claim]]` entries.
    pub added: usize,
    /// Existing claims extended in place.
    pub extended: usize,
    /// Functions skipped and suggestions not made, with the reason, for
    /// stderr.
    pub notes: Vec<String>,
}

/// What the audit of the gate run said about each theorem.
#[derive(Debug, Default)]
struct Evidence {
    audited: BTreeMap<String, bool>,
    totals: BTreeSet<(String, String)>,
    premises: BTreeMap<String, BTreeSet<String>>,
    subjects: BTreeMap<String, BTreeSet<String>>,
}

/// The audit evidence of a run that passed tools, extract, drift, build
/// and audit; otherwise why there is none.
fn evidence(report: &Report) -> Result<Evidence, Vec<String>> {
    if let Some(why) = &report.not_started {
        return Err(vec![format!("the gate could not start: {why}")]);
    }
    let mut stages: BTreeMap<&str, (&str, Option<&str>)> = BTreeMap::new();
    let mut found = Evidence::default();
    for event in &report.events {
        match event {
            Event::Stage {
                stage,
                status,
                detail,
                ..
            } => {
                stages.insert(stage, (status, detail.as_deref()));
            }
            Event::Theorem { theorem, status } => {
                let ok = status == "ok";
                let entry = found.audited.entry(theorem.clone()).or_insert(ok);
                *entry = *entry && ok;
            }
            Event::Total { total, function } => {
                found.totals.insert((total.clone(), function.clone()));
            }
            Event::Premises { theorem, premises } => {
                found
                    .premises
                    .entry(theorem.clone())
                    .or_default()
                    .extend(premises.iter().cloned());
            }
            Event::Subject { theorem, subject } => {
                found
                    .subjects
                    .entry(theorem.clone())
                    .or_default()
                    .insert(subject.clone());
            }
            Event::Establishes { .. } | Event::Mutant { .. } => {}
        }
    }
    let mut problems = Vec::new();
    for stage in ["tools", "extract", "drift", "build", "audit"] {
        match stages.get(stage) {
            Some(("ok", _)) => {}
            Some((status, detail)) => problems.push(format!(
                "the gate's `{stage}` stage is {status}{}{}",
                detail.map(|d| format!(": {d}")).unwrap_or_default(),
                match stage {
                    "tools" => "; a draft needs the proof toolchain",
                    "drift" => "; the committed translation is stale (sh proofs/refresh.sh)",
                    _ => "",
                }
            )),
            None => problems.push(format!("the gate reported no `{stage}` result")),
        }
        if !problems.is_empty() {
            break;
        }
    }
    if problems.is_empty() {
        Ok(found)
    } else {
        Err(problems)
    }
}

/// A function that may be claimed.
#[derive(Debug, Clone)]
struct Function {
    package: String,
    file: String,
    item: String,
    lean: String,
    name: String,
}

/// A theorem about a function (or one of its loops).
#[derive(Debug, Clone)]
struct About {
    theorem: String,
    function: usize,
    role: Role,
}

/// A mutant that targets a function.
#[derive(Debug, Clone)]
struct Targets {
    mutant: String,
    function: usize,
    /// Inventory: the theorems it is meant to break.
    theorems: Option<Vec<String>>,
}

/// The join of functions, theorems and mutants, before any claim is drafted.
#[derive(Debug, Default)]
struct Model {
    functions: Vec<Function>,
    about: Vec<About>,
    targets: Vec<Targets>,
    notes: Vec<String>,
}

/// Every translated declaration of the selected packages with the claim
/// form of its item, keyed by Lean name.
fn translation(inputs: &Inputs<'_>) -> BTreeMap<String, (Translated, Option<(usize, String)>)> {
    let mut out = BTreeMap::new();
    for text in inputs.extracted.values() {
        for t in translated(text) {
            let form = inputs.packages.iter().enumerate().find_map(|(i, p)| {
                let modules = module_of(&t.file, &p.dir)?;
                claim_function(&t.item, &p.lib, &modules).map(|name| (i, name))
            });
            out.insert(t.lean.clone(), (t, form));
        }
    }
    out
}

/// Whether `name` is exactly one function of `file`.
fn in_source(inputs: &Inputs<'_>, file: &str, name: &str) -> usize {
    inputs.sources.get(file).map_or(0, |text| {
        super::functions(text)
            .iter()
            .filter(|f| f.path == name)
            .count()
    })
}

/// The stock adapter: functions from the translation, theorem subjects from
/// the audit (`SUBJECT` and `TOTAL` lines), mutants from `[[mutant]]`
/// entries no claim lists yet, located by their unique site.
fn stock(inputs: &Inputs<'_>, evidence: &Evidence) -> Result<Model, Vec<String>> {
    if evidence.subjects.is_empty() {
        return Err(vec![
            "the audit printed no theorem subjects (`SUBJECT <theorem> <function>` lines), so no theorem can be joined to a function; update proofs/Audit.lean and proofs/gate.py from `cargo tauten init --proofs`, or pass --inventory".to_owned(),
        ]);
    }
    let decls = translation(inputs);
    let mut model = Model::default();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    for (lean, (t, form)) in &decls {
        if t.role != Role::Spec {
            continue;
        }
        let Some((p, name)) = form else { continue };
        let Some(package) = inputs.packages.get(*p) else {
            continue;
        };
        match in_source(inputs, &t.file, name) {
            1 => {}
            0 => {
                model.notes.push(format!(
                    "skipped {}: `{name}` is not a function of {} as written (cfg, macro or `#[path]`?)",
                    t.item, t.file
                ));
                continue;
            }
            _ => {
                model.notes.push(format!(
                    "skipped {}: {} defines `{name}` more than once",
                    t.item, t.file
                ));
                continue;
            }
        }
        index.insert(t.item.clone(), model.functions.len());
        model.functions.push(Function {
            package: package.name.clone(),
            file: t.file.clone(),
            item: t.item.clone(),
            lean: lean.clone(),
            name: name.clone(),
        });
    }
    let mut theorems: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (theorem, subjects) in &evidence.subjects {
        theorems
            .entry(theorem)
            .or_default()
            .extend(subjects.iter().map(String::as_str));
    }
    for (theorem, function) in &evidence.totals {
        theorems.entry(theorem).or_default().insert(function);
    }
    let mut refused = Vec::new();
    for (theorem, subjects) in theorems {
        let about: Vec<(usize, Role)> = subjects
            .iter()
            .filter_map(|s| {
                let (t, _) = decls.get(*s)?;
                index.get(&t.item).map(|&f| (f, t.role))
            })
            .collect();
        match about.as_slice() {
            [] => {}
            [(function, role)] if subjects.len() == 1 => model.about.push(About {
                theorem: theorem.to_owned(),
                function: *function,
                role: *role,
            }),
            _ => refused.push(format!(
                "theorem {theorem} is reported about several declarations ({}); name its function in --inventory",
                subjects.iter().copied().collect::<Vec<_>>().join(", ")
            )),
        }
    }
    let listed: BTreeSet<&str> = inputs
        .claims
        .claim
        .iter()
        .flat_map(|c| c.mutants.iter().map(String::as_str))
        .collect();
    let mut spans: BTreeMap<&str, Vec<FunctionSpan>> = BTreeMap::new();
    for mutant in &inputs.claims.mutant {
        if listed.contains(mutant.name.as_str()) {
            continue;
        }
        let Some(text) = inputs.sources.get(&mutant.file) else {
            continue;
        };
        let Some(site) = unique_site(text, &mutant.before) else {
            model.notes.push(format!(
                "mutant {}: its `before` text does not occur exactly once in {}",
                mutant.name, mutant.file
            ));
            continue;
        };
        let here = spans
            .entry(mutant.file.as_str())
            .or_insert_with(|| super::functions(text));
        let enclosing = super::enclosing(here, site.0).filter(|f| f.end >= site.1);
        let function = enclosing.and_then(|span| {
            model
                .functions
                .iter()
                .position(|f| f.file == mutant.file && f.name == span.path)
        });
        match function {
            Some(function) => model.targets.push(Targets {
                mutant: mutant.name.clone(),
                function,
                theorems: None,
            }),
            None => model.notes.push(format!(
                "mutant {}: its site in {} is not inside one translated function",
                mutant.name, mutant.file
            )),
        }
    }
    if refused.is_empty() {
        Ok(model)
    } else {
        Err(refused)
    }
}

/// The first and last line (1-based) of the only occurrence of `needle`.
fn unique_site(text: &str, needle: &str) -> Option<(u32, u32)> {
    if needle.is_empty() || text.matches(needle).count() != 1 {
        return None;
    }
    let at = text.find(needle)?;
    let line = |offset: usize| {
        let count = text.get(..offset).map_or(0, |t| t.matches('\n').count());
        u32::try_from(count.saturating_add(1)).unwrap_or(u32::MAX)
    };
    Some((line(at), line(at.saturating_add(needle.len()))))
}

/// The inventory adapter: every reference resolved against the manifest,
/// the translation and the audit; any contradiction refuses.
fn from_inventory(
    inputs: &Inputs<'_>,
    inventory: &Inventory,
    evidence: &Evidence,
) -> Result<Model, Vec<String>> {
    let decls = translation(inputs);
    let mut model = Model::default();
    let mut refused = Vec::new();
    let mut by_item: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &inventory.functions {
        let at = format!("inventory function {}", f.item);
        let Some((package_index, package)) = inputs
            .packages
            .iter()
            .enumerate()
            .find(|(_, p)| p.name == f.package)
        else {
            refused.push(format!(
                "{at}: package `{}` is not selected (or not one of [gate] crates)",
                f.package
            ));
            continue;
        };
        let Some((t, form)) = decls.get(&f.declaration) else {
            refused.push(format!(
                "{at}: `{}` is not a declaration of the translation",
                f.declaration
            ));
            continue;
        };
        if t.item != f.item || t.file != f.file || t.role != Role::Spec {
            refused.push(format!(
                "{at}: the translation says `{}` is {:?} {} in {}, not {} in {}",
                f.declaration, t.role, t.item, t.file, f.item, f.file
            ));
            continue;
        }
        let Some((p, name)) = form.as_ref().filter(|(p, _)| *p == package_index) else {
            refused.push(format!(
                "{at}: not an item of package `{}` with a claim form",
                package.name
            ));
            continue;
        };
        if in_source(inputs, &t.file, name) != 1 || *p != package_index {
            refused.push(format!(
                "{at}: `{name}` is not exactly one function of {}",
                t.file
            ));
            continue;
        }
        if by_item.contains_key(f.item.as_str()) {
            refused.push(format!("{at}: listed twice"));
            continue;
        }
        by_item.insert(&f.item, model.functions.len());
        model.functions.push(Function {
            package: package.name.clone(),
            file: f.file.clone(),
            item: f.item.clone(),
            lean: f.declaration.clone(),
            name: name.clone(),
        });
    }
    let mut seen: BTreeMap<&str, &InventoryTheorem> = BTreeMap::new();
    for th in &inventory.theorems {
        let at = format!("inventory theorem {}", th.name);
        if let Some(previous) = seen.insert(&th.name, th) {
            if previous != th {
                refused.push(format!(
                    "{at}: listed twice with different subjects or roles"
                ));
            }
            continue;
        }
        if !evidence.audited.contains_key(&th.name) {
            refused.push(format!("{at}: not a theorem the audit checked"));
            continue;
        }
        let Some((t, _)) = decls.get(&th.subject) else {
            refused.push(format!(
                "{at}: subject `{}` is not a declaration of the translation",
                th.subject
            ));
            continue;
        };
        let Some(&function) = by_item.get(t.item.as_str()) else {
            refused.push(format!(
                "{at}: subject `{}` is not (a loop of) an inventory function",
                th.subject
            ));
            continue;
        };
        let consistent = match th.role {
            Role::Loop => t.role == Role::Loop,
            Role::Spec => t.role == Role::Spec,
            Role::Total => {
                t.role == Role::Spec
                    && evidence
                        .totals
                        .contains(&(th.name.clone(), th.subject.clone()))
            }
        };
        if !consistent {
            refused.push(format!(
                "{at}: role {:?} contradicts the translation or the audit (`{}` is {:?}{})",
                th.role,
                th.subject,
                t.role,
                if th.role == Role::Total {
                    ", and the audit did not find it total"
                } else {
                    ""
                }
            ));
            continue;
        }
        if let Some(reported) = evidence.subjects.get(&th.name)
            && !reported.contains(&th.subject)
        {
            refused.push(format!(
                "{at}: the audit reports it about {}, not `{}`",
                reported.iter().cloned().collect::<Vec<_>>().join(", "),
                th.subject
            ));
            continue;
        }
        model.about.push(About {
            theorem: th.name.clone(),
            function,
            role: if th.role == Role::Loop {
                Role::Loop
            } else {
                Role::Spec
            },
        });
    }
    let mut mutants: BTreeSet<&str> = BTreeSet::new();
    for m in &inventory.mutants {
        let at = format!("inventory mutant {}", m.name);
        if !mutants.insert(&m.name) {
            refused.push(format!("{at}: listed twice"));
            continue;
        }
        let Some(&function) = by_item.get(m.item.as_str()) else {
            refused.push(format!(
                "{at}: target `{}` is not an inventory function",
                m.item
            ));
            continue;
        };
        if let Some(unknown) = m.theorems.iter().find(|t| !seen.contains_key(t.as_str())) {
            refused.push(format!("{at}: `{unknown}` is not an inventory theorem"));
            continue;
        }
        let file = model.functions.get(function).map(|f| f.file.as_str());
        if let Some(declared) = inputs.claims.mutant.iter().find(|d| d.name == m.name)
            && Some(declared.file.as_str()) != file
        {
            refused.push(format!(
                "{at}: proofs/claims.toml declares it in {}, the inventory targets {}",
                declared.file,
                file.unwrap_or("?")
            ));
            continue;
        }
        model.targets.push(Targets {
            mutant: m.name.clone(),
            function,
            theorems: Some(m.theorems.clone()),
        });
    }
    if refused.is_empty() {
        Ok(model)
    } else {
        Err(refused)
    }
}

/// Additions to one existing claim.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Extension {
    theorems: Vec<String>,
    total: Option<String>,
    subsumes: Vec<String>,
    mutants: Vec<String>,
}

impl Extension {
    fn is_empty(&self) -> bool {
        self.theorems.is_empty()
            && self.total.is_none()
            && self.subsumes.is_empty()
            && self.mutants.is_empty()
    }
}

/// The claims to add and the extensions of existing ones.
#[derive(Debug, Default)]
struct Plan {
    added: Vec<Claim>,
    extended: BTreeMap<usize, Extension>,
    notes: Vec<String>,
}

fn plan(inputs: &Inputs<'_>, model: &Model, evidence: &Evidence) -> Result<Plan, Vec<String>> {
    let mut out = Plan::default();
    let mut refused = Vec::new();
    // Two declarations with one claim key: never guess which one is meant.
    let mut keys: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (i, f) in model.functions.iter().enumerate() {
        keys.entry((&f.file, &f.name)).or_default().push(i);
    }
    let used: BTreeSet<usize> = model
        .about
        .iter()
        .map(|a| a.function)
        .chain(model.targets.iter().map(|t| t.function))
        .collect();
    for ((file, name), members) in &keys {
        if members.len() > 1 && members.iter().any(|m| used.contains(m)) {
            refused.push(format!(
                "`{name}` in {file} is several translated declarations ({}); name the one you mean in --inventory",
                members
                    .iter()
                    .filter_map(|m| model.functions.get(*m).map(|f| f.lean.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    if !refused.is_empty() {
        return Err(refused);
    }
    let selected: BTreeSet<&str> = inputs.packages.iter().map(|p| p.name.as_str()).collect();
    for (index, function) in model.functions.iter().enumerate() {
        let about: Vec<&About> = model.about.iter().filter(|a| a.function == index).collect();
        let targets: Vec<&Targets> = model
            .targets
            .iter()
            .filter(|t| t.function == index)
            .collect();
        let existing = inputs.claims.claim.iter().position(|c| {
            c.krate == function.package && c.file == function.file && c.function == function.name
        });
        let elsewhere = inputs
            .claims
            .claim
            .iter()
            .any(|c| c.file == function.file && c.function == function.name);
        let label = format!("{} ({})", function.item, function.file);
        // Theorems that passed the audit, with their reported premises.
        let mut usable: Vec<(&About, &BTreeSet<String>)> = Vec::new();
        for a in &about {
            if evidence.audited.get(&a.theorem) != Some(&true) {
                out.notes.push(format!(
                    "{label}: theorem {} did not pass the audit; not suggested",
                    a.theorem
                ));
                continue;
            }
            let Some(premises) = evidence.premises.get(&a.theorem) else {
                out.notes.push(format!(
                    "{label}: the audit reported no premises for theorem {}; not suggested",
                    a.theorem
                ));
                continue;
            };
            usable.push((a, premises));
        }
        let totals: Vec<&str> = usable
            .iter()
            .filter(|(a, premises)| {
                a.role == Role::Spec
                    && premises.is_empty()
                    && evidence
                        .totals
                        .contains(&(a.theorem.clone(), function.lean.clone()))
            })
            .map(|(a, _)| a.theorem.as_str())
            .collect();
        match existing.and_then(|i| inputs.claims.claim.get(i).map(|c| (i, c))) {
            Some((_, claim)) if claim.inherit.is_some() => {
                out.notes
                    .push(format!("{label}: inherits its claim; nothing suggested"));
            }
            Some((i, claim)) => {
                if claim.lean != function.lean {
                    out.notes.push(format!(
                        "{label}: the claim names `{}`, the translation `{}`; review it by hand",
                        claim.lean, function.lean
                    ));
                    continue;
                }
                let mut ext = Extension::default();
                let assumes: BTreeSet<&str> = claim.assumes.iter().map(String::as_str).collect();
                for (a, premises) in &usable {
                    if claim.theorems.contains(&a.theorem) {
                        continue;
                    }
                    if premises.iter().all(|p| assumes.contains(p.as_str())) {
                        ext.theorems.push(a.theorem.clone());
                    } else {
                        out.notes.push(format!(
                            "{label}: theorem {} rests on {} beyond the claim's `assumes`; changing its assumptions needs review",
                            a.theorem,
                            list(premises.iter())
                        ));
                    }
                }
                match (&claim.total, totals.as_slice()) {
                    (None, [one]) => {
                        ext.total = Some((*one).to_owned());
                        if !claim.theorems.iter().any(|t| t == one)
                            && !ext.theorems.iter().any(|t| t == one)
                        {
                            ext.theorems.push((*one).to_owned());
                        }
                        if claim.assumes.is_empty()
                            && claim.witness.is_none()
                            && claim.subsumes.is_empty()
                        {
                            ext.subsumes = SUBSUMABLE.iter().map(|r| (*r).to_owned()).collect();
                        }
                    }
                    (None, []) => {}
                    (None, several) => out.notes.push(format!(
                        "{label}: several total theorems ({}); set `total` by hand",
                        several.join(", ")
                    )),
                    (Some(current), found) => {
                        if !found.is_empty() && !found.contains(&current.as_str()) {
                            out.notes.push(format!(
                                "{label}: `total = {current}` but the audit finds {} total; a conflicting `total` needs review",
                                found.join(", ")
                            ));
                        }
                    }
                }
                let theorems: BTreeSet<&str> = claim
                    .theorems
                    .iter()
                    .chain(&ext.theorems)
                    .map(String::as_str)
                    .collect();
                for t in &targets {
                    if claim.mutants.contains(&t.mutant) || ext.mutants.contains(&t.mutant) {
                        continue;
                    }
                    if applies(t, &theorems) {
                        ext.mutants.push(t.mutant.clone());
                    } else {
                        out.notes.push(format!(
                            "{label}: mutant {} names none of the claim's theorems; not suggested",
                            t.mutant
                        ));
                    }
                }
                ext.theorems.sort();
                ext.mutants.sort();
                if !ext.is_empty() {
                    out.extended.insert(i, ext);
                }
            }
            None if elsewhere => out.notes.push(format!(
                "{label}: claimed under another crate; nothing suggested"
            )),
            None if !selected.contains(function.package.as_str()) => {}
            None => {
                let mut theorems: Vec<String> = Vec::new();
                let mut own = false;
                for (a, premises) in &usable {
                    if premises.is_empty() {
                        own = own || a.role == Role::Spec;
                        theorems.push(a.theorem.clone());
                    } else {
                        out.notes.push(format!(
                            "{label}: theorem {} rests on {}; a claim with assumptions needs `assumes` and a `witness` written by hand",
                            a.theorem,
                            list(premises.iter())
                        ));
                    }
                }
                theorems.sort();
                theorems.dedup();
                let names: BTreeSet<&str> = theorems.iter().map(String::as_str).collect();
                let mut mutants: Vec<String> = Vec::new();
                for t in &targets {
                    if applies(t, &names) {
                        mutants.push(t.mutant.clone());
                    } else {
                        out.notes.push(format!(
                            "{label}: mutant {} names none of the drafted theorems; not suggested",
                            t.mutant
                        ));
                    }
                }
                mutants.sort();
                mutants.dedup();
                if !own {
                    out.notes.push(format!(
                        "skipped {label}: no audited, assumption-free theorem about the function itself"
                    ));
                    continue;
                }
                if mutants.is_empty() {
                    out.notes.push(format!(
                        "skipped {label}: no declared mutant targets it (add a [[mutant]] inside it)"
                    ));
                    continue;
                }
                let total = match totals.as_slice() {
                    [one] => Some((*one).to_owned()),
                    [] => None,
                    several => {
                        out.notes.push(format!(
                            "{label}: several total theorems ({}); set `total` by hand",
                            several.join(", ")
                        ));
                        None
                    }
                };
                let subsumes = if total.is_some() {
                    SUBSUMABLE.iter().map(|r| (*r).to_owned()).collect()
                } else {
                    Vec::new()
                };
                out.added.push(Claim {
                    krate: function.package.clone(),
                    file: function.file.clone(),
                    function: function.name.clone(),
                    lean: function.lean.clone(),
                    theorems,
                    total,
                    subsumes,
                    mutants,
                    assumes: Vec::new(),
                    witness: None,
                    necessity: Vec::new(),
                    inherit: None,
                });
            }
        }
    }
    out.added
        .sort_by(|a, b| (&a.file, &a.function).cmp(&(&b.file, &b.function)));
    Ok(out)
}

/// Whether a mutant may be listed by a claim with `theorems`: always for a
/// located stock mutant (calibration decides), and for an inventory mutant
/// only when it names one of them.
fn applies(target: &Targets, theorems: &BTreeSet<&str>) -> bool {
    target
        .theorems
        .as_ref()
        .is_none_or(|named| named.iter().any(|t| theorems.contains(t.as_str())))
}

fn list<'a>(items: impl Iterator<Item = &'a String>) -> String {
    items
        .map(|s| format!("`{s}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Draft the patch.
///
/// # Errors
/// Every reason the draft is refused (missing tools or evidence, a stale
/// translation, an ambiguous or contradictory association, a manifest the
/// draft cannot edit safely). A refused draft prints no patch.
pub fn draft(inputs: &Inputs<'_>) -> Result<Drafted, Vec<String>> {
    let evidence = evidence(inputs.report)?;
    let model = match inputs.inventory {
        Some(inventory) => from_inventory(inputs, inventory, &evidence)?,
        None => stock(inputs, &evidence)?,
    };
    let plan = plan(inputs, &model, &evidence)?;
    let mut notes = model.notes;
    notes.extend(plan.notes.iter().cloned());
    if plan.added.is_empty() && plan.extended.is_empty() {
        return Ok(Drafted {
            text: inputs.text.to_owned(),
            notes,
            ..Drafted::default()
        });
    }
    let edits = edits(inputs.text, inputs.claims, &plan).map_err(|e| vec![e])?;
    let text = render(inputs.text, &edits);
    verify(inputs.claims, &text, &plan).map_err(|e| vec![e])?;
    Ok(Drafted {
        patch: unified_diff(CLAIMS_PATH, inputs.text, &edits),
        text,
        added: plan.added.len(),
        extended: plan.extended.len(),
        notes,
    })
}

/// The path the patch names, relative to the workspace root.
pub const CLAIMS_PATH: &str = "proofs/claims.toml";

/// The edited manifest must parse, keep every hand-written entry and
/// amendment, and hold exactly the drafted claims.
fn verify(before: &ClaimsFile, text: &str, plan: &Plan) -> Result<(), String> {
    let after = ClaimsFile::parse(text).map_err(|e| {
        format!("internal: the drafted manifest does not parse ({e}); nothing printed")
    })?;
    let mut expected = before.claim.clone();
    for (i, ext) in &plan.extended {
        if let Some(claim) = expected.get_mut(*i) {
            claim.theorems.extend(ext.theorems.iter().cloned());
            if ext.total.is_some() {
                claim.total.clone_from(&ext.total);
            }
            claim.subsumes.extend(ext.subsumes.iter().cloned());
            claim.mutants.extend(ext.mutants.iter().cloned());
        }
    }
    expected.extend(plan.added.iter().cloned());
    let mutants = |f: &ClaimsFile| -> Vec<(String, String, String, String, Vec<String>)> {
        f.mutant
            .iter()
            .map(|m| {
                (
                    m.name.clone(),
                    m.file.clone(),
                    m.before.clone(),
                    m.after.clone(),
                    m.direct.clone(),
                )
            })
            .collect()
    };
    if after.claim != expected
        || after.amend != before.amend
        || mutants(&after) != mutants(before)
        || after.version != before.version
    {
        return Err(
            "internal: the edited manifest would not hold exactly the drafted entries; nothing printed"
                .to_owned(),
        );
    }
    Ok(())
}

/// What a byte of a TOML text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Code,
    Text,
    Comment,
    Space,
}

/// Classify every byte: code, string, comment or whitespace.
fn classify(text: &str) -> Vec<Class> {
    let bytes = text.as_bytes();
    let mut out = vec![Class::Space; bytes.len()];
    let at = |i: usize| bytes.get(i).copied();
    let starts = |i: usize, s: &[u8]| bytes.get(i..i.saturating_add(s.len())) == Some(s);
    let mut i = 0usize;
    let mark = |out: &mut Vec<Class>, from: usize, to: usize, class: Class| {
        for slot in out.iter_mut().take(to).skip(from) {
            *slot = class;
        }
    };
    while let Some(b) = at(i) {
        if b == b'#' {
            let end = text
                .get(i..)
                .and_then(|r| r.find('\n'))
                .map_or(bytes.len(), |n| i.saturating_add(n));
            mark(&mut out, i, end, Class::Comment);
            i = end;
        } else if starts(i, b"\"\"\"") || starts(i, b"'''") {
            let quote: &[u8] = if b == b'"' { b"\"\"\"" } else { b"'''" };
            let mut j = i.saturating_add(3);
            while at(j).is_some() && !starts(j, quote) {
                j = j.saturating_add(if b == b'"' && at(j) == Some(b'\\') {
                    2
                } else {
                    1
                });
            }
            let mut end = j.saturating_add(3).min(bytes.len());
            // `""""` closes with the last three quotes.
            while at(end) == Some(b) {
                end = end.saturating_add(1);
            }
            mark(&mut out, i, end, Class::Text);
            i = end;
        } else if b == b'"' || b == b'\'' {
            let mut j = i.saturating_add(1);
            while let Some(c) = at(j) {
                if c == b || c == b'\n' {
                    break;
                }
                j = j.saturating_add(if b == b'"' && c == b'\\' { 2 } else { 1 });
            }
            let end = j.saturating_add(1).min(bytes.len());
            mark(&mut out, i, end, Class::Text);
            i = end;
        } else {
            if !b.is_ascii_whitespace()
                && let Some(slot) = out.get_mut(i)
            {
                *slot = Class::Code;
            }
            i = i.saturating_add(1);
        }
    }
    out
}

/// Byte offsets of each line's start.
fn line_starts(text: &str) -> Vec<usize> {
    let mut out = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' && i.saturating_add(1) < text.len() {
            out.push(i.saturating_add(1));
        }
    }
    out
}

/// The changes to one line: insertions at byte columns, and whole lines
/// added after it.
#[derive(Debug, Clone, Default)]
struct LineEdit {
    inserts: Vec<(usize, String)>,
    after: Vec<String>,
}

type Edits = BTreeMap<usize, LineEdit>;

/// A TOML basic string.
fn quoted(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04X}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn array(items: &[String]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(|s| quoted(s))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The line holding byte `offset`.
fn line_of(starts: &[usize], offset: usize) -> usize {
    starts
        .iter()
        .rposition(|&s| s <= offset)
        .unwrap_or_default()
}

/// The line edits that realise `plan`: arrays of existing claims extended
/// after their last element, `total`/`subsumes` added after `theorems`,
/// new claims appended at the end. Refuses a layout it cannot edit safely.
fn edits(text: &str, claims: &ClaimsFile, plan: &Plan) -> Result<Edits, String> {
    let classes = classify(text);
    let starts = line_starts(text);
    let bytes = text.as_bytes();
    let class = |i: usize| classes.get(i).copied().unwrap_or(Class::Space);
    // Bracket depth at each line start, from code bytes only.
    let mut depth_at: Vec<isize> = Vec::with_capacity(starts.len());
    let mut depth = 0isize;
    let mut line = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        while starts.get(line).is_some_and(|&s| s == i) {
            depth_at.push(depth);
            line = line.saturating_add(1);
        }
        if class(i) == Class::Code {
            match b {
                b'[' | b'{' => depth = depth.saturating_add(1),
                b']' | b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    while depth_at.len() < starts.len() {
        depth_at.push(depth);
    }
    let first_code = |l: usize| -> Option<usize> {
        let start = *starts.get(l)?;
        let end = starts
            .get(l.saturating_add(1))
            .copied()
            .unwrap_or(bytes.len());
        (start..end).find(|&i| class(i) == Class::Code)
    };
    // Table headers: a line whose first code byte opens a bracket at depth 0.
    let mut headers: Vec<(usize, String)> = Vec::new();
    for l in 0..starts.len() {
        if depth_at.get(l) != Some(&0) {
            continue;
        }
        if let Some(i) = first_code(l)
            && bytes.get(i) == Some(&b'[')
        {
            let end = starts
                .get(l.saturating_add(1))
                .copied()
                .unwrap_or(bytes.len());
            let name: String = (i..end)
                .filter(|&j| class(j) == Class::Code)
                .filter_map(|j| bytes.get(j).map(|b| char::from(*b)))
                .collect();
            headers.push((l, name));
        }
    }
    let blocks: Vec<(usize, usize)> = headers
        .iter()
        .enumerate()
        .filter(|(_, (_, name))| name == "[[claim]]")
        .map(|(k, (l, _))| {
            let end = headers
                .get(k.saturating_add(1))
                .map_or(starts.len(), |(next, _)| *next);
            (*l, end)
        })
        .collect();
    if blocks.len() != claims.claim.len() {
        return Err(format!(
            "{CLAIMS_PATH} writes some claims other than as `[[claim]]` tables; the draft cannot extend them safely, so edit by hand"
        ));
    }
    let mut out: Edits = BTreeMap::new();
    for (index, ext) in &plan.extended {
        let Some(&(from, to)) = blocks.get(*index) else {
            return Err(format!("internal: claim {index} has no table"));
        };
        let function = claims
            .claim
            .get(*index)
            .map_or("?", |c| c.function.as_str());
        // A key line of this table: depth 0, first code is the bare key.
        let key_line = |key: &str| -> Option<(usize, usize)> {
            (from.saturating_add(1)..to).find_map(|l| {
                if depth_at.get(l) != Some(&0) {
                    return None;
                }
                let i = first_code(l)?;
                let rest = text.get(i..)?;
                let after = rest.strip_prefix(key)?;
                let eq = after.len().saturating_sub(after.trim_start().len());
                after
                    .trim_start()
                    .starts_with('=')
                    .then(|| (l, i.saturating_add(key.len()).saturating_add(eq)))
            })
        };
        // The array after `=` at `eq`: (its `[`, its `]`, its last
        // significant byte before the `]`).
        let array_at = |eq: usize| -> Option<(usize, usize, usize)> {
            let open = (eq.saturating_add(1)..bytes.len()).find(|&i| class(i) == Class::Code)?;
            if bytes.get(open) != Some(&b'[') {
                return None;
            }
            let mut d = 0usize;
            let mut last = open;
            for i in open..bytes.len() {
                match class(i) {
                    Class::Code => {
                        match bytes.get(i) {
                            Some(b'[') => d = d.saturating_add(1),
                            Some(b']') => {
                                d = d.saturating_sub(1);
                                if d == 0 {
                                    return Some((open, i, last));
                                }
                            }
                            _ => {}
                        }
                        last = i;
                    }
                    Class::Text => last = i,
                    Class::Comment | Class::Space => {}
                }
            }
            None
        };
        let cannot = |what: &str| {
            format!(
                "claim `{function}`: cannot find `{what} = [...]` as a plain key of its table; extend it by hand"
            )
        };
        let mut append = |key: &str, items: &[String]| -> Result<usize, String> {
            let (_, eq) = key_line(key).ok_or_else(|| cannot(key))?;
            let (_, close, last) = array_at(eq).ok_or_else(|| cannot(key))?;
            let (last_line, close_line) = (line_of(&starts, last), line_of(&starts, close));
            if !items.is_empty() && bytes.get(last) == Some(&b',') && close_line > last_line {
                // One element per line, with a trailing comma: keep that.
                let start = starts.get(last_line).copied().unwrap_or_default();
                let indent: String = text
                    .get(start..)
                    .unwrap_or("")
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                out.entry(last_line)
                    .or_default()
                    .after
                    .extend(items.iter().map(|s| format!("{indent}{},", quoted(s))));
            } else if !items.is_empty() {
                let joined = items
                    .iter()
                    .map(|s| quoted(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                let insert = match bytes.get(last) {
                    Some(b'[') => joined,
                    Some(b',') => format!(" {joined}"),
                    _ => format!(", {joined}"),
                };
                let l = line_of(&starts, last);
                let column = last
                    .saturating_add(1)
                    .saturating_sub(*starts.get(l).unwrap_or(&0));
                out.entry(l).or_default().inserts.push((column, insert));
            }
            Ok(close_line)
        };
        let theorems_end = append("theorems", &ext.theorems)?;
        append("mutants", &ext.mutants)?;
        let mut lines = Vec::new();
        if let Some(total) = &ext.total {
            if key_line("total").is_some() {
                return Err(cannot("total"));
            }
            lines.push(format!("total = {}", quoted(total)));
        }
        if !ext.subsumes.is_empty() {
            if key_line("subsumes").is_some() {
                return Err(format!(
                    "claim `{function}`: already has `subsumes`; extend it by hand"
                ));
            }
            lines.push(format!("subsumes = {}", array(&ext.subsumes)));
        }
        if !lines.is_empty() {
            out.entry(theorems_end).or_default().after.extend(lines);
        }
    }
    if !plan.added.is_empty() {
        let last = starts.len().saturating_sub(1);
        let mut lines = Vec::new();
        for claim in &plan.added {
            lines.push(String::new());
            lines.push("[[claim]]".to_owned());
            lines.push(format!("crate = {}", quoted(&claim.krate)));
            lines.push(format!("file = {}", quoted(&claim.file)));
            lines.push(format!("function = {}", quoted(&claim.function)));
            lines.push(format!("lean = {}", quoted(&claim.lean)));
            lines.push(format!("theorems = {}", array(&claim.theorems)));
            if let Some(total) = &claim.total {
                lines.push(format!("total = {}", quoted(total)));
            }
            if !claim.subsumes.is_empty() {
                lines.push(format!("subsumes = {}", array(&claim.subsumes)));
            }
            lines.push(format!("mutants = {}", array(&claim.mutants)));
        }
        out.entry(last).or_default().after.extend(lines);
    }
    Ok(out)
}

/// The lines of `text`, each with its newline (the last may have none).
fn lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// One line after its edits (inserted text and added lines), with
/// newlines.
fn edited(line: &str, edit: &LineEdit) -> (String, Vec<String>) {
    let (body, newline) = match line.strip_suffix('\n') {
        Some(b) => (b, true),
        None => (line, false),
    };
    let mut inserts = edit.inserts.clone();
    inserts.sort_by_key(|(column, _)| *column);
    let mut out = String::new();
    let mut at = 0usize;
    for (column, text) in &inserts {
        out.push_str(body.get(at..*column).unwrap_or(""));
        out.push_str(text);
        at = *column;
    }
    out.push_str(body.get(at..).unwrap_or(""));
    if newline || !edit.after.is_empty() {
        out.push('\n');
    }
    let after = edit.after.iter().map(|l| format!("{l}\n")).collect();
    (out, after)
}

fn render(text: &str, edits: &Edits) -> String {
    let mut out = String::new();
    for (i, line) in lines(text).into_iter().enumerate() {
        match edits.get(&i) {
            Some(edit) => {
                let (line, after) = edited(line, edit);
                out.push_str(&line);
                for a in after {
                    out.push_str(&a);
                }
            }
            None => out.push_str(line),
        }
    }
    out
}

/// One line of a diff.
enum Op {
    Same(String),
    Removed(String),
    Added(String),
}

/// A unified diff (three lines of context) from `old` to `old` with
/// `edits`, naming `path` on both sides (`a/` and `b/`).
fn unified_diff(path: &str, old: &str, edits: &Edits) -> String {
    let mut ops = Vec::new();
    for (i, line) in lines(old).into_iter().enumerate() {
        match edits.get(&i) {
            Some(edit) => {
                let (new, after) = edited(line, edit);
                if new == line {
                    ops.push(Op::Same(line.to_owned()));
                } else {
                    ops.push(Op::Removed(line.to_owned()));
                    ops.push(Op::Added(new));
                }
                ops.extend(after.into_iter().map(Op::Added));
            }
            None => ops.push(Op::Same(line.to_owned())),
        }
    }
    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, op)| !matches!(op, Op::Same(_)))
        .map(|(i, _)| i)
        .collect();
    if changed.is_empty() {
        return String::new();
    }
    const CONTEXT: usize = 3;
    // Hunks as op ranges, merged when their context overlaps.
    let mut hunks: Vec<(usize, usize)> = Vec::new();
    for &c in &changed {
        let from = c.saturating_sub(CONTEXT);
        let to = c.saturating_add(CONTEXT).saturating_add(1).min(ops.len());
        match hunks.last_mut() {
            Some((_, end)) if from <= *end => *end = to.max(*end),
            _ => hunks.push((from, to)),
        }
    }
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    // Old and new line numbers (0-based) at the start of each op.
    let mut old_at = Vec::with_capacity(ops.len());
    let mut new_at = Vec::with_capacity(ops.len());
    let (mut o, mut n) = (0usize, 0usize);
    for op in &ops {
        old_at.push(o);
        new_at.push(n);
        match op {
            Op::Same(_) => {
                o = o.saturating_add(1);
                n = n.saturating_add(1);
            }
            Op::Removed(_) => o = o.saturating_add(1),
            Op::Added(_) => n = n.saturating_add(1),
        }
    }
    for (from, to) in hunks {
        let range = ops.get(from..to).unwrap_or(&[]);
        let old_count = range
            .iter()
            .filter(|op| !matches!(op, Op::Added(_)))
            .count();
        let new_count = range
            .iter()
            .filter(|op| !matches!(op, Op::Removed(_)))
            .count();
        let start = |at: &[usize], count: usize| {
            let first = at.get(from).copied().unwrap_or_default();
            if count == 0 {
                first
            } else {
                first.saturating_add(1)
            }
        };
        out.push_str(&format!(
            "@@ -{},{old_count} +{},{new_count} @@\n",
            start(&old_at, old_count),
            start(&new_at, new_count)
        ));
        for op in range {
            let (mark, line) = match op {
                Op::Same(l) => (' ', l),
                Op::Removed(l) => ('-', l),
                Op::Added(l) => ('+', l),
            };
            out.push(mark);
            out.push_str(line);
            if !line.ends_with('\n') {
                out.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
