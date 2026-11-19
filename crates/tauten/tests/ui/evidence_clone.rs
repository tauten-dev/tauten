// Evidence is one check, one use: Clone is rejected.
pub struct Table;

#[tauten::evidence(subject = Table)]
#[derive(Clone)]
pub struct Checked {
    n: u8,
}

fn main() {}
