//! Delegation handling for IC-SIWA
//!
//! This module provides utilities for creating delegation identities that allow
//! session keys to act on behalf of authenticated users.

use crate::error::SiwaError;
use crate::hash::sha256;
use crate::types::SignedDelegation;
use candid::Principal;
use simple_asn1::{oid, ASN1Block};

/// Delegation manager
pub struct Delegation {
    // TODO: Add delegation state
}

impl Delegation {
    /// Create a new delegation manager
    pub fn new() -> Self {
        Self {}
    }

    /// Create a delegation for a principal
    pub fn create(
        &self,
        _user_principal: Principal,
        _session_key: &[u8],
        _expiration: u64,
    ) -> Result<SignedDelegation, SiwaError> {
        // TODO: Implement delegation creation
        // 1. Create delegation with session key and expiration
        // 2. Sign delegation with canister key
        // 3. Return signed delegation
        Err(SiwaError::DelegationError("Not implemented".to_string()))
    }

    /// Verify a delegation
    pub fn verify(&self, _delegation: &SignedDelegation) -> Result<bool, SiwaError> {
        // TODO: Implement delegation verification
        Err(SiwaError::DelegationError("Not implemented".to_string()))
    }
}

impl Default for Delegation {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a seed for principal derivation from the salt and address.
///
/// This seed is used both for deriving the user's principal and for
/// creating the canister's public key that signs delegations.
///
/// # Arguments
/// * `salt` - The secret salt configured for the canister
/// * `address` - The Avalanche address (lowercase, with 0x prefix)
///
/// # Returns
/// A 32-byte seed derived from the inputs
pub fn generate_seed(salt: &str, address: &str) -> [u8; 32] {
    let mut seed_input: Vec<u8> = vec![];

    // Add salt with length prefix
    let salt_bytes = salt.as_bytes();
    seed_input.push(salt_bytes.len() as u8);
    seed_input.extend_from_slice(salt_bytes);

    // Add address with length prefix (lowercase for consistency)
    let address_bytes = address.to_lowercase().as_bytes().to_vec();
    seed_input.push(address_bytes.len() as u8);
    seed_input.extend_from_slice(&address_bytes);

    sha256(&seed_input)
}

/// Creates a DER-encoded public key for the user's canister identity.
///
/// This public key represents the canister's derived identity for a specific user.
/// It is used as the root of the delegation chain, allowing the canister to
/// delegate to session keys.
///
/// The key uses the IC's canister signature scheme (OID 1.3.6.1.4.1.56387.1.2).
///
/// # Arguments
/// * `canister_id` - The principal of the canister creating the delegation
/// * `seed` - The seed derived from the user's address and salt
///
/// # Returns
/// DER-encoded public key bytes that can be used in delegation chains
///
/// # Example
/// ```ignore
/// let seed = generate_seed(&salt, &address);
/// let pubkey = create_user_canister_pubkey(&ic_cdk::api::id(), &seed)?;
/// ```
pub fn create_user_canister_pubkey(
    canister_id: &Principal,
    seed: &[u8; 32],
) -> Result<Vec<u8>, SiwaError> {
    let canister_id_bytes: Vec<u8> = canister_id.as_slice().to_vec();

    // Build the key blob: [canister_id_len][canister_id][seed]
    let mut key_blob: Vec<u8> = vec![];
    key_blob.push(canister_id_bytes.len() as u8);
    key_blob.extend(&canister_id_bytes);
    key_blob.extend_from_slice(seed);

    // IC canister signature OID: 1.3.6.1.4.1.56387.1.2
    let algorithm_oid = oid!(1, 3, 6, 1, 4, 1, 56387, 1, 2);
    let algorithm = ASN1Block::Sequence(0, vec![ASN1Block::ObjectIdentifier(0, algorithm_oid)]);

    // Wrap the key blob in a BitString
    let subject_public_key = ASN1Block::BitString(0, key_blob.len() * 8, key_blob);

    // Create the SubjectPublicKeyInfo structure
    let subject_public_key_info = ASN1Block::Sequence(0, vec![algorithm, subject_public_key]);

    // Encode to DER
    simple_asn1::to_der(&subject_public_key_info)
        .map_err(|e| SiwaError::DelegationError(format!("Failed to encode public key: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_asn1::from_der;

    #[test]
    fn test_generate_seed() {
        let seed = generate_seed("test-salt", "0x1234567890abcdef1234567890abcdef12345678");
        assert_eq!(seed.len(), 32);

        // Same inputs should produce same seed
        let seed2 = generate_seed("test-salt", "0x1234567890abcdef1234567890abcdef12345678");
        assert_eq!(seed, seed2);

        // Different inputs should produce different seeds
        let seed3 = generate_seed("other-salt", "0x1234567890abcdef1234567890abcdef12345678");
        assert_ne!(seed, seed3);
    }

    #[test]
    fn test_generate_seed_case_insensitive() {
        let seed_lower =
            generate_seed("test-salt", "0x1234567890abcdef1234567890abcdef12345678");
        let seed_upper =
            generate_seed("test-salt", "0x1234567890ABCDEF1234567890ABCDEF12345678");
        assert_eq!(seed_lower, seed_upper);
    }

    #[test]
    fn test_create_user_canister_pubkey() {
        let canister_id = Principal::from_text("aaaaa-aa").unwrap();
        let seed = generate_seed("test-salt", "0x1234567890abcdef1234567890abcdef12345678");

        let pubkey = create_user_canister_pubkey(&canister_id, &seed).unwrap();

        // Verify it's valid DER
        let result = from_der(&pubkey);
        assert!(result.is_ok(), "Should be valid DER-encoded public key");

        // Verify it's not empty
        assert!(!pubkey.is_empty());
    }

    #[test]
    fn test_create_user_canister_pubkey_deterministic() {
        let canister_id = Principal::from_text("aaaaa-aa").unwrap();
        let seed = generate_seed("test-salt", "0x1234567890abcdef1234567890abcdef12345678");

        let pubkey1 = create_user_canister_pubkey(&canister_id, &seed).unwrap();
        let pubkey2 = create_user_canister_pubkey(&canister_id, &seed).unwrap();

        assert_eq!(pubkey1, pubkey2, "Same inputs should produce same pubkey");
    }
}
