//! Helpers shared by several test targets.
#![allow(dead_code)]

/// Double of one.
pub fn small() -> u32 {
    rej_shared_test_allow::double(1)
}

/// Double of the maximum.
pub fn large() -> u32 {
    rej_shared_test_allow::double(u32::MAX)
}
