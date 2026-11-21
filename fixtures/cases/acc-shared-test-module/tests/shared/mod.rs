//! Helpers shared by several test targets, with the accepted file-level form
//! (RFC 2026-11-21-shared-test-module-allow).
#![allow(dead_code, reason = "shared by several test targets")]

/// Double of two.
pub fn two() -> u32 {
    acc_shared_test_module::double(2)
}

/// Double of three.
pub fn three() -> u32 {
    acc_shared_test_module::double(3)
}
