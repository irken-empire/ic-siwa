//! Settings for IC-SIWA

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

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
            login_expiration_time: 5 * 60 * 1_000_000_000,    // 5 minutes in nanoseconds
            allowed_domains: vec![],
            allowed_canisters: vec![],
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
}

impl Default for Settings {
    fn default() -> Self {
        Self::new("localhost", "http://localhost", "development-salt")
    }
}

/// Avalanche chain IDs
pub mod chain_ids {
    /// Avalanche C-Chain Mainnet
    pub const AVALANCHE_MAINNET: u64 = 43114;
    /// Avalanche Fuji Testnet
    pub const AVALANCHE_FUJI: u64 = 43113;
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_settings_builder_chain() {
        let settings = Settings::new("app.com", "https://app.com", "secret")
            .with_chain_id(43113)
            .with_session_expiration(3600_000_000_000)
            .with_allowed_domains(vec!["*.app.com".to_string()]);

        assert_eq!(settings.domain, "app.com");
        assert_eq!(settings.chain_id, 43113);
        assert_eq!(settings.session_expiration_time, 3600_000_000_000);
        assert_eq!(settings.allowed_domains, vec!["*.app.com".to_string()]);
    }

    #[test]
    fn test_chain_ids_constants() {
        assert_eq!(chain_ids::AVALANCHE_MAINNET, 43114);
        assert_eq!(chain_ids::AVALANCHE_FUJI, 43113);
    }
}
