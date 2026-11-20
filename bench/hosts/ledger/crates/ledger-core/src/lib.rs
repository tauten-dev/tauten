//! Parsing ledger text and computing the report.

use core::fmt;
use std::io::Read;

use ledger_kernel::{Entry, Rejection, apply};

/// A line-level failure.
#[derive(Debug, PartialEq, Eq)]
pub struct LineError {
    /// 1-based line number.
    pub line: usize,
    /// What went wrong.
    pub reason: Reason,
}

/// What went wrong on a line.
#[derive(Debug, PartialEq, Eq)]
pub enum Reason {
    /// The first word is not a known entry kind.
    UnknownKind(String),
    /// The amount is missing or not a whole non-negative number.
    BadAmount(String),
    /// Extra words after the amount.
    TrailingInput,
    /// The kernel refused the entry.
    Rejected(Rejection),
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.reason {
            Reason::UnknownKind(kind) => write!(f, "unknown entry kind `{kind}`"),
            Reason::BadAmount(text) => write!(f, "bad amount `{text}`"),
            Reason::TrailingInput => f.write_str("unexpected text after the amount"),
            Reason::Rejected(Rejection::Insufficient) => f.write_str("insufficient funds"),
            Reason::Rejected(Rejection::Overflow) => f.write_str("balance overflow"),
        }
    }
}

/// Reading or processing the ledger failed.
#[derive(Debug)]
pub enum LedgerError {
    /// The input could not be read.
    Io(std::io::Error),
    /// A line was invalid.
    Line(LineError),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "cannot read input: {e}"),
            Self::Line(e) => e.fmt(f),
        }
    }
}

/// The result of processing a ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Final balance.
    pub balance: u64,
    /// Number of entries applied.
    pub entries: usize,
}

/// An amount: one or more ASCII digits, nothing else (SPEC G2).
fn parse_amount(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn parse_line(line: usize, text: &str) -> Result<Option<Entry>, LineError> {
    let text = text.trim();
    if text.is_empty() || text.starts_with('#') {
        return Ok(None);
    }
    let fail = |reason| LineError { line, reason };
    let mut words = text.split_whitespace();
    let kind = words.next().unwrap_or_default();
    let amount_text = words.next().unwrap_or_default();
    let amount = parse_amount(amount_text).ok_or_else(|| fail(Reason::BadAmount(amount_text.to_owned())))?;
    if words.next().is_some() {
        return Err(fail(Reason::TrailingInput));
    }
    match kind {
        "deposit" => Ok(Some(Entry::Deposit(amount))),
        "withdraw" => Ok(Some(Entry::Withdraw(amount))),
        other => Err(fail(Reason::UnknownKind(other.to_owned()))),
    }
}

/// Process ledger text.
///
/// # Errors
/// Returns the first invalid line.
pub fn process_text(text: &str) -> Result<Report, LineError> {
    let mut report = Report { balance: 0, entries: 0 };
    for (index, line_text) in text.lines().enumerate() {
        let line = index.saturating_add(1);
        if let Some(entry) = parse_line(line, line_text)? {
            report.balance = apply(report.balance, entry).map_err(|r| LineError { line, reason: Reason::Rejected(r) })?;
            report.entries = report.entries.saturating_add(1);
        }
    }
    Ok(report)
}

/// Read and process a ledger.
///
/// # Errors
/// Returns an IO error or the first invalid line.
pub fn process(source: &mut impl Read) -> Result<Report, LedgerError> {
    let mut text = String::new();
    source.read_to_string(&mut text).map_err(LedgerError::Io)?;
    process_text(&text).map_err(LedgerError::Line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposits_and_withdrawals() {
        let report = process_text("deposit 100\n# note\n\nwithdraw 30\n").unwrap();
        assert_eq!(report, Report { balance: 70, entries: 2 });
    }

    #[test]
    fn overdraft_is_an_error() {
        let err = process_text("deposit 5\nwithdraw 6\n").unwrap_err();
        assert_eq!(err.to_string(), "line 2: insufficient funds");
    }

    #[test]
    fn bad_lines_are_errors() {
        assert_eq!(process_text("fly 3").unwrap_err().reason, Reason::UnknownKind("fly".into()));
        assert_eq!(process_text("deposit -3").unwrap_err().reason, Reason::BadAmount("-3".into()));
        assert_eq!(process_text("deposit +3").unwrap_err().reason, Reason::BadAmount("+3".into()));
        assert_eq!(process_text("deposit 3 now").unwrap_err().reason, Reason::TrailingInput);
    }
}
