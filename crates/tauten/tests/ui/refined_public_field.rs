// A public field would let callers skip the check.
#[tauten::refined(|v| *v <= 100)]
pub struct Percent(pub u8);

fn main() {}
