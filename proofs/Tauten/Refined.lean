import Extracted.TautenProofInstances
import Tauten.MacroShapes

/-!
# `#[refined]` instances (P3; Plyward F01 targets 1, 2, 3, 5)

Over the Aeneas translation of `crates/tauten-proof-instances/src/refined.rs`,
which is the real expansion of `#[tauten::refined(..)]`. For each instance:

- `*_try_new_shape`: the extracted `try_new` equals
  `Tauten.Macros.refinedTryNew` at the instance's predicate, stated over
  the mathematical value (`.val`), with the exact `RefineError` the macro
  emits. Every other claim about `try_new` is proved through it.
- `*_try_new_ok_iff` (F01-1, reflection): `Ok` iff the predicate holds.
- `*_try_new_sound` (F01-2, introduction soundness and raw-value
  preservation): an `Ok` value reads back, through `get`, `into_inner` and
  `as_ref`, as the raw value given, and the raw value satisfies the
  predicate.
- `*_try_new_err_iff` (F01-3, rejection): `Err` iff the predicate is false;
  with `_ok_iff` and `refined_total` there is no third outcome (no clamping,
  no panic).
- `*_try_from_spec` (F01-5): `TryFrom::try_from` is `try_new`, so it
  constructs only through the same check. `Audit.lean` lists every
  extracted function that returns the refined type and allows only
  `try_new`, `try_from` and the derived `clone`.
- `*_derived_spec`: the derived and generated impls neither construct nor
  change a value (`clone` is the identity, `eq` compares the raw values).
- `*_boundaries`: boundary witnesses, by evaluation.
-/

open Aeneas Aeneas.Std Result
open tauten_proof_instances
open Tauten.Macros

namespace Tauten.Instances

/-! ## `Percent(u8)`, `|v| *v <= 100` -/

/-- `Percent::try_new` is the refined shape at `v ≤ 100`. -/
theorem percent_try_new_shape (x : Std.U8) :
    refined.__tauten_percent.Percent.try_new x
      = refinedTryNew (fun v : Std.U8 => decide (v.val ≤ 100))
          ({ type_name := toStr "Percent", predicate := toStr "| v | * v <= 100" }
            : tauten.error.RefineError) x := by
  unfold refined.__tauten_percent.Percent.try_new tauten.contract.holds refinedTryNew
  simp only [refined.__tauten_percent.Percent.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U8Bool.call_once,
    tauten.error.RefineError.new]
  by_cases h : x.val ≤ 100
  · have : x ≤ 100#u8 := by scalar_tac
    simp [this, h]
  · have : ¬ x ≤ 100#u8 := by scalar_tac
    simp [this, h]

theorem percent_try_new_ok_iff (x : Std.U8) :
    (∃ p, refined.__tauten_percent.Percent.try_new x = ok (.Ok p)) ↔ x.val ≤ 100 := by
  rw [percent_try_new_shape]; simpa using refined_ok_iff (fun v : Std.U8 => decide (v.val ≤ 100)) _ x

theorem percent_try_new_sound (x : Std.U8) (p : refined.__tauten_percent.Percent)
    (h : refined.__tauten_percent.Percent.try_new x = ok (.Ok p)) :
    refined.__tauten_percent.Percent.get p = ok x ∧
    refined.__tauten_percent.Percent.into_inner p = ok x ∧
    refined.__tauten_percent.Percent.Insts.CoreConvertAsRefU8.as_ref p = ok x ∧
    x.val ≤ 100 := by
  rw [percent_try_new_shape] at h
  obtain ⟨rfl, hp⟩ := refined_ok_value _ _ _ _ h
  simp_all [refined.__tauten_percent.Percent.get, refined.__tauten_percent.Percent.into_inner,
    refined.__tauten_percent.Percent.Insts.CoreConvertAsRefU8.as_ref]

theorem percent_try_new_err_iff (x : Std.U8) :
    (∃ e, refined.__tauten_percent.Percent.try_new x = ok (.Err e)) ↔ 100 < x.val := by
  rw [percent_try_new_shape]; simpa using refined_err_iff (fun v : Std.U8 => decide (v.val ≤ 100)) _ x

theorem percent_try_from_spec (x : Std.U8) :
    refined.__tauten_percent.Percent.Insts.CoreConvertTryFromU8RefineError.try_from x
      = refined.__tauten_percent.Percent.try_new x := by
  simp [refined.__tauten_percent.Percent.Insts.CoreConvertTryFromU8RefineError.try_from]

theorem percent_derived_spec (p q : refined.__tauten_percent.Percent) :
    refined.__tauten_percent.Percent.Insts.CoreCloneClone.clone p = ok p ∧
    refined.__tauten_percent.Percent.Insts.CoreCmpPartialEqPercent.eq p q = ok (decide (p = q)) ∧
    refined.__tauten_percent.Percent.Insts.CoreCmpEq.assert_fields_are_eq p = ok () := by
  simp [refined.__tauten_percent.Percent.Insts.CoreCloneClone.clone,
    refined.__tauten_percent.Percent.Insts.CoreCmpPartialEqPercent.eq,
    refined.__tauten_percent.Percent.Insts.CoreCmpEq.assert_fields_are_eq]

theorem percent_boundaries :
    refined.__tauten_percent.Percent.try_new 0#u8 = ok (.Ok 0#u8) ∧
    refined.__tauten_percent.Percent.try_new 100#u8 = ok (.Ok 100#u8) ∧
    (∃ e, refined.__tauten_percent.Percent.try_new 101#u8 = ok (.Err e)) ∧
    (∃ e, refined.__tauten_percent.Percent.try_new 255#u8 = ok (.Err e)) := by
  simp [percent_try_new_shape, refinedTryNew]

/-! ## `Port(u16)`, `|p| *p >= 1024 && *p <= 49151` -/

/-- `Port::try_new` is the refined shape at its predicate. -/
theorem port_try_new_shape (x : Std.U16) :
    refined.__tauten_port.Port.try_new x
      = refinedTryNew (fun v : Std.U16 => decide (1024 ≤ v.val ∧ v.val ≤ 49151))
          ({ type_name := toStr "Port", predicate := toStr "| p | * p >= 1024 && * p <= 49151" }
            : tauten.error.RefineError) x := by
  unfold refined.__tauten_port.Port.try_new tauten.contract.holds refinedTryNew
  simp only [refined.__tauten_port.Port.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U16Bool.call_once,
    tauten.error.RefineError.new]
  by_cases h1 : 1024 ≤ x.val
  · have a : x >= 1024#u16 := by scalar_tac
    by_cases h2 : x.val ≤ 49151
    · have b : x <= 49151#u16 := by scalar_tac
      simp [a, b, h1, h2]
    · have b : ¬ x <= 49151#u16 := by scalar_tac
      simp [a, b, h1, h2]
  · have a : ¬ x >= 1024#u16 := by scalar_tac
    simp [a, h1]

theorem port_try_new_ok_iff (x : Std.U16) :
    (∃ p, refined.__tauten_port.Port.try_new x = ok (.Ok p)) ↔ 1024 ≤ x.val ∧ x.val ≤ 49151 := by
  rw [port_try_new_shape]; simpa using refined_ok_iff (fun v : Std.U16 => decide (1024 ≤ v.val ∧ v.val ≤ 49151)) _ x

theorem port_try_new_sound (x : Std.U16) (p : refined.__tauten_port.Port)
    (h : refined.__tauten_port.Port.try_new x = ok (.Ok p)) :
    refined.__tauten_port.Port.get p = ok x ∧
    refined.__tauten_port.Port.into_inner p = ok x ∧
    refined.__tauten_port.Port.Insts.CoreConvertAsRefU16.as_ref p = ok x ∧
    (1024 ≤ x.val ∧ x.val ≤ 49151) := by
  rw [port_try_new_shape] at h
  obtain ⟨rfl, hp⟩ := refined_ok_value _ _ _ _ h
  simp_all [refined.__tauten_port.Port.get, refined.__tauten_port.Port.into_inner,
    refined.__tauten_port.Port.Insts.CoreConvertAsRefU16.as_ref]

theorem port_try_new_err_iff (x : Std.U16) :
    (∃ e, refined.__tauten_port.Port.try_new x = ok (.Err e)) ↔ x.val < 1024 ∨ 49151 < x.val := by
  rw [port_try_new_shape, refined_err_iff]
  simp only [decide_eq_false_iff_not]
  omega

theorem port_try_from_spec (x : Std.U16) :
    refined.__tauten_port.Port.Insts.CoreConvertTryFromU16RefineError.try_from x
      = refined.__tauten_port.Port.try_new x := by
  simp [refined.__tauten_port.Port.Insts.CoreConvertTryFromU16RefineError.try_from]

theorem port_derived_spec (p q : refined.__tauten_port.Port) :
    refined.__tauten_port.Port.Insts.CoreCloneClone.clone p = ok p ∧
    refined.__tauten_port.Port.Insts.CoreCmpPartialEqPort.eq p q = ok (decide (p = q)) ∧
    refined.__tauten_port.Port.Insts.CoreCmpEq.assert_fields_are_eq p = ok () := by
  simp [refined.__tauten_port.Port.Insts.CoreCloneClone.clone,
    refined.__tauten_port.Port.Insts.CoreCmpPartialEqPort.eq,
    refined.__tauten_port.Port.Insts.CoreCmpEq.assert_fields_are_eq]

theorem port_boundaries :
    (∃ e, refined.__tauten_port.Port.try_new 0#u16 = ok (.Err e)) ∧
    (∃ e, refined.__tauten_port.Port.try_new 1023#u16 = ok (.Err e)) ∧
    refined.__tauten_port.Port.try_new 1024#u16 = ok (.Ok 1024#u16) ∧
    refined.__tauten_port.Port.try_new 49151#u16 = ok (.Ok 49151#u16) ∧
    (∃ e, refined.__tauten_port.Port.try_new 49152#u16 = ok (.Err e)) ∧
    (∃ e, refined.__tauten_port.Port.try_new 65535#u16 = ok (.Err e)) := by
  simp [port_try_new_shape, refinedTryNew]

/-! ## `Offset(i32)`, `|x| *x >= -1000 && *x <= 1000` -/

/-- `Offset::try_new` is the refined shape at its predicate. -/
theorem offset_try_new_shape (x : Std.I32) :
    refined.__tauten_offset.Offset.try_new x
      = refinedTryNew (fun v : Std.I32 => decide (-1000 ≤ v.val ∧ v.val ≤ 1000))
          ({ type_name := toStr "Offset", predicate := toStr "| x | * x >= - 1000 && * x <= 1000" }
            : tauten.error.RefineError) x := by
  unfold refined.__tauten_offset.Offset.try_new tauten.contract.holds refinedTryNew
  simp only [refined.__tauten_offset.Offset.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0I32Bool.call_once,
    tauten.error.RefineError.new]
  by_cases h1 : (-1000) ≤ x.val
  · have a : x >= (-1000)#i32 := by scalar_tac
    by_cases h2 : x.val ≤ 1000
    · have b : x <= 1000#i32 := by scalar_tac
      simp [a, b, h1, h2]
    · have b : ¬ x <= 1000#i32 := by scalar_tac
      simp [a, b, h1, h2]
  · have a : ¬ x >= (-1000)#i32 := by scalar_tac
    simp [a, h1]

theorem offset_try_new_ok_iff (x : Std.I32) :
    (∃ p, refined.__tauten_offset.Offset.try_new x = ok (.Ok p)) ↔ -1000 ≤ x.val ∧ x.val ≤ 1000 := by
  rw [offset_try_new_shape]; simpa using refined_ok_iff (fun v : Std.I32 => decide (-1000 ≤ v.val ∧ v.val ≤ 1000)) _ x

theorem offset_try_new_sound (x : Std.I32) (p : refined.__tauten_offset.Offset)
    (h : refined.__tauten_offset.Offset.try_new x = ok (.Ok p)) :
    refined.__tauten_offset.Offset.get p = ok x ∧
    refined.__tauten_offset.Offset.into_inner p = ok x ∧
    refined.__tauten_offset.Offset.Insts.CoreConvertAsRefI32.as_ref p = ok x ∧
    (-1000 ≤ x.val ∧ x.val ≤ 1000) := by
  rw [offset_try_new_shape] at h
  obtain ⟨rfl, hp⟩ := refined_ok_value _ _ _ _ h
  simp_all [refined.__tauten_offset.Offset.get, refined.__tauten_offset.Offset.into_inner,
    refined.__tauten_offset.Offset.Insts.CoreConvertAsRefI32.as_ref]

theorem offset_try_new_err_iff (x : Std.I32) :
    (∃ e, refined.__tauten_offset.Offset.try_new x = ok (.Err e)) ↔ x.val < -1000 ∨ 1000 < x.val := by
  rw [offset_try_new_shape, refined_err_iff]
  simp only [decide_eq_false_iff_not]
  omega

theorem offset_try_from_spec (x : Std.I32) :
    refined.__tauten_offset.Offset.Insts.CoreConvertTryFromI32RefineError.try_from x
      = refined.__tauten_offset.Offset.try_new x := by
  simp [refined.__tauten_offset.Offset.Insts.CoreConvertTryFromI32RefineError.try_from]

theorem offset_derived_spec (p q : refined.__tauten_offset.Offset) :
    refined.__tauten_offset.Offset.Insts.CoreCloneClone.clone p = ok p ∧
    refined.__tauten_offset.Offset.Insts.CoreCmpPartialEqOffset.eq p q = ok (decide (p = q)) ∧
    refined.__tauten_offset.Offset.Insts.CoreCmpEq.assert_fields_are_eq p = ok () := by
  simp [refined.__tauten_offset.Offset.Insts.CoreCloneClone.clone,
    refined.__tauten_offset.Offset.Insts.CoreCmpPartialEqOffset.eq,
    refined.__tauten_offset.Offset.Insts.CoreCmpEq.assert_fields_are_eq]

theorem offset_boundaries :
    (∃ e, refined.__tauten_offset.Offset.try_new (-2147483648)#i32 = ok (.Err e)) ∧
    (∃ e, refined.__tauten_offset.Offset.try_new (-1001)#i32 = ok (.Err e)) ∧
    refined.__tauten_offset.Offset.try_new (-1000)#i32 = ok (.Ok (-1000)#i32) ∧
    refined.__tauten_offset.Offset.try_new 0#i32 = ok (.Ok 0#i32) ∧
    refined.__tauten_offset.Offset.try_new 1000#i32 = ok (.Ok 1000#i32) ∧
    (∃ e, refined.__tauten_offset.Offset.try_new 1001#i32 = ok (.Err e)) ∧
    (∃ e, refined.__tauten_offset.Offset.try_new 2147483647#i32 = ok (.Err e)) := by
  simp [offset_try_new_shape, refinedTryNew]

/-! ## `Count(u64)`, `|n| *n != 0` -/

/-- `Count::try_new` is the refined shape at its predicate. -/
theorem count_try_new_shape (x : Std.U64) :
    refined.__tauten_count.Count.try_new x
      = refinedTryNew (fun v : Std.U64 => decide (v.val ≠ 0))
          ({ type_name := toStr "Count", predicate := toStr "| n | * n != 0" }
            : tauten.error.RefineError) x := by
  unfold refined.__tauten_count.Count.try_new tauten.contract.holds refinedTryNew
  simp only [refined.__tauten_count.Count.try_new.closure.Insts.CoreOpsFunctionFnOnceTupleShared0U64Bool.call_once,
    tauten.error.RefineError.new]
  by_cases h : x.val = 0
  · have a : x = 0#u64 := by scalar_tac
    simp [a]
  · have a : x ≠ 0#u64 := by scalar_tac
    simp [a, h]

theorem count_try_new_ok_iff (x : Std.U64) :
    (∃ p, refined.__tauten_count.Count.try_new x = ok (.Ok p)) ↔ x.val ≠ 0 := by
  rw [count_try_new_shape]; simpa using refined_ok_iff (fun v : Std.U64 => decide (v.val ≠ 0)) _ x

theorem count_try_new_sound (x : Std.U64) (p : refined.__tauten_count.Count)
    (h : refined.__tauten_count.Count.try_new x = ok (.Ok p)) :
    refined.__tauten_count.Count.get p = ok x ∧
    refined.__tauten_count.Count.into_inner p = ok x ∧
    refined.__tauten_count.Count.Insts.CoreConvertAsRefU64.as_ref p = ok x ∧
    (x.val ≠ 0) := by
  rw [count_try_new_shape] at h
  obtain ⟨rfl, hp⟩ := refined_ok_value _ _ _ _ h
  simp_all [refined.__tauten_count.Count.get, refined.__tauten_count.Count.into_inner,
    refined.__tauten_count.Count.Insts.CoreConvertAsRefU64.as_ref]

theorem count_try_new_err_iff (x : Std.U64) :
    (∃ e, refined.__tauten_count.Count.try_new x = ok (.Err e)) ↔ x.val = 0 := by
  rw [count_try_new_shape]
  simpa using refined_err_iff (fun v : Std.U64 => decide (v.val ≠ 0)) _ x

theorem count_try_from_spec (x : Std.U64) :
    refined.__tauten_count.Count.Insts.CoreConvertTryFromU64RefineError.try_from x
      = refined.__tauten_count.Count.try_new x := by
  simp [refined.__tauten_count.Count.Insts.CoreConvertTryFromU64RefineError.try_from]

theorem count_derived_spec (p q : refined.__tauten_count.Count) :
    refined.__tauten_count.Count.Insts.CoreCloneClone.clone p = ok p ∧
    refined.__tauten_count.Count.Insts.CoreCmpPartialEqCount.eq p q = ok (decide (p = q)) ∧
    refined.__tauten_count.Count.Insts.CoreCmpEq.assert_fields_are_eq p = ok () := by
  simp [refined.__tauten_count.Count.Insts.CoreCloneClone.clone,
    refined.__tauten_count.Count.Insts.CoreCmpPartialEqCount.eq,
    refined.__tauten_count.Count.Insts.CoreCmpEq.assert_fields_are_eq]

theorem count_boundaries :
    (∃ e, refined.__tauten_count.Count.try_new 0#u64 = ok (.Err e)) ∧
    refined.__tauten_count.Count.try_new 1#u64 = ok (.Ok 1#u64) ∧
    refined.__tauten_count.Count.try_new 18446744073709551615#u64 = ok (.Ok 18446744073709551615#u64) := by
  simp [count_try_new_shape, refinedTryNew]

/-! ## Control: `#[newtype] #[derive(Default)] struct Tag(u32)`

Not refined, so `new` and a derived `Default` may construct it. The
constructor audit must find both, or it is not looking at derived impls. -/

theorem tag_control_spec (v : Std.U32) (g : refined.__tauten_tag.Tag) :
    refined.__tauten_tag.Tag.new v = ok v ∧
    refined.__tauten_tag.Tag.Insts.CoreDefaultDefault.default = ok 0#u32 ∧
    refined.__tauten_tag.Tag.get g = ok g ∧
    refined.__tauten_tag.Tag.into_inner g = ok g ∧
    refined.__tauten_tag.Tag.Insts.CoreConvertAsRefU32.as_ref g = ok g ∧
    refined.__tauten_tag.Tag.Insts.CoreCloneClone.clone g = ok g := by
  simp [refined.__tauten_tag.Tag.new, refined.__tauten_tag.Tag.Insts.CoreDefaultDefault.default,
    refined.__tauten_tag.Tag.get, refined.__tauten_tag.Tag.into_inner,
    refined.__tauten_tag.Tag.Insts.CoreConvertAsRefU32.as_ref,
    refined.__tauten_tag.Tag.Insts.CoreCloneClone.clone, core.default.DefaultU32.default]
  rfl

end Tauten.Instances
