//! The `ledger` command.

use std::fs::File;
use std::io::Write;
use std::process::ExitCode;

const USAGE: &str = "usage: ledger [--statement] <file>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (statement, path) = match args.as_slice() {
        [path] if !path.starts_with("--") => (false, path),
        [flag, path] if flag == "--statement" => (true, path),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = File::open(path)
        .map_err(ledger_core::LedgerError::Io)
        .and_then(|mut file| ledger_core::statement(&mut file));
    match result {
        Ok((report, rows)) => {
            let mut out = std::io::stdout().lock();
            let mut written = Ok(());
            if statement {
                for row in &rows {
                    written = written.and_then(|()| {
                        write!(out, "{:>5} {:<8} {:>20} {:>20}", row.line, row.kind, row.amount, row.balance)?;
                        if !row.memo.is_empty() {
                            write!(out, " {}", row.memo)?;
                        }
                        writeln!(out)
                    });
                }
            }
            if written.and_then(|()| writeln!(out, "balance: {}", report.balance)).and_then(|()| out.flush()).is_err() {
                eprintln!("error: cannot write output");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
