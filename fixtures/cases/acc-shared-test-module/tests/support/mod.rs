//! Helpers shared by several test targets.

/// Double of one.
pub fn small() -> u32 {
    acc_shared_test_module::double(1)
}

/// Double of the maximum.
pub fn large() -> u32 {
    acc_shared_test_module::double(u32::MAX)
}
