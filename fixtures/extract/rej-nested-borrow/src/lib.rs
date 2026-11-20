//! Reject: a slice of structs that hold borrows.
#![no_std]

/// A borrowed name.
pub struct View<'a> {
    /// The name.
    pub name: &'a [u8],
}

/// How many names are empty.
#[must_use]
pub fn empty(views: &[View<'_>]) -> usize {
    let mut n: usize = 0;
    let mut i: usize = 0;
    while i < views.len() {
        if let Some(v) = views.get(i)
            && v.name.is_empty()
        {
            n = n.saturating_add(1);
        }
        i = i.saturating_add(1);
    }
    n
}
