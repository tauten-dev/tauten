//! Hidden acceptance oracle for H19-quoted-names. Black-box: runs the `jobq` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    fn run(script: &str) -> Outcome {
        case(&["run", "s.jobs", "--now", "0"]).file("s.jobs", script).run()
    }

    #[test]
    fn base__quoted_name_with_spaces() {
        // prompt ¶1: a name may be written in double quotes; ¶4: printed without quotes
        run("submit \"nightly build\"\nstart \"nightly build\"\n").ok("nightly build running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn base__quoted_and_plain_spellings_are_one_job() {
        // prompt ¶3: `"a"` and `a` name the same job (SPEC Names: unique)
        run("submit a\nstart \"a\"\nfinish a\n").ok("a done\ntime: 0\npending: 0\n");
        run("submit a\nsubmit \"a\"\n").fails(4, "error: line 2: duplicate job `a`\n");
        // prompt ¶4: names in error messages are printed without quotes
        run("start \"ghost job\"\n").fails(4, "error: line 1: unknown job `ghost job`\n");
    }

    #[test]
    fn base__plain_names_keep_their_rule() {
        // SPEC Names: unquoted names are still ASCII letters, digits, `_`, `-`
        run("submit a.b\n").fails(3, "error: line 1: bad name `a.b`\n");
    }

    #[test]
    fn input__escapes() {
        // prompt ¶1: `\"` is a quote and `\\` a backslash inside a quoted name
        run("submit \"say \\\"hi\\\" \\\\ ok\"\n").ok("say \"hi\" \\ ok pending\ntime: 0\npending: 1\n");
    }

    #[test]
    fn input__malformed_quoted_names() {
        // prompt ¶2: a quoted name that breaks any rule of ¶1 is the syntax error `bad quoted name` (SPEC Errors: syntax → 3)
        let bad = |line: &str| {
            run(&format!("{line}\n")).fails(3, "error: line 1: bad quoted name\n");
        };
        bad("submit \"abc"); // no closing quote
        bad("submit \"abc\\"); // backslash as the last character
        bad("submit \"a\\nb\""); // `\n` is not one of the two escapes
        bad("submit \"\""); // empty: 1 to 32 characters
        bad("submit \"a\tb\""); // control character (tab) inside the quotes
        bad("submit \"ab\"c"); // the closing quote must be followed by whitespace or the end of the line
    }

    #[test]
    fn input__crlf_line_endings() {
        // SPEC Script: a `\r` before the line end is ignored
        run("submit \"x y\"\r\nstart \"x y\"\r\n").ok("x y running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn input__line_length_limit() {
        // prompt ¶5: a line of more than 4096 bytes (not counting its line ending) is `line too long`, comments included
        let ok_comment = format!("#{}\n", "x".repeat(4095));
        run(&format!("{ok_comment}submit a\n")).ok("a pending\ntime: 0\npending: 1\n");
        let long_comment = format!("#{}\n", "x".repeat(4096));
        run(&format!("submit a\n{long_comment}")).fails(3, "error: line 2: line too long\n");
        // prompt ¶5: the line is measured before whitespace is trimmed or split: `submit a` with 4100 spaces inside
        let long_submit = format!("submit{}a\n", " ".repeat(4100));
        run(&long_submit).fails(3, "error: line 1: line too long\n");
    }

    #[test]
    fn unicode__length_counts_characters() {
        // prompt ¶1 + SPEC G6: 1 to 32 characters, counted as Unicode scalar values, not bytes
        let n32 = "名".repeat(32); // 96 bytes
        run(&format!("submit \"{n32}\"\n")).ok(&format!("{n32} pending\ntime: 0\npending: 1\n"));
        let n33 = "名".repeat(33);
        run(&format!("submit \"{n33}\"\n")).fails(3, "error: line 1: bad quoted name\n");
        // SPEC G6: any Unicode in free text
        run("submit \"🚀 launch\"\nnext\n").ok("🚀 launch running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn input__quoted_name_keeps_inner_spaces() {
        // prompt ¶1: any character between the quotes; ¶4: printed as written, so two spaces stay two
        run("submit \"a  b\"\nstart \"a  b\"\n").ok("a  b running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn input__options_after_a_quoted_name() {
        // prompt ¶1: wherever a job name is expected; SPEC Script: submit options follow the name; SPEC Start order
        run("submit \"a b\" priority 7\nsubmit c\nnext\n").ok("a b running\nc pending\ntime: 0\npending: 1\n");
    }
}
