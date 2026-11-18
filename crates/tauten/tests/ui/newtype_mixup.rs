// Two IDs with the same representation are different types.
#[tauten::newtype]
pub struct UserId(u64);
#[tauten::newtype]
pub struct OrderId(u64);

fn load(_user: UserId) {}

fn main() {
    load(OrderId::new(1));
}
