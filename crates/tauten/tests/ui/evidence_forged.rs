// A fact can be created only by its module's `mint`: neither a struct
// literal nor `mint` compiles anywhere else.
mod table {
    pub struct Table;

    #[tauten::evidence(subject = Table)]
    pub struct Checked {
        n: u8,
    }
}

fn main() {
    let _ = table::Checked { n: 1 };
}
