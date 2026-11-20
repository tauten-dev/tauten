// A permit holds the only mutable borrow of its subject: changing the
// subject between the check and the action does not compile.
use tauten::evidence;
use tauten::origin::Permit;

pub struct Table {
    slots: Vec<u64>,
}

#[evidence(subject = Table, exclusive)]
pub struct Present {
    slot: usize,
}

impl Table {
    pub fn check_mut(&mut self, slot: usize) -> Option<Permit<'_, Present>> {
        (slot < self.slots.len()).then(|| Present::mint_exclusive(self, slot))
    }
}

fn main() {
    let mut table = Table { slots: vec![1] };
    let Some(permit) = table.check_mut(0) else { return };
    table.slots.clear();
    permit.act(|t, fact| t.slots[*fact.slot()] = 0);
}
