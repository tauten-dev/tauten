// Evidence borrows the state it checked: mutating the state while the
// evidence is alive does not compile.
use tauten::evidence;
use tauten::origin::Evidence;

pub struct Table {
    slots: Vec<u64>,
}

#[evidence(subject = Table)]
pub struct Present {
    slot: usize,
}

impl Table {
    pub fn check(&self, slot: usize) -> Option<Evidence<'_, Present>> {
        (slot < self.slots.len()).then(|| Present::mint(self, slot))
    }
    pub fn clear_slot(&mut self, fact: Present) {
        if let Some(s) = self.slots.get_mut(*fact.slot()) {
            *s = 0;
        }
    }
}

fn main() {
    let mut table = Table { slots: vec![1] };
    let Some(ev) = table.check(0) else { return };
    table.slots.clear();
    table.clear_slot(ev.redeem());
}
