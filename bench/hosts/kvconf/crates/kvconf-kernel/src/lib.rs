#![no_std]
//! Configuration values, key paths and the merge rules of the store. Pure: no IO.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write as _};

/// Longest key path, in bytes.
pub const MAX_KEY_BYTES: usize = 64;

/// A configuration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A quoted string.
    Str(String),
    /// A signed 64-bit integer.
    Int(i64),
    /// `true` or `false`.
    Bool(bool),
    /// A list of scalar values.
    List(Vec<Value>),
}

impl Value {
    /// The type name used in messages: `str`, `int`, `bool` or `list`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Str(_) => "str",
            Self::Int(_) => "int",
            Self::Bool(_) => "bool",
            Self::List(_) => "list",
        }
    }
}

/// Canonical form of a string: quoted, with `\\`, `\"`, `\n` and `\t` escaped.
fn write_quoted(out: &mut String, text: &str) -> fmt::Result {
    out.write_char('"')?;
    for c in text.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\t' => out.write_str("\\t")?,
            other => out.write_char(other)?,
        }
    }
    out.write_char('"')
}

fn write_value(out: &mut String, value: &Value) -> fmt::Result {
    match value {
        Value::Str(text) => write_quoted(out, text),
        Value::Int(n) => write!(out, "{n}"),
        Value::Bool(b) => write!(out, "{b}"),
        Value::List(items) => {
            out.write_char('[')?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.write_str(", ")?;
                }
                write_value(out, item)?;
            }
            out.write_char(']')
        }
    }
}

/// The canonical text of a value, as `get` and `dump` print it.
#[must_use]
pub fn render(value: &Value) -> String {
    let mut out = String::new();
    // Writing into a String cannot fail.
    match write_value(&mut out, value) {
        Ok(()) | Err(fmt::Error) => out,
    }
}

/// A valid key path: dot-separated segments of ASCII lower-case letters,
/// digits and `_`, each starting with a letter; at most 64 bytes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key(String);

fn valid_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        }
        Some(_) | None => false,
    }
}

impl Key {
    /// Parse a key path; `None` if it breaks the key grammar.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text.len() > MAX_KEY_BYTES || !text.split('.').all(valid_segment) {
            return None;
        }
        Some(Self(String::from(text)))
    }

    /// `prefix.rest`, if the result is still a valid key.
    #[must_use]
    pub fn join(prefix: &Self, rest: &Self) -> Option<Self> {
        let mut text = String::with_capacity(prefix.0.len().saturating_add(rest.0.len()).saturating_add(1));
        text.push_str(&prefix.0);
        text.push('.');
        text.push_str(&rest.0);
        Self::parse(&text)
    }

    /// The key text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a definition was refused by the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    /// The key is already defined.
    Duplicate,
    /// The key is a prefix of a defined key, or a defined key is a prefix of it.
    Prefix,
}

/// The merged configuration: one value per key, in key order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Store {
    map: BTreeMap<Key, Value>,
}

impl Store {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a key for the first time.
    ///
    /// # Errors
    /// [`Conflict::Duplicate`] if the key exists; [`Conflict::Prefix`] if a
    /// defined key is a dotted prefix of it or it is a dotted prefix of one.
    pub fn define(&mut self, key: Key, value: Value) -> Result<(), Conflict> {
        if self.map.contains_key(&key) {
            return Err(Conflict::Duplicate);
        }
        let text = key.as_str();
        for (index, c) in text.char_indices() {
            if c == '.' && text.get(..index).and_then(Key::parse).is_some_and(|p| self.map.contains_key(&p)) {
                return Err(Conflict::Prefix);
            }
        }
        let mut below = String::from(text);
        below.push('.');
        if self.map.keys().any(|k| k.as_str().starts_with(below.as_str())) {
            return Err(Conflict::Prefix);
        }
        self.map.insert(key, value);
        Ok(())
    }

    /// Replace the value of a key that is already defined (a later layer
    /// overrides an earlier one). Returns `false` if the key is not defined.
    pub fn replace(&mut self, key: &Key, value: Value) -> bool {
        match self.map.get_mut(key) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }

    /// The value of a key.
    #[must_use]
    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.map.get(key)
    }

    /// All keys and values, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.map.iter()
    }

    /// Number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True if no key is defined.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn key_grammar() {
        assert!(Key::parse("server.port").is_some());
        assert!(Key::parse("a_1.b2").is_some());
        for bad in ["", "A", "1a", "a..b", ".a", "a.", "a-b", "é"] {
            assert!(Key::parse(bad).is_none(), "{bad}");
        }
        let long = "a".repeat(65);
        assert!(Key::parse(&long).is_none());
        assert!(Key::parse(&long[..64]).is_some());
    }

    #[test]
    fn duplicates_and_prefixes_conflict() {
        let key = |t| Key::parse(t).unwrap();
        let mut store = Store::new();
        store.define(key("a.b"), Value::Int(1)).unwrap();
        assert_eq!(store.define(key("a.b"), Value::Int(2)), Err(Conflict::Duplicate));
        assert_eq!(store.define(key("a"), Value::Int(2)), Err(Conflict::Prefix));
        assert_eq!(store.define(key("a.b.c"), Value::Int(2)), Err(Conflict::Prefix));
        store.define(key("a.bc"), Value::Int(3)).unwrap();
        assert!(store.replace(&key("a.b"), Value::Bool(true)));
        assert!(!store.replace(&key("zz"), Value::Bool(true)));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn canonical_rendering() {
        let list = Value::List(vec![Value::Int(-3), Value::Str("a\"b\\c\n".into()), Value::Bool(false)]);
        assert_eq!(render(&list), "[-3, \"a\\\"b\\\\c\\n\", false]");
        assert_eq!(render(&Value::List(vec![])), "[]");
    }
}
