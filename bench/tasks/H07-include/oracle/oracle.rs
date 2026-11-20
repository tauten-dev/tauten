//! Hidden acceptance oracle for H07-include. Black-box: runs the `kvconf` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    #[test]
    fn base__include_in_the_same_directory() {
        case(&["dump", "conf/main.conf"])
            .file("conf/main.conf", "a = 1\ninclude \"b.conf\"\n")
            .file("conf/b.conf", "b = 2\n")
            .run()
            .ok("a = 1\nb = 2\n"); // prompt ¶2 (relative to the including file's directory); SPEC dump
    }

    #[test]
    fn base__sections_around_an_include() {
        case(&["dump", "conf/main.conf"])
            .file("conf/main.conf", "[s]\nx = 1\ninclude \"b.conf\"\ny = 2\n")
            .file("conf/b.conf", "z = 3\n")
            .run()
            .ok("s.x = 1\ns.y = 2\nz = 3\n"); // prompt ¶2 (included file starts with no section; section continues after)
    }

    #[test]
    fn base__errors_name_the_included_file() {
        case(&["check", "conf/main.conf"])
            .file("conf/main.conf", "include \"sub/b.conf\"\n")
            .file("conf/sub/b.conf", "\nx = oops\n")
            .run()
            .fails(1, "error: sub/b.conf: line 2: bad value `oops`\n"); // prompt ¶2 (path relative to the root); SPEC Errors
        case(&["check", "main.conf"])
            .file("main.conf", "a = 1\ninclude \"b.conf\"\n")
            .file("b.conf", "a = 2\n")
            .run()
            .fails(1, "error: b.conf: line 1: duplicate key `a`\n"); // prompt ¶2 (a key in two files is a duplicate)
    }

    #[test]
    fn authority__nested_includes_from_another_directory() {
        // The current directory holds decoys with the same relative names.
        case(&["get", "../conf/main.conf", "deep"])
            .file("conf/main.conf", "include \"sub/b.conf\"\n")
            .file("conf/sub/b.conf", "include \"c.conf\"\n")
            .file("conf/sub/c.conf", "deep = true\n")
            .file("work/sub/b.conf", "deep = false\n")
            .file("work/c.conf", "deep = false\n")
            .cwd("work")
            .run()
            .ok("true\n"); // prompt ¶2 (relative to the containing file); SPEC G4 (never relative to the current directory)
    }

    #[test]
    fn authority__parent_inside_the_root_is_allowed() {
        case(&["get", "conf/main.conf", "top"])
            .file("conf/main.conf", "include \"sub/b.conf\"\n")
            .file("conf/sub/b.conf", "include \"../top.conf\"\n")
            .file("conf/top.conf", "top = 1\n")
            .run()
            .ok("1\n"); // prompt ¶3 (only paths leading outside the root are refused)
    }

    #[test]
    fn authority__paths_outside_the_root() {
        case(&["check", "conf/main.conf"])
            .file("conf/main.conf", "a = 1\ninclude \"../outside.conf\"\n")
            .file("outside.conf", "b = 2\n")
            .run()
            .fails(1, "error: main.conf: line 2: include outside the configuration root\n"); // prompt ¶3; SPEC G4
        case(&["check", "conf/main.conf"])
            .file("conf/main.conf", "include \"sub/../../outside.conf\"\n")
            .file("outside.conf", "b = 2\n")
            .run()
            .fails(1, "error: main.conf: line 1: include outside the configuration root\n"); // prompt ¶3 (from the path text alone, `..` resolved lexically); SPEC G4
        case(&["check", "conf/main.conf"])
            .file("conf/main.conf", "include \"/dev/null\"\n")
            .run()
            .fails(1, "error: main.conf: line 1: include outside the configuration root\n"); // prompt ¶3 (absolute)
    }

    #[test]
    fn sequence__cycles() {
        case(&["check", "a.conf"])
            .file("a.conf", "x = 1\ninclude \"b.conf\"\n")
            .file("b.conf", "include \"a.conf\"\n")
            .run()
            .fails(1, "error: b.conf: line 1: include cycle\n"); // prompt ¶3 (through others); SPEC G5
        case(&["check", "a.conf"])
            .file("a.conf", "x = 1\ninclude \"./a.conf\"\n")
            .run()
            .fails(1, "error: a.conf: line 2: include cycle\n"); // prompt ¶3 (a file including itself); SPEC G5
    }

    #[test]
    fn sequence__depth_limit() {
        let chain = |last: &str| {
            let mut c = case(&["get", "main.conf", "k"]).file("main.conf", "include \"f1.conf\"\n");
            for i in 1..8 {
                c = c.file(&format!("f{i}.conf"), format!("include \"f{}.conf\"\n", i + 1));
            }
            c.file("f8.conf", last).file("f9.conf", "k = 9\n").run()
        };
        chain("k = 8\n").ok("8\n"); // prompt ¶3 (8 includes below the named file is allowed)
        chain("include \"f9.conf\"\n").fails(1, "error: f8.conf: line 1: include too deep\n"); // prompt ¶3 (a ninth is too deep)
    }

    #[test]
    fn input__missing_file() {
        case(&["check", "main.conf"])
            .file("main.conf", "include \"nope.conf\"\n")
            .run()
            .fails(1, "error: main.conf: line 1: cannot read `nope.conf`\n"); // prompt ¶3 (cannot be read)
    }

    #[test]
    fn sequence__the_same_file_on_two_branches_is_no_cycle() {
        case(&["check", "main.conf"])
            .file("main.conf", "include \"b.conf\"\ninclude \"c.conf\"\n")
            .file("b.conf", "b = 1\ninclude \"d.conf\"\n")
            .file("c.conf", "c = 1\ninclude \"d.conf\"\n")
            .file("d.conf", "# shared\n")
            .run()
            .ok("ok: 2 keys\n"); // prompt ¶3 (a cycle is a file already being read further up the chain); SPEC check
    }

    #[test]
    fn base__sections_inside_an_included_file() {
        case(&["dump", "main.conf"])
            .file("main.conf", "[s]\nx = 1\ninclude \"b.conf\"\ny = 2\n")
            .file("b.conf", "[t]\nz = 3\n")
            .run()
            .ok("s.x = 1\ns.y = 2\nt.z = 3\n"); // prompt ¶2 (after the include line, the including file's section continues); SPEC dump
    }

    #[test]
    fn authority__nested_error_names_the_path_from_the_root() {
        case(&["check", "conf/main.conf"])
            .file("conf/main.conf", "include \"sub/b.conf\"\n")
            .file("conf/sub/b.conf", "include \"c.conf\"\n")
            .file("conf/sub/c.conf", "x = oops\n")
            .run()
            .fails(1, "error: sub/c.conf: line 1: bad value `oops`\n"); // prompt ¶2 (relative to the containing file; errors name the path relative to the root)
    }

    #[test]
    fn base__set_keeps_include_lines() {
        case(&["set", "main.conf", "a", "2"])
            .file("main.conf", "include \"b.conf\"\na = 1\n")
            .file("b.conf", "b = 1\n")
            .run()
            .ok("include \"b.conf\"\na = 2\n"); // prompt ¶4; SPEC set
    }
}
