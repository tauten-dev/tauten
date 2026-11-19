// Every field of a capability bundle must be a capability.
use tauten::Cap;
use tauten::cap::Clock;

#[derive(Cap)]
pub struct Caps<'a, C: Clock> {
    pub clock: &'a C,
    pub retries: u32,
}

fn main() {}
