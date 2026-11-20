//! Hidden acceptance oracle for K03-depends (calibration). Black-box: runs the `jobq` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    fn run(script: &str) -> Outcome {
        case(&["run", "s.jobs", "--now", "0"]).file("s.jobs", script).run()
    }

    #[test]
    fn base__dependency_blocks_start() {
        // prompt ¶1: a job may not start until each dependency is done; ¶3: existing `cannot start` (state, 4)
        run("submit a\nsubmit b\nafter b a\nstart b\n").fails(4, "error: line 4: job `b` cannot start\n");
    }

    #[test]
    fn base__starts_once_dependency_is_done() {
        // prompt ¶1
        run("submit a\nsubmit b\nafter b a\nstart a\nfinish a\nstart b\n").ok("a done\nb running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn base__next_skips_blocked_jobs() {
        // prompt ¶3: `next` skips jobs whose dependencies are not all done (SPEC Start order would pick b)
        run("submit b priority 9\nsubmit a\nafter b a\nnext\n").ok("b pending\na running\ntime: 0\npending: 1\n");
    }

    #[test]
    fn sequence__cycles_are_refused() {
        // prompt ¶2: depending on itself directly or through other jobs is `dependency cycle` (state, 4)
        run("submit a\nafter a a\n").fails(4, "error: line 2: dependency cycle\n");
        run("submit a\nsubmit b\nafter a b\nafter b a\n").fails(4, "error: line 4: dependency cycle\n");
        run("submit a\nsubmit b\nsubmit c\nafter b a\nafter c b\nafter a c\n").fails(4, "error: line 6: dependency cycle\n");
    }

    #[test]
    fn sequence__repeated_dependency_has_no_further_effect() {
        // prompt ¶1: adding the same dependency twice has no further effect
        run("submit a\nsubmit b\nafter b a\nafter b a\nstart a\nfinish a\nstart b\n").ok("a done\nb running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn sequence__failure_spreads_through_dependants() {
        // prompt ¶4: every still-pending job depending on a failed job, directly or through others, fails
        run("submit a\nsubmit b\nsubmit c\nsubmit d\nafter b a\nafter c b\nstart a\nfail a\n")
            .ok("a failed\nb failed\nc failed\nd pending\ntime: 0\npending: 1\n");
    }

    #[test]
    fn sequence__retried_failure_does_not_spread() {
        // prompt ¶4: only `failed` spreads; a retried job is pending again (SPEC Failing)
        run("submit a retry 10\nsubmit b\nafter b a\nstart a\nfail a\n").ok("a pending (ready at 10)\nb pending\ntime: 0\npending: 2\n");
    }

    #[test]
    fn input__unknown_jobs_and_missing_names() {
        // prompt ¶1: both names must be jobs: existing `unknown job` (state, 4), first unknown name from the left
        run("submit a\nafter a ghost\n").fails(4, "error: line 2: unknown job `ghost`\n");
        run("submit a\nafter ghost a\n").fails(4, "error: line 2: unknown job `ghost`\n");
        // SPEC Errors: missing job name (syntax, 3)
        run("submit a\nafter a\n").fails(3, "error: line 2: missing job name\n");
    }

    #[test]
    fn hang__many_paths_between_two_jobs() {
        // SPEC G5 + G7: a chain of 40 diamonds has 2^40 paths; the run must still end quickly
        let mut s = String::from("submit x0\nsubmit y\n");
        for i in 0..40 {
            s.push_str(&format!(
                "submit l{i}\nsubmit r{i}\nsubmit x{n}\nafter l{i} x{i}\nafter r{i} x{i}\nafter x{n} l{i}\nafter x{n} r{i}\n",
                n = i + 1
            ));
        }
        // prompt ¶2: y does not depend on x40, so this is not a cycle
        s.push_str("after y x40\n");
        let out = run(&s);
        out.status(0);
        assert!(out.stdout.ends_with("time: 0\npending: 122\n"), "{}", out.stdout);
    }

    #[test]
    fn sequence__every_dependency_must_be_done() {
        // prompt ¶1: any number of dependencies; `c` waits for both `a` and `b`; ¶3: `cannot start` (state, 4)
        run("submit a\nsubmit b\nsubmit c\nafter c a\nafter c b\nstart a\nfinish a\nstart c\n")
            .fails(4, "error: line 8: job `c` cannot start\n");
    }
}
