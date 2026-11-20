//! The line grammar and the value grammar.

use kvconf_kernel::{Key, Value};

use crate::Reason;

/// One meaningful line of a configuration file.
#[derive(Debug, PartialEq, Eq)]
pub enum Line<'a> {
    /// Empty or comment.
    Blank,
    /// `[section]`
    Section(Key),
    /// `key = value`; `key_text` is the key as written.
    Assign {
        /// The key as written on the line.
        key_text: &'a str,
        /// The parsed key.
        key: Key,
        /// The value text after `=`, trimmed.
        value_text: &'a str,
    },
}

/// Classify one line (without its line terminator).
///
/// # Errors
/// The reason the line is invalid. The value is not parsed here.
pub fn line(text: &str) -> Result<Line<'_>, Reason> {
    let text = text.trim();
    if text.is_empty() || text.starts_with('#') {
        return Ok(Line::Blank);
    }
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        let name = inner.trim();
        return Key::parse(name).map(Line::Section).ok_or_else(|| Reason::BadSection(name.to_owned()));
    }
    let Some((key_text, value_text)) = text.split_once('=') else {
        return Err(Reason::ExpectedAssignment);
    };
    let key_text = key_text.trim();
    let key = Key::parse(key_text).ok_or_else(|| Reason::BadKey(key_text.to_owned()))?;
    Ok(Line::Assign { key_text, key, value_text: value_text.trim() })
}

/// A cursor over the remaining text of a value.
struct Cursor<'a> {
    rest: &'a str,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<char> {
        self.rest.chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let mut chars = self.rest.chars();
        let c = chars.next();
        self.rest = chars.as_str();
        c
    }

    fn skip_spaces(&mut self) {
        self.rest = self.rest.trim_start();
    }

    /// Text up to (not including) the first character in `stops`, trimmed.
    fn token(&mut self, stops: &[char]) -> &'a str {
        let end = self.rest.find(stops).unwrap_or(self.rest.len());
        let (token, rest) = self.rest.split_at_checked(end).unwrap_or((self.rest, ""));
        self.rest = rest;
        token.trim()
    }
}

/// Parse the body of a quoted string; the cursor is just after the opening quote.
fn string_body(cur: &mut Cursor<'_>) -> Result<String, Reason> {
    let mut out = String::new();
    loop {
        match cur.bump() {
            None => return Err(Reason::Unterminated),
            Some('"') => return Ok(out),
            Some('\\') => match cur.bump() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => return Err(Reason::BadEscape(other)),
                None => return Err(Reason::Unterminated),
            },
            Some(c) => out.push(c),
        }
    }
}

/// An integer: optional `-`, then one or more ASCII digits (SPEC G1, G2).
/// `Ok(None)` if the text is not integer-shaped.
///
/// # Errors
/// [`Reason::OutOfRange`] if it is integer-shaped but not an `i64`.
pub fn integer(text: &str) -> Result<Option<i64>, Reason> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(None);
    }
    text.parse::<i64>().map(Some).map_err(|_e| Reason::OutOfRange)
}

/// A scalar given as a bare token: `true`, `false` or an integer.
fn bare_scalar(token: &str) -> Result<Value, Reason> {
    match token {
        "true" => Ok(Value::Bool(true)),
        "false" => Ok(Value::Bool(false)),
        _ => integer(token)?.map(Value::Int).ok_or_else(|| Reason::BadValue(token.to_owned())),
    }
}

fn list_body(cur: &mut Cursor<'_>, whole: &str) -> Result<Value, Reason> {
    let bad = || Reason::BadValue(whole.to_owned());
    let mut items = Vec::new();
    cur.skip_spaces();
    if cur.peek() == Some(']') {
        cur.bump();
        return Ok(Value::List(items));
    }
    loop {
        cur.skip_spaces();
        let item = match cur.peek() {
            Some('"') => {
                cur.bump();
                Value::Str(string_body(cur)?)
            }
            Some('[') | None => return Err(bad()),
            Some(_) => {
                let token = cur.token(&[',', ']']);
                if token.is_empty() {
                    return Err(bad());
                }
                bare_scalar(token)?
            }
        };
        items.push(item);
        cur.skip_spaces();
        match cur.bump() {
            Some(',') => {}
            Some(']') => return Ok(Value::List(items)),
            Some(_) | None => return Err(bad()),
        }
    }
}

/// Parse a value: a quoted string, `true`/`false`, an integer, or a list of
/// those in `[...]` separated by commas.
///
/// # Errors
/// Why the text is not a value.
pub fn value(text: &str) -> Result<Value, Reason> {
    let text = text.trim();
    let mut cur = Cursor { rest: text };
    let parsed = match cur.peek() {
        Some('"') => {
            cur.bump();
            Value::Str(string_body(&mut cur)?)
        }
        Some('[') => {
            cur.bump();
            list_body(&mut cur, text)?
        }
        Some(_) => return bare_scalar(text),
        None => return Err(Reason::BadValue(String::new())),
    };
    if cur.rest.trim().is_empty() { Ok(parsed) } else { Err(Reason::TrailingText) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        assert_eq!(value("-12"), Ok(Value::Int(-12)));
        assert_eq!(value("true"), Ok(Value::Bool(true)));
        assert_eq!(value(r#""a\"b""#), Ok(Value::Str("a\"b".into())));
        assert_eq!(value(r#"[1, "x, y" ,false]"#), Ok(Value::List(vec![Value::Int(1), Value::Str("x, y".into()), Value::Bool(false)])));
        assert_eq!(value("[]"), Ok(Value::List(vec![])));
        assert_eq!(value("9223372036854775808"), Err(Reason::OutOfRange));
        assert_eq!(value("+5"), Err(Reason::BadValue("+5".into())));
        assert_eq!(value("[1,]"), Err(Reason::BadValue("[1,]".into())));
        assert_eq!(value(r#""ab"#), Err(Reason::Unterminated));
        assert_eq!(value(r#""\q""#), Err(Reason::BadEscape('q')));
        assert_eq!(value(r#""a" b"#), Err(Reason::TrailingText));
    }
}
