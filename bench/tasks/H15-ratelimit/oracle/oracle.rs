//! Hidden acceptance oracle for H15-ratelimit. Black-box: runs the `jobq` binary.
#[cfg(test)]
mod oracle {
    include!("support/oracle_support.rs");

    fn run(script: &str) -> Outcome {
        case(&["run", "s.jobs", "--now", "0"]).file("s.jobs", script).run()
    }

    fn submits(n: usize) -> String {
        (0..n).map(|i| format!("submit j{i}\n")).collect()
    }

    #[test]
    fn base__burst_then_rate_limited() {
        // prompt ¶1: starts with `burst` tokens; each start uses one; ¶2: `start` without a token is `rate limited` (state, 4)
        let s = format!("limit 1 per 1000 burst 2\n{}start j0\nstart j1\nstart j2\n", submits(3));
        run(&s).fails(4, "error: line 7: rate limited\n");
    }

    #[test]
    fn base__tokens_refill_and_next_waits() {
        // prompt ¶1: n tokens per <ms>; ¶2: `next` does nothing without a token
        let s = format!("limit 1 per 1000 burst 1\n{}next\nnext\n", submits(2));
        run(&s).ok("j0 running\nj1 pending\ntime: 0\npending: 1\n");
        let s = format!("limit 1 per 1000 burst 1\n{}next\ntick 1000\nnext\n", submits(2));
        run(&s).ok("j0 running\nj1 running\ntime: 1000\npending: 0\n");
    }

    #[test]
    fn base__no_limit_means_no_limit() {
        // prompt ¶1: the limit applies from the `limit` event on; before it, starts are unlimited
        let s = format!("{}next\nnext\nnext\n", submits(3));
        run(&s).ok("j0 running\nj1 running\nj2 running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn sequence__many_small_ticks_equal_one_big_tick() {
        // prompt ¶1: refill is continuous and exact; ¶3: splitting a wait never changes the outcome
        let small: String = "tick 1\n".repeat(1000);
        let head = format!("limit 3 per 1000 burst 3\n{}next\nnext\nnext\n", submits(4));
        let expected = "j0 running\nj1 running\nj2 running\nj3 running\ntime: 1000\npending: 0\n";
        run(&format!("{head}tick 1000\nnext\n")).ok(expected);
        run(&format!("{head}{small}next\n")).ok(expected);
    }

    #[test]
    fn rounding__fractional_tokens_are_kept() {
        // prompt ¶1: 1 token per 3 ms, exactly: 10 ms give 3 whole tokens and a third; 2 ms more complete the 4th
        let head = format!("limit 1 per 3 burst 5\n{}", submits(9));
        let five = "next\n".repeat(5);
        let s = format!("{head}{five}tick 10\nnext\nnext\nnext\nnext\n");
        run(&s).ok("j0 running\nj1 running\nj2 running\nj3 running\nj4 running\nj5 running\nj6 running\nj7 running\nj8 pending\ntime: 10\npending: 1\n");
        let s = format!("{head}{five}tick 5\ntick 5\nnext\nnext\nnext\ntick 1\ntick 1\nnext\n");
        run(&s).ok("j0 running\nj1 running\nj2 running\nj3 running\nj4 running\nj5 running\nj6 running\nj7 running\nj8 running\ntime: 12\npending: 0\n");
    }

    #[test]
    fn sequence__backwards_adds_nothing_forwards_refills() {
        // prompt ¶3: moving time backwards adds nothing; moving it forward refills
        // half a token from 0 → 50, nothing from 50 → 0, another half from 0 → 50
        let head = format!("{}limit 1 per 100 burst 1\nnext\nat 50\nat 0\nnext\n", submits(2));
        run(&head).ok("j0 running\nj1 pending\ntime: 0\npending: 1\n");
        run(&format!("{head}at 50\nnext\n")).ok("j0 running\nj1 running\ntime: 50\npending: 0\n");
    }

    #[test]
    fn overflow__elapsed_times_rate_beyond_u64() {
        // SPEC G1: elapsed × n above the largest u64 is an intermediate; the refill result is exact
        let s = format!(
            "limit 18446744073709551615 per 18446744073709551615 burst 1\n{}next\ntick 18446744073709551000\nnext\n",
            submits(2)
        );
        run(&s).ok("j0 running\nj1 running\ntime: 18446744073709551000\npending: 0\n");
    }

    #[test]
    fn input__bad_limits() {
        // prompt ¶4: `per` and `burst` at least 1; any other shape is `bad limit` (syntax, 3)
        run("limit 1 per 0 burst 1\n").fails(3, "error: line 1: bad limit\n");
        run("limit 1 per 10 burst 0\n").fails(3, "error: line 1: bad limit\n");
        run("limit 1 every 10 burst 1\n").fails(3, "error: line 1: bad limit\n");
        run("limit 1 per 10\n").fails(3, "error: line 1: bad limit\n");
        // prompt ¶4 + SPEC G2: numbers are digits only
        run("limit 1 per -10 burst 1\n").fails(3, "error: line 1: bad limit\n");
    }

    #[test]
    fn input__zero_rate_never_refills() {
        // prompt ¶1: `<n>` may be 0: no refill
        let s = format!("limit 0 per 10 burst 1\n{}next\ntick 1000000\nnext\n", submits(2));
        run(&s).ok("j0 running\nj1 pending\ntime: 1000000\npending: 1\n");
    }

    #[test]
    fn rounding__tokens_never_exceed_the_burst() {
        // prompt ¶1: refill up to at most `b` tokens: the 5 ms waited while full add nothing, so at 10 only half a token is back
        let s = format!("limit 1 per 10 burst 1\n{}tick 5\nnext\ntick 5\nnext\n", submits(2));
        run(&s).ok("j0 running\nj1 pending\ntime: 10\npending: 1\n");
    }

    #[test]
    fn origin__a_new_limit_starts_from_its_own_burst() {
        // prompt ¶1: a later `limit` replaces the earlier one and starts again with `b` tokens; the 0.9 token refilled under
        // the old limit is not carried over, so 1 ms later only 0.1 token has been refilled
        let s = format!("limit 1 per 10 burst 1\n{}next\ntick 9\nlimit 1 per 10 burst 1\nnext\ntick 1\nnext\n", submits(3));
        run(&s).ok("j0 running\nj1 running\nj2 pending\ntime: 10\npending: 1\n");
    }

    #[test]
    fn sequence__next_with_nothing_to_start_uses_no_token() {
        // prompt ¶1: *starting* a job uses a token; SPEC Script: `next` with no startable job does nothing
        run("limit 1 per 1000 burst 1\nnext\nsubmit j0\nnext\n").ok("j0 running\ntime: 0\npending: 0\n");
    }

    #[test]
    fn overflow__largest_rate_period_and_burst() {
        // SPEC G1 with prompt ¶1: n = per = burst = the largest u64; exact refill, capped at the burst
        let m = "18446744073709551615";
        let s = format!("limit {m} per {m} burst {m}\n{}next\ntick {m}\nnext\n", submits(2));
        run(&s).ok(&format!("j0 running\nj1 running\ntime: {m}\npending: 0\n"));
    }
}
