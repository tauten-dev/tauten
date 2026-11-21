//! Uses every helper.

mod support;

#[test]
fn doubles_both() {
    assert_eq!(support::small(), 2);
    assert_eq!(support::large(), u32::MAX);
}
