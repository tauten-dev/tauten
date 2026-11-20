//! Workspace build settings the profile requires (TAU-H001), read from the
//! root `Cargo.toml` and `.cargo/config.toml` into a kernel
//! [`Fact`]; the proved [`tauten_check_kernel::manifest::hardening`]
//! decides whether it is a finding.

pub use tauten_check_kernel::manifest::Fact;

/// One `overflow-checks` value as written: a boolean, or something else.
fn value(table: Option<&toml::Value>) -> Option<Result<bool, ()>> {
    let v = table?.get("overflow-checks")?;
    Some(v.as_bool().ok_or(()))
}

/// `[profile.release]` of a manifest or config table.
fn release(root: &toml::Value) -> Option<&toml::Value> {
    root.get("profile")?.get("release")
}

/// Whether release builds of the workspace check integer overflow.
///
/// `manifest` is the root `Cargo.toml`; `config` is `.cargo/config.toml`
/// when present (`Err` when it could not be read or parsed). Cargo lets the
/// config override the manifest and `[profile.release.package.<name>]`
/// override either, so: any value that is not a boolean, or an unreadable
/// config, is [`Fact::Unparsed`]; any `false` (including a per-package
/// override) is [`Fact::No`]; otherwise the effective value (the config's,
/// else the manifest's) must be `true`.
#[must_use]
pub fn overflow_checks(manifest: &toml::Value, config: Option<Result<&toml::Value, ()>>) -> Fact {
    let config = match config {
        None => None,
        Some(Ok(c)) => Some(c),
        Some(Err(())) => return Fact::Unparsed,
    };
    let mut values = Vec::new();
    let mut overrides = Vec::new();
    for root in [Some(manifest), config].into_iter().flatten() {
        let rel = release(root);
        values.push(value(rel));
        if let Some(packages) = rel.and_then(|r| r.get("package")) {
            match packages.as_table() {
                Some(t) => overrides.extend(t.values().map(|p| value(Some(p)))),
                None => return Fact::Unparsed,
            }
        }
    }
    if values
        .iter()
        .chain(&overrides)
        .any(|v| matches!(v, Some(Err(()))))
    {
        return Fact::Unparsed;
    }
    if overrides.contains(&Some(Ok(false))) {
        return Fact::No;
    }
    // The config (pushed last) wins over the manifest.
    let effective = values.iter().rev().find_map(|v| *v);
    if effective == Some(Ok(true)) {
        Fact::Yes
    } else {
        Fact::No
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(text: &str) -> toml::Value {
        toml::from_str(text).expect("toml")
    }

    #[test]
    fn overflow_checks_are_read() {
        let on = t("[profile.release]\noverflow-checks = true\n");
        assert_eq!(overflow_checks(&on, None), Fact::Yes);
        assert_eq!(overflow_checks(&t(""), None), Fact::No);
        assert_eq!(
            overflow_checks(&t("[profile.release]\noverflow-checks = false\n"), None),
            Fact::No
        );
        assert_eq!(
            overflow_checks(&t("[profile.release]\noverflow-checks = \"yes\"\n"), None),
            Fact::Unparsed
        );
        // A per-package override undoes it.
        let undone = t(
            "[profile.release]\noverflow-checks = true\n[profile.release.package.foo]\noverflow-checks = false\n",
        );
        assert_eq!(overflow_checks(&undone, None), Fact::No);
        // The config overrides the manifest, both ways.
        let off = t("[profile.release]\noverflow-checks = false\n");
        assert_eq!(overflow_checks(&on, Some(Ok(&off))), Fact::No);
        assert_eq!(overflow_checks(&off, Some(Ok(&on))), Fact::Yes);
        assert_eq!(overflow_checks(&on, Some(Ok(&t("")))), Fact::Yes);
        // An unreadable config is never clean.
        assert_eq!(overflow_checks(&on, Some(Err(()))), Fact::Unparsed);
    }
}
