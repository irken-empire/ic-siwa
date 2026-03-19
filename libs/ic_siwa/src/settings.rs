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
    /// Chain ID (Avalanche C-Chain mainnet: 43114, Fuji testnet: 43113, Anvil local: 31337)
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
        if self.chain_id != 31337 && self.chain_id != 43113 && self.chain_id != 43114 {
            return Err(format!(
                "Chain ID must be 31337 (Anvil), 43113 (Fuji) or 43114 (Mainnet), got {}",
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

        // Login expiration: at least 30 seconds, at most 10 minutes
        let thirty_secs_ns = 30 * 1_000_000_000u64;
        let ten_mins_ns = 10 * 60 * 1_000_000_000u64;
        if self.login_expiration_time < thirty_secs_ns {
            return Err("Login expiration must be at least 30 seconds".to_string());
        }
        if self.login_expiration_time > ten_mins_ns {
            return Err("Login expiration must not exceed 10 minutes".to_string());
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

#[cfg(any(test, debug_assertions))]
impl Default for Settings {
    fn default() -> Self {
        // cspell:disable-next-line
        // codeql[rust/hard-coded-cryptographic-value] dev-only default, never used in production
        Self::new("localhost", "http://localhost", "development-salt")
    }
}
