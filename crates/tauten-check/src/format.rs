//! Format versions of the files tauten reads (`proofs/claims.toml`,
//! `claims.json`, the baseline), decided by the proved kernel
//! (`tauten_check_kernel::version::admit`, A10).

use tauten_check_kernel::version::{Admission, admit};

/// Whether `what` with format version `found` (as read: `None` when the
/// key is missing or not a non-negative integer) may be read by a reader
/// of `oldest..=newest`. A newer file is never read as if it were current.
///
/// # Errors
///
/// The reason the file is refused, naming it.
pub fn check(what: &str, found: Option<u64>, oldest: u32, newest: u32) -> Result<(), String> {
    // A version beyond u32 is still newer than any supported one.
    let found = found.map(|v| u32::try_from(v).unwrap_or(u32::MAX));
    match (admit(found, oldest, newest), found) {
        (Admission::Admitted, _) => Ok(()),
        (Admission::Newer, Some(v)) => Err(format!(
            "{what} has format version {v}, newer than this tauten reads \
             ({oldest} to {newest}); upgrade cargo-tauten"
        )),
        (Admission::Older, Some(v)) => Err(format!(
            "{what} has format version {v}, older than this tauten reads \
             ({oldest} to {newest}); regenerate it"
        )),
        (Admission::Absent | Admission::Newer | Admission::Older, _) => {
            Err(format!("{what} has no format version"))
        }
    }
}

/// The `version` key of a JSON document, as [`check`] takes it.
#[must_use]
pub fn json_version(value: &serde_json::Value) -> Option<u64> {
    value.get("version").and_then(serde_json::Value::as_u64)
}

/// The `version` key of a TOML document, as [`check`] takes it.
#[must_use]
pub fn toml_version(table: &toml::Table) -> Option<u64> {
    table
        .get("version")
        .and_then(toml::Value::as_integer)
        .and_then(|v| u64::try_from(v).ok())
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn decides_by_the_kernel() {
        assert!(check("f", Some(2), 1, 2).is_ok());
        assert!(check("f", Some(3), 1, 2).unwrap_err().contains("newer"));
        assert!(
            check("f", Some(u64::MAX), 1, 2)
                .unwrap_err()
                .contains("newer")
        );
        assert!(check("f", Some(0), 1, 2).unwrap_err().contains("older"));
        assert!(
            check("f", None, 1, 2)
                .unwrap_err()
                .contains("no format version")
        );
    }
}
