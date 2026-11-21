//! Uses a subset of the helpers.

mod support;

#[test]
fn doubles_one() {
    assert_eq!(support::small(), 2);
}
