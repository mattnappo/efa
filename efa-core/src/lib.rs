#[macro_use]
pub mod bytecode;
pub mod asm;
pub mod db;
#[allow(dead_code)]
pub mod solver;
pub mod vm;

pub const HASH_SIZE: usize = 16;

pub type Hash = [u8; HASH_SIZE];

/// Determine if `name` is a valid name for a code object or type.
fn is_valid_name(name: &str) -> bool {
    // A name is valid if it is a valid Rust identifier
    syn::parse_str::<syn::Ident>(name).is_ok()
}

// TODO: convert all grep ..HASH_SIZE] to use this method
fn build_hash(hash: Vec<u8>) -> anyhow::Result<Hash> {
    let trunc: [u8; HASH_SIZE] = (&hash[0..HASH_SIZE])
        .try_into()
        .map_err(|_| anyhow::anyhow!("failed to truncate vector for hash"))?;

    Ok(trunc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_name() {
        assert!(is_valid_name("__hello_name3"));
        assert!(!is_valid_name("hello name"));
        assert!(!is_valid_name("hello$name"));
    }
}
