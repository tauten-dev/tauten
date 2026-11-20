import Extracted.TautenProofInstances
import Tauten.Machine

/-!
# The publish machine (`tauten::publish`), on `Tauten.Machine`

The worked instance of the machine library (roadmap A6). The extracted
`publish_step` (a wrapper over `tauten::publish::step`) is shown equal to a
pure model; the model is run against a world in which every command may
fail, leaving partial effects, and the temporary file may vanish at any
time (an adversary step).

Proved here:
* `publish_step_model`, `holds_temp_model`, `terminal_model`: the extracted
  functions never fail and agree with the pure model.
* `publish_inv`: by `sys_inv_of_step`, in every reachable configuration a
  temporary file exists only while the machine is responsible for it, and
  after a successful rename or removal it is gone.
* `publish_discharged`: by `discharged_of_inv`, in every reachable terminal
  phase (`Done`, `Failed`) no temporary file is left behind.

No assumption about the filesystem is needed for these. What the content
is after a crash is roadmap item A7.
-/

open Aeneas Aeneas.Std Result
open Tauten.Machine

namespace Tauten.Instances

open tauten_proof_instances tauten.publish

abbrev P := tauten.publish.Phase
abbrev Ev := tauten.publish.Event
abbrev Cmd := tauten.publish.Command

/-- The pure model of `tauten::publish::step`. -/
def stepN : P → Ev → P × Cmd
  | .Idle, .Start => (.Writing, .WriteTemp)
  | .Writing, .Ok => (.Written, .SyncTemp)
  | .Written, .Ok => (.Synced, .Rename)
  | .Synced, .Ok => (.Renamed, .SyncDir)
  | .Renamed, .Ok => (.Done, .Nothing)
  | .Writing, .Err | .Written, .Err | .Synced, .Err | .Cleaning, .Err => (.Cleaning, .RemoveTemp)
  | .Cleaning, .Ok | .Renamed, .Err => (.Failed, .Nothing)
  | p, _ => (p, .Nothing)

def holdsN : P → Bool
  | .Writing | .Written | .Synced | .Cleaning => true
  | _ => false

def terminalN : P → Bool
  | .Done | .Failed => true
  | _ => false

theorem publish_step_model (p : P) (e : Ev) :
    machines.publish_step p e = ok (stepN p e) := by
  cases p <;> cases e <;> simp [machines.publish_step, tauten.publish.step, stepN]

theorem holds_temp_model (p : P) : machines.publish_holds_temp p = ok (holdsN p) := by
  cases p <;> simp [machines.publish_holds_temp, tauten.publish.holds_temp, holdsN]

theorem terminal_model (p : P) : machines.publish_terminal p = ok (terminalN p) := by
  cases p <;> simp [machines.publish_terminal, tauten.publish.terminal, terminalN]

/-- The publish machine. -/
def publishM : Machine P Ev Cmd := ⟨stepN⟩

/-- The extracted step as an `RMachine`: its reachable phases are the
model's (`rreachable_of_model`). -/
def publishR : RMachine P Ev Cmd := ⟨machines.publish_step⟩

theorem publish_reachable_model (init : P → Prop) (p : P) (h : RReachable publishR init p) :
    Reachable publishM init p :=
  rreachable_of_model publishR publishM (fun s e => publish_step_model s e) init p h

/-- The world: whether the temporary file exists, and whether the last
command succeeded (the next `Ok`/`Err` reports exactly that). -/
structure Disk where
  temp : Bool
  lastOk : Bool

/-- What a command may do. A failed command may leave partial effects; a
successful rename or removal leaves no temporary file. `Nothing` changes
nothing, so an ignored event cannot be mistaken for a command's outcome. -/
def interpretD : Disk → Cmd → Disk → Prop
  | d, .Nothing, d' => d' = d
  | _, .WriteTemp, d' => d'.temp = true ∨ d'.lastOk = false
  | d, .SyncTemp, d' => d'.temp = d.temp
  | _, .Rename, d' => (d'.lastOk = true ∧ d'.temp = false) ∨ d'.lastOk = false
  | d, .SyncDir, d' => d'.temp = d.temp
  | _, .RemoveTemp, d' => (d'.lastOk = true ∧ d'.temp = false) ∨ d'.lastOk = false

def deliverD : Disk → Ev → Prop
  | _, .Start => True
  | d, .Ok => d.lastOk = true
  | d, .Err => d.lastOk = false

/-- The temporary file may vanish at any time. -/
def adversaryD (d d' : Disk) : Prop := d'.temp = false ∧ d'.lastOk = d.lastOk

def disk : World Ev Cmd Disk := ⟨deliverD, interpretD, adversaryD⟩

/-- The invariant: a temporary file exists only while the machine holds
it, and in `Synced` (rename issued) or `Cleaning` (removal issued), a
reported success means it is gone. -/
def PubInv : P × Disk → Prop
  | (p, d) => (d.temp = true → holdsN p = true) ∧
      ((p = .Synced ∨ p = .Cleaning) → d.lastOk = true → d.temp = false)

def pubInit : P × Disk → Prop
  | (p, d) => p = .Idle ∧ d.temp = false

theorem publish_inv : ∀ x, SysReachable publishM disk pubInit x → PubInv x := by
  apply sys_inv_of_step publishM disk pubInit PubInv
  · rintro ⟨p, d⟩ ⟨rfl, ht⟩
    simp [PubInv, ht]
  · intro s v v' e ⟨h1, h2⟩ hd hi
    cases s <;> cases e <;>
      simp_all [publishM, stepN, interpretD, deliverD, disk, PubInv, holdsN] <;>
      (try rcases hi with hi | hi) <;> (try rcases hi with ⟨hi1, hi2⟩) <;> simp_all
  · intro s v v' ⟨h1, h2⟩ ⟨ht, hl⟩
    simp_all [PubInv]

/-- In every reachable terminal phase, no temporary file is left behind. -/
theorem publish_discharged :
    Discharged publishM disk pubInit (fun p => terminalN p = true) (fun x => x.2.temp = true) := by
  intro p v h ht htemp
  have := (publish_inv (p, v) h).1 htemp
  cases p <;> simp_all [terminalN, holdsN]

end Tauten.Instances
