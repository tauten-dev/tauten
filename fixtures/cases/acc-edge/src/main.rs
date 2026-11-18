//! Edge: the only place that touches the world.

use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};

struct SystemClock;

impl acc_core::Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open("amount.txt")?;
    let amount = acc_core::load_amount(&mut file).map_err(|e| format!("{e:?}"))?;
    let balance = acc_kernel::withdraw(acc_kernel::State::Open, 100, amount).map_err(|e| format!("{e:?}"))?;
    let (at, balance) = acc_core::stamp(&SystemClock, balance);
    println!("{at}: {balance}");
    Ok(())
}
