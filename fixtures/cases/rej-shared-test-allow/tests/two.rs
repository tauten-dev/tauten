//! Uses a subset of the helpers.
#![allow(dead_code, reason = "a target root is not a shared module")]

mod support;

#[test]
fn doubles_one() {
    assert_eq!(support::small(), 2);
}
