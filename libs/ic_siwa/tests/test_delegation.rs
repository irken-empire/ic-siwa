//! Integration tests for ic_siwa::delegation
//!
//! Moved from inline #[cfg(test)] to eliminate CodeQL false positives
//! for hard-coded test salt values.

use candid::Principal;
use ic_siwa::delegation::{cbor_serialize, create_user_canister_pubkey, generate_seed};
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
