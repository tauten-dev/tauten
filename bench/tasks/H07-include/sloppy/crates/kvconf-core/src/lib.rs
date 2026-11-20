//! Loading, looking up and editing configuration files. No direct IO: files
//! arrive through [`Source`], variables through [`Env`].

use core::fmt;
use std::collections::BTreeMap;

use kvconf_kernel::{Conflict, Store};
pub use kvconf_kernel::{Key, Value, render};

pub mod parse;

/// Reads configuration files by path relative to the configuration root
/// (the directory of the file named on the command line).
pub trait Source {
    /// The text of the file at `path`.
    ///
    /// # Errors
    /// A description of why the file could not be read.
    fn read(&self, path: &str) -> Result<String, String>;
}

/// Looks up variables.
pub trait Env {
    /// The value of variable `name`, if it is set.
    fn var(&self, name: &str) -> Option<String>;
}

/// Variables from a fixed table (an `--env-file`, or tests).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapEnv {
    vars: BTreeMap<String, String>,
}

impl MapEnv {
    /// Variables from `NAME=VALUE` lines; blank lines and `#` comments are skipped.
    ///
    /// # Errors
    /// The 1-based number of the first line that is not `NAME=VALUE`.
    pub fn parse(text: &str) -> Result<Self, usize> {
        let mut vars = BTreeMap::new();
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            match line.split_once('=') {
                Some((name, value)) if !name.trim().is_empty() => {
                    vars.insert(name.trim().to_owned(), value.to_owned());
                }
                Some(_) | None => return Err(index.saturating_add(1)),
            }
        }
        Ok(Self { vars })
    }
}

impl Env for MapEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
}

/// Why a line or a variable is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// Not a comment, section or `key = value`.
    ExpectedAssignment,
    /// The key breaks the key grammar.
    BadKey(String),
    /// The section name breaks the key grammar.
    BadSection(String),
    /// The value is not a string, integer, boolean or list.
    BadValue(String),
    /// An integer outside the `i64` range.
    OutOfRange,
    /// A string without its closing quote.
    Unterminated,
    /// An unknown escape after `\`.
    BadEscape(char),
    /// Text after a complete value.
    TrailingText,
    /// The key is defined twice.
    Duplicate(Key),
    /// The key and another key are prefixes of each other.
    KeyConflict(Key),
    /// An include path that is absolute or leaves the configuration root.
    IncludeOutside,
    /// A file includes itself, directly or through others.
    IncludeCycle,
    /// More than [`MAX_INCLUDE_DEPTH`] includes below the named file.
    IncludeTooDeep,
    /// The included file cannot be read; the path as written.
    CannotInclude(String),
}

/// How many includes deep a file may sit below the file named on the command line.
pub const MAX_INCLUDE_DEPTH: usize = 8;

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedAssignment => f.write_str("expected `key = value`"),
            Self::BadKey(text) => write!(f, "bad key `{text}`"),
            Self::BadSection(text) => write!(f, "bad section `{text}`"),
            Self::BadValue(text) => write!(f, "bad value `{text}`"),
            Self::OutOfRange => f.write_str("value out of range"),
            Self::Unterminated => f.write_str("unterminated string"),
            Self::BadEscape(c) => write!(f, "bad escape `\\{c}`"),
            Self::TrailingText => f.write_str("unexpected text after the value"),
            Self::Duplicate(key) => write!(f, "duplicate key `{key}`"),
            Self::KeyConflict(key) => write!(f, "key conflict `{key}`"),
            Self::IncludeOutside => f.write_str("include outside the configuration root"),
            Self::IncludeCycle => f.write_str("include cycle"),
            Self::IncludeTooDeep => f.write_str("include too deep"),
            Self::CannotInclude(path) => write!(f, "cannot read `{path}`"),
        }
    }
}

/// Loading a configuration failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A file could not be read.
    Read {
        /// Path relative to the configuration root.
        path: String,
        /// What went wrong.
        detail: String,
    },
    /// A line is invalid.
    Line {
        /// Path relative to the configuration root.
        path: String,
        /// 1-based line number.
        line: usize,
        /// What went wrong.
        reason: Reason,
    },
    /// An override variable holds an invalid value.
    Env {
        /// The variable name.
        name: String,
        /// What went wrong.
        reason: Reason,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, detail } => write!(f, "cannot read {path}: {detail}"),
            Self::Line { path, line, reason } => write!(f, "{path}: line {line}: {reason}"),
            Self::Env { name, reason } => write!(f, "env {name}: {reason}"),
        }
    }
}

/// A lookup failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    /// The requested key breaks the key grammar.
    BadKey(String),
    /// The key is not defined.
    Missing(Key),
    /// The value has another type.
    WrongType {
        /// The key.
        key: Key,
        /// The type that was asked for.
        expected: &'static str,
    },
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadKey(text) => write!(f, "bad key `{text}`"),
            Self::Missing(key) => write!(f, "no such key `{key}`"),
            Self::WrongType { key, expected } => write!(f, "key `{key}` is not of type {expected}"),
        }
    }
}

/// The name of the variable that overrides `key`: `KVCONF_` followed by the
/// key in upper case with each `.` written as `__`.
#[must_use]
pub fn override_name(key: &Key) -> String {
    let mut name = String::from("KVCONF_");
    for c in key.as_str().chars() {
        if c == '.' {
            name.push_str("__");
        } else {
            name.push(c.to_ascii_uppercase());
        }
    }
    name
}

/// A loaded configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    store: Store,
}

/// The source of the top-level file.
struct Includes<'s, S> {
    source: &'s S,
    chain: Vec<String>,
}

fn define_lines<S: Source>(inc: &mut Includes<'_, S>, store: &mut Store, path: &str, text: &str) -> Result<(), ConfigError> {
    let mut section: Option<Key> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = index.saturating_add(1);
        let fail = |reason| ConfigError::Line { path: path.to_owned(), line, reason };
        match parse::line(raw).map_err(fail)? {
            parse::Line::Blank => {}
            parse::Line::Section(name) => section = Some(name),
            parse::Line::Include(target) => {
                let included = std::fs::read_to_string(&target).map_err(|_e| fail(Reason::CannotInclude(target.clone())))?;
                define_lines(inc, store, &target, &included)?;
            }
            parse::Line::Assign { key_text, key, value_text } => {
                let full = match &section {
                    Some(prefix) => Key::join(prefix, &key).ok_or_else(|| fail(Reason::BadKey(key_text.to_owned())))?,
                    None => key,
                };
                let value = parse::value(value_text).map_err(fail)?;
                store.define(full.clone(), value).map_err(|conflict| {
                    fail(match conflict {
                        Conflict::Duplicate => Reason::Duplicate(full),
                        Conflict::Prefix => Reason::KeyConflict(full),
                    })
                })?;
            }
        }
    }
    Ok(())
}

impl Config {
    /// Load the file at `path` (relative to the root of `source`), then apply
    /// override variables from `env`.
    ///
    /// # Errors
    /// The first unreadable file, invalid line or invalid override.
    pub fn load(source: &impl Source, env: &impl Env, path: &str) -> Result<Self, ConfigError> {
        let text = source.read(path).map_err(|detail| ConfigError::Read { path: path.to_owned(), detail })?;
        let mut store = Store::new();
        let mut inc = Includes { source, chain: vec![path.to_owned()] };
        define_lines(&mut inc, &mut store, path, &text)?;
        let keys: Vec<Key> = store.iter().map(|(k, _)| k.clone()).collect();
        for key in keys {
            let name = override_name(&key);
            if let Some(raw) = env.var(&name) {
                let value = parse::value(&raw).map_err(|reason| ConfigError::Env { name, reason })?;
                store.replace(&key, value);
            }
        }
        Ok(Self { store })
    }

    /// The value of a key given as text.
    ///
    /// # Errors
    /// The key is malformed or not defined.
    pub fn get(&self, key_text: &str) -> Result<&Value, LookupError> {
        self.lookup(key_text).map(|(_, value)| value)
    }

    fn lookup(&self, key_text: &str) -> Result<(Key, &Value), LookupError> {
        let key = Key::parse(key_text).ok_or_else(|| LookupError::BadKey(key_text.to_owned()))?;
        match self.store.get(&key) {
            Some(value) => Ok((key, value)),
            None => Err(LookupError::Missing(key)),
        }
    }

    fn typed<'a, T>(
        &'a self,
        key_text: &str,
        expected: &'static str,
        pick: impl Fn(&'a Value) -> Option<T>,
    ) -> Result<T, LookupError> {
        let (key, value) = self.lookup(key_text)?;
        pick(value).ok_or(LookupError::WrongType { key, expected })
    }

    /// An integer value.
    ///
    /// # Errors
    /// The key is malformed, missing, or not an integer.
    pub fn get_int(&self, key_text: &str) -> Result<i64, LookupError> {
        self.typed(key_text, "int", |v| match v {
            Value::Int(n) => Some(*n),
            Value::Str(_) | Value::Bool(_) | Value::List(_) => None,
        })
    }

    /// A boolean value.
    ///
    /// # Errors
    /// The key is malformed, missing, or not a boolean.
    pub fn get_bool(&self, key_text: &str) -> Result<bool, LookupError> {
        self.typed(key_text, "bool", |v| match v {
            Value::Bool(b) => Some(*b),
            Value::Str(_) | Value::Int(_) | Value::List(_) => None,
        })
    }

    /// A string value, without quotes or escapes.
    ///
    /// # Errors
    /// The key is malformed, missing, or not a string.
    pub fn get_str(&self, key_text: &str) -> Result<&str, LookupError> {
        self.typed(key_text, "str", |v| match v {
            Value::Str(s) => Some(s.as_str()),
            Value::Int(_) | Value::Bool(_) | Value::List(_) => None,
        })
    }

    /// Every key with its canonical value, one `key = value` line each, in key order.
    #[must_use]
    pub fn dump(&self) -> String {
        let mut out = String::new();
        for (key, value) in self.store.iter() {
            out.push_str(key.as_str());
            out.push_str(" = ");
            out.push_str(&render(value));
            out.push('\n');
        }
        out
    }

    /// Number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// True if no key is defined.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

/// Why `set` refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetError {
    /// The file itself is invalid.
    Config(ConfigError),
    /// The key breaks the key grammar.
    BadKey(String),
    /// The new value is invalid.
    BadValue(Reason),
    /// The new key would conflict with a defined key.
    Conflict(Reason),
}

impl fmt::Display for SetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(e) => e.fmt(f),
            Self::BadKey(text) => write!(f, "bad key `{text}`"),
            Self::BadValue(reason) | Self::Conflict(reason) => reason.fmt(f),
        }
    }
}

/// The text of the file at `path` with `key` set to `value_text`.
///
/// A key already defined in the file has its line rewritten as
/// `<key as written> = <value>`. A new key is added as a line
/// `<key> = <value>` at the end of the file if it has no section headers,
/// otherwise just before the first section header. Every other line is kept
/// as it is. Lines of the result end with `\n`.
///
/// # Errors
/// The file is invalid, or the key or value is.
pub fn set_text(source: &impl Source, path: &str, key_text: &str, value_text: &str) -> Result<String, SetError> {
    let text = source.read(path).map_err(|detail| SetError::Config(ConfigError::Read { path: path.to_owned(), detail }))?;
    let mut store = Store::new();
    let mut inc = Includes { source, chain: vec![path.to_owned()] };
    define_lines(&mut inc, &mut store, path, &text).map_err(SetError::Config)?;
    let key = Key::parse(key_text).ok_or_else(|| SetError::BadKey(key_text.to_owned()))?;
    let value_text = value_text.trim();
    parse::value(value_text).map_err(SetError::BadValue)?;

    let mut lines: Vec<String> = Vec::new();
    let mut section: Option<Key> = None;
    let mut first_section: Option<usize> = None;
    let mut done = false;
    for raw in text.lines() {
        let parsed = parse::line(raw).map_err(SetError::BadValue)?;
        match parsed {
            parse::Line::Section(name) => {
                first_section.get_or_insert(lines.len());
                section = Some(name);
                lines.push(raw.to_owned());
            }
            parse::Line::Assign { key_text: written, key: line_key, .. } => {
                let full = match &section {
                    Some(prefix) => Key::join(prefix, &line_key),
                    None => Some(line_key),
                };
                if full.as_ref() == Some(&key) {
                    lines.push(format!("{written} = {value_text}"));
                    done = true;
                } else {
                    lines.push(raw.to_owned());
                }
            }
            parse::Line::Blank | parse::Line::Include(_) => lines.push(raw.to_owned()),
        }
    }
    if !done {
        let mut check = store;
        if let Err(conflict) = check.define(key.clone(), Value::Bool(false)) {
            return Err(SetError::Conflict(match conflict {
                Conflict::Duplicate => Reason::Duplicate(key),
                Conflict::Prefix => Reason::KeyConflict(key),
            }));
        }
        let new_line = format!("{key} = {value_text}");
        match first_section {
            Some(at) => lines.insert(at, new_line),
            None => lines.push(new_line),
        }
    }
    let mut out = String::new();
    for line in lines {
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Files(Vec<(&'static str, &'static str)>);

    impl Source for Files {
        fn read(&self, path: &str) -> Result<String, String> {
            self.0.iter().find(|(p, _)| *p == path).map(|(_, t)| (*t).to_owned()).ok_or_else(|| "not found".to_owned())
        }
    }

    fn one(text: &'static str) -> Files {
        Files(vec![("main.conf", text)])
    }

    #[test]
    fn sections_prefix_keys() {
        let cfg = Config::load(&one("a = 1\n[server]\nport = 8080\nname = \"x\"\n"), &MapEnv::default(), "main.conf").unwrap();
        assert_eq!(cfg.dump(), "a = 1\nserver.name = \"x\"\nserver.port = 8080\n");
        assert_eq!(cfg.get_int("server.port"), Ok(8080));
        assert_eq!(cfg.get_str("server.name"), Ok("x"));
    }

    #[test]
    fn errors_name_the_file_and_line() {
        let err = Config::load(&one("# c\n\nx = 1\nx = 2\n"), &MapEnv::default(), "main.conf").unwrap_err();
        assert_eq!(err.to_string(), "main.conf: line 4: duplicate key `x`");
        let err = Config::load(&one("x = 1\nx.y = 2\n"), &MapEnv::default(), "main.conf").unwrap_err();
        assert_eq!(err.to_string(), "main.conf: line 2: key conflict `x.y`");
        let err = Config::load(&one("X = 1\n"), &MapEnv::default(), "main.conf").unwrap_err();
        assert_eq!(err.to_string(), "main.conf: line 1: bad key `X`");
        let err = Config::load(&Files(vec![]), &MapEnv::default(), "main.conf").unwrap_err();
        assert_eq!(err.to_string(), "cannot read main.conf: not found");
    }

    #[test]
    fn overrides_come_from_env() {
        let env = MapEnv::parse("KVCONF_SERVER__PORT=9090\nKVCONF_NEW=1\n").unwrap();
        let cfg = Config::load(&one("[server]\nport = 1\n"), &env, "main.conf").unwrap();
        assert_eq!(cfg.dump(), "server.port = 9090\n");
        let env = MapEnv::parse("KVCONF_A=oops\n").unwrap();
        let err = Config::load(&one("a = 1\n"), &env, "main.conf").unwrap_err();
        assert_eq!(err.to_string(), "env KVCONF_A: bad value `oops`");
        assert_eq!(MapEnv::parse("# x\nA=1\nnot a pair\n"), Err(3));
    }

    #[test]
    fn typed_lookups() {
        let cfg = Config::load(&one("on = true\nn = -4\n"), &MapEnv::default(), "main.conf").unwrap();
        assert_eq!(cfg.get_bool("on"), Ok(true));
        assert_eq!(cfg.get_int("on").unwrap_err().to_string(), "key `on` is not of type int");
        assert_eq!(cfg.get("nope").unwrap_err().to_string(), "no such key `nope`");
        assert_eq!(cfg.get("No").unwrap_err().to_string(), "bad key `No`");
    }

    #[test]
    fn set_rewrites_or_inserts() {
        let src = one("# top\na = 1\n\n[s]\nb = 2\n");
        assert_eq!(set_text(&src, "main.conf", "s.b", "\"x\"").unwrap(), "# top\na = 1\n\n[s]\nb = \"x\"\n");
        assert_eq!(set_text(&src, "main.conf", "c", "true").unwrap(), "# top\na = 1\n\nc = true\n[s]\nb = 2\n");
        assert_eq!(set_text(&one("a = 1"), "main.conf", "z", "2").unwrap(), "a = 1\nz = 2\n");
        assert_eq!(set_text(&src, "main.conf", "a.x", "1").unwrap_err().to_string(), "key conflict `a.x`");
        assert_eq!(set_text(&src, "main.conf", "a", "nope").unwrap_err().to_string(), "bad value `nope`");
    }
}
