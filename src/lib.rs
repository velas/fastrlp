#![doc = include_str!("../README.md")]

extern crate alloc;

mod decode;
mod encode;
mod types;

pub use bytes::BufMut;

pub use decode::{count, Decodable, DecodeError, Rlp};
pub use encode::{
    const_add, encode_fixed_size, encode_list, length_of_length, list_length, zeroless_view,
    Encodable, MaxEncodedLen, MaxEncodedLenAssoc,
};
pub use types::*;

#[cfg(feature = "derive")]
pub use fastrlp_derive::{Decodable, DecodableWrapper, Encodable, EncodableWrapper, MaxEncodedLen};
