use rkyv::{AlignedVec};

use crate::standardized_types::data_server_messaging::FundForgeError;

pub trait Bytes<T> {
    fn from_bytes<'a>(data: &'a [u8]) -> Result<T, FundForgeError>;

    fn to_bytes(&self) -> Vec<u8>;
}

pub trait VecBytes<T> {
    fn vec_to_aligned(data: &Vec<T>) -> AlignedVec;

    fn vec_to_bytes(data: &Vec<T>) -> Vec<u8>;

    fn from_array_bytes(data: &Vec<u8>) -> Result<Vec<T>, core::fmt::Error>;
}





