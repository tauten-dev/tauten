-- A hand-written stand-in in the shape Aeneas generates (doc comments
-- naming each Rust item and its source); not a real translation.
-- [draft_kernel]
import Aeneas
open Aeneas Aeneas.Std Result ControlFlow Error
set_option linter.dupNamespace false

namespace draft_kernel

/-- [draft_kernel::alpha::step]:
    Source: 'draft-kernel/src/alpha.rs', lines 5:0-7:1
    Visibility: public -/
def alpha.step (x : Std.U32) : Result Std.U32 := do
  ok (core.num.U32.saturating_add x 1#u32)

/-- [draft_kernel::beta::step]:
    Source: 'draft-kernel/src/beta.rs', lines 5:0-7:1
    Visibility: public -/
def beta.step (x : Std.U32) : Result Std.U32 := do
  x + 2#u32

/-- [draft_kernel::add_capped]:
    Source: 'draft-kernel/src/lib.rs', lines 9:0-11:1
    Visibility: public -/
def add_capped (a : Std.U32) (b : Std.U32) : Result Std.U32 := do
  let i ← core.num.U32.MAX - b
  if a > i
  then ok core.num.U32.MAX
  else a + b

/-- [draft_kernel::clamp]:
    Source: 'draft-kernel/src/lib.rs', lines 15:0-17:1
    Visibility: public -/
def clamp (x : Std.U32) (hi : Std.U32) : Result Std.U32 := do
  if x > hi
  then ok hi
  else ok x

/-- [draft_kernel::double]:
    Source: 'draft-kernel/src/lib.rs', lines 21:0-23:1
    Visibility: public -/
def double (a : Std.U32) : Result Std.U32 := do
  a * 2#u32

/-- [draft_kernel::halve]:
    Source: 'draft-kernel/src/lib.rs', lines 27:0-29:1
    Visibility: public -/
def halve (a : Std.U32) : Result Std.U32 := do
  a / 2#u32

/-- [draft_kernel::quarter]:
    Source: 'draft-kernel/src/lib.rs', lines 34:0-36:1
    Visibility: public -/
def quarter (a : Std.U32) : Result Std.U32 := do
  let i ← halve a
  halve i

/-- [draft_kernel::count_up]: loop 0:
    Source: 'draft-kernel/src/lib.rs', lines 42:4-44:5
    Visibility: public -/
@[rust_loop]
def count_up_loop (n : Std.U32) (i : Std.U32) : Result Std.U32 := do
  if i < n
  then
    let i1 ← i + 1#u32
    count_up_loop n i1
  else ok i
partial_fixpoint

/-- [draft_kernel::count_up]:
    Source: 'draft-kernel/src/lib.rs', lines 40:0-46:1
    Visibility: public -/
def count_up (n : Std.U32) : Result Std.U32 := do
  count_up_loop n 0#u32

/-- [draft_kernel::latest]:
    Source: 'draft-kernel/src/lib.rs', lines 50:0-52:1
    Visibility: public -/
def latest (stored : Std.U32) (seen : Std.U32) : Result Std.U32 := do
  if seen >= stored
  then ok seen
  else ok stored

end draft_kernel
