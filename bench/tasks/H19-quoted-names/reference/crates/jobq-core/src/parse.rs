//! Parsing one script line into a command.

use core::fmt;

/// One script event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `submit <name> [priority <p>] [retry <ms>]`
    Submit {
        /// Job name.
        name: String,
        /// 0 to 255; default 0.
        priority: u8,
        /// Fixed retry delay after a failure, if any.
        retry: Option<u64>,
    },
    /// `start <name>`
    Start(String),
    /// `next`: start the first ready pending job in start order.
    Next,
    /// `finish <name>`
    Finish(String),
    /// `fail <name>`
    Fail(String),
    /// `tick <ms>`: time moves forward.
    Tick(u64),
    /// `at <ms>`: time is set.
    At(u64),
}

/// Why a line is not a valid event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxError {
    /// The first word is not an event.
    UnknownEvent(String),
    /// The event needs a job name.
    MissingName,
    /// The job name breaks the name rule.
    BadName(String),
    /// The event needs a number.
    MissingNumber,
    /// Not a valid number.
    BadNumber(String),
    /// Not a valid priority.
    BadPriority(String),
    /// A submit option that does not exist.
    UnknownOption(String),
    /// A submit option without its value.
    MissingValue(String),
    /// A submit option given twice.
    DuplicateOption(String),
    /// Words after a complete event.
    Unexpected(String),
    /// A malformed quoted name, or quotes outside a name.
    BadQuotedName,
    /// The line is longer than [`MAX_LINE`] bytes.
    LineTooLong,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(w) => write!(f, "unknown event `{w}`"),
            Self::MissingName => f.write_str("missing job name"),
            Self::BadName(w) => write!(f, "bad name `{w}`"),
            Self::MissingNumber => f.write_str("missing number"),
            Self::BadNumber(w) => write!(f, "bad number `{w}`"),
            Self::BadPriority(w) => write!(f, "bad priority `{w}`"),
            Self::UnknownOption(w) => write!(f, "unknown option `{w}`"),
            Self::MissingValue(w) => write!(f, "missing value for `{w}`"),
            Self::DuplicateOption(w) => write!(f, "duplicate option `{w}`"),
            Self::Unexpected(w) => write!(f, "unexpected `{w}`"),
            Self::BadQuotedName => f.write_str("bad quoted name"),
            Self::LineTooLong => f.write_str("line too long"),
        }
    }
}

/// Longest job name: bytes for a plain name, characters for a quoted one.
pub const MAX_NAME: usize = 32;

/// Longest script line, in bytes, without its line ending.
pub const MAX_LINE: usize = 4096;

/// One word of a line: plain text, or a quoted name with its escapes replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Word<'a> {
    Plain(&'a str),
    Quoted(String),
}


/// A job name: 1 to 32 ASCII letters, digits, `_` or `-`.
///
/// # Errors
/// Returns [`SyntaxError::BadName`] when the word breaks the rule.
pub fn parse_name(word: &str) -> Result<String, SyntaxError> {
    let ok = !word.is_empty()
        && word.len() <= MAX_NAME
        && word.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok { Ok(word.to_owned()) } else { Err(SyntaxError::BadName(word.to_owned())) }
}

/// A number: one or more ASCII digits that fit in a `u64`.
///
/// # Errors
/// Returns [`SyntaxError::BadNumber`] otherwise.
pub fn parse_number(word: &str) -> Result<u64, SyntaxError> {
    if word.is_empty() || !word.bytes().all(|b| b.is_ascii_digit()) {
        return Err(SyntaxError::BadNumber(word.to_owned()));
    }
    word.parse().map_err(|_e: core::num::ParseIntError| SyntaxError::BadNumber(word.to_owned()))
}

fn parse_priority(word: &str) -> Result<u8, SyntaxError> {
    let bad = || SyntaxError::BadPriority(word.to_owned());
    let n = parse_number(word).map_err(|_e| bad())?;
    u8::try_from(n).map_err(|_e| bad())
}

/// Split a line into words; a word starting with `"` is a quoted name.
fn split_words(text: &str) -> Result<Vec<Word<'_>>, SyntaxError> {
    let mut words = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        if let Some(body) = rest.strip_prefix('"') {
            let (name, tail) = quoted(body)?;
            words.push(Word::Quoted(name));
            rest = tail.trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let (word, tail) = rest.split_at_checked(end).unwrap_or((rest, ""));
            words.push(Word::Plain(word));
            rest = tail.trim_start();
        }
    }
    Ok(words)
}

/// The quoted name at the start of `body` (just after the opening quote), and the text after it.
fn quoted(body: &str) -> Result<(String, &str), SyntaxError> {
    let bad = SyntaxError::BadQuotedName;
    let mut name = String::new();
    let mut chars = body.char_indices();
    let end = loop {
        match chars.next() {
            None => return Err(bad),
            Some((index, '"')) => break index,
            Some((_, '\\')) => match chars.next() {
                Some((_, escaped @ ('"' | '\\'))) => name.push(escaped),
                Some(_) | None => return Err(bad),
            },
            Some((_, c)) if c.is_control() => return Err(bad),
            Some((_, c)) => name.push(c),
        }
    };
    let tail = body.split_at_checked(end).and_then(|(_, t)| t.strip_prefix('"')).ok_or(SyntaxError::BadQuotedName)?;
    let length = name.chars().count();
    if length == 0 || length > MAX_NAME || !(tail.is_empty() || tail.starts_with(char::is_whitespace)) {
        return Err(SyntaxError::BadQuotedName);
    }
    Ok((name, tail))
}

fn plain(word: Word<'_>) -> Result<&str, SyntaxError> {
    match word {
        Word::Plain(text) => Ok(text),
        Word::Quoted(_) => Err(SyntaxError::BadQuotedName),
    }
}

fn name_arg<'a>(words: &mut impl Iterator<Item = Word<'a>>) -> Result<String, SyntaxError> {
    match words.next() {
        None => Err(SyntaxError::MissingName),
        Some(Word::Plain(text)) => parse_name(text),
        Some(Word::Quoted(name)) => Ok(name),
    }
}

fn number_arg<'a>(words: &mut impl Iterator<Item = Word<'a>>) -> Result<u64, SyntaxError> {
    words.next().ok_or(SyntaxError::MissingNumber).and_then(plain).and_then(parse_number)
}

fn submit<'a>(words: &mut impl Iterator<Item = Word<'a>>) -> Result<Command, SyntaxError> {
    let name = name_arg(words)?;
    let mut priority = None;
    let mut retry = None;
    while let Some(option) = words.next() {
        let option = plain(option)?;
        let value = plain(words.next().ok_or_else(|| SyntaxError::MissingValue(option.to_owned()))?)?;
        let duplicate = || SyntaxError::DuplicateOption(option.to_owned());
        match option {
            "priority" => {
                if priority.replace(parse_priority(value)?).is_some() {
                    return Err(duplicate());
                }
            }
            "retry" => {
                if retry.replace(parse_number(value)?).is_some() {
                    return Err(duplicate());
                }
            }
            other => return Err(SyntaxError::UnknownOption(other.to_owned())),
        }
    }
    Ok(Command::Submit { name, priority: priority.unwrap_or(0), retry })
}

/// Parse one line (without its line ending). Blank lines and `#` comments give `None`.
///
/// # Errors
/// Returns the [`SyntaxError`] for an invalid line.
pub fn parse_line(text: &str) -> Result<Option<Command>, SyntaxError> {
    if text.len() > MAX_LINE {
        return Err(SyntaxError::LineTooLong);
    }
    let text = text.trim();
    if text.is_empty() || text.starts_with('#') {
        return Ok(None);
    }
    let mut words = split_words(text)?.into_iter();
    let event = match words.next() {
        Some(word) => plain(word)?,
        None => return Ok(None),
    };
    let command = match event {
        "submit" => return submit(&mut words).map(Some),
        "start" => Command::Start(name_arg(&mut words)?),
        "next" => Command::Next,
        "finish" => Command::Finish(name_arg(&mut words)?),
        "fail" => Command::Fail(name_arg(&mut words)?),
        "tick" => Command::Tick(number_arg(&mut words)?),
        "at" => Command::At(number_arg(&mut words)?),
        other => return Err(SyntaxError::UnknownEvent(other.to_owned())),
    };
    match words.next() {
        Some(Word::Plain(extra)) => Err(SyntaxError::Unexpected(extra.to_owned())),
        Some(Word::Quoted(extra)) => Err(SyntaxError::Unexpected(extra)),
        None => Ok(Some(command)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events() {
        assert_eq!(parse_line("  # note").unwrap(), None);
        assert_eq!(parse_line("tick 15").unwrap(), Some(Command::Tick(15)));
        assert_eq!(
            parse_line("submit build retry 50 priority 7").unwrap(),
            Some(Command::Submit { name: "build".into(), priority: 7, retry: Some(50) })
        );
    }

    #[test]
    fn syntax_errors() {
        assert_eq!(parse_line("fly x").unwrap_err(), SyntaxError::UnknownEvent("fly".into()));
        assert_eq!(parse_line("tick +5").unwrap_err(), SyntaxError::BadNumber("+5".into()));
        assert_eq!(parse_line("submit a priority 256").unwrap_err(), SyntaxError::BadPriority("256".into()));
        assert_eq!(parse_line("submit a.b").unwrap_err(), SyntaxError::BadName("a.b".into()));
        assert_eq!(parse_line("start a b").unwrap_err(), SyntaxError::Unexpected("b".into()));
        assert_eq!(parse_line("submit a retry").unwrap_err(), SyntaxError::MissingValue("retry".into()));
    }

    #[test]
    fn quoted_names() {
        assert_eq!(parse_line(r#"start "a \"b\" \\""#).unwrap(), Some(Command::Start("a \"b\" \\".into())));
        assert_eq!(parse_line(r#"start "a"#).unwrap_err(), SyntaxError::BadQuotedName);
        assert_eq!(parse_line(r#"start """#).unwrap_err(), SyntaxError::BadQuotedName);
        assert_eq!(parse_line(&"#".repeat(4097)).unwrap_err(), SyntaxError::LineTooLong);
    }
}
