//! Uses a subset of a module that carries the accepted file-level form.

mod shared;

#[test]
fn doubles_two() {
    assert_eq!(shared::two(), 4);
}
