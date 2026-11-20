import Lean
import Tauten

/-!
# Axiom audit, constructor enumeration and coverage of the extraction

Adapted from writfold's `proofs/Audit.lean` and the P0 probe. Covers
`tauten_check_kernel` (P1, P2, P4, P5) and `tauten_proof_instances` (P3, the
macros' expansion of its instances).

1. Every theorem under `Tauten` depends only on Lean's standard three axioms
   (`propext`, `Classical.choice`, `Quot.sound`). This reads the transitive
   axiom closure (`collectAxioms`); an Aeneas `axiom` for an opaque function,
   a `sorry` (`sorryAx`) or a native-evaluation axiom would fail it.
2. Every required claim exists and is a theorem. `proofs/mutants.py` also
   requires every theorem a mutant names to be in this list.
3. Constructor enumeration (Plyward F01 target 5; plan section 2, binding
   3): for each refined type of `tauten_proof_instances`, every extracted
   function whose result is `Result T` with `T` mentioning the type is
   listed and must be `try_new`, `TryFrom::try_from` or the derived `clone`
   (each stated in a claim: `try_from` is `try_new`, `clone` is the
   identity). A generated or derived impl that builds a refined value any
   other way (a `Default`, say) fails the build. The control, the plain
   newtype `Tag` with `derive(Default)`, must show `new` and the derived
   `default`, or the enumeration is not seeing derived impls.
4. Every function extracted from either crate is reviewed (listed
   below) and is covered by a claim: it is mentioned in the statement of a
   required claim, or it is called (transitively, through the extracted
   definitions) by a function that is. Loop bodies and the ratchet's
   per-finding helpers are covered this way by the theorems about `apply`,
   `stale`, `count_errors` and `increases`. A function added to
   `start_from` without a theorem fails the build.

Pitfall (P0): Lean's `Name.isInternal` treats the macro's `__tauten_<name>`
module as internal, so no filter here uses it on extracted names.
-/
open Lean Elab Command Meta

def allowedAxioms : List Name := [``propext, ``Classical.choice, ``Quot.sound]

def claims : List Name := [
  ``Tauten.Kernel.rank_spec,
  ``Tauten.Kernel.may_depend_on_spec,
  ``Tauten.Kernel.may_depend_on_refl,
  ``Tauten.Kernel.may_depend_on_trans,
  ``Tauten.Kernel.may_depend_on_total,
  ``Tauten.Kernel.may_depend_on_antisymm,
  ``Tauten.Kernel.kernel_depends_only_on_kernel,
  ``Tauten.Kernel.is_blocking_spec,
  ``Tauten.Kernel.severity_spec,
  ``Tauten.Kernel.permitted_spec,
  ``Tauten.Kernel.exception_spec,
  ``Tauten.Kernel.kernel_never_excuses_levelled_rule,
  ``Tauten.Kernel.no_exceptions_never_excused,
  -- the baseline ratchet (P2)
  ``Tauten.Kernel.apply_conserves,
  ``Tauten.Kernel.apply_covered_iff,
  ``Tauten.Kernel.apply_cover_count,
  ``Tauten.Kernel.apply_suppressed_le_accepted,
  ``Tauten.Kernel.apply_left_spec,
  ``Tauten.Kernel.ratchet_stale_iff,
  ``Tauten.Kernel.ratchet_passes_iff,
  ``Tauten.Kernel.ratchet_headroom,
  ``Tauten.Kernel.count_errors_counts,
  ``Tauten.Kernel.increases_flags,
  ``Tauten.Kernel.increases_none_iff,
  ``Tauten.Kernel.regenerated_baseline_never_grows,
  ``Tauten.Kernel.ratchet_example,
  ``Tauten.Kernel.increase_example,
  -- rule-table well-formedness (P4)
  ``Tauten.Kernel.validate_ok_iff,
  ``Tauten.Kernel.validate_examples,
  -- proof subsumption of arithmetic and indexing findings (P5)
  ``Tauten.Kernel.subsumes_spec,
  ``Tauten.Kernel.stale_proof_never_subsumes,
  ``Tauten.Kernel.subsumes_only_in_kernel,
  ``Tauten.Kernel.subsumes_only_arithmetic_and_indexing,
  -- declared effects against the signature (TAU-F001)
  ``Tauten.Kernel.derive_spec,
  ``Tauten.Kernel.missing_spec,
  ``Tauten.Kernel.extra_spec,
  ``Tauten.Kernel.agrees_spec,
  ``Tauten.Kernel.f001_decision,
  ``Tauten.Kernel.f001_examples,
  ``Tauten.Kernel.derive_perm,
  ``Tauten.Kernel.verdict_spec,
  ``Tauten.Kernel.f001_error_iff,
  ``Tauten.Kernel.f001_silent_iff,
  -- the outcome of a claim (T3)
  ``Tauten.Kernel.outcome_proved_iff,
  ``Tauten.Kernel.outcome_tool_missing_iff,
  ``Tauten.Kernel.outcome_mutant_survived_iff,
  ``Tauten.Kernel.outcome_examples,
  -- the function a finding is in (T3)
  ``Tauten.Kernel.innermost_spec,
  ``Tauten.Kernel.innermost_examples,
  -- whether a set of checks passed (S2)
  ``Tauten.Kernel.control_ok_spec,
  ``Tauten.Kernel.admit_spec,
  ``Tauten.Kernel.admit_ok_iff,
  ``Tauten.Kernel.infrastructure_never_passes,
  ``Tauten.Kernel.admit_infrastructure_refused,
  ``Tauten.Kernel.named_iff,
  ``Tauten.Kernel.admit_examples,
  ``Tauten.Kernel.of_run_proved_iff,
  -- whether a recorded result is still about the current inputs (S4)
  ``Tauten.Kernel.same_bytes_spec,
  ``Tauten.Kernel.fresh_iff,
  ``Tauten.Kernel.missing_key_never_fresh,
  ``Tauten.Kernel.extra_key_never_fresh,
  ``Tauten.Kernel.changed_never_fresh,
  -- the badge a claim earns (S3)
  ``Tauten.Kernel.badge_spec,
  ``Tauten.Kernel.badge_rank_spec,
  ``Tauten.Kernel.badge_proved_iff,
  ``Tauten.Kernel.badge_proved_under_iff,
  ``Tauten.Kernel.non_lean_never_proved,
  ``Tauten.Kernel.model_only_never_proved,
  ``Tauten.Kernel.stale_never_established,
  ``Tauten.Kernel.badge_monotone,
  ``Tauten.Kernel.inherit_spec,
  ``Tauten.Kernel.inherited_badge_le,
  ``Tauten.Kernel.inherit_needs_version_and_record,
  ``Tauten.Kernel.kept_spec,
  ``Tauten.Kernel.conserves_spec,
  ``Tauten.Kernel.conserves_iff,
  ``Tauten.Kernel.conserves_refl,
  ``Tauten.Kernel.conserves_trans,
  ``Tauten.Kernel.amendment_is_explicit,
  ``Tauten.Kernel.assumptions_ok_iff,
  ``Tauten.Kernel.undeclared_assumption_refused,
  ``Tauten.Kernel.unused_assumption_refused,
  ``Tauten.Kernel.unwitnessed_refused,
  ``Tauten.Kernel.hardening_iff,
  ``Tauten.Kernel.unparsed_never_clean,
  ``Tauten.Kernel.incomplete_never_error,
  ``Tauten.Kernel.finding_severity_spec,
  ``Tauten.Kernel.unparsed_not_clean,
  ``Tauten.Kernel.clean_spec,
  ``Tauten.Kernel.fixtures_ok_spec,
  ``Tauten.Kernel.fixtures_ok_iff,
  -- exception groups, conditions and manifest tables (T4)
  ``Tauten.Kernel.worst_spec,
  ``Tauten.Kernel.decide_spec,
  ``Tauten.Kernel.conditional_never_allowed_in_kernel,
  ``Tauten.Kernel.group_never_weaker,
  ``Tauten.Kernel.decide_single,
  ``Tauten.Kernel.weakens_spec,
  ``Tauten.Kernel.loosens_spec,
  ``Tauten.Kernel.x003_iff,
  -- workspace layout rules (T6)
  ``Tauten.Kernel.t001_iff,
  ``Tauten.Kernel.t003_iff,
  ``Tauten.Kernel.k003_iff,
  ``Tauten.Kernel.conditional_no_std_is_finding,
  ``Tauten.Kernel.unparsed_root_is_finding,
  -- compiler output and the exit verdict (T5)
  ``Tauten.Kernel.kept_iff,
  ``Tauten.Kernel.applies_spec,
  ``Tauten.Kernel.passes_spec,
  ``Tauten.Kernel.check_passes_iff,
  ``Tauten.Kernel.warnings_never_fail,
  -- declared layers (B6)
  ``Tauten.Kernel.may_use_spec,
  ``Tauten.Kernel.may_use_refl,
  ``Tauten.Kernel.may_use_trans,
  ``Tauten.Kernel.may_use_antisymm,
  ``Tauten.Kernel.may_use_total,
  ``Tauten.Kernel.layer_finding_iff,
  ``Tauten.Kernel.unknown_layer_is_finding,
  -- the machine library (A6) and its worked instance
  ``Tauten.Machine.inv_of_step,
  ``Tauten.Machine.inv_run,
  ``Tauten.Machine.rinv_of_step,
  ``Tauten.Machine.rreachable_of_model,
  ``Tauten.Machine.sys_inv_of_step,
  ``Tauten.Machine.discharged_of_inv,
  ``Tauten.Instances.publish_step_model,
  ``Tauten.Instances.holds_temp_model,
  ``Tauten.Instances.terminal_model,
  ``Tauten.Instances.publish_reachable_model,
  ``Tauten.Instances.publish_inv,
  ``Tauten.Instances.publish_discharged,
  -- atomic publish under named filesystem assumptions (A7)
  ``Tauten.Machine.rsys_of_model,
  ``Tauten.Durable.publish_step_model,
  ``Tauten.Durable.publish_old_or_new,
  ``Tauten.Durable.publish_durable_on_done,
  ``Tauten.Durable.durable_model,
  ``Tauten.Durable.publish_stops_on_failure,
  ``Tauten.Durable.publish_frame,
  ``Tauten.Durable.create_new_never_replaces,
  ``Tauten.Durable.publish_create_step_model,
  ``Tauten.Durable.profile_mono,
  ``Tauten.Durable.crash_characterisation,
  ``Tauten.Durable.posix_satisfies,
  ``Tauten.Durable.posix_done_reachable,
  ``Tauten.Durable.old_or_new_needs_rename_atomic,
  ``Tauten.Durable.old_or_new_needs_sync_file,
  ``Tauten.Durable.durable_needs_sync_dir,
  -- a stamped kernel action (A5)
  ``Tauten.Instances.stale_refused,
  ``Tauten.Instances.fresh_accepted,
  -- the macros' generated code, per instance (P3)
  ``Tauten.Instances.percent_try_new_shape,
  ``Tauten.Instances.percent_try_new_ok_iff,
  ``Tauten.Instances.percent_try_new_sound,
  ``Tauten.Instances.percent_try_new_err_iff,
  ``Tauten.Instances.percent_try_from_spec,
  ``Tauten.Instances.percent_derived_spec,
  ``Tauten.Instances.percent_boundaries,
  ``Tauten.Instances.port_try_new_shape,
  ``Tauten.Instances.port_try_new_ok_iff,
  ``Tauten.Instances.port_try_new_sound,
  ``Tauten.Instances.port_try_new_err_iff,
  ``Tauten.Instances.port_try_from_spec,
  ``Tauten.Instances.port_derived_spec,
  ``Tauten.Instances.port_boundaries,
  ``Tauten.Instances.offset_try_new_shape,
  ``Tauten.Instances.offset_try_new_ok_iff,
  ``Tauten.Instances.offset_try_new_sound,
  ``Tauten.Instances.offset_try_new_err_iff,
  ``Tauten.Instances.offset_try_from_spec,
  ``Tauten.Instances.offset_derived_spec,
  ``Tauten.Instances.offset_boundaries,
  ``Tauten.Instances.count_try_new_shape,
  ``Tauten.Instances.count_try_new_ok_iff,
  ``Tauten.Instances.count_try_new_sound,
  ``Tauten.Instances.count_try_new_err_iff,
  ``Tauten.Instances.count_try_from_spec,
  ``Tauten.Instances.count_derived_spec,
  ``Tauten.Instances.count_boundaries,
  ``Tauten.Instances.tag_control_spec,
  ``Tauten.Instances.deposit_shape,
  ``Tauten.Instances.deposit_requires_iff,
  ``Tauten.Instances.deposit_never_ensures,
  ``Tauten.Instances.deposit_ok,
  ``Tauten.Instances.deposit_overflow,
  ``Tauten.Instances.withdraw_shape,
  ``Tauten.Instances.withdraw_requires_iff,
  ``Tauten.Instances.withdraw_requires_keeps_balance,
  ``Tauten.Instances.withdraw_never_ensures,
  ``Tauten.Instances.withdraw_ok,
  ``Tauten.Instances.withdraw_insufficient,
  ``Tauten.Instances.scale_shape,
  ``Tauten.Instances.scale_requires_iff,
  ``Tauten.Instances.scale_ensures_iff,
  ``Tauten.Instances.scale_ok,
  ``Tauten.Instances.scale_overflow,
  ``Tauten.Instances.share_shape,
  ``Tauten.Instances.share_requires_skips_body,
  ``Tauten.Instances.share_requires_iff,
  ``Tauten.Instances.share_ok,
  ``Tauten.Instances.contract_examples,
  -- schematic: what every expansion reduces to (P3); no extracted function
  ``Tauten.Macros.refined_ok_iff,
  ``Tauten.Macros.refined_ok_value,
  ``Tauten.Macros.refined_err_iff,
  ``Tauten.Macros.refined_total,
  ``Tauten.Macros.contract_requires,
  ``Tauten.Macros.contract_ok,
  ``Tauten.Macros.contract_err,
  ``Tauten.Macros.contract_mut_requires,
  ``Tauten.Macros.contract_mut_ok,
  ``Tauten.Macros.contract_mut_err,
]

/-- The functions translated from each crate's `[package.metadata.charon] start_from`. -/
def reviewedFunctions : List Name := [
  ``tauten_check_kernel.tiers.rank,
  ``tauten_check_kernel.tiers.may_depend_on,
  ``tauten_check_kernel.severity.is_blocking,
  ``tauten_check_kernel.severity.severity,
  ``tauten_check_kernel.exception.permitted,
  ``tauten_check_kernel.exception.excuse,
  ``tauten_check_kernel.exception.rank,
  ``tauten_check_kernel.exception.worse,
  ``tauten_check_kernel.exception.worst_loop.body,
  ``tauten_check_kernel.exception.worst_loop,
  ``tauten_check_kernel.exception.worst,
  ``tauten_check_kernel.exception.decide,
  ``tauten_check_kernel.exception.weakens,
  ``tauten_check_kernel.exception.loosens_loop.body,
  ``tauten_check_kernel.exception.loosens_loop,
  ``tauten_check_kernel.exception.loosens,
  ``tauten_check_kernel.layout.t001,
  ``tauten_check_kernel.layout.t003,
  ``tauten_check_kernel.layout.k003,
  ``tauten_check_kernel.compiler.kept,
  ``tauten_check_kernel.compiler.applies,
  ``tauten_check_kernel.compiler.passes_loop.body,
  ``tauten_check_kernel.compiler.passes_loop,
  ``tauten_check_kernel.compiler.passes,
  ``tauten_check_kernel.layer.may_use,
  ``tauten_check_kernel.layer.finding,
  ``tauten_check_kernel.ratchet.count_at,
  ``tauten_check_kernel.ratchet.take,
  ``tauten_check_kernel.ratchet.take_at,
  ``tauten_check_kernel.ratchet.apply,
  ``tauten_check_kernel.ratchet.apply_loop,
  ``tauten_check_kernel.ratchet.apply_loop.body,
  ``tauten_check_kernel.ratchet.stale,
  ``tauten_check_kernel.ratchet.stale_loop,
  ``tauten_check_kernel.ratchet.stale_loop.body,
  ``tauten_check_kernel.ratchet.zeros,
  ``tauten_check_kernel.ratchet.zeros_loop,
  ``tauten_check_kernel.ratchet.zeros_loop.body,
  ``tauten_check_kernel.ratchet.add_error,
  ``tauten_check_kernel.ratchet.add_error_at,
  ``tauten_check_kernel.ratchet.count_errors,
  ``tauten_check_kernel.ratchet.count_errors_loop,
  ``tauten_check_kernel.ratchet.count_errors_loop.body,
  ``tauten_check_kernel.ratchet.grows_at,
  ``tauten_check_kernel.ratchet.increases,
  ``tauten_check_kernel.ratchet.increases_loop,
  ``tauten_check_kernel.ratchet.increases_loop.body,
  ``tauten_check_kernel.subsume.subsumes,
  ``tauten_check_kernel.effects.empty,
  ``tauten_check_kernel.effects.add,
  ``tauten_check_kernel.effects.add_at,
  ``tauten_check_kernel.effects.derive,
  ``tauten_check_kernel.effects.derive_loop,
  ``tauten_check_kernel.effects.derive_loop.body,
  ``tauten_check_kernel.effects.missing,
  ``tauten_check_kernel.effects.extra,
  ``tauten_check_kernel.effects.is_empty,
  ``tauten_check_kernel.effects.agrees,
  ``tauten_check_kernel.effects.verdict,
  ``tauten_check_kernel.outcome.passed,
  ``tauten_check_kernel.outcome.outcome,
  ``tauten_check_kernel.span.covers,
  ``tauten_check_kernel.span.width,
  ``tauten_check_kernel.span.consider,
  ``tauten_check_kernel.span.innermost,
  ``tauten_check_kernel.span.innermost_loop,
  ``tauten_check_kernel.span.innermost_loop.body,
  ``tauten_check_kernel.control.control_ok,
  ``tauten_check_kernel.control.member_loop.body,
  ``tauten_check_kernel.control.member_loop,
  ``tauten_check_kernel.control.member,
  ``tauten_check_kernel.control.named_loop.body,
  ``tauten_check_kernel.control.named_loop,
  ``tauten_check_kernel.control.named,
  ``tauten_check_kernel.control.count_id_loop.body,
  ``tauten_check_kernel.control.count_id_loop,
  ``tauten_check_kernel.control.count_id,
  ``tauten_check_kernel.control.required_id_loop.body,
  ``tauten_check_kernel.control.required_id_loop,
  ``tauten_check_kernel.control.required_id,
  ``tauten_check_kernel.control.met_loop.body,
  ``tauten_check_kernel.control.met_loop,
  ``tauten_check_kernel.control.met,
  ``tauten_check_kernel.control.observations_ok_loop.body,
  ``tauten_check_kernel.control.observations_ok_loop,
  ``tauten_check_kernel.control.observations_ok,
  ``tauten_check_kernel.control.controls_ok_loop.body,
  ``tauten_check_kernel.control.controls_ok_loop,
  ``tauten_check_kernel.control.controls_ok,
  ``tauten_check_kernel.control.admit,
  ``tauten_check_kernel.outcome.of_run,
  ``tauten_check_kernel.badge.badge,
  ``tauten_check_kernel.badge.rank,
  ``tauten_check_kernel.badge.inherit,
  ``tauten_check_kernel.binding.same_bytes_loop.body,
  ``tauten_check_kernel.binding.same_bytes_loop,
  ``tauten_check_kernel.binding.same_bytes,
  ``tauten_check_kernel.binding.has_key_loop.body,
  ``tauten_check_kernel.binding.has_key_loop,
  ``tauten_check_kernel.binding.has_key,
  ``tauten_check_kernel.binding.agrees_loop.body,
  ``tauten_check_kernel.binding.agrees_loop,
  ``tauten_check_kernel.binding.agrees,
  ``tauten_check_kernel.binding.covered_loop.body,
  ``tauten_check_kernel.binding.covered_loop,
  ``tauten_check_kernel.binding.covered,
  ``tauten_check_kernel.binding.present_loop.body,
  ``tauten_check_kernel.binding.present_loop,
  ``tauten_check_kernel.binding.present,
  ``tauten_check_kernel.binding.fresh,
  ``tauten_check_kernel.registry.kept_loop.body,
  ``tauten_check_kernel.registry.kept_loop,
  ``tauten_check_kernel.registry.kept,
  ``tauten_check_kernel.registry.conserves_loop.body,
  ``tauten_check_kernel.registry.conserves_loop,
  ``tauten_check_kernel.registry.conserves,
  ``tauten_check_kernel.assumption.assumptions_ok,
  ``tauten_check_kernel.manifest.hardening,
  ``tauten_check_kernel.rule.finding_severity,
  ``tauten_check_kernel.rule.clean,
  ``tauten_check_kernel.rule.fixtures_ok_loop.body,
  ``tauten_check_kernel.rule.fixtures_ok_loop,
  ``tauten_check_kernel.rule.fixtures_ok,
  ``tauten_check_kernel.table.is_at,
  ``tauten_check_kernel.table.find_before,
  ``tauten_check_kernel.table.find_before_loop,
  ``tauten_check_kernel.table.find_before_loop.body,
  ``tauten_check_kernel.table.contains,
  ``tauten_check_kernel.table.repeat_at,
  ``tauten_check_kernel.table.first_duplicate,
  ``tauten_check_kernel.table.first_duplicate_loop,
  ``tauten_check_kernel.table.first_duplicate_loop.body,
  ``tauten_check_kernel.table.absent,
  ``tauten_check_kernel.table.missing_at,
  ``tauten_check_kernel.table.first_missing,
  ``tauten_check_kernel.table.first_missing_loop,
  ``tauten_check_kernel.table.first_missing_loop.body,
  ``tauten_check_kernel.table.owner_at,
  ``tauten_check_kernel.table.duplicate,
  ``tauten_check_kernel.table.validate,
  -- tauten_proof_instances (P3): the macros' expansion, the closures it
  -- generates, the derives, and the runtime helpers made transparent by
  -- `include = ["tauten::contract", "tauten::error"]`
  ``tauten_proof_instances.contracts.OpError.Insts.CoreConvertFromContractViolation.from,
  ``tauten_proof_instances.contracts.deposit,
  ``tauten_proof_instances.contracts.deposit.closure.Insts.CoreOpsFunctionFnTupleResultU64OpError.call,
  ``tauten_proof_instances.contracts.deposit.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once,
  ``tauten_proof_instances.contracts.scale,
  ``tauten_proof_instances.tauten.publish.step,
  ``tauten_proof_instances.tauten.publish.holds_temp,
  ``tauten_proof_instances.tauten.publish.terminal,
  ``tauten_proof_instances.tauten.publish.create_step,
  ``tauten_proof_instances.tauten.publish.durable,
  ``tauten_proof_instances.machines.publish_step,
  ``tauten_proof_instances.machines.publish_holds_temp,
  ``tauten_proof_instances.machines.publish_terminal,
  ``tauten_proof_instances.machines.publish_create_step,
  ``tauten_proof_instances.machines.publish_durable,
  ``tauten_proof_instances.tauten.origin.Evidence.__mint,
  ``tauten_proof_instances.tauten.origin.Evidence.redeem,
  ``tauten_proof_instances.tauten.origin.Stamped.verify,
  ``tauten_proof_instances.stamped.Counter.Insts.TautenOriginVersioned.version,
  ``tauten_proof_instances.stamped.__tauten_evidence_belowlimit.BelowLimit.impl.value,
  ``tauten_proof_instances.stamped.bump,
  ``tauten_proof_instances.tauten.origin.Versioned.version,
  ``tauten_proof_instances.contracts.scale.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call,
  ``tauten_proof_instances.contracts.scale.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U32Bool.call_once,
  ``tauten_proof_instances.contracts.share,
  ``tauten_proof_instances.contracts.share.closure.Insts.CoreOpsFunctionFnTupleResultU32OpError.call,
  ``tauten_proof_instances.contracts.withdraw,
  ``tauten_proof_instances.contracts.withdraw.closure.Insts.CoreOpsFunctionFnMutTupleResultU64OpError.call_mut,
  ``tauten_proof_instances.contracts.withdraw.closure_1.Insts.CoreOpsFunctionFnOnceTupleShared1U64Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCmpPartialEqCount.eq,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreConvertAsRefU64.as_ref,
  ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreConvertTryFromU64RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_count.Count.get,
  ``tauten_proof_instances.refined.__tauten_count.Count.into_inner,
  ``tauten_proof_instances.refined.__tauten_count.Count.try_new,
  ``tauten_proof_instances.refined.__tauten_count.Count.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U64Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCmpPartialEqOffset.eq,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreConvertAsRefI32.as_ref,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreConvertTryFromI32RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.get,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.into_inner,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.try_new,
  ``tauten_proof_instances.refined.__tauten_offset.Offset.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0I32Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCmpPartialEqPercent.eq,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreConvertAsRefU8.as_ref,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreConvertTryFromU8RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.get,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.into_inner,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.try_new,
  ``tauten_proof_instances.refined.__tauten_percent.Percent.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U8Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCmpEq.assert_fields_are_eq,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCmpPartialEqPort.eq,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreConvertAsRefU16.as_ref,
  ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreConvertTryFromU16RefineError.try_from,
  ``tauten_proof_instances.refined.__tauten_port.Port.get,
  ``tauten_proof_instances.refined.__tauten_port.Port.into_inner,
  ``tauten_proof_instances.refined.__tauten_port.Port.try_new,
  ``tauten_proof_instances.refined.__tauten_port.Port.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U16Bool.call_once,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreCloneClone.clone,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreConvertAsRefU32.as_ref,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreDefaultDefault.default,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.get,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.into_inner,
  ``tauten_proof_instances.refined.__tauten_tag.Tag.new,
  ``tauten_proof_instances.tauten.contract.holds,
  ``tauten_proof_instances.tauten.contract.snapshot,
  ``tauten_proof_instances.tauten.error.ContractViolation.ensures,
  ``tauten_proof_instances.tauten.error.ContractViolation.requires,
  ``tauten_proof_instances.tauten.error.RefineError.new,
]

def auditedTheorems (env : Environment) : Array Name :=
  let audited : Name := `Tauten
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if audited.isPrefixOf decl && decl.isInternal = false && info.isThm then acc.push decl
    else acc).qsort (fun a b => a.toString < b.toString)

def auditAxioms : CommandElabM Unit := do
  let env ← getEnv
  let sorted := auditedTheorems env
  if sorted.isEmpty then
    throwError "axiom audit: no theorems under Tauten"
  for decl in sorted do
    let axioms := (← collectAxioms decl).qsort (fun a b => a.toString < b.toString)
    logInfo m!"AXIOMS {decl}: {axioms.toList}"
    for ax in axioms do
      unless allowedAxioms.contains ax do
        throwError "axiom audit: {decl} depends on {ax}, which is not a standard Lean axiom"
  for claim in claims do
    unless sorted.contains claim do
      throwError "axiom audit: required claim {claim} is missing or is not a theorem"
  logInfo m!"AXIOM_AUDIT ok: {sorted.size} theorems, {claims.length} required claims, axioms ⊆ {allowedAxioms}"

run_cmd auditAxioms

/-! ## Named assumptions (A8)

For every audited theorem, the named assumptions (`@[tauten_assumption
"ID"]`) its hypotheses mention, through definitions under `Tauten`, are
printed as `PREMISES <theorem>: [IDs]`. A theorem with no tagged hypothesis
whose conclusion is a tagged proposition, a conjunction of them or an
existential over them (a witness that the assumptions can hold together) is
printed as `ESTABLISHES <theorem>: [IDs]`. Same derivation as the proofs
template's `Audit.lean`.
-/

/-- Whether `decl` is declared in a `Tauten` module. -/
def inTauten (env : Environment) (decl : Name) : Bool :=
  match env.getModuleIdxFor? decl with
  | some idx => (`Tauten).isPrefixOf env.header.moduleNames[idx.toNat]!
  | none => false

/-- The assumption IDs mentioned by `e`, unfolding definitions declared in
`Tauten` modules (never theorems, never other libraries). -/
def assumptionsIn (env : Environment) (e : Expr) : Array String := Id.run do
  let mut seen : NameSet := {}
  let mut todo : Array Name := e.getUsedConstants
  let mut ids : Array String := #[]
  while h : todo.size > 0 do
    let c := todo[todo.size - 1]
    todo := todo.pop
    if seen.contains c then continue
    seen := seen.insert c
    if let some id := tautenAssumptionAttr.getParam? env c then
      unless ids.contains id do ids := ids.push id
    else if inTauten env c then
      if let some (.defnInfo d) := env.find? c then
        todo := todo ++ d.value.getUsedConstants
  return ids.qsort (· < ·)

/-- The IDs a conclusion establishes: a tagged proposition, a conjunction
or an existential of such. -/
partial def establishedBy (env : Environment) (e : Expr) : MetaM (Array String) := do
  let e ← whnfR e
  if e.isAppOfArity ``And 2 then
    return (← establishedBy env e.appFn!.appArg!) ++ (← establishedBy env e.appArg!)
  if e.isAppOfArity ``Exists 2 then
    let body := e.appArg!
    if let .lam n ty b bi := body then
      return ← withLocalDecl n bi ty fun x => establishedBy env (b.instantiate1 x)
    return #[]
  let fn := e.getAppFn
  if fn.isConst then
    if let some id := tautenAssumptionAttr.getParam? env fn.constName! then
      return #[id]
  return #[]

/-- (premise IDs, established IDs) of a theorem's statement. -/
def assumptionFacts (env : Environment) (type : Expr) : MetaM (Array String × Array String) :=
  forallTelescope type fun xs body => do
    let mut premises : Array String := #[]
    for x in xs do
      let t ← inferType x
      if ← isProp t then
        for id in assumptionsIn env t do
          unless premises.contains id do premises := premises.push id
    let sorted := premises.qsort (· < ·)
    if sorted.isEmpty then
      let est := (← establishedBy env body).qsort (· < ·)
      return (sorted, est.toList.eraseDups.toArray)
    return (sorted, #[])

def auditAssumptions : CommandElabM Unit := do
  let env ← getEnv
  for decl in auditedTheorems env do
    if let some info := env.find? decl then
      let (premises, established) ← liftTermElabM (assumptionFacts env info.type)
      unless premises.isEmpty do
        logInfo m!"PREMISES {decl}: {premises.toList}"
      unless established.isEmpty do
        logInfo m!"ESTABLISHES {decl}: {established.toList}"

run_cmd auditAssumptions

/-- The crates whose translation is audited. -/
def extractedCrates : List Name := [`tauten_check_kernel, `tauten_proof_instances]

/-- Extracted functions (of the audited crates) whose result is `Result T`
with `T` mentioning `ty`, read from the unreduced signature (Aeneas emits a
refined type as a reducible `def` of its raw type, so a reduced signature
would hide it). -/
def constructorsOf (env : Environment) (ty : Name) : Array Name :=
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if extractedCrates.any (·.isPrefixOf decl) && info.isDefinition then
      let rec target : Expr → Expr
        | .forallE _ _ b _ => target b
        | e => e
      let result := target info.type
      if result.getAppFn.isConstOf ``Aeneas.Std.Result
          && (result.getUsedConstants.contains ty) then acc.push decl else acc
    else acc).qsort (fun a b => a.toString < b.toString)

/-- Refined types of `tauten_proof_instances`, with the only functions
allowed to return them: the checked constructor, `TryFrom`, which a claim
equates with it, and the derived `Clone`, which a claim shows is the
identity. -/
def refinedTypes : List (Name × List Name) := [
  (``tauten_proof_instances.refined.__tauten_percent.Percent, [
    ``tauten_proof_instances.refined.__tauten_percent.Percent.try_new,
    ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreConvertTryFromU8RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_percent.Percent.Insts.CoreCloneClone.clone]),
  (``tauten_proof_instances.refined.__tauten_port.Port, [
    ``tauten_proof_instances.refined.__tauten_port.Port.try_new,
    ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreConvertTryFromU16RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_port.Port.Insts.CoreCloneClone.clone]),
  (``tauten_proof_instances.refined.__tauten_offset.Offset, [
    ``tauten_proof_instances.refined.__tauten_offset.Offset.try_new,
    ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreConvertTryFromI32RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_offset.Offset.Insts.CoreCloneClone.clone]),
  (``tauten_proof_instances.refined.__tauten_count.Count, [
    ``tauten_proof_instances.refined.__tauten_count.Count.try_new,
    ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreConvertTryFromU64RefineError.try_from,
    ``tauten_proof_instances.refined.__tauten_count.Count.Insts.CoreCloneClone.clone]),
]

/-- The control: a plain newtype with `derive(Default)`, and the
constructors the enumeration must find for it. -/
def controlType : Name × List Name :=
  (``tauten_proof_instances.refined.__tauten_tag.Tag, [
    ``tauten_proof_instances.refined.__tauten_tag.Tag.new,
    ``tauten_proof_instances.refined.__tauten_tag.Tag.Insts.CoreDefaultDefault.default])

def auditConstructors : CommandElabM Unit := do
  let env ← getEnv
  for (ty, allowed) in refinedTypes do
    let found := constructorsOf env ty
    logInfo m!"CONSTRUCTORS {ty}: {found.toList}"
    for decl in found do
      unless allowed.contains decl do
        throwError "constructor audit: {decl} returns a {ty} and is not try_new, try_from or clone"
    for decl in allowed do
      unless found.contains decl do
        throwError "constructor audit: {decl} was not found; the enumeration is not seeing {ty}"
  let (control, expected) := controlType
  let found := constructorsOf env control
  logInfo m!"CONSTRUCTORS {control} (control): {found.toList}"
  for decl in expected do
    unless found.contains decl do
      throwError "constructor audit is vacuous: {decl} of the control was not found"
  logInfo m!"CONSTRUCTOR_AUDIT ok: {refinedTypes.length} refined types, control found {expected.length} constructors"

run_cmd auditConstructors

/-- Definitions Aeneas wrote for functions of the audited crates: the
extracted functions return `Result _`; inductive types, closure state
types, trait-instance records and generated helpers do not. -/
def extractedFunctions (env : Environment) : Array Name :=
  (env.constants.fold (init := (#[] : Array Name)) fun acc decl info =>
    if extractedCrates.any (·.isPrefixOf decl) && info.isDefinition then
      let rec target : Expr → Expr
        | .forallE _ _ b _ => target b
        | e => e
      if (target info.type).getAppFn.isConstOf ``Aeneas.Std.Result then acc.push decl else acc
    else acc).qsort (fun a b => a.toString < b.toString)

/-- The constants of the audited crates a definition's body uses: extracted
functions, and also the trait-instance records through which a function
calls a closure (`holds Inst value closure` reaches the closure's
`call_once` only through `Inst`). -/
def calls (env : Environment) (decl : Name) : Array Name :=
  match env.find? decl with
  | some info => match info.value? with
    | some value => value.getUsedConstants.filter fun c => extractedCrates.any (·.isPrefixOf c)
    | none => #[]
  | none => #[]

/-- The extracted functions stated in a claim, then everything they call. -/
def coveredByClaims (env : Environment) (found : Array Name) : Array Name := Id.run do
  let mut visited : Array Name := #[]
  let mut todo : Array Name := #[]
  for claim in claims do
    if let some info := env.find? claim then
      todo := todo ++ info.type.getUsedConstants.filter found.contains
  while h : todo.size > 0 do
    let decl := todo[todo.size - 1]
    todo := todo.pop
    unless visited.contains decl do
      visited := visited.push decl
      todo := todo ++ calls env decl
  return visited.filter found.contains

def auditCoverage : CommandElabM Unit := do
  let env ← getEnv
  let found := extractedFunctions env
  logInfo m!"EXTRACTED {found.toList}"
  let covered := coveredByClaims env found
  for decl in found do
    unless reviewedFunctions.contains decl do
      throwError "coverage audit: extracted {decl} is not in reviewedFunctions"
    unless covered.contains decl do
      throwError "coverage audit: no required claim states anything about {decl} or a caller of it"
  for decl in reviewedFunctions do
    unless found.contains decl do
      throwError "coverage audit: reviewed {decl} was not extracted"
  logInfo m!"COVERAGE ok: {found.size} extracted functions, each stated in a claim or called by one"

run_cmd auditCoverage
