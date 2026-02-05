//! Hash utilities for IC-SIWA

use ic_certified_map::Hash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::collections::HashMap;

/// Hash data with Keccak256 (for Ethereum/Avalanche compatibility)
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash data with SHA256
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash a message with Ethereum personal sign prefix
/// "\x19Ethereum Signed Message:\n" + len(message) + message
pub fn hash_personal_message(message: &str) -> [u8; 32] {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut data = prefix.into_bytes();
    data.extend_from_slice(message.as_bytes());
    keccak256(&data)
}

/// Represents different types of values that can be hashed for IC request IDs
#[derive(Clone, Serialize, Deserialize)]
pub enum Value<'a> {
    /// Raw bytes
    Bytes(#[serde(with = "serde_bytes")] &'a [u8]),
    /// String value
    String(&'a str),
    /// 64-bit unsigned integer (LEB128 encoded for hashing)
    U64(u64),
    /// Array of values
    Array(Vec<Value<'a>>),
}

/// Hash bytes with SHA256, returning a Hash type
pub fn hash_bytes<T: AsRef<[u8]>>(data: T) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data.as_ref());
    hasher.finalize().into()
}

/// Hash a string with SHA256
pub fn hash_string(value: &str) -> Hash {
    hash_bytes(value.as_bytes())
}

/// Hash a 64-bit integer using LEB128 encoding (IC standard)
fn hash_u64(value: u64) -> Hash {
    let mut buf = [0u8; 10];
    let mut n = value;
    let mut i = 0;

    loop {
        let byte = (n & 0x7f) as u8;
        n >>= 7;
        buf[i] = byte | if n != 0 { 0x80 } else { 0 };

        if n == 0 {
            break;
        }
        i += 1;
    }

    hash_bytes(&buf[..=i])
}

/// Hash an array of values
fn hash_array(elements: Vec<Value<'_>>) -> Hash {
    let mut hasher = Sha256::new();
    for element in elements {
        hasher.update(&hash_value(element)[..]);
    }
    hasher.finalize().into()
}

/// Hash a Value
fn hash_value(val: Value<'_>) -> Hash {
    match val {
        Value::String(s) => hash_string(s),
        Value::Bytes(b) => hash_bytes(b),
        Value::U64(n) => hash_u64(n),
        Value::Array(a) => hash_array(a),
    }
}

/// Helper function to hash a key-value pair
fn hash_key_value(key: &str, val: Value<'_>) -> Vec<u8> {
    let mut key_hash = hash_string(key).to_vec();
    let val_hash = hash_value(val);
    key_hash.extend_from_slice(&val_hash[..]);
    key_hash
}

/// Compute a hash of a map following IC request ID conventions
pub fn hash_of_map<S: AsRef<str>>(map: HashMap<S, Value<'_>>) -> Hash {
    let mut hashes = map
        .into_iter()
        .map(|(key, val)| hash_key_value(key.as_ref(), val))
        .collect::<Vec<_>>();

    hashes.sort_unstable();
    let mut hasher = Sha256::new();
    for hash in hashes {
        hasher.update(&hash);
    }

    hasher.finalize().into()
}

/// Compute a hash with a domain separator (IC standard)
pub fn hash_with_domain(sep: &[u8], bytes: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([sep.len() as u8]);
    hasher.update(sep);
    hasher.update(bytes);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak256() {
        let hash = keccak256(b"hello");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_sha256() {
        let hash = sha256(b"hello");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_personal_message_hash() {
        let hash = hash_personal_message("hello");
        assert_eq!(hash.len(), 32);
    }
}
