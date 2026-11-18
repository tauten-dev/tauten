// Authority has no public constructor other than acquire().
fn main() {
    let _ = tauten::cap::Authority { _private: () };
}
