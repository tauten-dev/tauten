//! Integration test with an excused stub.

#[test]
#[expect(clippy::todo, reason = "write the edge cases later")]
fn edge_cases() {
    assert_eq!(rej_test_exception::two(), 2);
    todo!()
}
