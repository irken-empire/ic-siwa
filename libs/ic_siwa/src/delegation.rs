//! Delegation handling for IC-SIWA
//!
//! This module provides utilities for creating delegation identities that allow
//! session keys to act on behalf of authenticated users.

use crate::error::SiwaError;
use crate::hash::{hash_of_map, hash_with_domain, sha256, Value};
use candid::Principal;
use ic_certified_map::{Hash, HashTree};
use serde::Serialize;
use serde_bytes::ByteBuf;
use simple_asn1::{oid, ASN1Block};
use std::collections::HashMap;

/// Information about a delegation for hashing
///
/// This struct contains the fields needed to create a delegation hash
/// that matches the IC's delegation hash format.
pub struct DelegationInfo<'a> {
    /// The public key being delegated to (session key)
    pub pubkey: &'a [u8],
    /// Expiration time in nanoseconds
    pub expiration: u64,
    /// Optional target canisters
    pub targets: Option<&'a [Principal]>,
}

/// Create a hash of a delegation following IC conventions
///
/// The hash is computed as:
/// hash_with_domain("ic-request-auth-delegation", hash_of_map({
///   "pubkey": pubkey,
///   "expiration": expiration,
///   "targets": targets (optional)
/// }))
///
/// # Arguments
/// * `delegation` - The delegation information to hash
///
/// # Returns
/// A 32-byte hash of the delegation
pub fn create_delegation_hash(delegation: &DelegationInfo<'_>) -> Hash {
    let mut delegation_map: HashMap<&str, Value<'_>> = HashMap::new();

    delegation_map.insert("pubkey", Value::Bytes(delegation.pubkey));
    delegation_map.insert("expiration", Value::U64(delegation.expiration));

    if let Some(targets) = delegation.targets {
        let arr: Vec<Value<'_>> = targets.iter().map(|t| Value::Bytes(t.as_slice())).collect();
        delegation_map.insert("targets", Value::Array(arr));
    }

    let delegation_map_hash = hash_of_map(delegation_map);

    hash_with_domain(b"ic-request-auth-delegation", &delegation_map_hash)
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

    // Add salt with length prefix.
    // Settings validation (ticket #035) enforces salt <= 255 bytes at init,
    // so the `as u8` cast is safe. Assert in debug builds as defense-in-depth.
    let salt_bytes = salt.as_bytes();
    debug_assert!(
        salt_bytes.len() <= 255,
        "Salt exceeds 255 bytes; length prefix would truncate"
    );
    seed_input.push(salt_bytes.len() as u8);
    seed_input.extend_from_slice(salt_bytes);

    // Add address with length prefix (lowercase for consistency).
    // Avalanche addresses are always 42 bytes ("0x" + 40 hex chars).
    let address_bytes = address.to_lowercase().as_bytes().to_vec();
    debug_assert!(
        address_bytes.len() <= 255,
        "Address exceeds 255 bytes; length prefix would truncate"
    );
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

/// Structure for the certified signature that combines the IC certificate and hash tree
#[derive(Serialize)]
struct CertificateSignature<'a> {
    /// The certificate from `ic_cdk::api::data_certificate()`
    certificate: ByteBuf,
    /// The hash tree witness proving the delegation exists
    tree: HashTree<'a>,
}

/// Serialize data to CBOR with the self-describing tag (0xD9D9F7)
///
/// The IC requires canister signatures to be CBOR-encoded with the self-describing tag.
/// This function ensures the correct format for IC verification.
///
/// # Arguments
/// * `data` - The data to serialize
///
/// # Returns
/// CBOR-encoded bytes with self-describing tag prefix
pub fn cbor_serialize<T: Serialize>(data: &T) -> Result<Vec<u8>, SiwaError> {
    let mut serializer = serde_cbor::ser::Serializer::new(Vec::new());

    // Add the self-describing tag (0xD9D9F7 = tag 55799)
    serializer
        .self_describe()
        .map_err(|e| SiwaError::DelegationError(format!("CBOR self-describe failed: {}", e)))?;

    // Serialize the data
    data.serialize(&mut serializer)
        .map_err(|e| SiwaError::DelegationError(format!("CBOR serialization failed: {}", e)))?;

    Ok(serializer.into_inner())
}

/// Create a certified signature for a delegation
///
/// This creates the signature structure required by the IC for canister signatures.
/// The signature is CBOR-encoded with the self-describing tag and contains:
/// - The IC system certificate (proves the canister's certified data)
/// - The hash tree witness (proves the delegation hash is in the certified data)
///
/// # Arguments
/// * `certificate` - The certificate from `ic_cdk::api::data_certificate()`
/// * `tree` - The hash tree witness from the SignatureMap
///
/// # Returns
/// CBOR-encoded certified signature bytes
///
/// # Example
/// ```ignore
/// let certificate = ic_cdk::api::data_certificate()
///     .expect("Must be called in a query");
/// let tree = signature_map.witness(seed_hash, delegation_hash)?;
/// let signature = create_certified_signature(certificate, tree)?;
/// ```
pub fn create_certified_signature(
    certificate: Vec<u8>,
    tree: HashTree<'_>,
) -> Result<Vec<u8>, SiwaError> {
    let cert_sig = CertificateSignature {
        certificate: ByteBuf::from(certificate),
        tree,
    };

    cbor_serialize(&cert_sig)
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
        let seed_lower = generate_seed("test-salt", "0x1234567890abcdef1234567890abcdef12345678");
        let seed_upper = generate_seed("test-salt", "0x1234567890ABCDEF1234567890ABCDEF12345678");
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

    #[test]
    fn test_cbor_serialize_with_self_describing_tag() {
        let data = vec![1u8, 2, 3, 4, 5];
        let cbor = cbor_serialize(&data).unwrap();

        // First 3 bytes should be the self-describing tag: 0xD9 0xD9 0xF7
        assert!(cbor.len() >= 3);
        assert_eq!(cbor[0], 0xD9);
        assert_eq!(cbor[1], 0xD9);
        assert_eq!(cbor[2], 0xF7);

        // Should be deserializable
        let deserialized: Vec<u8> = serde_cbor::from_slice(&cbor).unwrap();
        assert_eq!(deserialized, data);
    }
}
