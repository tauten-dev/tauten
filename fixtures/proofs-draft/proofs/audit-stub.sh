#!/bin/sh
# A hand-written stand-in for the result lines of a gate run up to the
# audit (tools, extract, drift, build, audit; no mutants), so the drafting
# test needs no Docker. No Lean is built here, and nothing is proved: the
# lines are what `proofs/gate.py` would print for theorems with these
# subjects, shapes and premises. Set DRAFT_STUB to vary them in tests.
p() { printf 'TAUTEN-PROOFS %s\n' "$1"; }
p '{"stage":"tools","status":"ok"}'
if [ "${DRAFT_STUB:-}" = "missing" ]; then
    p '{"stage":"tools","status":"missing","detail":"docker is not installed"}'
    exit 3
fi
p '{"stage":"extract","status":"ok"}'
if [ "${DRAFT_STUB:-}" = "drift" ]; then
    p '{"stage":"drift","status":"fail","detail":"differs: DraftKernel.lean; run sh proofs/refresh.sh"}'
    exit 1
fi
p '{"stage":"drift","status":"ok"}'
p '{"stage":"build","status":"ok"}'
K=Proofs.DraftKernel
for t in add_capped_spec add_capped_comm clamp_spec double_spec halve_spec count_up_spec \
         count_up_loop_spec alpha_step_spec beta_step_spec latest_is_later_reading \
         latest_idempotent latest_with_clock latest_total constant_counter_is_monotone; do
    p "{\"theorem\":\"$K.$t\",\"status\":\"ok\"}"
done
subject() { p "{\"theorem\":\"$K.$1\",\"subject\":\"draft_kernel.$2\"}"; }
subject add_capped_spec add_capped
subject add_capped_comm add_capped
subject clamp_spec clamp
subject double_spec double
subject halve_spec halve
subject count_up_spec count_up
subject count_up_loop_spec count_up_loop
subject alpha_step_spec alpha.step
subject beta_step_spec beta.step
subject latest_is_later_reading latest
subject latest_idempotent latest
subject latest_with_clock latest
subject latest_total latest
if [ "${DRAFT_STUB:-}" = "ambiguous" ]; then
    subject clamp_spec double
fi
total() { p "{\"total\":\"$K.$1\",\"function\":\"draft_kernel.$2\"}"; }
total add_capped_spec add_capped
total clamp_spec clamp
total halve_spec halve
total count_up_loop_spec count_up_loop
total alpha_step_spec alpha.step
total latest_total latest
for t in add_capped_spec add_capped_comm clamp_spec double_spec halve_spec count_up_spec \
         count_up_loop_spec alpha_step_spec beta_step_spec latest_total constant_counter_is_monotone; do
    p "{\"theorem\":\"$K.$t\",\"premises\":[]}"
done
p "{\"theorem\":\"$K.latest_is_later_reading\",\"premises\":[\"FIXTURE.counter-monotone\"]}"
p "{\"theorem\":\"$K.latest_idempotent\",\"premises\":[\"FIXTURE.counter-monotone\"]}"
p "{\"theorem\":\"$K.latest_with_clock\",\"premises\":[\"FIXTURE.clock-sane\"]}"
p "{\"theorem\":\"$K.constant_counter_is_monotone\",\"establishes\":[\"FIXTURE.counter-monotone\"]}"
p '{"stage":"audit","status":"ok"}'
p '{"stage":"mutants","status":"ok"}'
