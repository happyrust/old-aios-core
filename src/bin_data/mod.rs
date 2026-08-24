use pretty_hex::*;
use std::io::{Read, Write};

pub mod test_data;

pub const WORD_ATT_NAMES: [&'static str; 2] = ["PURP", "SPLATE"];

pub fn convert_str_to_bytes(data_str: &str) -> Vec<u8> {
    data_str
        .trim()
        .split_whitespace()
        .map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}
