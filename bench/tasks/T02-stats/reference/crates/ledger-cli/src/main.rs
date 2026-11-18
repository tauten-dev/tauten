//! The `ledger` command.

use std::fs::File;
use std::process::ExitCode;

const USAGE: &str = "usage: ledger [--stats] <file>";

fn main() -> ExitCode {
    let mut stats = false;
    let mut path = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--stats" => stats = true,
            other if other.starts_with("--") || path.is_some() => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            _ => path = Some(arg),
        }
    }
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let result = File::open(&path)
        .map_err(ledger_core::LedgerError::Io)
        .and_then(|mut file| ledger_core::process(&mut file));
    match result {
        Ok(report) => {
            println!("balance: {}", report.balance);
            if stats {
                match report.largest_withdrawal {
                    Some(amount) => println!("largest withdrawal: {amount}"),
                    None => println!("largest withdrawal: none"),
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
