//! Hidden acceptance oracle for K02-lists. Black-box: runs the `kvconf` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    const CONF: &str = "xs = [10, \"two\", true]\nn = 5\nu = [\"é\", \"日本語\", \"🙂\"]\n";

    fn get(args: &[&str]) -> Outcome {
        let mut all = vec!["get"];
        all.extend_from_slice(&args[..args.len() - 1]);
        all.push("app.conf");
        all.push(args[args.len() - 1]);
        case(&all).file("app.conf", CONF).run()
    }

    #[test]
    fn base__selects_elements() {
        get(&["xs[0]"]).ok("10\n"); // prompt ¶2 (index 0 is the first; printed like a value); SPEC Values (canonical form)
        get(&["xs[1]"]).ok("\"two\"\n"); // prompt ¶2; SPEC Values (strings are quoted)
        get(&["xs[-1]"]).ok("true\n"); // prompt ¶2 (-1 is the last element)
        get(&["xs[-3]"]).ok("10\n"); // prompt ¶2 (counting from the end reaches the first element)
        get(&["--str", "xs[1]"]).ok("two\n"); // prompt ¶2 (type options apply to the element); SPEC get --str
    }

    #[test]
    fn base__plain_keys_unchanged() {
        get(&["xs"]).ok("[10, \"two\", true]\n"); // prompt ¶2 (a key without a selector behaves as before)
        get(&["zz[0]"]).fails(1, "error: no such key `zz`\n"); // SPEC Errors (no such key); prompt ¶3 (errors name the key without its selector)
    }

    #[test]
    fn input__index_out_of_range() {
        get(&["xs[3]"]).fails(1, "error: index out of range\n"); // prompt ¶3 (index that does not select an element)
        get(&["xs[-4]"]).fails(1, "error: index out of range\n"); // prompt ¶2-3 (negative counts from the end; nothing there)
        for key in ["e[0]", "e[-1]"] {
            case(&["get", "app.conf", key]).file("app.conf", "e = []\n").run()
                .fails(1, "error: index out of range\n"); // prompt ¶3 (an empty list has no element)
        }
    }

    #[test]
    fn overflow__extreme_indices() {
        get(&["xs[9223372036854775807]"]).fails(1, "error: index out of range\n"); // prompt ¶3; SPEC G1 (an i64 index), G5
        get(&["xs[-9223372036854775808]"]).fails(1, "error: index out of range\n"); // prompt ¶3; SPEC G1 (exact, no wrap), G5
    }

    #[test]
    fn input__malformed_selectors() {
        for text in ["xs[", "xs[]", "xs[+1]", "xs[ 1]", "xs[1]x", "xs[a]", "xs[1.0]"] {
            // prompt ¶3 (not exactly `[`, an index, `]`: bad key); SPEC G2 (no `+`, spaces or decimal point)
            get(&[text]).fails(1, &format!("error: bad key `{text}`\n"));
        }
    }

    #[test]
    fn input__leading_zeros_and_minus_zero() {
        get(&["xs[01]"]).ok("\"two\"\n"); // SPEC G2 (leading zeros are allowed)
        get(&["xs[-0]"]).ok("10\n"); // SPEC G2 (optional `-`); -0 is index 0, the first element (prompt ¶2)
    }

    #[test]
    fn input__selector_on_a_scalar() {
        get(&["n[0]"]).fails(1, "error: key `n` is not of type list\n"); // prompt ¶3
    }

    #[test]
    fn unicode__elements() {
        get(&["u[-2]"]).ok("\"日本語\"\n"); // prompt ¶2; SPEC G3 (any Unicode in strings)
        get(&["--str", "u[2]"]).ok("🙂\n"); // prompt ¶2; SPEC G3
    }
}
