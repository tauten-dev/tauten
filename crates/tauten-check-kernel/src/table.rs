//! Well-formedness of the rule table, over interned names.
//!
//! `tauten_check::profile` interns every rule ID it reads (the rules' own
//! IDs, the rules API bans name, and the IDs the checker emits) to one index
//! per distinct string, and every lint name to one index per distinct
//! string, and calls [`validate`]. A table is well formed when no two rules
//! share an ID, no lint is owned twice (by two rules, or twice by one), every
//! API ban names an existing rule, and every rule the checker emits exists.
//!
//! The code is written for translation and for the kernel profile: `while`
//! loops over `get`, saturating arithmetic, and no indexing. The module is
//! not called `profile` so that no parameter shares its name.

/// Why a rule table is rejected. Positions refer to the slices given to
/// [`validate`].
pub enum Invalid {
    /// The rule at this position repeats the ID of an earlier rule.
    DuplicateId(usize),
    /// The lint at the second position (of the flattened lint list) is the
    /// lint at the first, earlier position.
    DuplicateLint(usize, usize),
    /// The API ban at this position names no rule of the table.
    UnknownRule(usize),
    /// The required ID at this position is not the ID of a rule.
    MissingRule(usize),
}

/// Whether `names` has `name` at position `i`.
#[must_use]
pub fn is_at(names: &[usize], i: usize, name: usize) -> bool {
    match names.get(i) {
        Some(here) => *here == name,
        None => false,
    }
}

/// The first position before `end` where `names` has `name`.
#[must_use]
pub fn find_before(names: &[usize], end: usize, name: usize) -> Option<usize> {
    let mut i: usize = 0;
    while i < end {
        if is_at(names, i, name) {
            return Some(i);
        }
        i = i.saturating_add(1);
    }
    None
}

/// Whether `names` contains `name`.
#[must_use]
#[expect(
    clippy::redundant_pattern_matching,
    reason = "a plain match translates to Lean; `Option::is_some` is avoided in kernel code"
)]
pub fn contains(names: &[usize], name: usize) -> bool {
    match find_before(names, names.len(), name) {
        Some(_) => true,
        None => false,
    }
}

/// The earlier position holding the same name as position `j`, if any.
#[must_use]
pub fn repeat_at(names: &[usize], j: usize) -> Option<usize> {
    match names.get(j) {
        Some(name) => find_before(names, j, *name),
        None => None,
    }
}

/// The first position `j` whose name occurs earlier, at `i`: `(i, j)`.
#[must_use]
pub fn first_duplicate(names: &[usize]) -> Option<(usize, usize)> {
    let mut j: usize = 0;
    while j < names.len() {
        if let Some(i) = repeat_at(names, j) {
            return Some((i, j));
        }
        j = j.saturating_add(1);
    }
    None
}

/// Whether `names` lacks `name`.
#[must_use]
pub fn absent(names: &[usize], name: usize) -> bool {
    !contains(names, name)
}

/// Whether the name at position `j` of `wanted` is absent from `present`.
#[must_use]
pub fn missing_at(present: &[usize], wanted: &[usize], j: usize) -> bool {
    match wanted.get(j) {
        Some(name) => absent(present, *name),
        None => false,
    }
}

/// The first position of `wanted` whose name is absent from `present`.
#[must_use]
pub fn first_missing(present: &[usize], wanted: &[usize]) -> Option<usize> {
    let mut j: usize = 0;
    while j < wanted.len() {
        if missing_at(present, wanted, j) {
            return Some(j);
        }
        j = j.saturating_add(1);
    }
    None
}

/// The rule owning lint `k` (0 for a position past the end, which the
/// shell never passes: it gives one owner per lint).
#[must_use]
pub fn owner_at(lint_rules: &[usize], k: usize) -> usize {
    match lint_rules.get(k) {
        Some(rule) => *rule,
        None => 0,
    }
}

/// The first duplicate ID or lint, in the order the table is read: rule by
/// rule, its ID before its lints.
///
/// `lint_rules[k]` is the position of the rule owning lint `k` of the
/// flattened lint list `lints` (non-decreasing, as the shell flattens the
/// rules in order).
#[must_use]
pub fn duplicate(ids: &[usize], lint_rules: &[usize], lints: &[usize]) -> Option<Invalid> {
    let lint = first_duplicate(lints);
    if let Some((_, rule)) = first_duplicate(ids) {
        return match lint {
            Some((first, second)) if owner_at(lint_rules, second) < rule => {
                Some(Invalid::DuplicateLint(first, second))
            }
            Some(_) | None => Some(Invalid::DuplicateId(rule)),
        };
    }
    if let Some((first, second)) = lint {
        return Some(Invalid::DuplicateLint(first, second));
    }
    None
}

/// Validates an interned rule table.
///
/// `ids` holds each rule's ID, `lint_rules` and `lints` the flattened lint
/// list (owning rule position, lint name), `api_rules` the rule each API ban
/// names, `required` the IDs the checker emits.
///
/// # Errors
/// The first problem in the order the table is read: duplicate IDs and
/// lints rule by rule, then API bans naming unknown rules, then missing
/// required IDs.
pub fn validate(
    ids: &[usize],
    lint_rules: &[usize],
    lints: &[usize],
    api_rules: &[usize],
    required: &[usize],
) -> Result<(), Invalid> {
    if let Some(invalid) = duplicate(ids, lint_rules, lints) {
        return Err(invalid);
    }
    if let Some(api) = first_missing(ids, api_rules) {
        return Err(Invalid::UnknownRule(api));
    }
    match first_missing(ids, required) {
        Some(id) => Err(Invalid::MissingRule(id)),
        None => Ok(()),
    }
}
