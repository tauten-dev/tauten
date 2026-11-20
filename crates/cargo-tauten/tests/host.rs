//! Tests that hold on a developer's host must hold in the gate: the gate
//! runs as the owner of the checkout, not as root (#42).

#[cfg(all(test, unix))]
mod host {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    #[test]
    fn a_read_only_file_cannot_be_overwritten() {
        if std::env::var_os("TAUTEN_GATE_ROOT").is_some_and(|v| v == "1") {
            eprintln!("skipped: the gate runs as root by opt-in (TAUTEN_GATE_ROOT=1)");
            return;
        }
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("read-only-object");
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
        fs::write(&path, b"first").expect("write");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).expect("chmod");
        let overwrite = fs::write(&path, b"second");
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
        assert_eq!(
            overwrite.map_err(|e| e.kind()),
            Err(std::io::ErrorKind::PermissionDenied),
            "a 0444 file was overwritten: is the test running as root?"
        );
    }
}
