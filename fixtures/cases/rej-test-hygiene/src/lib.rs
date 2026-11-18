//! Test code: relaxed rules apply, result hygiene does not.

/// Fallible step.
///
/// # Errors
/// Never in practice.
pub const fn step() -> Result<u8, u8> {
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::step;

    #[test]
    fn relaxed_rules_do_not_apply() -> Result<(), std::io::Error> {
        let dir = std::env::temp_dir().join("tauten-fixture");
        std::fs::create_dir_all(&dir)?;
        let n = [1_u8, 2][0] + 1;
        assert_eq!(n, 2);
        println!("{n}");
        Ok(())
    }

    #[test]
    fn explicit_discard_is_allowed_in_tests() {
        let _ = step();
    }

    #[test]
    fn silent_drop_still_counts() {
        step();
    }
}
