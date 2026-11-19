//! Whether a function's declared effects match its signature (TAU-F001).
//!
//! The shell reads each `pub fn` that carries `#[effects(..)]` (or must, in
//! a crate with `effects = "required"`), classifies every parameter type as
//! one [`Source`] (expanding `#[derive(Cap)]` bundles into their fields),
//! and calls [`derive`] for the row the signature grants and [`agrees`] /
//! [`missing`] / [`extra`] to decide the finding. The decision is proved:
//! a finding is reported exactly when the declared and derived rows differ,
//! and it names exactly the differing labels.

/// What a parameter grants, as classified by the shell from its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A clock (`&impl Clock`, `SystemClock`, `ManualClock`).
    Clock,
    /// The environment (`Env`).
    Env,
    /// A writable directory (`Dir`): read and write.
    Dir,
    /// A read-only directory (`ReadDir`).
    ReadDir,
    /// A network capability.
    Net,
    /// Anything else: plain data grants nothing.
    Plain,
}

/// An effect row: one flag per label of `tauten::effects::LABELS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// `clock`
    pub clock: bool,
    /// `env`
    pub env: bool,
    /// `fs_read`
    pub fs_read: bool,
    /// `fs_write`
    pub fs_write: bool,
    /// `net`
    pub net: bool,
    /// `fail`
    pub fail: bool,
    /// `suspend`
    pub suspend: bool,
}

/// The row of a function with no capability, no `Result` and no `async`.
#[must_use]
pub const fn empty() -> Row {
    Row {
        clock: false,
        env: false,
        fs_read: false,
        fs_write: false,
        net: false,
        fail: false,
        suspend: false,
    }
}

/// `row` widened by what `source` grants.
#[must_use]
pub const fn add(row: Row, source: Source) -> Row {
    let (clock, env, fs_read, fs_write, net) = match source {
        Source::Clock => (true, false, false, false, false),
        Source::Env => (false, true, false, false, false),
        Source::Dir => (false, false, true, true, false),
        Source::ReadDir => (false, false, true, false, false),
        Source::Net => (false, false, false, false, true),
        Source::Plain => (false, false, false, false, false),
    };
    Row {
        clock: row.clock || clock,
        env: row.env || env,
        fs_read: row.fs_read || fs_read,
        fs_write: row.fs_write || fs_write,
        net: row.net || net,
        fail: row.fail,
        suspend: row.suspend,
    }
}

/// `row` widened by the source at position `i`, if there is one.
#[must_use]
pub fn add_at(row: Row, sources: &[Source], i: usize) -> Row {
    match sources.get(i) {
        Some(source) => add(row, *source),
        None => row,
    }
}

/// The row a signature grants: the union of its parameters' sources, plus
/// `fail` for a `Result` return and `suspend` for an `async fn`.
#[must_use]
pub fn derive(sources: &[Source], fallible: bool, asynchronous: bool) -> Row {
    let start = empty();
    let mut row = Row {
        clock: start.clock,
        env: start.env,
        fs_read: start.fs_read,
        fs_write: start.fs_write,
        net: start.net,
        fail: fallible,
        suspend: asynchronous,
    };
    let mut i: usize = 0;
    while i < sources.len() {
        row = add_at(row, sources, i);
        i = i.saturating_add(1);
    }
    row
}

/// Labels the signature grants but the declaration leaves out: the
/// function can do more than it says.
#[must_use]
pub const fn missing(declared: Row, derived: Row) -> Row {
    Row {
        clock: derived.clock && !declared.clock,
        env: derived.env && !declared.env,
        fs_read: derived.fs_read && !declared.fs_read,
        fs_write: derived.fs_write && !declared.fs_write,
        net: derived.net && !declared.net,
        fail: derived.fail && !declared.fail,
        suspend: derived.suspend && !declared.suspend,
    }
}

/// Labels declared without anything in the signature that grants them.
#[must_use]
pub const fn extra(declared: Row, derived: Row) -> Row {
    missing(derived, declared)
}

/// Whether a row has no label.
#[must_use]
pub const fn is_empty(row: Row) -> bool {
    !(row.clock || row.env || row.fs_read || row.fs_write || row.net || row.fail || row.suspend)
}

/// Whether the declaration matches the signature: no label missing, none
/// extra. TAU-F001 reports a finding exactly when this is false.
#[must_use]
pub const fn agrees(declared: Row, derived: Row) -> bool {
    is_empty(missing(declared, derived)) && is_empty(extra(declared, derived))
}

/// What TAU-F001 reports for one function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing to report.
    Silent,
    /// A finding that does not block: the crate does not require effects,
    /// or a parameter type could not be classified.
    Warning,
    /// A blocking finding.
    Error,
}

/// The finding for one function.
///
/// * `declared`: the row of its `#[effects(..)]`, or `None` without one.
/// * `derived`: [`derive`] of its signature.
/// * `required`: the crate sets `effects = "required"`.
/// * `resolved`: every parameter type was classified; an unclassified type
///   (a bundle from another crate, an unbounded generic) may grant
///   anything, so a mismatch is then only a warning.
///
/// Without a declaration a function is reported only when the crate
/// requires one, and then always as an error: the absence does not depend
/// on classification.
#[must_use]
pub const fn verdict(
    declared: Option<Row>,
    derived: Row,
    required: bool,
    resolved: bool,
) -> Verdict {
    match declared {
        None => {
            if required {
                Verdict::Error
            } else {
                Verdict::Silent
            }
        }
        Some(d) => {
            if agrees(d, derived) {
                Verdict::Silent
            } else if required && resolved {
                Verdict::Error
            } else {
                Verdict::Warning
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows() {
        let r = derive(
            &[Source::Clock, Source::Plain, Source::ReadDir],
            true,
            false,
        );
        assert!(r.clock && r.fs_read && !r.fs_write && r.fail && !r.suspend && !r.env);
        let declared = Row {
            fs_write: true,
            ..r
        };
        assert!(!agrees(declared, r));
        assert!(extra(declared, r).fs_write && is_empty(missing(declared, r)));
        assert!(agrees(r, r));
        assert!(derive(&[Source::Dir], false, false).fs_write);
        assert!(is_empty(derive(&[], false, false)));
        assert_eq!(verdict(Some(r), r, true, false), Verdict::Silent);
        assert_eq!(verdict(Some(declared), r, true, true), Verdict::Error);
        assert_eq!(verdict(Some(declared), r, true, false), Verdict::Warning);
        assert_eq!(verdict(Some(declared), r, false, true), Verdict::Warning);
        assert_eq!(verdict(None, r, true, false), Verdict::Error);
        assert_eq!(verdict(None, r, false, true), Verdict::Silent);
    }
}
