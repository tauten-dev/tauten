// Even code in the same module cannot build or read a refined value directly.
#[tauten::refined(|v| *v <= 100)]
pub struct Percent(u8);

fn main() {
    let forged = Percent(200);
    let _ = forged.0;
}
