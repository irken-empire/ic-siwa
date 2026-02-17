//! Integration tests for ic_siwa::settings
//!
//! Moved from inline #[cfg(test)] to eliminate CodeQL false positives
//! for hard-coded test salt values.

use candid::Principal;
use ic_siwa::chain_ids;
use ic_siwa::settings::{RateLimitSettings, Settings};

#[test]
fn test_settings_new() {
    let settings = Settings::new("example.com", "https://example.com", "test-salt");

    assert_eq!(settings.domain, "example.com");
    assert_eq!(settings.uri, "https://example.com");
    assert_eq!(settings.salt, "test-salt");
    assert_eq!(settings.chain_id, chain_ids::AVALANCHE_MAINNET);
    assert_eq!(settings.session_expiration_time, 30 * 60 * 1_000_000_000);
    assert_eq!(settings.login_expiration_time, 5 * 60 * 1_000_000_000);
    assert!(settings.allowed_domains.is_empty());
    assert!(settings.allowed_canisters.is_empty());
    assert!(settings.delegation_targets.is_empty());
    assert_eq!(settings.rate_limits, RateLimitSettings::default());
}

#[test]
fn test_settings_default() {
    let settings = Settings::default();

    assert_eq!(settings.domain, "localhost");
    assert_eq!(settings.uri, "http://localhost");
    assert_eq!(settings.salt, "development-salt");
}

#[test]
fn test_settings_with_chain_id() {
    let settings = Settings::new("test.com", "https://test.com", "salt")
        .with_chain_id(chain_ids::AVALANCHE_FUJI);

    assert_eq!(settings.chain_id, 43113);
}

#[test]
fn test_settings_with_session_expiration() {
    let one_hour_ns = 60 * 60 * 1_000_000_000;
    let settings =
        Settings::new("test.com", "https://test.com", "salt").with_session_expiration(one_hour_ns);

    assert_eq!(settings.session_expiration_time, one_hour_ns);
}

#[test]
fn test_settings_with_allowed_domains() {
    let domains = vec!["example.com".to_string(), "*.example.com".to_string()];
    let settings =
        Settings::new("test.com", "https://test.com", "salt").with_allowed_domains(domains.clone());

    assert_eq!(settings.allowed_domains, domains);
}

#[test]
fn test_settings_with_allowed_canisters() {
    let canister = Principal::from_slice(&[1, 2, 3]);
    let settings = Settings::new("test.com", "https://test.com", "salt")
        .with_allowed_canisters(vec![canister]);

    assert_eq!(settings.allowed_canisters.len(), 1);
    assert_eq!(settings.allowed_canisters[0], canister);
}

#[test]
fn test_settings_with_delegation_targets() {
    let target = Principal::from_slice(&[4, 5, 6]);
    let settings =
        Settings::new("test.com", "https://test.com", "salt").with_delegation_targets(vec![target]);

    assert_eq!(settings.delegation_targets.len(), 1);
    assert_eq!(settings.delegation_targets[0], target);
    assert!(settings.has_delegation_targets());
    assert_eq!(settings.delegation_targets_option(), Some(vec![target]));
}

#[test]
fn test_settings_delegation_targets_empty() {
    let settings = Settings::new("test.com", "https://test.com", "salt");

    assert!(!settings.has_delegation_targets());
    assert_eq!(settings.delegation_targets_option(), None);
}

#[test]
fn test_settings_with_rate_limits() {
    let rate_limits = RateLimitSettings {
        max_logins_per_address: 5,
        max_logins_total: 100,
        window_seconds: 1800,
    };
    let settings =
        Settings::new("test.com", "https://test.com", "salt").with_rate_limits(rate_limits.clone());

    assert_eq!(settings.rate_limits, rate_limits);
}

#[test]
fn test_settings_builder_chain() {
    let settings = Settings::new("app.com", "https://app.com", "secret")
        .with_chain_id(43113)
        .with_session_expiration(3_600_000_000_000)
        .with_allowed_domains(vec!["*.app.com".to_string()]);

    assert_eq!(settings.domain, "app.com");
    assert_eq!(settings.chain_id, 43113);
    assert_eq!(settings.session_expiration_time, 3_600_000_000_000);
    assert_eq!(settings.allowed_domains, vec!["*.app.com".to_string()]);
}

#[test]
fn test_chain_ids_constants() {
    assert_eq!(chain_ids::AVALANCHE_MAINNET, 43114);
    assert_eq!(chain_ids::AVALANCHE_FUJI, 43113);
}

#[test]
fn test_rate_limit_settings_default() {
    let rate_limits = RateLimitSettings::default();

    assert_eq!(rate_limits.max_logins_per_address, 10);
    assert_eq!(rate_limits.max_logins_total, 1000);
    assert_eq!(rate_limits.window_seconds, 3600);
}

// --- Validation tests ---

#[test]
fn test_validate_valid_settings() {
    let settings = Settings::new("example.com", "https://example.com", "my-secret-salt");
    assert!(settings.validate().is_ok());
}

#[test]
fn test_validate_fuji_settings() {
    let settings = Settings::new("example.com", "https://example.com", "salt")
        .with_chain_id(chain_ids::AVALANCHE_FUJI);
    assert!(settings.validate().is_ok());
}

#[test]
fn test_validate_empty_salt() {
    let settings = Settings::new("example.com", "https://example.com", "");
    assert_eq!(
        settings.validate(),
        Err("Salt must not be empty".to_string())
    );
}

#[test]
fn test_validate_salt_too_long() {
    let long_salt = "a".repeat(256);
    let settings = Settings::new("example.com", "https://example.com", &long_salt);
    assert_eq!(
        settings.validate(),
        Err("Salt must not exceed 255 bytes".to_string())
    );
}

#[test]
fn test_validate_salt_max_length() {
    let max_salt = "a".repeat(255);
    let settings = Settings::new("example.com", "https://example.com", &max_salt);
    assert!(settings.validate().is_ok());
}

#[test]
fn test_validate_empty_domain() {
    let mut settings = Settings::new("example.com", "https://example.com", "salt");
    settings.domain = String::new();
    assert_eq!(
        settings.validate(),
        Err("Domain must not be empty".to_string())
    );
}

#[test]
fn test_validate_empty_uri() {
    let mut settings = Settings::new("example.com", "https://example.com", "salt");
    settings.uri = String::new();
    assert_eq!(
        settings.validate(),
        Err("URI must not be empty".to_string())
    );
}

#[test]
fn test_validate_invalid_chain_id() {
    let settings = Settings::new("example.com", "https://example.com", "salt").with_chain_id(1);
    assert!(settings
        .validate()
        .unwrap_err()
        .contains("Chain ID must be"));
}

#[test]
fn test_validate_zero_session_expiration() {
    let settings =
        Settings::new("example.com", "https://example.com", "salt").with_session_expiration(0);
    assert!(settings
        .validate()
        .unwrap_err()
        .contains("at least 1 minute"));
}

#[test]
fn test_validate_excessive_session_expiration() {
    let one_year_ns = 365 * 24 * 60 * 60 * 1_000_000_000u64;
    let settings = Settings::new("example.com", "https://example.com", "salt")
        .with_session_expiration(one_year_ns);
    assert!(settings
        .validate()
        .unwrap_err()
        .contains("must not exceed 30 days"));
}

#[test]
fn test_validate_min_session_expiration() {
    let one_minute_ns = 60 * 1_000_000_000u64;
    let settings = Settings::new("example.com", "https://example.com", "salt")
        .with_session_expiration(one_minute_ns);
    assert!(settings.validate().is_ok());
}

#[test]
fn test_validate_max_session_expiration() {
    let thirty_days_ns = 30 * 24 * 60 * 60 * 1_000_000_000u64;
    let settings = Settings::new("example.com", "https://example.com", "salt")
        .with_session_expiration(thirty_days_ns);
    assert!(settings.validate().is_ok());
}
