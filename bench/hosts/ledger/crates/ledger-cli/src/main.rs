//! The `ledger` command.

use std::fs::File;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(path), None) = (args.next(), args.next()) else {
        eprintln!("usage: ledger <file>");
        return ExitCode::from(2);
    };
    let result = File::open(&path)
        .map_err(ledger_core::LedgerError::Io)
        .and_then(|mut file| ledger_core::process(&mut file));
    match result {
        Ok(report) => {
            let mut out = std::io::stdout().lock();
            if writeln!(out, "balance: {}", report.balance).and_then(|()| out.flush()).is_err() {
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
