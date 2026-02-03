//! Integration tests for ic_siwa_provider canister
//!
//! These tests exercise the canister's service layer logic.
//! For true end-to-end tests with IC runtime, use PocketIC or dfx-based testing.

use candid::Principal;

// Test helpers
mod helpers {
    use candid::Principal;

    pub fn test_address() -> String {
        // Valid EIP-55 checksummed address
        "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed".to_string()
    }

    pub fn test_address_2() -> String {
        "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B".to_string()
    }

    pub fn test_principal() -> Principal {
        Principal::from_slice(&[1, 2, 3, 4, 5])
    }

    pub fn zero_address() -> String {
        "0x0000000000000000000000000000000000000000".to_string()
    }
}

// Note: These tests are designed to test the library logic.
// Full integration tests would use PocketIC or dfx canister calls.

#[cfg(test)]
mod library_tests {
    use ic_siwa::siwa::{derive_principal, hash_session_key, validate_address};
    use ic_siwa::types::{DomainPattern, DomainValidator};
    use ic_siwa::{CanisterGuard, SecurityMode, Settings};

    use super::helpers::*;

    #[test]
    fn test_address_validation_valid() {
        assert!(validate_address(&test_address()).is_ok());
        assert!(validate_address(&test_address_2()).is_ok());
    }

    #[test]
    fn test_address_validation_invalid() {
        // Missing 0x prefix
        assert!(validate_address("5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed").is_err());

        // Wrong length
        assert!(validate_address("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1Be").is_err());

        // Invalid checksum (mixed case but wrong)
        assert!(validate_address("0x5AAEB6053f3e94c9b9a09f33669435e7ef1beaed").is_err());
    }

    #[test]
    fn test_principal_derivation_consistency() {
        let salt = "test-salt";

        // Same address and salt should always produce same principal
        let p1 = derive_principal(&test_address(), salt).unwrap();
        let p2 = derive_principal(&test_address(), salt).unwrap();
        assert_eq!(p1, p2);

        // Different addresses should produce different principals
        let p3 = derive_principal(&test_address_2(), salt).unwrap();
        assert_ne!(p1, p3);

        // Different salts should produce different principals
        let p4 = derive_principal(&test_address(), "different-salt").unwrap();
        assert_ne!(p1, p4);
    }

    #[test]
    fn test_principal_derivation_case_insensitive() {
        let salt = "test-salt";
        let lower = test_address().to_lowercase();
        let upper = test_address().to_uppercase();

        let p1 = derive_principal(&lower, salt).unwrap();
        let p2 = derive_principal(&upper, salt).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_session_key_hashing() {
        let key1 = vec![1, 2, 3, 4, 5];
        let key2 = vec![1, 2, 3, 4, 6];

        let hash1 = hash_session_key(&key1);
        let hash2 = hash_session_key(&key1);
        let hash3 = hash_session_key(&key2);

        // Same key produces same hash
        assert_eq!(hash1, hash2);

        // Different keys produce different hashes
        assert_ne!(hash1, hash3);

        // Hash is 32 hex chars (16 bytes)
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_domain_validation_exact() {
        let validator = DomainValidator::new(&["example.com".to_string()]);

        assert!(validator.is_allowed("example.com"));
        assert!(!validator.is_allowed("other.com"));
        assert!(!validator.is_allowed("sub.example.com"));
    }

    #[test]
    fn test_domain_validation_wildcard() {
        let validator = DomainValidator::new(&["*.example.com".to_string()]);

        assert!(validator.is_allowed("example.com"));
        assert!(validator.is_allowed("sub.example.com"));
        assert!(validator.is_allowed("deep.sub.example.com"));
        assert!(!validator.is_allowed("other.com"));
    }

    #[test]
    fn test_domain_validation_multiple() {
        let validator = DomainValidator::new(&[
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            "*.example.com".to_string(),
        ]);

        assert!(validator.is_allowed("localhost"));
        assert!(validator.is_allowed("127.0.0.1"));
        assert!(validator.is_allowed("example.com"));
        assert!(validator.is_allowed("app.example.com"));
        assert!(!validator.is_allowed("other.com"));
    }

    #[test]
    fn test_domain_validation_empty() {
        let validator = DomainValidator::new(&[]);

        // Empty validator allows nothing
        assert!(!validator.is_allowed("example.com"));
        assert!(validator.is_empty());
    }

    #[test]
    fn test_canister_guard_production() {
        let allowed = test_principal();
        let guard = CanisterGuard::new(vec![allowed]).with_mode(SecurityMode::Production);

        // Allowed principal passes
        assert!(guard.validate(&allowed).is_ok());

        // Other principal rejected
        let other = candid::Principal::from_slice(&[6, 7, 8]);
        assert!(guard.validate(&other).is_err());

        // Anonymous rejected
        assert!(guard.validate(&candid::Principal::anonymous()).is_err());
    }

    #[test]
    fn test_canister_guard_development() {
        let guard = CanisterGuard::development();

        // Development mode allows all
        assert!(guard.validate(&test_principal()).is_ok());
        assert!(guard.validate(&candid::Principal::anonymous()).is_ok());
    }

    #[test]
    fn test_canister_guard_empty_whitelist() {
        let guard = CanisterGuard::new(vec![]).with_mode(SecurityMode::Production);

        // Empty whitelist allows all authenticated
        assert!(guard.validate(&test_principal()).is_ok());

        // Anonymous still rejected by default
        assert!(guard.validate(&candid::Principal::anonymous()).is_err());
    }

    #[test]
    fn test_canister_guard_allow_anonymous() {
        let guard = CanisterGuard::new(vec![])
            .with_mode(SecurityMode::Production)
            .with_allow_anonymous(true);

        // Now anonymous is allowed
        assert!(guard.validate(&candid::Principal::anonymous()).is_ok());
    }

    #[test]
    fn test_settings_creation() {
        let settings = Settings::new("example.com", "https://example.com", "secret");

        assert_eq!(settings.domain, "example.com");
        assert_eq!(settings.uri, "https://example.com");
        assert_eq!(settings.salt, "secret");
        assert_eq!(settings.chain_id, 43114); // Avalanche mainnet
    }

    #[test]
    fn test_settings_builder() {
        let settings = Settings::new("app.com", "https://app.com", "salt")
            .with_chain_id(43113) // Fuji testnet
            .with_session_expiration(3600_000_000_000)
            .with_allowed_domains(vec!["*.app.com".to_string()]);

        assert_eq!(settings.chain_id, 43113);
        assert_eq!(settings.session_expiration_time, 3600_000_000_000);
        assert_eq!(settings.allowed_domains, vec!["*.app.com"]);
    }
}

#[cfg(test)]
mod siwa_message_tests {
    use ic_siwa::SiwaMessage;

    #[test]
    fn test_siwa_message_format() {
        let msg = SiwaMessage {
            domain: "example.com".to_string(),
            address: "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed".to_string(),
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

        // Verify EIP-4361 format
        assert!(message.contains("example.com wants you to sign in with your Avalanche account:"));
        assert!(message.contains("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"));
        assert!(message.contains("Sign in with Avalanche to the app."));
        assert!(message.contains("URI: https://example.com"));
        assert!(message.contains("Version: 1"));
        assert!(message.contains("Chain ID: 43114"));
        assert!(message.contains("Nonce: abc123"));
        assert!(message.contains("Issued At: 2024-01-15T12:00:00Z"));
        assert!(message.contains("Expiration Time: 2024-01-15T12:05:00Z"));
    }

    #[test]
    fn test_siwa_message_with_resources() {
        let msg = SiwaMessage {
            domain: "app.com".to_string(),
            address: "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed".to_string(),
            statement: None,
            uri: "https://app.com".to_string(),
            version: "1".to_string(),
            chain_id: 43113,
            nonce: "xyz".to_string(),
            issued_at: "2024-01-01T00:00:00Z".to_string(),
            expiration_time: None,
            not_before: Some("2024-01-01T00:00:00Z".to_string()),
            request_id: Some("req-123".to_string()),
            resources: Some(vec![
                "https://api.app.com".to_string(),
                "https://storage.app.com".to_string(),
            ]),
        };

        let message = msg.to_message();

        assert!(message.contains("Chain ID: 43113")); // Fuji testnet
        assert!(message.contains("Not Before: 2024-01-01T00:00:00Z"));
        assert!(message.contains("Request ID: req-123"));
        assert!(message.contains("Resources:"));
        assert!(message.contains("- https://api.app.com"));
        assert!(message.contains("- https://storage.app.com"));
    }

    #[test]
    fn test_siwa_message_minimal() {
        let msg = SiwaMessage {
            domain: "test.com".to_string(),
            address: "0x0000000000000000000000000000000000000000".to_string(),
            statement: None,
            uri: "https://test.com".to_string(),
            version: "1".to_string(),
            chain_id: 43114,
            nonce: "n".to_string(),
            issued_at: "2024-01-01T00:00:00Z".to_string(),
            expiration_time: None,
            not_before: None,
            request_id: None,
            resources: None,
        };

        let message = msg.to_message();

        // Should not contain optional fields
        assert!(!message.contains("Expiration Time:"));
        assert!(!message.contains("Not Before:"));
        assert!(!message.contains("Request ID:"));
        assert!(!message.contains("Resources:"));
    }
}

// Note: Full integration tests with actual IC canister calls would use PocketIC
// Example structure for PocketIC tests (requires pocket-ic dependency):
//
// #[cfg(test)]
// mod pocket_ic_tests {
//     use pocket_ic::PocketIc;
//
//     #[test]
//     fn test_full_login_flow() {
//         let pic = PocketIc::new();
//         // Deploy canister
//         // Call siwa_prepare_login
//         // Sign message (mock)
//         // Call siwa_login
//         // Verify principal
//     }
// }
