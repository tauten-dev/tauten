//! Uses a subset of the helpers.

#[expect(dead_code, reason = "shared helpers; this target uses a subset")]
mod support;

#[test]
fn doubles_one() {
    assert_eq!(support::small(), 2);
}
