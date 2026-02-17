//! Integration tests for ic_siwa::siwa
//!
//! Moved from inline #[cfg(test)] to eliminate CodeQL false positives
//! for hard-coded test salt values.
//!
//! Private helper function tests (parse_signature, to_eip55_checksum,
//! days_to_ymd, is_leap_year) remain inline since they cannot be tested
//! from integration tests and do not trigger CodeQL.

use ic_siwa::siwa::{derive_principal, hash_session_key, validate_address};
use ic_siwa::SiwaMessage;

// Well-known EIP-55 test vector addresses (not secrets)
const TEST_ADDRESS: &str = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
const TEST_ADDRESS_LOWER: &str = "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed";
const TEST_ADDRESS_UPPER: &str = "0x5AAEB6053F3E94C9B9A09F33669435E7EF1BEAED";
const TEST_ADDRESS_2: &str = "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B";

// --- validate_address tests ---

#[test]
fn test_validate_address_valid() {
    assert!(validate_address(TEST_ADDRESS).is_ok());
}

#[test]
fn test_validate_address_invalid_prefix() {
    assert!(validate_address(&TEST_ADDRESS[2..]).is_err());
}

#[test]
fn test_validate_address_invalid_length() {
    assert!(validate_address(&TEST_ADDRESS[..40]).is_err());
}

#[test]
fn test_validate_address_invalid_checksum() {
    // All lowercase is valid (no checksum enforcement)
    assert!(validate_address(TEST_ADDRESS_LOWER).is_ok());
    // Mixed case with wrong checksum should fail
    assert!(validate_address("0x5AAEB6053f3e94c9b9a09f33669435e7ef1beaed").is_err());
}

#[test]
fn test_validate_address_all_uppercase_hex() {
    // All uppercase is valid (no checksum enforcement for single-case)
    assert!(validate_address("0xABCDEF0123456789ABCDEF0123456789ABCDEF01").is_ok());
}

#[test]
fn test_validate_address_invalid_hex_chars() {
    // Contains invalid hex characters
    // 'g' is not valid hex
    assert!(validate_address(&format!("{}g", &TEST_ADDRESS[..41])).is_err());
}

// --- derive_principal tests ---

#[test]
fn test_derive_principal_deterministic() {
    let addr = TEST_ADDRESS;
    let salt = "test-salt";

    let principal1 = derive_principal(addr, salt).unwrap();
    let principal2 = derive_principal(addr, salt).unwrap();

    // Same input should produce same output
    assert_eq!(principal1, principal2);
}

#[test]
fn test_derive_principal_case_insensitive() {
    let lower = TEST_ADDRESS_LOWER;
    let upper = TEST_ADDRESS_UPPER;
    let mixed = TEST_ADDRESS;
    let salt = "test-salt";

    let p1 = derive_principal(lower, salt).unwrap();
    let p2 = derive_principal(upper, salt).unwrap();
    let p3 = derive_principal(mixed, salt).unwrap();

    // All cases should produce the same principal
    assert_eq!(p1, p2);
    assert_eq!(p2, p3);
}

#[test]
fn test_derive_principal_different_salts() {
    let addr = TEST_ADDRESS;

    let p1 = derive_principal(addr, "salt1").unwrap();
    let p2 = derive_principal(addr, "salt2").unwrap();

    // Different salts should produce different principals
    assert_ne!(p1, p2);
}

#[test]
fn test_derive_principal_different_addresses() {
    let salt = "same-salt";

    let p1 = derive_principal(TEST_ADDRESS, salt).unwrap();
    let p2 = derive_principal(TEST_ADDRESS_2, salt).unwrap();

    // Different addresses should produce different principals
    assert_ne!(p1, p2);
}

// --- hash_session_key tests ---

#[test]
fn test_hash_session_key() {
    let key = vec![1, 2, 3, 4, 5];
    let hash = hash_session_key(&key);

    // Should be 32 hex chars (16 bytes)
    assert_eq!(hash.len(), 32);

    // Should be deterministic
    let hash2 = hash_session_key(&key);
    assert_eq!(hash, hash2);
}

#[test]
fn test_hash_session_key_different_keys() {
    let key1 = vec![1, 2, 3];
    let key2 = vec![1, 2, 4];

    let hash1 = hash_session_key(&key1);
    let hash2 = hash_session_key(&key2);

    // Different keys should produce different hashes
    assert_ne!(hash1, hash2);
}

// --- SiwaMessage tests ---

#[test]
fn test_siwa_message_format() {
    let msg = SiwaMessage {
        domain: "example.com".to_string(),
        address: TEST_ADDRESS.to_string(),
        statement: Some("Sign in with Avalanche to the app.".to_string()),
        uri: "https://example.com".to_string(),
        version: "1".to_string(),
        chain_id: 43114,
        nonce: "abc123".to_string(),
        issued_at: "2024-01-15T12:00:00Z".to_string(),
        expiration_time: Some("2024-01-15T12:05:00Z".to_string()),
        not_before: None,
        request_id: None,
        resources: None,
    };

    let message = msg.to_message();
    assert!(message.contains("example.com wants you to sign in with your Avalanche account"));
    assert!(message.contains(TEST_ADDRESS));
    assert!(message.contains("Chain ID: 43114"));
    assert!(message.contains("Nonce: abc123"));
}

#[test]
fn test_siwa_message_with_all_optional_fields() {
    let msg = SiwaMessage {
        domain: "app.example.com".to_string(),
        address: TEST_ADDRESS.to_string(),
        statement: Some("Custom statement".to_string()),
        uri: "https://app.example.com".to_string(),
        version: "1".to_string(),
        chain_id: 43113, // Fuji testnet
        nonce: "unique-nonce-123".to_string(),
        issued_at: "2024-01-15T12:00:00Z".to_string(),
        expiration_time: Some("2024-01-15T12:30:00Z".to_string()),
        not_before: Some("2024-01-15T11:55:00Z".to_string()),
        request_id: Some("req-456".to_string()),
        resources: Some(vec!["https://api.example.com".to_string()]),
    };

    let message = msg.to_message();
    assert!(message.contains("Chain ID: 43113"));
    assert!(message.contains("Custom statement"));
    assert!(message.contains("Not Before: 2024-01-15T11:55:00Z"));
    assert!(message.contains("Request ID: req-456"));
    assert!(message.contains("Resources:"));
    assert!(message.contains("- https://api.example.com"));
}

#[test]
fn test_siwa_message_without_optional_fields() {
    let msg = SiwaMessage {
        domain: "example.com".to_string(),
        address: TEST_ADDRESS.to_string(),
        statement: None,
        uri: "https://example.com".to_string(),
        version: "1".to_string(),
        chain_id: 43114,
        nonce: "abc".to_string(),
        issued_at: "2024-01-15T12:00:00Z".to_string(),
        expiration_time: None,
        not_before: None,
        request_id: None,
        resources: None,
    };

    let message = msg.to_message();
    assert!(!message.contains("Expiration Time:"));
    assert!(!message.contains("Not Before:"));
    assert!(!message.contains("Request ID:"));
    assert!(!message.contains("Resources:"));
}

// --- from_message roundtrip tests ---

#[test]
fn test_from_message_roundtrip() {
    let msg = SiwaMessage {
        domain: "example.com".to_string(),
        address: TEST_ADDRESS.to_string(),
        statement: Some("Sign in with Avalanche to the app.".to_string()),
        uri: "https://example.com".to_string(),
        version: "1".to_string(),
        chain_id: 43114,
        nonce: "abc123".to_string(),
        issued_at: "2024-01-15T12:00:00Z".to_string(),
        expiration_time: Some("2024-01-15T12:05:00Z".to_string()),
        not_before: None,
        request_id: None,
        resources: None,
    };

    let message_string = msg.to_message();
    let parsed = SiwaMessage::from_message(&message_string).unwrap();

    assert_eq!(parsed.domain, msg.domain);
    assert_eq!(parsed.address, msg.address);
    assert_eq!(parsed.statement, msg.statement);
    assert_eq!(parsed.uri, msg.uri);
    assert_eq!(parsed.version, msg.version);
    assert_eq!(parsed.chain_id, msg.chain_id);
    assert_eq!(parsed.nonce, msg.nonce);
    assert_eq!(parsed.issued_at, msg.issued_at);
    assert_eq!(parsed.expiration_time, msg.expiration_time);
    assert_eq!(parsed.to_message(), message_string);
}

#[test]
fn test_from_message_with_all_fields() {
    let msg = SiwaMessage {
        domain: "full-test.example.com".to_string(),
        address: TEST_ADDRESS.to_string(),
        statement: Some("Please sign to verify your identity.".to_string()),
        uri: "https://full-test.example.com".to_string(),
        version: "1".to_string(),
        chain_id: 43114,
        nonce: "nonce-xyz".to_string(),
        issued_at: "2024-06-01T10:00:00Z".to_string(),
        expiration_time: Some("2024-06-01T10:30:00Z".to_string()),
        not_before: Some("2024-06-01T09:55:00Z".to_string()),
        request_id: Some("req-abc".to_string()),
        resources: Some(vec!["https://api.example.com".to_string()]),
    };

    let message_string = msg.to_message();
    let parsed = SiwaMessage::from_message(&message_string).unwrap();

    assert_eq!(
        parsed.expiration_time,
        Some("2024-06-01T10:30:00Z".to_string())
    );
    assert_eq!(parsed.not_before, Some("2024-06-01T09:55:00Z".to_string()));
    assert_eq!(parsed.request_id, Some("req-abc".to_string()));
    assert_eq!(
        parsed.resources,
        Some(vec!["https://api.example.com".to_string()])
    );

    // Verify roundtrip produces identical message string
    assert_eq!(parsed.to_message(), message_string);
}

#[test]
fn test_from_message_without_optional_fields() {
    let msg = SiwaMessage {
        domain: "example.com".to_string(),
        address: TEST_ADDRESS.to_string(),
        statement: None,
        uri: "https://example.com".to_string(),
        version: "1".to_string(),
        chain_id: 43114,
        nonce: "abc".to_string(),
        issued_at: "2024-01-15T12:00:00Z".to_string(),
        expiration_time: None,
        not_before: None,
        request_id: None,
        resources: None,
    };

    let message_string = msg.to_message();
    let parsed = SiwaMessage::from_message(&message_string).unwrap();

    assert_eq!(parsed.statement, None);
    assert_eq!(parsed.expiration_time, None);
    assert_eq!(parsed.to_message(), message_string);
}

#[test]
fn test_from_message_multi_tenant_domain() {
    let msg = SiwaMessage {
        domain: "game.tresr.community".to_string(),
        address: TEST_ADDRESS.to_string(),
        statement: Some("Sign in with Avalanche to the app.".to_string()),
        uri: "https://game.tresr.community".to_string(),
        version: "1".to_string(),
        chain_id: 43114,
        nonce: "tenant-nonce-789".to_string(),
        issued_at: "2024-01-15T12:00:00Z".to_string(),
        expiration_time: Some("2024-01-15T12:05:00Z".to_string()),
        not_before: None,
        request_id: None,
        resources: None,
    };

    let message_string = msg.to_message();
    let parsed = SiwaMessage::from_message(&message_string).unwrap();

    // The parsed message must preserve the original domain/URI, not canister defaults
    assert_eq!(parsed.domain, "game.tresr.community");
    assert_eq!(parsed.uri, "https://game.tresr.community");
    assert_eq!(parsed.to_message(), message_string);
}

#[test]
fn test_from_message_invalid_empty() {
    assert!(SiwaMessage::from_message("").is_err());
}

#[test]
fn test_from_message_invalid_first_line() {
    assert!(SiwaMessage::from_message("not a valid SIWA message").is_err());
}

#[test]
fn test_from_message_missing_uri() {
    let msg = format!("example.com wants you to sign in with your Avalanche account:\n{}\n\nVersion: 1\nChain ID: 43114\nNonce: abc\nIssued At: 2024-01-15T12:00:00Z", TEST_ADDRESS);
    assert!(SiwaMessage::from_message(&msg).is_err());
}

// --- format_timestamp / parse_timestamp tests ---

#[test]
fn test_format_timestamp() {
    use ic_siwa::siwa::format_timestamp;

    // Unix epoch
    assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
    // Test a known timestamp - verify format is correct
    let ts = format_timestamp(1705322245);
    assert!(ts.starts_with("2024-01-15T"));
    assert!(ts.ends_with("Z"));
}

#[test]
fn test_parse_timestamp_roundtrip() {
    use ic_siwa::siwa::{format_timestamp, parse_timestamp};

    // Verify parse_timestamp is the inverse of format_timestamp
    for &secs in &[0u64, 1705322245, 86400, 1_700_000_000] {
        let formatted = format_timestamp(secs);
        assert_eq!(
            parse_timestamp(&formatted),
            Some(secs),
            "roundtrip failed for {secs}"
        );
    }
}

#[test]
fn test_parse_timestamp_invalid() {
    use ic_siwa::siwa::parse_timestamp;

    assert_eq!(parse_timestamp(""), None);
    assert_eq!(parse_timestamp("not-a-timestamp-here"), None);
    assert_eq!(parse_timestamp("2024-01-15T12:00:00"), None); // missing Z
    assert_eq!(parse_timestamp("2024-13-15T12:00:00Z"), None); // month 13
}
