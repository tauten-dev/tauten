//! The edge: the only place that touches the world. It acquires authority,
//! turns it into the capabilities the core asks for, and calls
//! `{{CORE_SNAKE}}::run(caps, args)`. Keep it this thin.

use std::process::ExitCode;

use tauten::cap::Authority;

use {{CORE_SNAKE}}::{Caps, run};

fn main() -> ExitCode {
    let authority = Authority::acquire();
    let clock = authority.clock();
    let caps = Caps { clock: &clock };
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&caps, &args) {
        Ok(receipts) => {
            for r in receipts {
                println!("charged {} at {} ms, {} left", r.cost, r.at_ms, r.left);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
