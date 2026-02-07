//! Settings for IC-SIWA

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Rate limiting configuration
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RateLimitSettings {
    /// Max prepare_login calls per address per window
    pub max_logins_per_address: u32,
    /// Max total prepare_login calls per window (across all addresses)
    pub max_logins_total: u32,
    /// Time window in seconds for rate limiting
    pub window_seconds: u64,
}

impl Default for RateLimitSettings {
    fn default() -> Self {
        Self {
            max_logins_per_address: 10,
            max_logins_total: 1000,
            window_seconds: 3600, // 1 hour
        }
    }
}

/// SIWA settings
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    /// Domain for SIWA messages
    pub domain: String,
    /// URI for SIWA messages
    pub uri: String,
    /// Salt for principal derivation
    pub salt: String,
    /// Chain ID (Avalanche C-Chain mainnet: 43114, Fuji testnet: 43113)
    pub chain_id: u64,
    /// Session expiration in nanoseconds
    pub session_expiration_time: u64,
    /// Login message expiration in nanoseconds
    pub login_expiration_time: u64,
    /// Allowed domains for requests
    pub allowed_domains: Vec<String>,
    /// Allowed canister IDs for inter-canister calls
    pub allowed_canisters: Vec<Principal>,
    /// Delegation targets - canisters that delegations are valid for
    /// If empty, delegations are unrestricted (work for any canister)
    pub delegation_targets: Vec<Principal>,
    /// Rate limiting configuration
    pub rate_limits: RateLimitSettings,
    /// Debug mode - enables diagnostic endpoints (should be false in production)
    pub debug: bool,
}

impl Settings {
    /// Create new settings with default values
    pub fn new(domain: &str, uri: &str, salt: &str) -> Self {
        Self {
            domain: domain.to_string(),
            uri: uri.to_string(),
            salt: salt.to_string(),
            chain_id: 43114, // Avalanche C-Chain mainnet
            session_expiration_time: 30 * 60 * 1_000_000_000, // 30 minutes in nanoseconds
            login_expiration_time: 5 * 60 * 1_000_000_000, // 5 minutes in nanoseconds
            allowed_domains: vec![],
            allowed_canisters: vec![],
            delegation_targets: vec![],
            rate_limits: RateLimitSettings::default(),
            debug: false,
        }
    }

    /// Set chain ID
    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = chain_id;
        self
    }

    /// Set session expiration time
    pub fn with_session_expiration(mut self, time_ns: u64) -> Self {
        self.session_expiration_time = time_ns;
        self
    }

    /// Set allowed domains
    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = domains;
        self
    }

    /// Set allowed canisters
    pub fn with_allowed_canisters(mut self, canisters: Vec<Principal>) -> Self {
        self.allowed_canisters = canisters;
        self
    }

    /// Set delegation targets
    pub fn with_delegation_targets(mut self, targets: Vec<Principal>) -> Self {
        self.delegation_targets = targets;
        self
    }

    /// Set rate limits
    pub fn with_rate_limits(mut self, rate_limits: RateLimitSettings) -> Self {
        self.rate_limits = rate_limits;
        self
    }

    /// Validate settings values
    ///
    /// Checks that all critical fields are within acceptable bounds.
    /// Should be called during canister init/post_upgrade.
    pub fn validate(&self) -> Result<(), String> {
        // Salt must be non-empty and within length prefix bounds
        if self.salt.is_empty() {
            return Err("Salt must not be empty".to_string());
        }
        if self.salt.len() > 255 {
            return Err("Salt must not exceed 255 bytes".to_string());
        }

        // Domain and URI must be non-empty
        if self.domain.is_empty() {
            return Err("Domain must not be empty".to_string());
        }
        if self.uri.is_empty() {
            return Err("URI must not be empty".to_string());
        }

        // Chain ID must be a known Avalanche chain
        if self.chain_id != 43113 && self.chain_id != 43114 {
            return Err(format!(
                "Chain ID must be 43113 (Fuji) or 43114 (Mainnet), got {}",
                self.chain_id
            ));
        }

        // Session expiration: at least 1 minute, at most 30 days
        let one_minute_ns = 60 * 1_000_000_000u64;
        let thirty_days_ns = 30 * 24 * 60 * 60 * 1_000_000_000u64;
        if self.session_expiration_time < one_minute_ns {
            return Err("Session expiration must be at least 1 minute".to_string());
        }
        if self.session_expiration_time > thirty_days_ns {
            return Err("Session expiration must not exceed 30 days".to_string());
        }

        Ok(())
    }

    /// Check if delegation targets are configured
    pub fn has_delegation_targets(&self) -> bool {
        !self.delegation_targets.is_empty()
    }

    /// Get delegation targets as Option for Candid serialization
    pub fn delegation_targets_option(&self) -> Option<Vec<Principal>> {
        if self.delegation_targets.is_empty() {
            None
        } else {
            Some(self.delegation_targets.clone())
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new("localhost", "http://localhost", "development-salt")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_ids;

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
        let settings = Settings::new("test.com", "https://test.com", "salt")
            .with_session_expiration(one_hour_ns);

        assert_eq!(settings.session_expiration_time, one_hour_ns);
    }

    #[test]
    fn test_settings_with_allowed_domains() {
        let domains = vec!["example.com".to_string(), "*.example.com".to_string()];
        let settings = Settings::new("test.com", "https://test.com", "salt")
            .with_allowed_domains(domains.clone());

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
        let settings = Settings::new("test.com", "https://test.com", "salt")
            .with_delegation_targets(vec![target]);

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
        let settings = Settings::new("test.com", "https://test.com", "salt")
            .with_rate_limits(rate_limits.clone());

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
        let settings = Settings::new("example.com", "https://example.com", "salt").with_chain_id(1); // Ethereum mainnet
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
}
