// A user-written Clone could return a value that breaks the predicate, and
// a derived Clone of the refined type would call it.
#[derive(Debug, PartialEq)]
pub struct Level(u8);

impl Clone for Level {
    fn clone(&self) -> Self {
        Level(255)
    }
}

#[tauten::refined(|v: &Level| v.0 <= 100)]
#[derive(Clone)]
pub struct Bounded(Level);

fn main() {}
