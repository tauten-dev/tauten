// A permit is one check, one action: it cannot be cloned.
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
    let again = permit.clone();
    permit.act(|_, _| ());
    again.act(|_, _| ());
}
