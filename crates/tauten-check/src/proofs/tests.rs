//! Unit tests of the proof tier: claims.toml validation, the outcome of
//! each claim from gate results, fingerprints, function spans and the
//! subsumption of findings.

use super::*;
use crate::diag::EnforcedBy;
use crate::tier::Tier;

const CLAIMS: &str = r#"
version = 1

[gate]
command = ["sh", "proofs/run.sh"]
crates = ["crates/k"]

[[claim]]
crate = "k"
file = "crates/k/src/lib.rs"
function = "add"
lean = "k.add"
theorems = ["K.add_total", "K.add_spec"]
total = "K.add_total"
subsumes = ["TAU-N001"]
mutants = ["add-wraps"]

[[mutant]]
name = "add-wraps"
file = "crates/k/src/lib.rs"
before = "checked_add"
after = "wrapping_add"
"#;

fn claims() -> ClaimsFile {
    ClaimsFile::parse(CLAIMS).expect("valid claims")
}

fn claim() -> Claim {
    claims().claim.into_iter().next().expect("one claim")
}

fn event(json: &str) -> Event {
    Event::parse(&format!("{MARKER}{json}")).expect("event")
}

fn passing() -> Report {
    Report {
        events: [
            r#"{"stage":"tools","status":"ok"}"#,
            r#"{"stage":"extract","status":"ok"}"#,
            r#"{"stage":"drift","status":"ok"}"#,
            r#"{"stage":"build","status":"ok"}"#,
            r#"{"theorem":"K.add_total","status":"ok"}"#,
            r#"{"theorem":"K.add_spec","status":"ok"}"#,
            r#"{"total":"K.add_total","function":"k.add"}"#,
            r#"{"stage":"audit","status":"ok"}"#,
            r#"{"mutant":"add-wraps","status":"killed","broken":["K.add_spec"]}"#,
            r#"{"stage":"mutants","status":"ok"}"#,
        ]
        .iter()
        .map(|line| event(line))
        .collect(),
        ..Report::default()
    }
}

fn without(report: &Report, needle: &str) -> Report {
    let mut out = report.clone();
    out.events.retain(|e| !format!("{e:?}").contains(needle));
    out
}

fn with(report: &Report, json: &str) -> Report {
    let mut out = report.clone();
    out.events.push(event(json));
    out
}

#[test]
fn claims_file_parses() {
    let file = claims();
    assert_eq!(file.gate.command, ["sh", "proofs/run.sh"]);
    assert_eq!(file.mutant.len(), 1);
    assert_eq!(claim().total.as_deref(), Some("K.add_total"));
}

#[test]
fn inconsistent_claims_are_rejected() {
    let cases = [
        ("mutants = [\"add-wraps\"]", "mutants = []", "no mutant"),
        (
            "theorems = [\"K.add_total\", \"K.add_spec\"]",
            "theorems = []",
            "no theorem",
        ),
        (
            "total = \"K.add_total\"",
            "total = \"K.other\"",
            "one of its",
        ),
        (
            "subsumes = [\"TAU-N001\"]",
            "subsumes = [\"TAU-E001\"]",
            "only TAU-N001",
        ),
        ("total = \"K.add_total\"\n", "", "needs a `total`"),
        ("version = 1", "version = 3", "version 3"),
        (
            "command = [\"sh\", \"proofs/run.sh\"]",
            "command = []",
            "empty",
        ),
    ];
    for (from, to, expected) in cases {
        assert!(CLAIMS.contains(from), "{from}");
        let error = ClaimsFile::parse(&CLAIMS.replacen(from, to, 1)).expect_err(expected);
        assert!(error.to_string().contains(expected), "{error}");
    }
    let twice = format!(
        "{CLAIMS}\n{}",
        &CLAIMS[CLAIMS.find("[[claim]]").expect("claim")..]
    );
    let error = ClaimsFile::parse(&twice).expect_err("claimed twice");
    assert!(error.to_string().contains("claimed twice"), "{error}");
}

#[test]
fn every_outcome_is_reachable() {
    let claim = claim();
    let ok = passing();
    assert_eq!(outcome(&claim, &ok), (Outcome::Proved, None));

    let cases: Vec<(Report, Outcome)> = vec![
        (
            Report {
                not_started: Some("docker: not found".to_owned()),
                ..Report::default()
            },
            Outcome::ToolMissing,
        ),
        (Report::default(), Outcome::ToolMissing),
        (
            with(
                &Report::default(),
                r#"{"stage":"tools","status":"missing","detail":"no image"}"#,
            ),
            Outcome::ToolMissing,
        ),
        (
            with(&ok, r#"{"stage":"extract","status":"fail"}"#),
            Outcome::BuildFailed,
        ),
        (
            with(&ok, r#"{"stage":"drift","status":"fail"}"#),
            Outcome::Drift,
        ),
        (
            with(&ok, r#"{"stage":"build","status":"fail"}"#),
            Outcome::BuildFailed,
        ),
        (
            with(&ok, r#"{"stage":"audit","status":"fail"}"#),
            Outcome::AuditFailed,
        ),
        (without(&ok, "stage: \"audit\""), Outcome::AuditFailed),
        (without(&ok, "K.add_spec\", status"), Outcome::AuditFailed),
        (
            with(&ok, r#"{"theorem":"K.add_spec","status":"fail"}"#),
            Outcome::AuditFailed,
        ),
        (without(&ok, "Total"), Outcome::AuditFailed),
        (without(&ok, "Mutant"), Outcome::MutantSurvived),
        (
            with(
                &without(&ok, "Mutant"),
                r#"{"mutant":"add-wraps","status":"killed","broken":["Other.thm"]}"#,
            ),
            Outcome::MutantSurvived,
        ),
        (
            with(
                &without(&ok, "Mutant"),
                r#"{"mutant":"add-wraps","status":"survived","broken":[]}"#,
            ),
            Outcome::MutantSurvived,
        ),
        (
            Report {
                changed_during_run: true,
                ..ok.clone()
            },
            Outcome::Drift,
        ),
    ];
    for (index, (report, expected)) in cases.iter().enumerate() {
        let (got, detail) = outcome(&claim, report);
        assert_eq!(got, *expected, "case {index}: {detail:?}");
        assert!(detail.is_some(), "case {index} gives a reason");
    }
}

#[test]
fn ordinary_output_is_not_an_event() {
    assert_eq!(Event::parse("lake build"), None);
    assert_eq!(Event::parse("TAUTEN-PROOFS not json"), None);
    assert!(matches!(
        Event::parse("TAUTEN-PROOFS {\"stage\":\"drift\",\"status\":\"ok\"}\n"),
        Some(Event::Stage { .. })
    ));
}

#[test]
fn fingerprint_changes_with_any_byte_or_path() {
    let files = vec![
        ("a.rs".to_owned(), b"fn a() {}".to_vec()),
        ("b.rs".to_owned(), b"fn b() {}".to_vec()),
    ];
    let base = fingerprint(&files);
    assert!(base.starts_with("fnv1a64:"));
    let reversed: Vec<_> = files.iter().rev().cloned().collect();
    assert_eq!(
        fingerprint(&reversed),
        base,
        "order of reading does not matter"
    );
    let mut edited = files.clone();
    edited[0].1 = b"fn a() { }".to_vec();
    assert_ne!(fingerprint(&edited), base);
    let mut moved = files.clone();
    moved[1].0 = "c.rs".to_owned();
    assert_ne!(fingerprint(&moved), base);
    // The boundary between path and contents is part of the hash.
    let split = vec![("ab".to_owned(), b"c".to_vec())];
    let other = vec![("a".to_owned(), b"bc".to_vec())];
    assert_ne!(fingerprint(&split), fingerprint(&other));
}

const SOURCE: &str = "\
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

pub struct Acc(u32);

impl Acc {
    pub fn bump(&mut self) {
        self.0 += 1;
        fn helper(x: u32) -> u32 {
            x * 2
        }
    }
}

mod inner {
    pub fn get(xs: &[u32]) -> u32 {
        xs[0]
    }
}
";

#[test]
fn functions_are_found_with_their_lines() {
    let spans = functions(SOURCE);
    let paths: Vec<(&str, u32, u32)> = spans
        .iter()
        .map(|s| (s.path.as_str(), s.start, s.end))
        .collect();
    assert_eq!(
        paths,
        [
            ("add", 1, 3),
            ("Acc::bump", 8, 13),
            ("Acc::bump::helper", 10, 12),
            ("inner::get", 17, 19),
        ]
    );
    assert_eq!(
        enclosing(&spans, 11).map(|s| s.path.as_str()),
        Some("Acc::bump::helper")
    );
    assert_eq!(
        enclosing(&spans, 9).map(|s| s.path.as_str()),
        Some("Acc::bump")
    );
    assert_eq!(enclosing(&spans, 5), None);
    assert!(functions("fn (").is_empty());
}

fn finding(id: &str, tier: Tier, line: u32) -> Diagnostic {
    Diagnostic {
        id: id.to_owned(),
        severity: Severity::Error,
        krate: "k".to_owned(),
        tier: Some(tier),
        file: Some("crates/k/src/lib.rs".to_owned()),
        line: Some(line),
        column: Some(5),
        message: "m".to_owned(),
        rule: None,
        why: None,
        fix: None,
        enforced_by: EnforcedBy::Clippy,
        source: "clippy::arithmetic_side_effects".to_owned(),
        subsumed_by: None,
    }
}

fn record(outcome: Outcome) -> ClaimRecord {
    ClaimRecord {
        claim: claim(),
        outcome,
        badge: badge(&claim(), outcome, true),
        detail: None,
        fingerprint: "fnv1a64:0".to_owned(),
        inputs: BTreeMap::new(),
        mutant_results: Vec::new(),
    }
}

fn run(diag: Diagnostic, record: &ClaimRecord, fresh: bool) -> Diagnostic {
    let spans = BTreeMap::from([("crates/k/src/lib.rs".to_owned(), functions(SOURCE))]);
    let recorded = [Recorded { record, fresh }];
    subsume(vec![diag], &recorded, &spans)
        .pop()
        .expect("one diagnostic")
}

#[test]
fn a_fresh_proof_subsumes_listed_findings_in_its_function() {
    let proved = record(Outcome::Proved);
    let d = run(finding("TAU-N001", Tier::Kernel, 2), &proved, true);
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.subsumed_by.as_deref(), Some("K.add_total"));
}

#[test]
fn nothing_else_is_subsumed() {
    let proved = record(Outcome::Proved);
    let unchanged = |d: &Diagnostic| d.severity == Severity::Error && d.subsumed_by.is_none();
    // stale proof
    assert!(unchanged(&run(
        finding("TAU-N001", Tier::Kernel, 2),
        &proved,
        false
    )));
    // not a kernel crate
    assert!(unchanged(&run(
        finding("TAU-N001", Tier::Core, 2),
        &proved,
        true
    )));
    // rule not listed under `subsumes`
    assert!(unchanged(&run(
        finding("TAU-P003", Tier::Kernel, 2),
        &proved,
        true
    )));
    // rule that can never be subsumed
    assert!(unchanged(&run(
        finding("TAU-E001", Tier::Kernel, 2),
        &proved,
        true
    )));
    // outside the claimed function
    assert!(unchanged(&run(
        finding("TAU-N001", Tier::Kernel, 9),
        &proved,
        true
    )));
    // the gate did not prove the claim
    for outcome in [
        Outcome::Drift,
        Outcome::BuildFailed,
        Outcome::AuditFailed,
        Outcome::MutantSurvived,
        Outcome::ToolMissing,
    ] {
        assert!(unchanged(&run(
            finding("TAU-N001", Tier::Kernel, 2),
            &record(outcome),
            true
        )));
    }
    // a claim without a total theorem
    let mut partial = record(Outcome::Proved);
    partial.claim.total = None;
    assert!(unchanged(&run(
        finding("TAU-N001", Tier::Kernel, 2),
        &partial,
        true
    )));
    // another crate with the same file path
    let mut other = finding("TAU-N001", Tier::Kernel, 2);
    other.krate = "j".to_owned();
    assert!(unchanged(&run(other, &proved, true)));
}

#[test]
fn records_round_trip_as_json() {
    let record = Record {
        version: 1,
        generated_by: "cargo tauten proofs".to_owned(),
        claims: vec![record(Outcome::Proved), record(Outcome::MutantSurvived)],
        commit: Some("0123abc".to_owned()),
    };
    let text = serde_json::to_string_pretty(&record).expect("json");
    assert!(text.contains("\"outcome\": \"mutant-survived\""));
    assert!(text.contains("\"crate\": \"k\""));
    let back: Record = serde_json::from_str(&text).expect("parses");
    assert_eq!(back, record);
}

#[test]
fn the_repository_claims_files_are_valid() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for path in [
        "proofs/claims.toml",
        "bench/hosts/ledger/proofs/claims.toml",
        "fixtures/proofs-subsume/proofs/claims.toml",
    ] {
        let text = std::fs::read_to_string(root.join(path)).expect(path);
        let file = ClaimsFile::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert!(!file.claim.is_empty(), "{path}");
    }
}

/// A proof of the inherent `Acc::add` must not cover `<Acc as Add>::add`,
/// and a path that names two functions covers neither.
#[test]
fn trait_methods_and_ambiguous_paths_are_not_subsumed() {
    const COLLIDE: &str = "\
struct Acc;
impl Acc {
    fn add(a: u32, b: u32) -> u32 { a + b }
}
impl Add for Acc {
    fn add(a: u32, b: u32) -> u32 { a + b }
}
impl G<u8> {
    fn get(x: u8) -> u8 { x + 1 }
}
impl G<u16> {
    fn get(x: u16) -> u16 { x + 1 }
}
";
    let spans = functions(COLLIDE);
    let paths: Vec<&str> = spans.iter().map(|s| s.path.as_str()).collect();
    assert_eq!(paths, ["Acc::add", "<Acc as Add>::add", "G::get", "G::get"]);
    let mut claimed = record(Outcome::Proved);
    let file = BTreeMap::from([("crates/k/src/lib.rs".to_owned(), spans)]);
    let check = |claimed: &ClaimRecord, line: u32| {
        let recorded = [Recorded {
            record: claimed,
            fresh: true,
        }];
        subsume(
            vec![finding("TAU-N001", Tier::Kernel, line)],
            &recorded,
            &file,
        )
        .pop()
        .expect("one")
        .severity
    };
    claimed.claim.function = "Acc::add".to_owned();
    assert_eq!(check(&claimed, 3), Severity::Warning);
    assert_eq!(check(&claimed, 6), Severity::Error);
    claimed.claim.function = "G::get".to_owned();
    assert_eq!(check(&claimed, 9), Severity::Error);
    assert_eq!(check(&claimed, 12), Severity::Error);
}

/// The previous (unproved) enclosing-function lookup, kept as a reference.
fn reference_enclosing(spans: &[FunctionSpan], line: u32) -> Option<&FunctionSpan> {
    spans
        .iter()
        .filter(|span| span.start <= line && line <= span.end)
        .min_by_key(|span| span.end.saturating_sub(span.start))
}

#[test]
fn proved_lookup_agrees_with_the_reference_on_random_spans() {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = |bound: u32| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        u32::try_from(state % u64::from(bound)).expect("bounded")
    };
    for _ in 0..2000 {
        let count = next(8);
        let spans: Vec<FunctionSpan> = (0..count)
            .map(|i| {
                let start = next(40);
                FunctionSpan {
                    path: format!("f{i}"),
                    start,
                    end: start + next(20),
                }
            })
            .collect();
        let line = next(60);
        assert_eq!(
            enclosing(&spans, line).map(|s| &s.path),
            reference_enclosing(&spans, line).map(|s| &s.path),
            "{spans:?} at {line}"
        );
    }
}

/// Control admission (S2): results the previous aggregation accepted or
/// conflated are now refused, each for its own reason.
#[test]
fn controls_are_admitted_as_exact_sets() {
    let c = claim();
    assert_eq!(outcome(&c, &passing()).0, Outcome::Proved);

    // A mutant reported twice is not one kill.
    let twice = with(
        &passing(),
        r#"{"mutant":"add-wraps","status":"killed","broken":["K.add_spec"]}"#,
    );
    let (o, why) = outcome(&c, &twice);
    assert_eq!(o, Outcome::MutantSurvived);
    assert_eq!(why.as_deref(), Some("a mutant was reported more than once"));

    // A mutant that timed out is not a kill.
    let timed_out = with(
        &without(&passing(), "add-wraps"),
        r#"{"mutant":"add-wraps","status":"timeout","broken":[]}"#,
    );
    assert_eq!(outcome(&c, &timed_out).0, Outcome::MutantSurvived);

    // A mutant that broke only another claim's theorem is not this claim's kill.
    let elsewhere = with(
        &without(&passing(), "add-wraps"),
        r#"{"mutant":"add-wraps","status":"killed","broken":["K.other_spec"]}"#,
    );
    assert_eq!(outcome(&c, &elsewhere).0, Outcome::MutantSurvived);

    // A theorem reported twice (once failing) is refused by the audit.
    let doubled = with(&passing(), r#"{"theorem":"K.add_spec","status":"fail"}"#);
    assert_eq!(outcome(&c, &doubled).0, Outcome::AuditFailed);

    // A claim with no mutant has nothing showing its theorems can fail.
    let mut bare = claim();
    bare.mutants.clear();
    let (o, why) = outcome(&bare, &passing());
    assert_eq!(o, Outcome::MutantSurvived);
    assert!(why.is_some_and(|w| w.contains("no mutant")));
}

/// Freshness goes through the proved binding kernel (S4).
#[test]
fn freshness_is_decided_by_bindings() {
    let map = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    };
    let now = map(&[("k", "fnv1a64:01"), ("m", "fnv1a64:02")]);
    assert!(claim_fresh("k", "fnv1a64:01", &now));
    assert!(!claim_fresh("k", "fnv1a64:00", &now));
    assert!(!claim_fresh("x", "fnv1a64:01", &now));
    assert!(unchanged(&now, &now.clone()));
    // Any edited, added or removed input is a change.
    assert!(!unchanged(
        &now,
        &map(&[("k", "fnv1a64:01"), ("m", "fnv1a64:03")])
    ));
    assert!(!unchanged(&now, &map(&[("k", "fnv1a64:01")])));
    assert!(!unchanged(
        &now,
        &map(&[
            ("k", "fnv1a64:01"),
            ("m", "fnv1a64:02"),
            ("n", "fnv1a64:04")
        ])
    ));
}

/// Named assumptions (S3): a claim resting on one records `proved-under`,
/// never `proved`, and never subsumes a finding.
#[test]
fn assumptions_lower_the_badge() {
    let text = CLAIMS.replace("version = 1", "version = 2").replace(
        "mutants = [\"add-wraps\"]",
        "mutants = [\"add-wraps\"]\nassumes = [\"FIXTURE.example\"]\nwitness = \"K.example_holds\"",
    );
    let file = ClaimsFile::parse(&text).expect("valid v2 claims");
    let c = file.claim[0].clone();
    assert_eq!(c.assumes, ["FIXTURE.example"]);
    // Undefined assumption IDs are refused; defined ones are accepted.
    assert!(file.check_assumptions(&AssumptionsFile::default()).is_err());
    let known = AssumptionsFile::parse(
        "[[assumption]]\nid = \"FIXTURE.example\"\nstatement = \"an example\"\n",
    )
    .expect("assumptions");
    assert!(file.check_assumptions(&known).is_ok());
    assert!(AssumptionsFile::parse(
        "[[assumption]]\nid = \"A\"\nstatement = \"x\"\n[[assumption]]\nid = \"A\"\nstatement = \"y\"\n"
    )
    .is_err());
    // `assumes` needs version 2.
    assert!(ClaimsFile::parse(&text.replace("version = 2", "version = 1")).is_err());

    let (o, _) = outcome(&c, &assumed(&["FIXTURE.example"], &["FIXTURE.example"]));
    assert_eq!(o, Outcome::Proved);
    assert_eq!(badge(&c, o, true), Badge::ProvedUnder);
    assert_eq!(badge(&c, o, false), Badge::Pending);
    assert_eq!(badge(&claim(), o, true), Badge::Proved);

    // A proved-under claim does not subsume.
    let rec = ClaimRecord {
        claim: c.clone(),
        outcome: Outcome::Proved,
        badge: Badge::ProvedUnder,
        detail: None,
        fingerprint: "fnv1a64:0".to_owned(),
        inputs: BTreeMap::new(),
        mutant_results: Vec::new(),
    };
    let d = run(finding("TAU-N001", Tier::Kernel, 2), &rec, true);
    assert_eq!(d.severity, Severity::Error);
    assert!(d.subsumed_by.is_none());
}

/// A passing report in which `K.add_spec` assumes `premises` and the
/// witness `K.example_holds` establishes `established`.
fn assumed(premises: &[&str], established: &[&str]) -> Report {
    let list = |ids: &[&str]| {
        ids.iter()
            .map(|i| format!("\"{i}\""))
            .collect::<Vec<_>>()
            .join(",")
    };
    let r = with(&passing(), r#"{"theorem":"K.example_holds","status":"ok"}"#);
    let r = with(
        &r,
        &format!(
            r#"{{"theorem":"K.add_spec","premises":[{}]}}"#,
            list(premises)
        ),
    );
    with(
        &r,
        &format!(
            r#"{{"theorem":"K.example_holds","establishes":[{}]}}"#,
            list(established)
        ),
    )
}

/// A8: a claim's `assumes` is exactly what its theorems assume, and a
/// witness shows the assumptions can hold.
#[test]
fn assumptions_are_derived_and_witnessed() {
    let v2 = CLAIMS.replace("version = 1", "version = 2");
    let declared = |assumes: &str| {
        ClaimsFile::parse(&v2.replace(
            "mutants = [\"add-wraps\"]",
            &format!(
                "mutants = [\"add-wraps\"]\nassumes = [{assumes}]\nwitness = \"K.example_holds\""
            ),
        ))
        .expect("valid")
        .claim
        .remove(0)
    };
    let a = declared("\"A\"");
    assert_eq!(outcome(&a, &assumed(&["A"], &["A"])).0, Outcome::Proved);
    // A theorem assumes something the claim does not declare.
    let (o, why) = outcome(&a, &assumed(&["A", "B"], &["A", "B"]));
    assert_eq!(o, Outcome::AuditFailed);
    assert!(why.is_some_and(|w| w.contains("assumes")));
    assert_eq!(
        outcome(&claim(), &assumed(&["A"], &[])).0,
        Outcome::AuditFailed
    );
    // The claim declares something no theorem assumes (removing a
    // hypothesis keeps a stale `assumes`).
    let ab = declared("\"A\", \"B\"");
    assert_eq!(
        outcome(&ab, &assumed(&["A"], &["A", "B"])).0,
        Outcome::AuditFailed
    );
    // The witness does not establish every assumption.
    assert_eq!(outcome(&a, &assumed(&["A"], &[])).0, Outcome::AuditFailed);
    // The witness itself must pass the audit.
    let failed_witness = without(
        &assumed(&["A"], &["A"]),
        "theorem: \"K.example_holds\", status: \"ok\"",
    );
    let failed_witness = with(
        &failed_witness,
        r#"{"theorem":"K.example_holds","status":"fail"}"#,
    );
    assert_eq!(outcome(&a, &failed_witness).0, Outcome::AuditFailed);
    // No premise lines at all: a declared assumption is not confirmed.
    assert_eq!(outcome(&a, &passing()).0, Outcome::AuditFailed);
    // `assumes` without `witness` does not parse.
    assert!(
        ClaimsFile::parse(&v2.replace(
            "mutants = [\"add-wraps\"]",
            "mutants = [\"add-wraps\"]\nassumes = [\"A\"]",
        ))
        .is_err()
    );
    // Necessity theorems must pass like theorems.
    let mut n = a.clone();
    n.necessity = vec!["K.a_needed".to_owned()];
    assert_eq!(
        outcome(&n, &assumed(&["A"], &["A"])).0,
        Outcome::AuditFailed
    );
    let ok = with(
        &assumed(&["A"], &["A"]),
        r#"{"theorem":"K.a_needed","status":"ok"}"#,
    );
    assert_eq!(outcome(&n, &ok).0, Outcome::Proved);
}

fn committed(claims: &ClaimsFile) -> Record {
    Record {
        version: 1,
        generated_by: "cargo tauten proofs".to_owned(),
        commit: None,
        claims: claims
            .claim
            .iter()
            .map(|c| ClaimRecord {
                claim: c.clone(),
                outcome: Outcome::Proved,
                badge: Badge::Proved,
                detail: None,
                fingerprint: String::new(),
                inputs: BTreeMap::new(),
                mutant_results: Vec::new(),
            })
            .collect(),
    }
}

const AMEND_MUTANT: &str = r#"
[[amend]]
file = "crates/k/src/lib.rs"
function = "add"
kind = "mutant"
name = "add-wraps"
reason = "replaced by a stronger mutant"
"#;

#[test]
fn recorded_claims_may_not_be_weakened() {
    let record = committed(&claims());
    // Unchanged, or strengthened: conserved.
    assert_eq!(conservation(&claims(), &record), Conservation::default());
    let stronger = ClaimsFile::parse(&CLAIMS.replace(
        r#"mutants = ["add-wraps"]"#,
        r#"mutants = ["add-wraps", "add-saturates"]"#,
    ))
    .expect("valid");
    assert!(conservation(&stronger, &record).weakened.is_empty());

    // Deleting the mutant (replaced by another) is refused...
    let swapped = CLAIMS.replace(
        r#"mutants = ["add-wraps"]"#,
        r#"mutants = ["add-saturates"]"#,
    );
    let found = conservation(&ClaimsFile::parse(&swapped).expect("valid"), &record);
    assert_eq!(found.weakened.len(), 1);
    assert_eq!(found.weakened[0].3, AmendKind::Mutant);
    assert_eq!(found.weakened[0].4, "add-wraps");
    let profile = crate::profile::Profile::embedded().expect("profile");
    let diags = conservation_diagnostics(&profile, &claims(), &found);
    assert!(
        diags
            .iter()
            .all(|d| d.id == "TAU-R002" && d.severity == Severity::Error)
    );

    // ... unless an amendment names it.
    let amended = ClaimsFile::parse(&format!("{swapped}{AMEND_MUTANT}")).expect("valid");
    assert_eq!(conservation(&amended, &record), Conservation::default());

    // A theorem dropped, or the whole claim, is refused too.
    let dropped = CLAIMS.replace(r#", "K.add_spec""#, "");
    let found = conservation(&ClaimsFile::parse(&dropped).expect("valid"), &record);
    assert_eq!(found.weakened.len(), 1);
    let mut gone = claims();
    gone.claim.clear();
    assert_eq!(conservation(&gone, &record).weakened.len(), 3);

    // Once re-recorded, the amendment excuses nothing and is reported.
    let rerecorded = committed(&ClaimsFile::parse(&swapped).expect("valid"));
    let found = conservation(&amended, &rerecorded);
    assert!(found.weakened.is_empty());
    assert_eq!(found.unneeded, vec![0]);

    // An amendment needs a reason.
    let silent = format!(
        "{swapped}{}",
        AMEND_MUTANT.replace("replaced by a stronger mutant", " ")
    );
    assert!(ClaimsFile::parse(&silent).is_err());
}

#[test]
fn assumptions_may_not_grow() {
    let v2 = CLAIMS.replace("version = 1", "version = 2");
    let base = ClaimsFile::parse(&v2).expect("valid");
    let record = committed(&base);
    let assumed = ClaimsFile::parse(&v2.replace(
        r#"mutants = ["add-wraps"]"#,
        "mutants = [\"add-wraps\"]\nassumes = [\"FS.rename-atomic\"]\nwitness = \"K.fs_holds\"",
    ))
    .expect("valid");
    let found = conservation(&assumed, &record);
    assert_eq!(found.weakened.len(), 1);
    assert_eq!(found.weakened[0].3, AmendKind::Assumption);
    // Removing an assumption is a strengthening.
    assert!(
        conservation(&base, &committed(&assumed))
            .weakened
            .is_empty()
    );
    let amend = r#"
[[amend]]
file = "crates/k/src/lib.rs"
function = "add"
kind = "assumption"
name = "FS.rename-atomic"
reason = "the claim now covers the durable path"
"#;
    let amended = ClaimsFile::parse(&format!(
        "{}{amend}",
        v2.replace(
            r#"mutants = ["add-wraps"]"#,
            "mutants = [\"add-wraps\"]\nassumes = [\"FS.rename-atomic\"]\nwitness = \"K.fs_holds\"",
        )
    ))
    .expect("valid");
    assert_eq!(conservation(&amended, &record), Conservation::default());
}

#[test]
fn a_new_claim_states_its_own_assumptions() {
    let v2 = CLAIMS.replace("version = 1", "version = 2");
    let record = committed(&ClaimsFile::parse(&v2).expect("valid"));
    let extra = format!(
        "{v2}\n[[claim]]\ncrate = \"k\"\nfile = \"crates/k/src/lib.rs\"\nfunction = \"sub\"\nlean = \"k.sub\"\ntheorems = [\"K.sub_spec\"]\nmutants = [\"sub-wraps\"]\nassumes = [\"A\"]\nwitness = \"K.a_holds\"\n"
    );
    let found = conservation(&ClaimsFile::parse(&extra).expect("valid"), &record);
    assert_eq!(found, Conservation::default());
}

/// A4: `cargo tauten proofs extract` passes only when every crate is
/// observed translating exactly once.
#[test]
fn extraction_is_admitted_crate_by_crate() {
    let crates = vec!["a".to_owned(), "b".to_owned()];
    let report = |lines: &[&str]| Report {
        events: lines.iter().map(|l| event(l)).collect(),
        ..Report::default()
    };
    let ok_a = r#"{"stage":"extract","status":"ok","crate":"a"}"#;
    let ok_b = r#"{"stage":"extract","status":"ok","crate":"b"}"#;
    let fail_b = r#"{"stage":"extract","status":"fail","crate":"b","detail":"b/src/lib.rs:3:1-4:2: Dynamic trait types are not supported yet"}"#;
    let (yes, seen) = extraction(&crates, &report(&[ok_a, ok_b]));
    assert!(yes);
    assert!(seen.iter().all(|(_, s)| *s == Extracted::Ok));
    let (no, seen) = extraction(&crates, &report(&[ok_a, fail_b]));
    assert!(!no);
    assert!(matches!(&seen[1].1, Extracted::Failed(why) if why.contains("b/src/lib.rs:3")));
    // A crate with no result, a repeated result, a missing tool or a gate
    // that never started is never a pass.
    assert!(!extraction(&crates, &report(&[ok_a])).0);
    assert!(!extraction(&crates, &report(&[ok_a, ok_b, ok_b])).0);
    let missing = r#"{"stage":"tools","status":"missing","detail":"no image"}"#;
    assert!(!extraction(&crates, &report(&[missing])).0);
    let mut never = report(&[ok_a, ok_b]);
    never.not_started = Some("cannot run".to_owned());
    assert!(!extraction(&crates, &never).0);
    assert!(!extraction(&[], &report(&[])).0);
}

/// #44: `proofs try` selects mutants by theorem or name and narrows each
/// affected claim to them.
#[test]
fn try_selects_mutants_by_theorem_and_name() {
    let two = format!(
        "{CLAIMS}\n[[claim]]\ncrate = \"k\"\nfile = \"crates/k/src/lib.rs\"\nfunction = \"sub\"\nlean = \"k.sub\"\ntheorems = [\"K.sub_spec\"]\nmutants = [\"sub-wraps\", \"sub-flips\"]\n"
    );
    let file = ClaimsFile::parse(&two).expect("valid");
    let (sel, affected) = try_selection(&file, &["K.sub_spec".to_owned()], &[]);
    assert_eq!(sel, ["sub-wraps", "sub-flips"]);
    assert_eq!(affected.len(), 1);
    assert_eq!(affected[0].function, "sub");
    let (sel, affected) = try_selection(&file, &[], &["sub-flips".to_owned()]);
    assert_eq!(sel, ["sub-flips"]);
    assert_eq!(affected[0].mutants, ["sub-flips"]);
    let (sel, affected) = try_selection(&file, &["K.nothing".to_owned()], &[]);
    assert!(sel.is_empty() && affected.is_empty());
}

/// #39: a mutant may declare the exact set of declarations whose own proof
/// must fail. A headline `H` uses a helper `L`.
#[test]
fn exact_direct_failures_are_admitted_as_exact_sets() {
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    // Expected {L}: {L} calibrates; {H} and {L, H} do not; nothing observed
    // is never a kill.
    assert!(exact_direct(&s(&["L"]), &s(&["L"])));
    assert!(!exact_direct(&s(&["L"]), &s(&["H"])));
    assert!(!exact_direct(&s(&["L"]), &s(&["L", "H"])));
    assert!(!exact_direct(&s(&["L"]), &s(&[])));
    // Expected {L, H}: both must fail directly.
    assert!(exact_direct(&s(&["L", "H"]), &s(&["H", "L"])));
    assert!(!exact_direct(&s(&["L", "H"]), &s(&["L"])));

    // Through a claim's outcome: with `direct = ["K.helper"]` the kill is
    // judged on the direct set, not on the claim's broken theorems.
    let c = claim();
    let mut report = with(
        &without(&passing(), "add-wraps"),
        r#"{"mutant":"add-wraps","status":"killed","broken":["K.add_spec"],"direct":["K.helper"],"dependent":["K.add_spec"]}"#,
    );
    assert_eq!(
        outcome(&c, &report).0,
        Outcome::Proved,
        "claim-level policy by default"
    );
    report
        .expected_direct
        .insert("add-wraps".to_owned(), s(&["K.helper"]));
    assert_eq!(outcome(&c, &report).0, Outcome::Proved);
    report
        .expected_direct
        .insert("add-wraps".to_owned(), s(&["K.helper", "K.add_spec"]));
    assert_eq!(outcome(&c, &report).0, Outcome::MutantSurvived);
    // An infrastructure failure is not a kill, whatever it reports.
    let mut broken_build = with(
        &without(&passing(), "add-wraps"),
        r#"{"mutant":"add-wraps","status":"error","broken":[],"direct":["K.helper"]}"#,
    );
    broken_build
        .expected_direct
        .insert("add-wraps".to_owned(), s(&["K.helper"]));
    assert_eq!(outcome(&c, &broken_build).0, Outcome::MutantSurvived);
}

/// A7b: an inherited claim is never stronger than the dependency's, and a
/// version mismatch or a missing record establishes nothing.
#[test]
fn inheriting_never_strengthens_a_claim() {
    let mut mine = claim();
    mine.theorems.clear();
    mine.mutants.clear();
    mine.inherit = Some("dep@1.2.3:apply".to_owned());
    let mut theirs = claim();
    theirs.function = "apply".to_owned();
    theirs.assumes = vec!["FS.sync-file".to_owned()];
    let shipped = Record {
        version: 1,
        generated_by: "cargo tauten proofs export".to_owned(),
        claims: vec![ClaimRecord {
            claim: theirs,
            outcome: Outcome::Proved,
            badge: Badge::ProvedUnder,
            detail: None,
            fingerprint: String::new(),
            inputs: BTreeMap::new(),
            mutant_results: Vec::new(),
        }],
        commit: None,
    };
    let (record, outcome, badge, _) = inherited(&mine, Some("1.2.3"), Some(&shipped));
    assert_eq!((outcome, badge), (Outcome::Proved, Badge::ProvedUnder));
    assert_eq!(record.assumes, vec!["FS.sync-file".to_owned()]);
    let (_, _, badge, why) = inherited(&mine, Some("1.2.4"), Some(&shipped));
    assert_eq!(badge, Badge::NotEstablished);
    assert!(why.is_some_and(|w| w.contains("locked at 1.2.4")));
    assert_eq!(
        inherited(&mine, Some("1.2.3"), None).2,
        Badge::NotEstablished
    );
    assert_eq!(
        inherited(&mine, None, Some(&shipped)).2,
        Badge::NotEstablished
    );
}

#[test]
fn a_run_passes_only_when_every_claim_of_a_nonempty_set_is_proved() {
    assert!(!records_pass(&[]), "an empty inventory never passes (#51)");
    assert!(records_pass(&[record(Outcome::Proved)]));
    assert!(records_pass(&[
        record(Outcome::Proved),
        record(Outcome::Proved)
    ]));
    assert!(!records_pass(&[
        record(Outcome::Proved),
        record(Outcome::ToolMissing)
    ]));
    assert!(!records_pass(&[record(Outcome::Drift)]));
}

#[test]
fn only_a_gate_that_reached_a_proof_stage_started() {
    let only = |json: &[&str]| Report {
        events: json.iter().map(|j| event(j)).collect(),
        ..Report::default()
    };
    assert!(!Report::default().started(), "printed nothing");
    let not_started = Report {
        not_started: Some("no such command".to_owned()),
        ..passing()
    };
    assert!(!not_started.started());
    assert!(!only(&[r#"{"stage":"tools","status":"missing"}"#]).started());
    assert!(only(&[r#"{"stage":"tools","status":"ok"}"#]).started());
    // A real failure after the tools stage is a result, not an attempt.
    assert!(
        only(&[
            r#"{"stage":"tools","status":"ok"}"#,
            r#"{"stage":"build","status":"fail"}"#
        ])
        .started()
    );
    assert!(passing().started());
}

#[test]
fn a_pending_merge_record_is_never_a_result() {
    let pending = r#"{"version":2,"state":"pending-merge","parents":["a","b"],"registry":[]}"#;
    // A v1 reader refuses it rather than reading an empty success.
    assert!(serde_json::from_str::<Record>(pending).is_err());
    let parsed = RecordFile::parse(pending).expect("pending");
    assert!(parsed.result().is_none());
    assert!(matches!(parsed, RecordFile::Pending(_)));
    for bad in [
        r#"{"version":2,"state":"pending-merge","parents":["a","a"],"registry":[]}"#,
        r#"{"version":2,"state":"pending-merge","parents":["a"],"registry":[]}"#,
        r#"{"version":2,"state":"pending-merge","parents":["a","b"],"registry":[],"claims":[]}"#,
        r#"{"version":3,"state":"pending-merge","parents":["a","b"],"registry":[]}"#,
        r#"{"version":2,"state":"proved","parents":["a","b"],"registry":[]}"#,
    ] {
        assert!(RecordFile::parse(bad).is_err(), "{bad}");
    }
    let v1 = r#"{"version":1,"generated_by":"cargo tauten proofs","claims":[]}"#;
    assert!(RecordFile::parse(v1).expect("v1").result().is_some());
}

/// A10 (#14): every versioned file is admitted by the kernel's
/// `version::admit`; a file from a newer tauten is refused, never read.
#[test]
fn newer_or_missing_versions_are_refused() {
    let newer = CLAIMS.replacen("version = 1", "version = 3", 1);
    assert_ne!(newer, CLAIMS);
    let error = ClaimsFile::parse(&newer).expect_err("newer claims.toml");
    assert!(
        error.to_string().contains("newer than this tauten reads"),
        "{error}"
    );
    let none = CLAIMS.replacen("version = 1", "", 1);
    let error = ClaimsFile::parse(&none).expect_err("no version");
    assert!(error.to_string().contains("no format version"), "{error}");

    let v2 = r#"{"version":2,"generated_by":"cargo tauten proofs","claims":[]}"#;
    assert!(Record::parse(v2).expect_err("newer").contains("newer"));
    assert!(RecordFile::parse(v2).expect_err("newer").contains("newer"));
    let v0 = r#"{"version":0,"generated_by":"cargo tauten proofs","claims":[]}"#;
    assert!(Record::parse(v0).expect_err("older").contains("older"));
    let missing = r#"{"generated_by":"cargo tauten proofs","claims":[]}"#;
    assert!(
        Record::parse(missing)
            .expect_err("missing")
            .contains("no format version")
    );
    let pending = r#"{"version":3,"state":"pending-merge","parents":["a","b"],"registry":[]}"#;
    assert!(
        RecordFile::parse(pending)
            .expect_err("newer")
            .contains("newer")
    );
}

#[test]
fn both_parents_must_conserve() {
    let fine = Conservation::default();
    let weak = Conservation {
        weakened: vec![(
            "k".to_owned(),
            "f.rs".to_owned(),
            "f".to_owned(),
            AmendKind::Theorem,
            "t".to_owned(),
        )],
        ..Conservation::default()
    };
    assert!(parents_conserved(&[fine.clone(), fine.clone()]));
    assert!(!parents_conserved(&[fine.clone(), weak.clone()]));
    assert!(!parents_conserved(&[weak, fine.clone()]));
    assert!(
        !parents_conserved(std::slice::from_ref(&fine)),
        "a missing parent never passes"
    );
    assert!(!parents_conserved(&[]));
}
