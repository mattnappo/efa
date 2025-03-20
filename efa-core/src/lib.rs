use anyhow::{anyhow, bail, Result};

#[macro_use]
pub mod bytecode;
pub mod asm;
pub mod db;
#[allow(dead_code)]
pub mod solver;
pub mod vm;

pub const HASH_SIZE: usize = 16;

// TODO: consider making a wrapper, since impl FromStr would be nice
pub type Hash = [u8; HASH_SIZE];

/// Determine if `name` is a valid name for a code object or type.
fn is_valid_name(name: &str) -> bool {
    // A name is valid if it is a valid Rust identifier
    syn::parse_str::<syn::Ident>(name).is_ok()
}

// TODO: convert all grep ..HASH_SIZE] to use this method
fn hash_from_vec(hash: Vec<u8>) -> Result<Hash> {
    let trunc: [u8; HASH_SIZE] = (&hash[0..HASH_SIZE])
        .try_into()
        .map_err(|_| anyhow!("failed to build hash from {hash:?}"))?;

    Ok(trunc)
}

/// Build hash from hex string of the form 0xHASH.
fn hash_from_str(hash_str: &str) -> Result<Hash> {
    if hash_str.starts_with("0x") {
        let hash_b = hex::decode(&hash_str[2..])?;
        hash_b
            .try_into()
            .map_err(|_| anyhow!("failed to build hash '{hash_str}': invalid hash"))
    } else {
        bail!("failed to build hash '{hash_str}': does not start with '0x'")
    }
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

    #[test]
    fn test_build_hash() {
        assert!(hash_from_str("0xdeadbeefdeadbeefcafebabecafebabe").is_ok());
        assert!(hash_from_str("0xdeadbeefdeadbeef").is_err());
    }
}
