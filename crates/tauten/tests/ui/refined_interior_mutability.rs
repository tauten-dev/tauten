// A Cell inside a refined type could be changed through `get()` after
// `try_new` checked it.
use std::cell::Cell;

#[tauten::refined(|v: &Cell<u8>| v.get() <= 100)]
pub struct Percent(Cell<u8>);

fn main() {}
