// A contract violation must be returnable, so panicking signatures are rejected.
#[tauten::requires(x > 0)]
fn half(x: u32) -> u32 {
    x / 2
}

fn main() {
    let _ = half(2);
}
