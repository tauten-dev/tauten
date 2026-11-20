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

/// One applied entry, as a statement row shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 1-based line number.
    pub line: usize,
    /// The entry kind as written in the file.
    pub kind: &'static str,
    /// The entry's amount.
    pub amount: u64,
    /// The balance after the entry.
    pub balance: u64,
    /// The memo as shown: at most 16 characters (SPEC G3).
    pub memo: String,
}

/// The memo as a statement shows it: longer than 16 characters becomes the
/// first 15 followed by `…`.
fn shown_memo(memo: &str) -> String {
    if memo.chars().count() > 16 {
        let mut shown: String = memo.chars().take(15).collect();
        shown.push('\u{2026}');
        shown
    } else {
        memo.to_owned()
    }
}

/// An amount: one or more ASCII digits, nothing else (SPEC G2).
fn parse_amount(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn parse_line(line: usize, text: &str) -> Result<Option<(Entry, &str)>, LineError> {
    let text = text.trim();
    if text.is_empty() || text.starts_with('#') {
        return Ok(None);
    }
    let (text, memo) = text.split_once(';').unwrap_or((text, ""));
    let fail = |reason| LineError { line, reason };
    let mut words = text.split_whitespace();
    let kind = words.next().unwrap_or_default();
    let amount_text = words.next().unwrap_or_default();
    let amount = parse_amount(amount_text).ok_or_else(|| fail(Reason::BadAmount(amount_text.to_owned())))?;
    if words.next().is_some() {
        return Err(fail(Reason::TrailingInput));
    }
    match kind {
        "deposit" => Ok(Some((Entry::Deposit(amount), memo.trim()))),
        "withdraw" => Ok(Some((Entry::Withdraw(amount), memo.trim()))),
        other => Err(fail(Reason::UnknownKind(other.to_owned()))),
    }
}

/// Process ledger text.
///
/// # Errors
/// Returns the first invalid line.
pub fn process_text(text: &str) -> Result<Report, LineError> {
    process_rows(text).map(|(report, _rows)| report)
}

/// Process ledger text, keeping one statement row per entry.
///
/// # Errors
/// Returns the first invalid line.
pub fn process_rows(text: &str) -> Result<(Report, Vec<Row>), LineError> {
    let mut report = Report { balance: 0, entries: 0 };
    let mut rows = Vec::new();
    for (index, line_text) in text.lines().enumerate() {
        let line = index.saturating_add(1);
        if let Some((entry, memo)) = parse_line(line, line_text)? {
            report.balance = apply(report.balance, entry).map_err(|r| LineError { line, reason: Reason::Rejected(r) })?;
            report.entries = report.entries.saturating_add(1);
            let (kind, amount) = match entry {
                Entry::Deposit(amount) => ("deposit", amount),
                Entry::Withdraw(amount) => ("withdraw", amount),
            };
            rows.push(Row { line, kind, amount, balance: report.balance, memo: shown_memo(memo) });
        }
    }
    Ok((report, rows))
}

/// Read and process a ledger, keeping the statement rows.
///
/// # Errors
/// Returns an IO error or the first invalid line.
pub fn statement(source: &mut impl Read) -> Result<(Report, Vec<Row>), LedgerError> {
    let mut text = String::new();
    source.read_to_string(&mut text).map_err(LedgerError::Io)?;
    process_rows(&text).map_err(LedgerError::Line)
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
    fn memos_are_cut_to_sixteen_characters() {
        let (_, rows) = process_rows("deposit 1 ; abcdefghijklmnopq\ndeposit 1 ;abcdefghijklmnop\n").unwrap();
        assert_eq!(rows[0].memo, "abcdefghijklmno\u{2026}");
        assert_eq!(rows[1].memo, "abcdefghijklmnop");
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
