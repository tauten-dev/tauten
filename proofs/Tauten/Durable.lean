import Extracted.TautenProofInstances
import Tauten.Machine
import Tauten.TautenAssumption

/-!
# Atomic publish, proved: old-or-new and durable under named FS laws (A7)

The publish machine (`tauten::publish::step`, and `create_step` for a
create-new publish) is run against a small filesystem model with crashes.
Each filesystem guarantee is a switchable flag of a `Profile`, and each is a
named assumption (`@[tauten_assumption]`), never an axiom: a theorem that
needs it takes it as a hypothesis, and the audit derives which IDs each
theorem rests on.

The model. One target name and one temporary name. Each name has a visible
directory entry and a durable one, and the new file has a visible content
and a durable content (the old file is durable from the start). A crash
keeps, for each entry, its durable value or its pending (visible) value
(`Crash`, `crash_characterisation`). Every command may fail, leaving partial
effects; a profile without a guarantee admits more outcomes:
* without `FS.rename-atomic`, a rename may leave the target unlinked;
* without `FS.sync-file`, a reported file sync may not have synced;
* without `FS.sync-dir`, a reported directory sync may not have synced;
* without `FS.exclusive-create`, a no-replace rename may still replace.

Proved here (theorems stated over the extracted step functions):
* `publish_old_or_new` (under `FS.rename-atomic`, `FS.sync-file`): in every
  reachable configuration, a reader sees the old or the new content, and so
  does a reader after any crash.
* `publish_durable_on_done` (adding `FS.sync-dir`): once the machine is
  `Done`, every crash leaves the new content.
* `publish_stops_on_failure`, `publish_frame`: after a failure the machine
  only cleans up; no command touches any other name. No assumption.
* `create_new_never_replaces` (under `FS.exclusive-create`): a create-new
  publish never replaces an existing target.
* `profile_mono`: a profile with more guarantees admits fewer outcomes.
* `posix_satisfies`, `posix_done_reachable`: one profile satisfies every
  assumption together, and under it a publish does reach `Done`.
* `old_or_new_needs_rename_atomic`, `old_or_new_needs_sync_file`,
  `durable_needs_sync_dir`: with that one guarantee switched off, the
  property fails on a concrete run.

Started with the target durably absent (`init0Absent`, #58):
* `publish_absent_or_new` (under `FS.sync-file` only): a reader, and a
  reader after any crash, sees nothing or the new content.
* `publish_absent_durable_on_done` (all three): once `Done`, every crash
  leaves the new content.
* `posix_absent_done_reachable`; `absent_or_new_needs_sync_file`,
  `absent_durable_needs_sync_dir`, `absent_durable_needs_rename_atomic`.

Trusted, not proved: that a real filesystem behaves like a profile with the
named guarantees (that is what the assumptions say), and the driver that
performs the commands (no driver is shipped yet).
-/

open Aeneas Aeneas.Std Result
open Tauten.Machine
open tauten_proof_instances tauten.publish

namespace Tauten.Durable

abbrev P := tauten.publish.Phase
abbrev Ev := tauten.publish.Event
abbrev Cmd := tauten.publish.Command

/-- The pure model of `tauten::publish::step`. The same as
`Tauten.Instances.stepN`, restated here so that this file depends on the
translation directly: a mutant of the step breaks these theorems, not only
`Publish.lean`'s. -/
def stepN : P → Ev → P × Cmd
  | .Idle, .Start => (.Writing, .WriteTemp)
  | .Writing, .Ok => (.Written, .SyncTemp)
  | .Written, .Ok => (.Synced, .Rename)
  | .Synced, .Ok => (.Renamed, .SyncDir)
  | .Renamed, .Ok => (.Done, .Nothing)
  | .Writing, .Err | .Written, .Err | .Synced, .Err | .Cleaning, .Err => (.Cleaning, .RemoveTemp)
  | .Cleaning, .Ok | .Renamed, .Err => (.Failed, .Nothing)
  | p, _ => (p, .Nothing)

theorem publish_step_model (p : P) (e : Ev) :
    machines.publish_step p e = ok (stepN p e) := by
  cases p <;> cases e <;> simp [machines.publish_step, tauten.publish.step, stepN]

def publishM : Machine P Ev Cmd := ⟨stepN⟩

/-- The two files: the one the target names at the start, and the new one. -/
inductive Ino where
  | old
  | new
  deriving DecidableEq

/-- The filesystem. `rest` stands for every other name. -/
structure Fs where
  tgt : Option Ino
  tgtD : Option Ino
  tmp : Option Ino
  newV : Nat
  newD : Nat
  rest : Nat → Option Nat
  lastOk : Bool

/-- Which guarantees a filesystem gives. -/
structure Profile where
  renameAtomic : Bool
  syncFile : Bool
  syncDir : Bool
  exclusiveCreate : Bool

@[tauten_assumption "FS.rename-atomic"]
def RenameAtomic (p : Profile) : Prop := p.renameAtomic = true

@[tauten_assumption "FS.sync-file"]
def SyncFile (p : Profile) : Prop := p.syncFile = true

@[tauten_assumption "FS.sync-dir"]
def SyncDir (p : Profile) : Prop := p.syncDir = true

@[tauten_assumption "FS.exclusive-create"]
def ExclusiveCreate (p : Profile) : Prop := p.exclusiveCreate = true

/-- Move the temporary entry to the target. -/
def moved (fs : Fs) (ok : Bool) : Fs := { fs with tgt := fs.tmp, tmp := none, lastOk := ok }

/-- What a command may do on profile `pr`, writing content `n`. Every
outcome leaves `rest` alone; a failure reports `lastOk = false`. -/
def exec (pr : Profile) (n : Nat) (fs : Fs) : Cmd → Fs → Prop
  | .Nothing, fs' => fs' = fs
  | .WriteTemp, fs' =>
      fs' = { fs with tmp := some .new, newV := n, lastOk := true } ∨
      (fs'.lastOk = false ∧ fs'.tgt = fs.tgt ∧ fs'.tgtD = fs.tgtD ∧ fs'.rest = fs.rest)
  | .SyncTemp, fs' =>
      fs' = { fs with newD := fs.newV, lastOk := true } ∨
      (pr.syncFile = false ∧ fs' = { fs with lastOk := true }) ∨
      (fs'.lastOk = false ∧ fs'.tgt = fs.tgt ∧ fs'.tgtD = fs.tgtD ∧ fs'.tmp = fs.tmp ∧
        fs'.newV = fs.newV ∧ fs'.rest = fs.rest)
  | .Rename, fs' =>
      fs' = moved fs true ∨ fs' = moved fs false ∨ fs' = { fs with lastOk := false } ∨
      (pr.renameAtomic = false ∧ ∃ ok, fs' = { fs with tgt := none, lastOk := ok })
  | .RenameNoReplace, fs' =>
      (fs.tgt = none ∧ (fs' = moved fs true ∨ fs' = moved fs false)) ∨
      fs' = { fs with lastOk := false } ∨
      (pr.exclusiveCreate = false ∧ fs' = moved fs true)
  | .SyncDir, fs' =>
      fs' = { fs with tgtD := fs.tgt, lastOk := true } ∨
      (pr.syncDir = false ∧ fs' = { fs with lastOk := true }) ∨
      fs' = { fs with lastOk := false } ∨ fs' = { fs with tgtD := fs.tgt, lastOk := false }
  | .RemoveTemp, fs' =>
      fs' = { fs with tmp := none, lastOk := true } ∨ fs' = { fs with lastOk := false } ∨
      fs' = { fs with tmp := none, lastOk := false }

/-- `Ok` reports a success, `Err` a failure. -/
def deliverF (fs : Fs) : Ev → Prop
  | .Start => True
  | .Ok => fs.lastOk = true
  | .Err => fs.lastOk = false

def world (pr : Profile) (n : Nat) : World Ev Cmd Fs := ⟨deliverF, exec pr n, fun _ _ => False⟩

/-- Before a publish: the target names the old file, durably, and there is
no temporary file. The new file's contents are anything. -/
def init0 (x : P × Fs) : Prop :=
  x.1 = .Idle ∧ x.2.tgt = some .old ∧ x.2.tgtD = some .old ∧ x.2.tmp = none

/-! ## Crashes -/

/-- A crash: each entry keeps its durable value or its pending one. -/
def Crash (fs r : Fs) : Prop :=
  (r.tgt = fs.tgtD ∨ r.tgt = fs.tgt) ∧ r.tgtD = r.tgt ∧
  (r.newV = fs.newD ∨ r.newV = fs.newV) ∧ r.newD = r.newV ∧ r.rest = fs.rest

/-- What a reader of the target sees: the old content `o`, the new file's
content, or nothing. -/
def readT (o : Nat) (fs : Fs) : Option Nat :=
  match fs.tgt with
  | some .old => some o
  | some .new => some fs.newV
  | none => none

theorem crash_characterisation (fs r : Fs) (h : Crash fs r) :
    (r.tgt = fs.tgtD ∨ r.tgt = fs.tgt) ∧ (r.newV = fs.newD ∨ r.newV = fs.newV) ∧
    r.rest = fs.rest :=
  ⟨h.1, h.2.2.1, h.2.2.2.2⟩

/-! ## The machines, as extracted -/

/-- The create-new step, as a pure model. -/
def createN (s : P) (e : Ev) : P × Cmd :=
  match stepN s e with
  | (p, .Rename) => (p, .RenameNoReplace)
  | r => r

theorem publish_create_step_model (p : P) (e : Ev) :
    machines.publish_create_step p e = ok (createN p e) := by
  cases p <;> cases e <;>
    simp [machines.publish_create_step, tauten.publish.create_step, tauten.publish.step,
      createN, stepN]

/-- The extracted steps. -/
def publishR : RMachine P Ev Cmd := ⟨machines.publish_step⟩
def createR : RMachine P Ev Cmd := ⟨machines.publish_create_step⟩

/-- The pure model's configurations are reached by the extracted step (for
the non-vacuity and necessity runs below). -/
theorem rsys_of_sys (m : RMachine P Ev Cmd) (p : Machine P Ev Cmd)
    (hmodel : ∀ s e, m.step s e = ok (p.step s e)) (w : World Ev Cmd Fs)
    (init : P × Fs → Prop) : ∀ x, SysReachable p w init x → RSysReachable m w init x := by
  intro x h
  induction h with
  | init x hx => exact RSysReachable.init x hx
  | step s v v' e _ hd hi ih => exact RSysReachable.step s _ _ v v' e ih hd (hmodel s e) hi
  | adversary s v v' _ ha ih => exact RSysReachable.adversary s v v' ih ha

theorem publish_model (x : P × Fs) (pr : Profile) (n : Nat) (init : P × Fs → Prop)
    (h : RSysReachable publishR (world pr n) init x) : SysReachable publishM (world pr n) init x :=
  rsys_of_model publishR publishM (fun s e => publish_step_model s e) _ _ x h

/-! ## Old or new -/

/-- The invariant behind old-or-new. `n` is the new content. -/
def Inv (pr : Profile) (n : Nat) : P × Fs → Prop
  | (p, fs) =>
    fs.tgt ≠ none ∧ fs.tgtD ≠ none ∧
    ((fs.tgt = some .new ∨ fs.tgtD = some .new) → fs.newV = n ∧ fs.newD = n) ∧
    (p = .Idle → fs.tgt = some .old ∧ fs.tgtD = some .old) ∧
    (p = .Writing → fs.tgt = some .old ∧ fs.tgtD = some .old ∧
      (fs.lastOk = true → fs.newV = n ∧ fs.tmp = some .new)) ∧
    (p = .Written → fs.tgt = some .old ∧ fs.tgtD = some .old ∧
      (fs.lastOk = true → fs.newV = n ∧ fs.newD = n ∧ fs.tmp = some .new)) ∧
    (p = .Synced → fs.newV = n ∧ fs.newD = n ∧ (fs.lastOk = true → fs.tgt = some .new)) ∧
    (p = .Renamed → fs.newV = n ∧ fs.newD = n ∧ fs.tgt = some .new ∧
      (pr.syncDir = true → fs.lastOk = true → fs.tgtD = some .new)) ∧
    (p = .Done → fs.newV = n ∧ fs.newD = n ∧ fs.tgt = some .new ∧
      (pr.syncDir = true → fs.tgtD = some .new))

theorem inv_holds (pr : Profile) (n : Nat) (hR : RenameAtomic pr) (hS : SyncFile pr) :
    ∀ x, SysReachable publishM (world pr n) init0 x → Inv pr n x := by
  unfold RenameAtomic at hR
  unfold SyncFile at hS
  apply sys_inv_of_step publishM (world pr n) init0 (Inv pr n)
  · rintro ⟨p, fs⟩ ⟨h1, h2, h3, h4⟩
    simp only at h1 h2 h3 h4
    subst h1
    simp [Inv, h2, h3]
  · rintro s ⟨tgt, tgtD, tmp, nv, nd, rest, ok⟩ v' e hi hd hx
    simp only [world, deliverF] at hd hx
    obtain ⟨i1, i2, i3, i4, i5, i6, i7, i8, i9⟩ := hi
    cases s <;> cases e <;>
      simp only [publishM, stepN, exec, moved] at hx hd ⊢ <;>
      simp_all [Inv] <;>
      (try (rcases hx with hx | hx | hx | hx)) <;>
      (try (rcases hx with ⟨hx1, hx2, hx3, hx4, hx5, hx6⟩)) <;>
      (try (rcases hx with ⟨hx1, hx2, hx3, hx4⟩)) <;>
      (try (rcases hx with ⟨hx1, ok', hx2⟩)) <;>
      (try subst_vars) <;> simp_all
  · intro s v v' _ h
    exact h.elim

/-- Under `FS.rename-atomic` and `FS.sync-file`, a reader of the target sees
the old or the new content in every reachable configuration, and after any
crash. -/
theorem publish_old_or_new (pr : Profile) (o n : Nat) (hR : RenameAtomic pr)
    (hS : SyncFile pr) (x : P × Fs) (h : RSysReachable publishR (world pr n) init0 x) :
    (readT o x.2 = some o ∨ readT o x.2 = some n) ∧
    ∀ r, Crash x.2 r → readT o r = some o ∨ readT o r = some n := by
  have h := publish_model x pr n _ h
  obtain ⟨i1, i2, i3, -⟩ := inv_holds pr n hR hS x h
  refine ⟨?_, ?_⟩
  · unfold readT
    rcases ht : x.2.tgt with _ | _ | _
    · exact absurd ht i1
    · simp
    · simp [(i3 (Or.inl ht)).1]
  · rintro r ⟨c1, -, c3, -, -⟩
    unfold readT
    rcases ht : r.tgt with _ | _ | _
    · rcases c1 with c1 | c1 <;> rw [c1] at ht
      · exact absurd ht i2
      · exact absurd ht i1
    · simp
    · have hnew : x.2.tgt = some .new ∨ x.2.tgtD = some .new := by
        rcases c1 with c1 | c1 <;> rw [c1] at ht <;> simp [ht]
      obtain ⟨hv, hd⟩ := i3 hnew
      rcases c3 with c3 | c3 <;> simp [c3, hv, hd]

def durableN : P → Bool
  | .Done => true
  | _ => false

theorem durable_model (p : P) : machines.publish_durable p = ok (durableN p) := by
  cases p <;> simp [machines.publish_durable, tauten.publish.durable, durableN]

/-- Adding `FS.sync-dir`: in every reachable configuration whose phase
`durable` reports, every crash leaves the new content. -/
theorem publish_durable_on_done (pr : Profile) (o n : Nat) (hR : RenameAtomic pr)
    (hS : SyncFile pr) (hD : SyncDir pr) (p : P) (fs : Fs)
    (h : RSysReachable publishR (world pr n) init0 (p, fs))
    (hdur : machines.publish_durable p = ok true) :
    ∀ r, Crash fs r → readT o r = some n := by
  rw [durable_model] at hdur
  have hp : p = .Done := by cases p <;> simp_all [durableN]
  subst hp
  have h := publish_model _ pr n _ h
  obtain ⟨-, -, -, -, -, -, -, -, i9⟩ := inv_holds pr n hR hS _ h
  obtain ⟨hv, hd, ht, htd⟩ := i9 rfl
  have htd := htd hD
  rintro r ⟨c1, -, c3, -, -⟩
  unfold readT
  have : r.tgt = some .new := by rcases c1 with c1 | c1 <;> simp [c1, ht, htd]
  rw [this]
  rcases c3 with c3 | c3 <;> simp [c3, hv, hd]

/-! ## No assumption needed -/

/-- After a failure the machine only cleans up: an `Err` in an active phase
leads to `Cleaning` or `Failed`, and from those no step writes, syncs or
renames. -/
theorem publish_stops_on_failure (p : P) (e : Ev) :
    ((p = .Writing ∨ p = .Written ∨ p = .Synced ∨ p = .Renamed ∨ p = .Cleaning) →
      ∃ c, machines.publish_step p .Err = ok (.Cleaning, c) ∨
        machines.publish_step p .Err = ok (.Failed, c)) ∧
    ((p = .Cleaning ∨ p = .Failed) →
      ∃ q c, machines.publish_step p e = ok (q, c) ∧ (q = .Cleaning ∨ q = .Failed) ∧
        (c = .RemoveTemp ∨ c = .Nothing)) := by
  rw [publish_step_model, publish_step_model]
  cases p <;> cases e <;> simp [stepN]

/-- Every outcome of every command leaves the other names alone. -/
theorem exec_rest (pr : Profile) (n : Nat) (fs fs' : Fs) (c : Cmd) (h : exec pr n fs c fs') :
    fs'.rest = fs.rest := by
  cases c <;> simp only [exec, moved] at h
  · rw [h]
  · rcases h with h | ⟨-, -, -, h⟩
    · rw [h]
    · exact h
  · rcases h with h | ⟨-, h⟩ | ⟨-, -, -, -, -, h⟩
    · rw [h]
    · rw [h]
    · exact h
  · rcases h with h | h | h | ⟨-, ok, h⟩ <;> rw [h]
  · rcases h with ⟨-, h | h⟩ | h | ⟨-, h⟩ <;> rw [h]
  · rcases h with h | ⟨-, h⟩ | h | h <;> rw [h]
  · rcases h with h | h | h <;> rw [h]

/-- No command of the machine changes any other name, on any profile. -/
theorem publish_frame (pr : Profile) (n : Nat) (rest : Nat → Option Nat) (x : P × Fs)
    (h : RSysReachable publishR (world pr n) (fun y => init0 y ∧ y.2.rest = rest) x) :
    x.2.rest = rest := by
  have h' := publish_model x pr n _ h
  clear h
  induction h' with
  | init y hy => exact hy.2
  | step s v v' e _ _ hx ih => exact (exec_rest pr n _ _ _ hx).trans ih
  | adversary s v v' _ ha _ => exact ha.elim

/-! ## Create-new -/

/-- Under `FS.exclusive-create`, a create-new publish never replaces an
existing target: started with the target present, the target keeps naming
the old file in every reachable configuration. -/
theorem create_new_never_replaces (pr : Profile) (n : Nat) (hX : ExclusiveCreate pr)
    (x : P × Fs) (h : RSysReachable createR (world pr n) init0 x) :
    x.2.tgt = some .old := by
  unfold ExclusiveCreate at hX
  have h' := rsys_of_model createR ⟨createN⟩ (fun s e => publish_create_step_model s e) _ _ x h
  clear h
  induction h' with
  | init y hy => exact hy.2.1
  | step s v v' e _ _ hx ih =>
    simp only [world] at hx
    have hnr : (createN s e).2 ≠ .Rename := by
      cases s <;> cases e <;> simp [createN, stepN]
    simp only at hnr hx ih ⊢
    revert hx hnr
    cases (createN s e).2 <;> intro hx hnr <;> simp only [exec, moved] at hx
    · rw [hx]; exact ih
    · rcases hx with hx | ⟨-, ht, -⟩
      · rw [hx]; exact ih
      · rw [ht]; exact ih
    · rcases hx with hx | ⟨-, hx⟩ | ⟨-, ht, -⟩
      · rw [hx]; exact ih
      · rw [hx]; exact ih
      · rw [ht]; exact ih
    · exact absurd rfl hnr
    · rcases hx with ⟨hn, -⟩ | hx | ⟨hf, -⟩
      · rw [ih] at hn; exact absurd hn (by simp)
      · rw [hx]; exact ih
      · rw [hX] at hf; exact absurd hf (by simp)
    · rcases hx with hx | ⟨-, hx⟩ | hx | hx <;> rw [hx] <;> exact ih
    · rcases hx with hx | hx | hx <;> rw [hx] <;> exact ih
  | adversary s v v' _ ha _ => exact ha.elim

/-! ## Profiles -/

/-- `q` gives every guarantee `p` gives. -/
def ProfileLe (p q : Profile) : Prop :=
  (p.renameAtomic = true → q.renameAtomic = true) ∧ (p.syncFile = true → q.syncFile = true) ∧
  (p.syncDir = true → q.syncDir = true) ∧ (p.exclusiveCreate = true → q.exclusiveCreate = true)

/-- A profile with more guarantees admits fewer outcomes: every outcome of a
command on `q` is an outcome on `p`. -/
theorem profile_mono (p q : Profile) (hle : ProfileLe p q) (n : Nat) (fs fs' : Fs) (c : Cmd)
    (h : exec q n fs c fs') : exec p n fs c fs' := by
  obtain ⟨h1, h2, h3, h4⟩ := hle
  cases c <;> simp only [exec] at h ⊢
  · exact h
  · exact h
  · rcases h with h | ⟨hq, h⟩ | h
    · exact Or.inl h
    · cases hp : p.syncFile
      · exact Or.inr (Or.inl ⟨rfl, h⟩)
      · exact absurd (h2 hp) (by simp [hq])
    · exact Or.inr (Or.inr h)
  · rcases h with h | h | h | ⟨hq, h⟩
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr (Or.inl h))
    · cases hp : p.renameAtomic
      · exact Or.inr (Or.inr (Or.inr ⟨rfl, h⟩))
      · exact absurd (h1 hp) (by simp [hq])
  · rcases h with h | h | ⟨hq, h⟩
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
    · cases hp : p.exclusiveCreate
      · exact Or.inr (Or.inr ⟨rfl, h⟩)
      · exact absurd (h4 hp) (by simp [hq])
  · rcases h with h | ⟨hq, h⟩ | h | h
    · exact Or.inl h
    · cases hp : p.syncDir
      · exact Or.inr (Or.inl ⟨rfl, h⟩)
      · exact absurd (h3 hp) (by simp [hq])
    · exact Or.inr (Or.inr (Or.inl h))
    · exact Or.inr (Or.inr (Or.inr h))
  · exact h

/-! ## Non-vacuity and necessity -/

/-- A profile with every guarantee. -/
def posix : Profile := ⟨true, true, true, true⟩

/-- The assumptions can all hold together. -/
theorem posix_satisfies :
    RenameAtomic posix ∧ SyncFile posix ∧ SyncDir posix ∧ ExclusiveCreate posix := by
  simp [RenameAtomic, SyncFile, SyncDir, ExclusiveCreate, posix]

def fs0 : Fs := ⟨some .old, some .old, none, 2, 2, fun _ => none, true⟩

/-- Under `posix`, a publish does reach `Done` (the durable theorem is not
vacuous). -/
theorem posix_done_reachable :
    ∃ fs, RSysReachable publishR (world posix 1) init0 (.Done, fs) := by
  let f1 : Fs := { fs0 with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with newD := f1.newV, lastOk := true }
  let f3 : Fs := moved f2 true
  let f4 : Fs := { f3 with tgtD := f3.tgt, lastOk := true }
  refine ⟨f4, ?_⟩
  apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
  have r0 : SysReachable publishM (world posix 1) init0 (.Idle, fs0) :=
    .init _ ⟨rfl, rfl, rfl, rfl⟩
  have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
    (by simp [world, exec, publishM, stepN, f1])
  have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
    (by simp [world, exec, publishM, stepN, f2])
  have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
    (by simp [world, exec, publishM, stepN, f3])
  have r4 := SysReachable.step _ _ f4 .Ok r3 (by simp [world, deliverF, f3, moved])
    (by simp [world, exec, publishM, stepN, f4])
  have r5 := SysReachable.step _ _ f4 .Ok r4 (by simp [world, deliverF, f4])
    (by simp [world, exec, publishM, stepN])
  simpa [publishM, stepN] using r5

/-- Without `FS.rename-atomic` (all else given), a reader can find the
target missing. -/
theorem old_or_new_needs_rename_atomic :
    ∃ pr x, SyncFile pr ∧ SyncDir pr ∧ ¬ RenameAtomic pr ∧
      RSysReachable publishR (world pr 1) init0 x ∧ readT 0 x.2 = none := by
  let pr : Profile := ⟨false, true, true, true⟩
  let f1 : Fs := { fs0 with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with newD := f1.newV, lastOk := true }
  let f3 : Fs := { f2 with tgt := none, lastOk := true }
  refine ⟨pr, (.Synced, f3), rfl, rfl, by simp [RenameAtomic, pr], ?_,
    by simp [readT, f3]⟩
  apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
  have r0 : SysReachable publishM (world pr 1) init0 (.Idle, fs0) := .init _ ⟨rfl, rfl, rfl, rfl⟩
  have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
    (by simp [world, exec, publishM, stepN, f1])
  have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
    (by simp [world, exec, publishM, stepN, f2])
  have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
    (by simp [world, exec, publishM, stepN, f3, pr])
  simpa [publishM, stepN] using r3

/-- Without `FS.sync-file` (all else given), a crash after the rename can
leave content that is neither old nor new. -/
theorem old_or_new_needs_sync_file :
    ∃ pr x r, RenameAtomic pr ∧ SyncDir pr ∧ ¬ SyncFile pr ∧
      RSysReachable publishR (world pr 1) init0 x ∧ Crash x.2 r ∧ readT 0 r = some 2 := by
  let pr : Profile := ⟨true, false, true, true⟩
  let f1 : Fs := { fs0 with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with lastOk := true }
  let f3 : Fs := moved f2 true
  let r : Fs := { f3 with tgtD := some .new, newV := 2, newD := 2 }
  refine ⟨pr, (.Synced, f3), r, rfl, rfl, by simp [SyncFile, pr], ?_, ?_,
    by simp [readT, r, f3, f2, f1, fs0, moved]⟩
  · apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
    have r0 : SysReachable publishM (world pr 1) init0 (.Idle, fs0) := .init _ ⟨rfl, rfl, rfl, rfl⟩
    have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
      (by simp [world, exec, publishM, stepN, f1])
    have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
      (by simp [world, exec, publishM, stepN, f2, pr])
    have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
      (by simp [world, exec, publishM, stepN, f3])
    simpa [publishM, stepN] using r3
  · simp [Crash, r, f3, f2, f1, fs0, moved]

/-- Without `FS.sync-dir` (all else given), a crash after `Done` can leave
the old content. -/
theorem durable_needs_sync_dir :
    ∃ pr fs r, RenameAtomic pr ∧ SyncFile pr ∧ ¬ SyncDir pr ∧
      machines.publish_durable .Done = ok true ∧
      RSysReachable publishR (world pr 1) init0 (.Done, fs) ∧ Crash fs r ∧ readT 0 r = some 0 := by
  let pr : Profile := ⟨true, true, false, true⟩
  let f1 : Fs := { fs0 with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with newD := f1.newV, lastOk := true }
  let f3 : Fs := moved f2 true
  let f4 : Fs := { f3 with lastOk := true }
  let r : Fs := { f4 with tgt := some .old, tgtD := some .old }
  refine ⟨pr, f4, r, rfl, rfl, by simp [SyncDir, pr], by rw [durable_model]; rfl, ?_, ?_,
    by simp [readT, r, f4, f3, f2, f1, fs0, moved]⟩
  · apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
    have r0 : SysReachable publishM (world pr 1) init0 (.Idle, fs0) := .init _ ⟨rfl, rfl, rfl, rfl⟩
    have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
      (by simp [world, exec, publishM, stepN, f1])
    have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
      (by simp [world, exec, publishM, stepN, f2])
    have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
      (by simp [world, exec, publishM, stepN, f3])
    have r4 := SysReachable.step _ _ f4 .Ok r3 (by simp [world, deliverF, f3, moved])
      (by simp [world, exec, publishM, stepN, f4, pr])
    have r5 := SysReachable.step _ _ f4 .Ok r4 (by simp [world, deliverF, f4])
      (by simp [world, exec, publishM, stepN])
    simpa [publishM, stepN] using r5
  · simp [Crash, r, f4, f3, f2, f1, fs0, moved]

/-! ## Publishing to an absent target (#58)

Companions of the replacement results for a first publish: the target does
not exist, durably, before the machine starts. The step function, `exec`,
`Crash` and the profiles are the ones above; `init0` and `Inv` are unchanged.
Absent-or-new needs only `FS.sync-file`: a non-atomic rename can only leave
the target missing, which a reader of a fresh name may see anyway. Durability
on `Done` needs all three guarantees. -/

/-- Before a first publish: the target does not exist, durably, and there is
no temporary file. -/
def init0Absent (x : P × Fs) : Prop :=
  x.1 = .Idle ∧ x.2.tgt = none ∧ x.2.tgtD = none ∧ x.2.tmp = none

/-- The invariant behind absent-or-new. `n` is the new content. -/
def InvAbsent (pr : Profile) (n : Nat) : P × Fs → Prop
  | (p, fs) =>
    fs.tgt ≠ some .old ∧ fs.tgtD ≠ some .old ∧
    ((fs.tgt = some .new ∨ fs.tgtD = some .new) → fs.newV = n ∧ fs.newD = n) ∧
    (p = .Idle → fs.tgt = none ∧ fs.tgtD = none) ∧
    (p = .Writing → fs.tgt = none ∧ fs.tgtD = none ∧
      (fs.lastOk = true → fs.newV = n ∧ fs.tmp = some .new)) ∧
    (p = .Written → fs.tgt = none ∧ fs.tgtD = none ∧
      (fs.lastOk = true → fs.newV = n ∧ fs.newD = n ∧ fs.tmp = some .new)) ∧
    (p = .Synced → fs.newV = n ∧ fs.newD = n ∧
      (pr.renameAtomic = true → fs.lastOk = true → fs.tgt = some .new)) ∧
    (p = .Renamed → fs.newV = n ∧ fs.newD = n ∧
      (pr.renameAtomic = true → fs.tgt = some .new ∧
        (pr.syncDir = true → fs.lastOk = true → fs.tgtD = some .new))) ∧
    (p = .Done → fs.newV = n ∧ fs.newD = n ∧
      (pr.renameAtomic = true → fs.tgt = some .new ∧ (pr.syncDir = true → fs.tgtD = some .new)))

theorem inv_absent_holds (pr : Profile) (n : Nat) (hS : SyncFile pr) :
    ∀ x, SysReachable publishM (world pr n) init0Absent x → InvAbsent pr n x := by
  unfold SyncFile at hS
  apply sys_inv_of_step publishM (world pr n) init0Absent (InvAbsent pr n)
  · rintro ⟨p, fs⟩ ⟨h1, h2, h3, h4⟩
    simp only at h1 h2 h3 h4
    subst h1
    simp [InvAbsent, h2, h3]
  · rintro s ⟨tgt, tgtD, tmp, nv, nd, rest, ok⟩ v' e hi hd hx
    simp only [world, deliverF] at hd hx
    obtain ⟨i1, i2, i3, i4, i5, i6, i7, i8, i9⟩ := hi
    cases s <;> cases e <;>
      simp only [publishM, stepN, exec, moved] at hx hd ⊢ <;>
      simp_all [InvAbsent] <;>
      (try (rcases hx with hx | hx | hx | hx)) <;>
      (try (rcases hx with ⟨hx1, hx2, hx3, hx4, hx5, hx6⟩)) <;>
      (try (rcases hx with ⟨hx1, hx2, hx3, hx4⟩)) <;>
      (try (rcases hx with ⟨hx1, ok', hx2⟩)) <;>
      (try subst_vars) <;> simp_all
  · intro s v v' _ h
    exact h.elim

/-- Under `FS.sync-file`, publishing to an absent target: in every reachable
configuration, and after any crash, a reader of the target sees nothing or
the new content, never anything else. -/
theorem publish_absent_or_new (pr : Profile) (o n : Nat) (hS : SyncFile pr) (x : P × Fs)
    (h : RSysReachable publishR (world pr n) init0Absent x) :
    (readT o x.2 = none ∨ readT o x.2 = some n) ∧
    ∀ r, Crash x.2 r → readT o r = none ∨ readT o r = some n := by
  have h := publish_model x pr n _ h
  obtain ⟨i1, i2, i3, -⟩ := inv_absent_holds pr n hS x h
  refine ⟨?_, ?_⟩
  · unfold readT
    rcases ht : x.2.tgt with _ | _ | _
    · simp
    · exact absurd ht i1
    · simp [(i3 (Or.inl ht)).1]
  · rintro r ⟨c1, -, c3, -, -⟩
    unfold readT
    rcases ht : r.tgt with _ | _ | _
    · simp
    · rcases c1 with c1 | c1 <;> rw [c1] at ht
      · exact absurd ht i2
      · exact absurd ht i1
    · have hnew : x.2.tgt = some .new ∨ x.2.tgtD = some .new := by
        rcases c1 with c1 | c1 <;> rw [c1] at ht <;> simp [ht]
      obtain ⟨hv, hd⟩ := i3 hnew
      rcases c3 with c3 | c3 <;> simp [c3, hv, hd]

/-- Under `FS.rename-atomic`, `FS.sync-file` and `FS.sync-dir`, publishing
to an absent target: once `durable` reports, every crash leaves the new
content. -/
theorem publish_absent_durable_on_done (pr : Profile) (o n : Nat) (hR : RenameAtomic pr)
    (hS : SyncFile pr) (hD : SyncDir pr) (p : P) (fs : Fs)
    (h : RSysReachable publishR (world pr n) init0Absent (p, fs))
    (hdur : machines.publish_durable p = ok true) :
    ∀ r, Crash fs r → readT o r = some n := by
  rw [durable_model] at hdur
  have hp : p = .Done := by cases p <;> simp_all [durableN]
  subst hp
  have h := publish_model _ pr n _ h
  obtain ⟨-, -, -, -, -, -, -, -, i9⟩ := inv_absent_holds pr n hS _ h
  obtain ⟨hv, hd, hrest⟩ := i9 rfl
  obtain ⟨ht, htd⟩ := hrest hR
  have htd := htd hD
  rintro r ⟨c1, -, c3, -, -⟩
  unfold readT
  have : r.tgt = some .new := by rcases c1 with c1 | c1 <;> simp [c1, ht, htd]
  rw [this]
  rcases c3 with c3 | c3 <;> simp [c3, hv, hd]

def fs0Absent : Fs := ⟨none, none, none, 2, 2, fun _ => none, true⟩

/-- Under `posix`, a first publish does reach `Done` (the absent-start
durable theorem is not vacuous). -/
theorem posix_absent_done_reachable :
    ∃ fs, RSysReachable publishR (world posix 1) init0Absent (.Done, fs) ∧
      readT 0 fs = some 1 := by
  let f1 : Fs := { fs0Absent with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with newD := f1.newV, lastOk := true }
  let f3 : Fs := moved f2 true
  let f4 : Fs := { f3 with tgtD := f3.tgt, lastOk := true }
  refine ⟨f4, ?_, by simp [readT, f4, f3, f2, f1, moved]⟩
  apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
  have r0 : SysReachable publishM (world posix 1) init0Absent (.Idle, fs0Absent) :=
    .init _ ⟨rfl, rfl, rfl, rfl⟩
  have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
    (by simp [world, exec, publishM, stepN, f1])
  have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
    (by simp [world, exec, publishM, stepN, f2])
  have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
    (by simp [world, exec, publishM, stepN, f3])
  have r4 := SysReachable.step _ _ f4 .Ok r3 (by simp [world, deliverF, f3, moved])
    (by simp [world, exec, publishM, stepN, f4])
  have r5 := SysReachable.step _ _ f4 .Ok r4 (by simp [world, deliverF, f4])
    (by simp [world, exec, publishM, stepN])
  simpa [publishM, stepN] using r5

/-- Without `FS.sync-file` (all else given), a crash after the first rename
can leave content that is neither absent nor new. -/
theorem absent_or_new_needs_sync_file :
    ∃ pr x r, RenameAtomic pr ∧ SyncDir pr ∧ ¬ SyncFile pr ∧
      RSysReachable publishR (world pr 1) init0Absent x ∧ Crash x.2 r ∧ readT 0 r = some 2 := by
  let pr : Profile := ⟨true, false, true, true⟩
  let f1 : Fs := { fs0Absent with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with lastOk := true }
  let f3 : Fs := moved f2 true
  let r : Fs := { f3 with tgtD := some .new, newV := 2, newD := 2 }
  refine ⟨pr, (.Synced, f3), r, rfl, rfl, by simp [SyncFile, pr], ?_, ?_,
    by simp [readT, r, f3, f2, f1, fs0Absent, moved]⟩
  · apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
    have r0 : SysReachable publishM (world pr 1) init0Absent (.Idle, fs0Absent) :=
      .init _ ⟨rfl, rfl, rfl, rfl⟩
    have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
      (by simp [world, exec, publishM, stepN, f1])
    have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
      (by simp [world, exec, publishM, stepN, f2, pr])
    have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
      (by simp [world, exec, publishM, stepN, f3])
    simpa [publishM, stepN] using r3
  · simp [Crash, r, f3, f2, f1, fs0Absent, moved]

/-- Without `FS.sync-dir` (all else given), a crash after `Done` can lose
the new name: the reader sees nothing. -/
theorem absent_durable_needs_sync_dir :
    ∃ pr fs r, RenameAtomic pr ∧ SyncFile pr ∧ ¬ SyncDir pr ∧
      machines.publish_durable .Done = ok true ∧
      RSysReachable publishR (world pr 1) init0Absent (.Done, fs) ∧ Crash fs r ∧
      readT 0 r = none := by
  let pr : Profile := ⟨true, true, false, true⟩
  let f1 : Fs := { fs0Absent with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with newD := f1.newV, lastOk := true }
  let f3 : Fs := moved f2 true
  let f4 : Fs := { f3 with lastOk := true }
  let r : Fs := { f4 with tgt := none, tgtD := none }
  refine ⟨pr, f4, r, rfl, rfl, by simp [SyncDir, pr], by rw [durable_model]; rfl, ?_, ?_,
    by simp [readT, r]⟩
  · apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
    have r0 : SysReachable publishM (world pr 1) init0Absent (.Idle, fs0Absent) :=
      .init _ ⟨rfl, rfl, rfl, rfl⟩
    have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
      (by simp [world, exec, publishM, stepN, f1])
    have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
      (by simp [world, exec, publishM, stepN, f2])
    have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
      (by simp [world, exec, publishM, stepN, f3])
    have r4 := SysReachable.step _ _ f4 .Ok r3 (by simp [world, deliverF, f3, moved])
      (by simp [world, exec, publishM, stepN, f4, pr])
    have r5 := SysReachable.step _ _ f4 .Ok r4 (by simp [world, deliverF, f4])
      (by simp [world, exec, publishM, stepN])
    simpa [publishM, stepN] using r5
  · simp [Crash, r, f4, f3, f2, f1, fs0Absent, moved]

/-- Without `FS.rename-atomic` (all else given), a first publish can reach
`Done` with the target missing: the rename unlinked instead of moving. -/
theorem absent_durable_needs_rename_atomic :
    ∃ pr fs r, SyncFile pr ∧ SyncDir pr ∧ ¬ RenameAtomic pr ∧
      machines.publish_durable .Done = ok true ∧
      RSysReachable publishR (world pr 1) init0Absent (.Done, fs) ∧ Crash fs r ∧
      readT 0 r = none := by
  let pr : Profile := ⟨false, true, true, true⟩
  let f1 : Fs := { fs0Absent with tmp := some .new, newV := 1, lastOk := true }
  let f2 : Fs := { f1 with newD := f1.newV, lastOk := true }
  let f3 : Fs := { f2 with tgt := none, lastOk := true }
  let f4 : Fs := { f3 with tgtD := f3.tgt, lastOk := true }
  refine ⟨pr, f4, f4, rfl, rfl, by simp [RenameAtomic, pr], by rw [durable_model]; rfl, ?_, ?_,
    by simp [readT, f4, f3]⟩
  · apply rsys_of_sys publishR publishM (fun s e => publish_step_model s e)
    have r0 : SysReachable publishM (world pr 1) init0Absent (.Idle, fs0Absent) :=
      .init _ ⟨rfl, rfl, rfl, rfl⟩
    have r1 := SysReachable.step _ _ f1 .Start r0 (by simp [world, deliverF])
      (by simp [world, exec, publishM, stepN, f1])
    have r2 := SysReachable.step _ _ f2 .Ok r1 (by simp [world, deliverF, f1])
      (by simp [world, exec, publishM, stepN, f2])
    have r3 := SysReachable.step _ _ f3 .Ok r2 (by simp [world, deliverF, f2])
      (by simp [world, exec, publishM, stepN, f3, pr])
    have r4 := SysReachable.step _ _ f4 .Ok r3 (by simp [world, deliverF, f3])
      (by simp [world, exec, publishM, stepN, f4])
    have r5 := SysReachable.step _ _ f4 .Ok r4 (by simp [world, deliverF, f4])
      (by simp [world, exec, publishM, stepN])
    simpa [publishM, stepN] using r5
  · simp [Crash, f4, f3, f2, f1, fs0Absent]

end Tauten.Durable
