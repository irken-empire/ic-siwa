//! Hash utilities for IC-SIWA

use sha2::{Digest, Sha256};
use sha3::Keccak256;

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
