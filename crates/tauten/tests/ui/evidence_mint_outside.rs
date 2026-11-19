// `mint` is visible only in the module that declares the fact.
mod table {
    pub struct Table;

    #[tauten::evidence(subject = Table)]
    pub struct Checked {
        n: u8,
    }
}

fn main() {
    let _ = table::Checked::mint(&table::Table, 1);
}
