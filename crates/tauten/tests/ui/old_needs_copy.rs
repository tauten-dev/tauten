// `old(..)` snapshots only Copy values: it never clones a resource.
#[derive(Debug)]
struct E;
impl From<tauten::ContractViolation> for E {
    fn from(_: tauten::ContractViolation) -> Self { E }
}

#[tauten::ensures(|n: &usize| *n == old(items).len())]
fn count(items: Vec<u8>) -> Result<usize, E> {
    Ok(items.len())
}

fn main() {
    let _ = count(vec![1]);
}
