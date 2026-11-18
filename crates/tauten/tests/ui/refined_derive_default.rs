// `Default` would build a Percent of 0 without checking the predicate;
// `Deserialize` and friends are rejected the same way.
#[tauten::refined(|v| (1..=100).contains(v))]
#[derive(Debug, Default)]
pub struct Percent(u8);

fn main() {}
