
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};


#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Copy)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum Result {
    Win,
    Loss,
    BreakEven,
    InProgress,
}
