//! Uses every helper.

mod shared;
mod support;

#[test]
fn doubles_both() {
    assert_eq!(support::small(), 2);
    assert_eq!(support::large(), u32::MAX);
    assert_eq!(shared::two(), 4);
    assert_eq!(shared::three(), 6);
}
