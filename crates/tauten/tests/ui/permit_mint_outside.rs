// Only the module that declares the fact can mint a permit for it.
mod facts {
    pub struct Table {
        pub slots: Vec<u64>,
    }

    #[tauten::evidence(subject = Table, exclusive)]
    pub struct Present {
        slot: usize,
    }
}

fn main() {
    let mut table = facts::Table { slots: vec![1] };
    let _permit = facts::Present::mint_exclusive(&mut table, 0);
}
