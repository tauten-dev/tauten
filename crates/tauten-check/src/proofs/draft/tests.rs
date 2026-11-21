//! Unit tests of the claim drafter (#49): the translation scanner, the
//! join, the controls of the RFC, refusals, the inventory adapter and the
//! patch text.

use super::*;
use crate::proofs::MARKER;

const EXTRACTED: &str = r#"-- [k]
import Aeneas
namespace k

/-- [k::Acc]
    Source: 'k/src/lib.rs', lines 1:0-1:20 -/
structure Acc where
  n : Std.U32

/-- [k::a::step]:
    Source: 'k/src/a.rs', lines 1:0-3:1
    Visibility: public -/
def a.step (x : Std.U32) : Result Std.U32 := do
  ok x

/-- [k::b::step]:
    Source: 'k/src/b.rs', lines 1:0-3:1
    Visibility: public -/
def b.step (x : Std.U32) : Result Std.U32 := do
  ok x

/-- [k::helper]:
    Source: 'k/src/lib.rs', lines 3:0-5:1
    Visibility: public -/
def helper (x : Std.U32) : Result Std.U32 := do
  ok x

/-- [k::caller]:
    Source: 'k/src/lib.rs', lines 7:0-9:1
    Visibility: public -/
def caller (x : Std.U32) : Result Std.U32 := do
  helper x

/-- [k::sum]: loop 0:
    Source: 'k/src/lib.rs', lines 12:4-14:5
    Visibility: public -/
@[rust_loop]
def sum_loop (n : Std.U32) (i : Std.U32) : Result Std.U32 := do
  ok i

/-- [k::sum]:
    Source: 'k/src/lib.rs', lines 11:0-16:1
    Visibility: public -/
def sum (n : Std.U32) : Result Std.U32 := do
  sum_loop n 0#u32

/-- [k::{k::Acc}::push]:
    Source: 'k/src/lib.rs', lines 19:4-21:5
    Visibility: public -/
def Acc.push (self : Acc) (x : Std.U32) : Result Acc := do
  ok self

/-- [k::cond]:
    Source: 'k/src/lib.rs', lines 24:0-26:1
    Visibility: public -/
def cond (x : Std.U32) : Result Std.U32 := do
  x * 2#u32

/-- [k::kept]:
    Source: 'k/src/lib.rs', lines 28:0-30:1
    Visibility: public -/
def kept (x : Std.U32) : Result Std.U32 := do
  ok x

end k
"#;

const LIB: &str = "pub mod a;
pub mod b;
pub fn helper(x: u32) -> u32 {
    x | 0
}
pub fn caller(x: u32) -> u32 {
    helper(x) + 0
}
pub fn sum(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}
pub struct Acc { n: u32 }
impl Acc {
    pub fn push(self, x: u32) -> Self {
        Acc { n: x }
    }
}
pub fn cond(x: u32) -> u32 {
    x * 2
}
pub fn kept(x: u32) -> u32 {
    x - 0
}
";

const A: &str = "pub fn step(x: u32) -> u32 {\n    x + 1\n}\n";
const B: &str = "pub fn step(x: u32) -> u32 {\n    x + 2\n}\n";

const CLAIMS: &str = r#"version = 2

[gate]
command = ["sh", "gate.sh"]
try = ["sh", "gate.sh"]
crates = ["k"]

# A hand-written claim under an assumption; keep this comment.
[[claim]]
crate = "k"
file = "k/src/lib.rs"
function = "kept"
lean = "k.kept"
theorems = ["K.kept_spec"]
assumes = ["X.a"]
witness = "K.kept_witness"
mutants = ["kept-m"]

[[amend]]
file = "k/src/lib.rs"
function = "kept"
kind = "theorem"
name = "K.kept_old"
reason = "replaced"

[[mutant]]
name = "kept-m"
file = "k/src/lib.rs"
before = "x - 0"
after = "x - 1"

[[mutant]]
name = "a-m"
file = "k/src/a.rs"
before = "x + 1"
after = "x + 3"

[[mutant]]
name = "b-m"
file = "k/src/b.rs"
before = "x + 2"
after = "x + 4"

[[mutant]]
name = "helper-m"
file = "k/src/lib.rs"
before = "x | 0"
after = "x | 1"

[[mutant]]
name = "caller-m"
file = "k/src/lib.rs"
before = "helper(x) + 0"
after = "helper(x) + 1"

[[mutant]]
name = "sum-m"
file = "k/src/lib.rs"
before = "while i < n"
after = "while i <= n"

[[mutant]]
name = "push-m"
file = "k/src/lib.rs"
before = "Acc { n: x }"
after = "Acc { n: 0 }"

[[mutant]]
name = "cond-m"
file = "k/src/lib.rs"
before = "x * 2"
after = "x * 3"
"#;

/// The audit lines of a passing run without mutants.
const AUDIT: &[&str] = &[
    r#"{"stage":"tools","status":"ok"}"#,
    r#"{"stage":"extract","status":"ok"}"#,
    r#"{"stage":"drift","status":"ok"}"#,
    r#"{"stage":"build","status":"ok"}"#,
    r#"{"theorem":"K.a_total","status":"ok"}"#,
    r#"{"theorem":"K.b_spec","status":"ok"}"#,
    r#"{"theorem":"K.helper_total","status":"ok"}"#,
    r#"{"theorem":"K.sum_loop_total","status":"ok"}"#,
    r#"{"theorem":"K.sum_spec","status":"ok"}"#,
    r#"{"theorem":"K.push_total","status":"ok"}"#,
    r#"{"theorem":"K.cond_spec","status":"ok"}"#,
    r#"{"theorem":"K.kept_spec","status":"ok"}"#,
    r#"{"theorem":"K.kept_total","status":"ok"}"#,
    r#"{"theorem":"K.kept_more","status":"ok"}"#,
    r#"{"theorem":"K.kept_witness","status":"ok"}"#,
    r#"{"theorem":"K.a_total","subject":"k.a.step"}"#,
    r#"{"theorem":"K.b_spec","subject":"k.b.step"}"#,
    r#"{"theorem":"K.helper_total","subject":"k.helper"}"#,
    r#"{"theorem":"K.sum_loop_total","subject":"k.sum_loop"}"#,
    r#"{"theorem":"K.sum_spec","subject":"k.sum"}"#,
    r#"{"theorem":"K.push_total","subject":"k.Acc.push"}"#,
    r#"{"theorem":"K.cond_spec","subject":"k.cond"}"#,
    r#"{"theorem":"K.kept_spec","subject":"k.kept"}"#,
    r#"{"theorem":"K.kept_total","subject":"k.kept"}"#,
    r#"{"theorem":"K.kept_more","subject":"k.kept"}"#,
    r#"{"total":"K.a_total","function":"k.a.step"}"#,
    r#"{"total":"K.helper_total","function":"k.helper"}"#,
    r#"{"total":"K.sum_loop_total","function":"k.sum_loop"}"#,
    r#"{"total":"K.push_total","function":"k.Acc.push"}"#,
    r#"{"total":"K.kept_total","function":"k.kept"}"#,
    r#"{"theorem":"K.a_total","premises":[]}"#,
    r#"{"theorem":"K.b_spec","premises":[]}"#,
    r#"{"theorem":"K.helper_total","premises":[]}"#,
    r#"{"theorem":"K.sum_loop_total","premises":[]}"#,
    r#"{"theorem":"K.sum_spec","premises":[]}"#,
    r#"{"theorem":"K.push_total","premises":[]}"#,
    r#"{"theorem":"K.cond_spec","premises":[]}"#,
    r#"{"theorem":"K.kept_spec","premises":["X.a"]}"#,
    r#"{"theorem":"K.kept_total","premises":[]}"#,
    r#"{"theorem":"K.kept_more","premises":["X.b"]}"#,
    r#"{"theorem":"K.kept_witness","premises":[]}"#,
    r#"{"stage":"audit","status":"ok"}"#,
];

fn report(lines: &[&str]) -> Report {
    Report {
        events: lines
            .iter()
            .filter_map(|l| Event::parse(&format!("{MARKER}{l}")))
            .collect(),
        ..Report::default()
    }
}

struct Fixture {
    claims: ClaimsFile,
    text: String,
    packages: Vec<Package>,
    extracted: BTreeMap<String, String>,
    sources: BTreeMap<String, String>,
    report: Report,
}

fn fixture() -> Fixture {
    fixture_with(CLAIMS, AUDIT)
}

fn fixture_with(claims: &str, audit: &[&str]) -> Fixture {
    Fixture {
        claims: ClaimsFile::parse(claims).expect("claims"),
        text: claims.to_owned(),
        packages: vec![Package {
            name: "k".to_owned(),
            dir: "k".to_owned(),
            lib: "k".to_owned(),
        }],
        extracted: BTreeMap::from([("K.lean".to_owned(), EXTRACTED.to_owned())]),
        sources: BTreeMap::from([
            ("k/src/lib.rs".to_owned(), LIB.to_owned()),
            ("k/src/a.rs".to_owned(), A.to_owned()),
            ("k/src/b.rs".to_owned(), B.to_owned()),
        ]),
        report: report(audit),
    }
}

fn run(f: &Fixture, inventory: Option<&Inventory>) -> Result<Drafted, Vec<String>> {
    draft(&Inputs {
        claims: &f.claims,
        text: &f.text,
        packages: &f.packages,
        extracted: &f.extracted,
        sources: &f.sources,
        report: &f.report,
        inventory,
    })
}

fn drafted_claims(d: &Drafted) -> ClaimsFile {
    ClaimsFile::parse(&d.text).expect("drafted manifest parses")
}

fn claim<'a>(file: &'a ClaimsFile, path: &str, function: &str) -> Option<&'a Claim> {
    file.claim
        .iter()
        .find(|c| c.file == path && c.function == function)
}

#[test]
fn the_translation_names_items_files_and_loops() {
    let found = translated(EXTRACTED);
    let rows: Vec<(&str, Role, &str, &str)> = found
        .iter()
        .map(|t| (t.item.as_str(), t.role, t.file.as_str(), t.lean.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("k::a::step", Role::Spec, "k/src/a.rs", "k.a.step"),
            ("k::b::step", Role::Spec, "k/src/b.rs", "k.b.step"),
            ("k::helper", Role::Spec, "k/src/lib.rs", "k.helper"),
            ("k::caller", Role::Spec, "k/src/lib.rs", "k.caller"),
            ("k::sum", Role::Loop, "k/src/lib.rs", "k.sum_loop"),
            ("k::sum", Role::Spec, "k/src/lib.rs", "k.sum"),
            (
                "k::{k::Acc}::push",
                Role::Spec,
                "k/src/lib.rs",
                "k.Acc.push"
            ),
            ("k::cond", Role::Spec, "k/src/lib.rs", "k.cond"),
            ("k::kept", Role::Spec, "k/src/lib.rs", "k.kept"),
        ]
    );
    // A loop body is a loop too; an unknown suffix is not a function.
    let body = "namespace k\n/-- [k::f]: loop body 0:\n    Source: 'k/src/lib.rs', lines 1:0-1:1 -/\n@[rust_loop_body]\ndef f_body (x : U32) : Result U32 := ok x\n/-- [k::g]: something else:\n    Source: 'k/src/lib.rs', lines 1:0-1:1 -/\ndef g : Nat := 0\nend k\n";
    let found = translated(body);
    assert_eq!(found.len(), 1);
    assert_eq!(found.first().map(|t| t.role), Some(Role::Loop));
}

#[test]
fn claim_functions_follow_the_source_module() {
    let m = |file: &str| module_of(file, "k").expect("module");
    assert_eq!(m("k/src/lib.rs"), Vec::<String>::new());
    assert_eq!(m("k/src/a.rs"), ["a"]);
    assert_eq!(m("k/src/a/mod.rs"), ["a"]);
    assert_eq!(m("k/src/a/b.rs"), ["a", "b"]);
    assert_eq!(module_of("k/src/bin/x.rs", "k"), None);
    assert_eq!(module_of("other/src/lib.rs", "k"), None);
    assert_eq!(module_of("src/lib.rs", ""), Some(Vec::new()));
    let f = |item: &str, modules: &[&str]| {
        let modules: Vec<String> = modules.iter().map(|s| (*s).to_owned()).collect();
        claim_function(item, "k", &modules)
    };
    assert_eq!(f("k::a::step", &["a"]).as_deref(), Some("step"));
    assert_eq!(f("k::a::step", &[]).as_deref(), Some("a::step"));
    assert_eq!(f("k::a::step", &["b"]), None);
    assert_eq!(f("other::step", &[]), None);
    assert_eq!(
        f("k::m::{k::m::Acc<'a, T>}::push", &["m"]).as_deref(),
        Some("Acc::push")
    );
    assert_eq!(
        f("k::{impl core::convert::From<k::E> for k::Op}::from", &[]).as_deref(),
        Some("<Op as From>::from")
    );
    assert_eq!(f("k::deposit::{closure#1}", &[]), None);
}

#[test]
fn drafts_new_claims_from_audited_theorems_and_located_mutants() {
    let f = fixture();
    let d = run(&f, None).expect("draft");
    assert_eq!((d.added, d.extended), (6, 1), "{:#?}", d.notes);
    let out = drafted_claims(&d);
    // Two functions named `step`, each with its own theorem and mutant.
    let a = claim(&out, "k/src/a.rs", "step").expect("a::step");
    assert_eq!(a.lean, "k.a.step");
    assert_eq!(a.theorems, ["K.a_total"]);
    assert_eq!(a.total.as_deref(), Some("K.a_total"));
    assert_eq!(a.subsumes, ["TAU-N001", "TAU-P003"]);
    assert_eq!(a.mutants, ["a-m"]);
    let b = claim(&out, "k/src/b.rs", "step").expect("b::step");
    assert_eq!(b.theorems, ["K.b_spec"]);
    assert_eq!(b.total, None);
    assert!(b.subsumes.is_empty());
    assert_eq!(b.mutants, ["b-m"]);
    // A helper's total theorem is the helper's, never its caller's.
    let helper = claim(&out, "k/src/lib.rs", "helper").expect("helper");
    assert_eq!(helper.total.as_deref(), Some("K.helper_total"));
    assert_eq!(helper.mutants, ["helper-m"]);
    assert!(claim(&out, "k/src/lib.rs", "caller").is_none());
    assert!(
        d.notes.iter().any(|n| n.contains("skipped k::caller")),
        "{:#?}",
        d.notes
    );
    // A loop's total theorem joins its function, but is not `total`.
    let sum = claim(&out, "k/src/lib.rs", "sum").expect("sum");
    assert_eq!(sum.theorems, ["K.sum_loop_total", "K.sum_spec"]);
    assert_eq!(sum.total, None);
    assert!(sum.subsumes.is_empty());
    // A method is `Type::name`.
    let push = claim(&out, "k/src/lib.rs", "Acc::push").expect("push");
    assert_eq!(push.lean, "k.Acc.push");
    // A conditional theorem (a hypothesis, no TOTAL line) is no `total`.
    let cond = claim(&out, "k/src/lib.rs", "cond").expect("cond");
    assert_eq!(cond.total, None);
}

#[test]
fn a_claim_with_assumptions_gains_no_subsumes_and_no_new_assumption() {
    let d = run(&fixture(), None).expect("draft");
    let out = drafted_claims(&d);
    let kept = claim(&out, "k/src/lib.rs", "kept").expect("kept");
    assert_eq!(kept.theorems, ["K.kept_spec", "K.kept_total"]);
    assert_eq!(kept.total.as_deref(), Some("K.kept_total"));
    assert!(kept.subsumes.is_empty(), "conditional claim: no subsumes");
    assert_eq!(kept.assumes, ["X.a"]);
    assert!(
        d.notes
            .iter()
            .any(|n| n.contains("K.kept_more") && n.contains("X.b")),
        "{:#?}",
        d.notes
    );
}

#[test]
fn hand_written_text_survives_and_a_second_draft_is_empty() {
    let f = fixture();
    let d = run(&f, None).expect("draft");
    // Only additions: every original line is still there, in order.
    let mut new_lines = d.text.lines();
    for old in CLAIMS.lines() {
        if old == "theorems = [\"K.kept_spec\"]" {
            assert!(new_lines.any(|l| l == "theorems = [\"K.kept_spec\", \"K.kept_total\"]"));
            continue;
        }
        assert!(new_lines.any(|l| l == old), "lost or reordered: {old}");
    }
    assert!(
        d.text
            .contains("# A hand-written claim under an assumption; keep this comment.\n")
    );
    assert!(d.text.contains("reason = \"replaced\"\n"));
    assert!(
        d.patch
            .starts_with("--- a/proofs/claims.toml\n+++ b/proofs/claims.toml\n@@ ")
    );
    let removed: Vec<&str> = d
        .patch
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .collect();
    assert_eq!(removed, ["-theorems = [\"K.kept_spec\"]"]);
    let again = fixture_with(&d.text, AUDIT);
    let second = run(&again, None).expect("second draft");
    assert!(second.patch.is_empty(), "{}", second.patch);
    assert_eq!(second.text, d.text);
}

#[test]
fn a_draft_is_deterministic() {
    let one = run(&fixture(), None).expect("draft");
    let two = run(&fixture(), None).expect("draft");
    assert_eq!(one.patch, two.patch);
}

fn stage(name: &str, status: &str, detail: Option<&str>) -> Event {
    Event::Stage {
        stage: name.to_owned(),
        status: status.to_owned(),
        detail: detail.map(str::to_owned),
        krate: None,
    }
}

#[test]
fn missing_evidence_refuses_the_draft() {
    type Edit = fn(&mut Report);
    let cases: [(Edit, &str); 5] = [
        (
            |r| r.not_started = Some("cannot run `sh`".to_owned()),
            "could not start",
        ),
        (
            |r| r.events.push(stage("tools", "missing", Some("no docker"))),
            "`tools` stage is missing: no docker",
        ),
        (|r| r.events.push(stage("drift", "fail", None)), "stale"),
        (
            |r| {
                r.events
                    .retain(|e| !matches!(e, Event::Stage { stage, .. } if stage == "audit"));
            },
            "no `audit` result",
        ),
        (
            |r| r.events.retain(|e| !matches!(e, Event::Subject { .. })),
            "no theorem subjects",
        ),
    ];
    for (edit, needle) in cases {
        let mut f = fixture();
        edit(&mut f.report);
        let refused = run(&f, None).expect_err(needle);
        assert!(
            refused.iter().any(|r| r.contains(needle)),
            "{needle}: {refused:?}"
        );
    }
}

#[test]
fn ambiguous_associations_refuse_the_draft() {
    // A theorem reported about two declarations.
    let mut audit = AUDIT.to_vec();
    audit.push(r#"{"theorem":"K.a_total","subject":"k.b.step"}"#);
    let refused = run(&fixture_with(CLAIMS, &audit), None).expect_err("two subjects");
    assert!(
        refused.iter().any(|r| r.contains("several declarations")),
        "{refused:?}"
    );
    // Two declarations for one claim key.
    let mut f = fixture();
    let twin = EXTRACTED.replace(
        "end k\n",
        "/-- [k::a::step]:\n    Source: 'k/src/a.rs', lines 1:0-3:1 -/\ndef a.step_twin (x : Std.U32) : Result Std.U32 := ok x\nend k\n",
    );
    f.extracted.insert("K.lean".to_owned(), twin);
    let refused = run(&f, None).expect_err("twin");
    assert!(
        refused
            .iter()
            .any(|r| r.contains("several translated declarations")),
        "{refused:?}"
    );
}

#[test]
fn unusable_mutants_and_failed_theorems_are_reported_not_used() {
    let claims = CLAIMS
        .replace("before = \"x + 2\"", "before = \"x\"")
        .replace(
            "name = \"push-m\"\nfile = \"k/src/lib.rs\"\nbefore = \"Acc { n: x }\"",
            "name = \"push-m\"\nfile = \"k/src/lib.rs\"\nbefore = \"Acc { n: x }\\n    }\\n}\\npub fn cond\"",
        );
    let mut audit = AUDIT.to_vec();
    audit.push(r#"{"theorem":"K.cond_spec","status":"fail"}"#);
    let d = run(&fixture_with(&claims, &audit), None).expect("draft");
    let out = drafted_claims(&d);
    assert!(claim(&out, "k/src/b.rs", "step").is_none());
    assert!(claim(&out, "k/src/lib.rs", "cond").is_none());
    // A site across two functions targets neither.
    assert!(claim(&out, "k/src/lib.rs", "Acc::push").is_none());
    assert!(
        d.notes.iter().any(
            |n| n.contains("mutant push-m") && n.contains("not inside one translated function")
        ),
        "{:#?}",
        d.notes
    );
    assert!(
        d.notes
            .iter()
            .any(|n| n.contains("mutant b-m") && n.contains("exactly once"))
    );
    assert!(
        d.notes
            .iter()
            .any(|n| n.contains("K.cond_spec did not pass the audit"))
    );
}

#[test]
fn only_the_selected_packages_are_drafted() {
    let mut f = fixture();
    f.packages = vec![Package {
        name: "other".to_owned(),
        dir: "other".to_owned(),
        lib: "other".to_owned(),
    }];
    let d = run(&f, None).expect("draft");
    assert!(d.patch.is_empty(), "{}", d.patch);
}

#[test]
fn a_layout_the_draft_cannot_edit_is_refused() {
    // `kept` written as an inline table: no `[[claim]]` table to extend.
    let inline = CLAIMS
        .replace(
            "[[claim]]\ncrate = \"k\"\nfile = \"k/src/lib.rs\"\nfunction = \"kept\"\nlean = \"k.kept\"\ntheorems = [\"K.kept_spec\"]\nassumes = [\"X.a\"]\nwitness = \"K.kept_witness\"\nmutants = [\"kept-m\"]\n",
            "",
        )
        .replace(
            "version = 2\n",
            "version = 2\nclaim = [{ crate = \"k\", file = \"k/src/lib.rs\", function = \"kept\", lean = \"k.kept\", theorems = [\"K.kept_spec\"], assumes = [\"X.a\"], witness = \"K.kept_witness\", mutants = [\"kept-m\"] }]\n",
        );
    let refused = run(&fixture_with(&inline, AUDIT), None).expect_err("inline");
    assert!(
        refused.iter().any(|r| r.contains("edit by hand")),
        "{refused:?}"
    );
}

fn inventory(json: &str) -> Inventory {
    Inventory::parse(json).expect("inventory")
}

const INVENTORY: &str = r#"{
  "version": 1,
  "functions": [
    {"package": "k", "file": "k/src/a.rs", "item": "k::a::step", "declaration": "k.a.step"},
    {"package": "k", "file": "k/src/lib.rs", "item": "k::sum", "declaration": "k.sum"}
  ],
  "theorems": [
    {"name": "K.a_total", "subject": "k.a.step", "role": "total"},
    {"name": "K.sum_spec", "subject": "k.sum", "role": "spec"},
    {"name": "K.sum_loop_total", "subject": "k.sum_loop", "role": "loop"}
  ],
  "mutants": [
    {"name": "a-m", "item": "k::a::step", "theorems": ["K.a_total"]},
    {"name": "sum-m", "item": "k::sum", "theorems": ["K.sum_spec"]},
    {"name": "custom-m", "item": "k::sum", "theorems": ["K.sum_loop_total"]}
  ]
}"#;

#[test]
fn an_inventory_drafts_only_what_it_names_and_checks_it() {
    let inv = inventory(INVENTORY);
    let d = run(&fixture(), Some(&inv)).expect("draft");
    assert_eq!((d.added, d.extended), (2, 0), "{:#?}", d.notes);
    let out = drafted_claims(&d);
    let a = claim(&out, "k/src/a.rs", "step").expect("a");
    assert_eq!(a.total.as_deref(), Some("K.a_total"));
    let sum = claim(&out, "k/src/lib.rs", "sum").expect("sum");
    assert_eq!(sum.total, None);
    assert_eq!(sum.mutants, ["custom-m", "sum-m"]);
    // Subject lines are not needed with an inventory.
    let mut f = fixture();
    f.report
        .events
        .retain(|e| !matches!(e, Event::Subject { .. }));
    assert_eq!(run(&f, Some(&inv)).expect("draft").patch, d.patch);
}

#[test]
fn a_contradictory_inventory_is_refused() {
    let cases = [
        (
            INVENTORY.replace(
                r#""subject": "k.sum", "role": "spec""#,
                r#""subject": "k.sum", "role": "total""#,
            ),
            "did not find it total",
        ),
        (
            INVENTORY.replace(r#""declaration": "k.sum""#, r#""declaration": "k.cond""#),
            "the translation says",
        ),
        (
            INVENTORY.replace(r#""declaration": "k.sum""#, r#""declaration": "k.nowhere""#),
            "not a declaration of the translation",
        ),
        (
            INVENTORY.replace(r#""role": "loop""#, r#""role": "spec""#),
            "contradicts",
        ),
        (
            INVENTORY.replace(
                r#""theorems": ["K.sum_spec"]"#,
                r#""theorems": ["K.unknown"]"#,
            ),
            "not an inventory theorem",
        ),
        (
            INVENTORY.replace(
                r#""name": "sum-m", "item": "k::sum""#,
                r#""name": "b-m", "item": "k::sum""#,
            ),
            "declares it in k/src/b.rs",
        ),
        (
            INVENTORY.replace(
                r#""package": "k", "file": "k/src/a.rs""#,
                r#""package": "z", "file": "k/src/a.rs""#,
            ),
            "not selected",
        ),
        (
            INVENTORY.replace(r#""name": "K.sum_spec""#, r#""name": "K.never_audited""#),
            "not a theorem the audit checked",
        ),
    ];
    for (json, needle) in cases {
        let inv = inventory(&json);
        let refused = run(&fixture(), Some(&inv)).expect_err(needle);
        assert!(
            refused.iter().any(|r| r.contains(needle)),
            "{needle}: {refused:?}"
        );
    }
    assert!(Inventory::parse(&INVENTORY.replace("\"version\": 1", "\"version\": 2")).is_err());
    assert!(
        Inventory::parse(&INVENTORY.replace("\"version\": 1", "\"version\": 1, \"extra\": 0"))
            .is_err()
    );
}

#[test]
fn toml_classes_and_quoting() {
    let text = "a = \"# not a comment\" # a comment\nb = '''x\n]'''\n";
    let classes = classify(text);
    let at = |needle: &str| text.find(needle).and_then(|i| classes.get(i).copied());
    assert_eq!(at("# not"), Some(Class::Text));
    assert_eq!(at("# a comment"), Some(Class::Comment));
    assert_eq!(at("]'''"), Some(Class::Text));
    assert_eq!(at("a ="), Some(Class::Code));
    assert_eq!(quoted("a\"b\\c"), "\"a\\\"b\\\\c\"");
}

#[test]
fn a_manifest_without_a_final_newline_gets_a_valid_patch() {
    let claims = CLAIMS.trim_end_matches('\n');
    let d = run(&fixture_with(claims, AUDIT), None).expect("draft");
    assert!(
        d.patch.contains("\n\\ No newline at end of file\n"),
        "{}",
        d.patch
    );
    assert!(d.text.ends_with('\n'));
    ClaimsFile::parse(&d.text).expect("parses");
}

#[test]
fn multi_line_arrays_keep_one_element_per_line() {
    let claims = CLAIMS.replace(
        "theorems = [\"K.kept_spec\"]",
        "theorems = [\n    \"K.kept_spec\", # why\n]",
    );
    let d = run(&fixture_with(&claims, AUDIT), None).expect("draft");
    assert!(
        d.text.contains(
            "theorems = [\n    \"K.kept_spec\", # why\n    \"K.kept_total\",\n]\ntotal = \"K.kept_total\"\n"
        ),
        "{}",
        d.text
    );
}
