pub mod bytecode;
pub mod db;
pub mod vm;

pub const HASH_SIZE: usize = 16;

pub type Hash = [u8; HASH_SIZE];
