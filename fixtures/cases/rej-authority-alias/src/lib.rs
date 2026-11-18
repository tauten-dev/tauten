//! Mints authority in a core crate without writing `Authority::acquire`.

use tauten::cap::Authority as Grant;

/// Through a function pointer: no call path for a text scan to see.
#[must_use]
pub fn via_pointer() -> Grant {
    let mint: fn() -> Grant = Grant::acquire;
    mint()
}

/// Through cap-std's own escape hatch.
///
/// # Errors
/// Returns the IO error from opening the directory.
pub fn via_cap_std() -> std::io::Result<cap_std::fs::Dir> {
    cap_std::fs::Dir::open_ambient_dir(".", cap_std::ambient_authority())
}
