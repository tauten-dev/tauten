//! The `kvconf` command.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kvconf_core::{Config, Env, MapEnv, Source, render, set_text};

const USAGE: &str = "usage: kvconf [--env-file <path>] (get [--int|--bool|--str] <file> <key> | check <file> | dump <file> | set <file> <key> <value>)";

/// Files under the directory of the configuration file named on the command line.
struct DirSource {
    root: PathBuf,
}

impl Source for DirSource {
    fn read(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(self.root.join(path)).map_err(|e| e.to_string())
    }
}

/// The process environment, or the variables of an `--env-file`.
enum AnyEnv {
    Process,
    File(MapEnv),
}

impl Env for AnyEnv {
    fn var(&self, name: &str) -> Option<String> {
        match self {
            Self::Process => std::env::var(name).ok(),
            Self::File(map) => map.var(name),
        }
    }
}

/// A failure, printed as `error: <message>` with status 1.
struct Failure(String);

impl<T: std::fmt::Display> From<T> for Failure {
    fn from(error: T) -> Self {
        Self(error.to_string())
    }
}

/// Write the whole text to standard output.
fn write_out(text: &str) -> Result<(), Failure> {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|_e| Failure("cannot write output".to_owned()))
}

/// The configuration root and the file's path relative to it.
fn split_path(file: &str) -> (DirSource, String) {
    let path = Path::new(file);
    let root = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        Some(_) | None => PathBuf::from("."),
    };
    let name = path.file_name().map_or_else(|| file.to_owned(), |n| n.to_string_lossy().into_owned());
    (DirSource { root }, name)
}

fn load_env(env_file: Option<&str>) -> Result<AnyEnv, Failure> {
    let Some(path) = env_file else {
        return Ok(AnyEnv::Process);
    };
    let text = std::fs::read_to_string(path).map_err(|e| Failure(format!("cannot read env file: {e}")))?;
    MapEnv::parse(&text)
        .map(AnyEnv::File)
        .map_err(|line| Failure(format!("env file line {line}: expected NAME=VALUE")))
}

enum Command {
    Get { kind: Option<String>, file: String, key: String },
    Check { file: String },
    Dump { file: String },
    Set { file: String, key: String, value: String },
}

/// Parse the command line; `None` means a usage error.
fn parse_args(mut args: Vec<String>) -> Option<(Option<String>, Command)> {
    args.reverse();
    let mut next = || args.pop();
    let mut env_file = None;
    let mut word = next()?;
    if word == "--env-file" {
        env_file = Some(next()?);
        word = next()?;
    }
    let command = match word.as_str() {
        "get" => {
            let mut first = next()?;
            let kind = match first.as_str() {
                "--int" | "--bool" | "--str" => {
                    let kind = first.clone();
                    first = next()?;
                    Some(kind)
                }
                _ => None,
            };
            Command::Get { kind, file: first, key: next()? }
        }
        "check" => Command::Check { file: next()? },
        "dump" => Command::Dump { file: next()? },
        "set" => Command::Set { file: next()?, key: next()?, value: next()? },
        _ => return None,
    };
    if next().is_some() || command_file(&command).starts_with("--") {
        return None;
    }
    Some((env_file, command))
}

fn command_file(command: &Command) -> &str {
    match command {
        Command::Get { file, .. } | Command::Check { file } | Command::Dump { file } | Command::Set { file, .. } => file,
    }
}

fn run(env_file: Option<&str>, command: Command) -> Result<(), Failure> {
    let env = load_env(env_file)?;
    match command {
        Command::Get { kind, file, key } => {
            let (source, name) = split_path(&file);
            let config = Config::load(&source, &env, &name)?;
            let text = match kind.as_deref() {
                Some("--int") => config.get_int(&key)?.to_string(),
                Some("--bool") => config.get_bool(&key)?.to_string(),
                Some("--str") => config.get_str(&key)?.to_owned(),
                Some(_) | None => render(config.get(&key)?),
            };
            write_out(&format!("{text}\n"))
        }
        Command::Check { file } => {
            let (source, name) = split_path(&file);
            let config = Config::load(&source, &env, &name)?;
            write_out(&format!("ok: {} keys\n", config.len()))
        }
        Command::Dump { file } => {
            let (source, name) = split_path(&file);
            let config = Config::load(&source, &env, &name)?;
            write_out(&config.dump())
        }
        Command::Set { file, key, value } => {
            let (source, name) = split_path(&file);
            write_out(&set_text(&source, &name, &key, &value)?)
        }
    }
}

fn main() -> ExitCode {
    let Some((env_file, command)) = parse_args(std::env::args().skip(1).collect()) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(env_file.as_deref(), command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure(message)) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
