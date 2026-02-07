//! Configuration loading from YAML files

use crate::error::SiwaError;
use crate::settings::{RateLimitSettings, Settings};
use crate::types::DomainPattern;
use candid::Principal;
use serde::{Deserialize, Serialize};

/// Configuration file structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Avalanche chain configuration
    pub avalanche: AvalancheConfig,
    /// Internet Computer configuration
    pub ic: IcConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Library configuration
    #[serde(default)]
    pub library: LibraryConfig,
    /// Debug mode - enables diagnostic endpoints
    #[serde(default)]
    pub debug: bool,
}

/// Avalanche chain configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AvalancheConfig {
    /// Chain ID (43114 for mainnet, 43113 for Fuji)
    pub chain_id: u64,
    /// RPC endpoint URL (for reference, not used by canisters)
    #[serde(default)]
    pub rpc_url: Option<String>,
}

/// Internet Computer configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IcConfig {
    /// Network (local, ic)
    pub network: String,
    /// Canister IDs
    #[serde(default)]
    pub canisters: CanisterIds,
}

/// Canister ID configuration
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CanisterIds {
    /// IC-SIWA Provider canister ID
    pub ic_siwa_provider: Option<String>,
}

/// Security configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecurityConfig {
    /// Allowed domains for SIWA messages
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Allowed canister IDs for inter-canister calls
    #[serde(default)]
    pub allowed_canisters: Vec<String>,
    /// Delegation targets - canisters that delegations are valid for
    #[serde(default)]
    pub delegation_targets: Vec<String>,
    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limits: RateLimitConfig,
    /// Session expiration in seconds
    #[serde(default = "default_session_expiration")]
    pub session_expiration_seconds: u64,
    /// Login message expiration in seconds
    #[serde(default = "default_login_expiration")]
    pub login_expiration_seconds: u64,
}

/// Rate limiting configuration from YAML
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RateLimitConfig {
    /// Max prepare_login calls per address per window
    #[serde(default = "default_max_logins_per_address")]
    pub max_logins_per_address: u32,
    /// Max total prepare_login calls per window
    #[serde(default = "default_max_logins_total")]
    pub max_logins_total: u32,
    /// Time window in seconds
    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_logins_per_address: default_max_logins_per_address(),
            max_logins_total: default_max_logins_total(),
            window_seconds: default_window_seconds(),
        }
    }
}

fn default_session_expiration() -> u64 {
    1800 // 30 minutes
}

fn default_login_expiration() -> u64 {
    300 // 5 minutes
}

fn default_max_logins_per_address() -> u32 {
    10
}

fn default_max_logins_total() -> u32 {
    1000
}

fn default_window_seconds() -> u64 {
    3600 // 1 hour
}

/// Library-specific configuration
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LibraryConfig {
    /// Request timeout in milliseconds
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl Config {
    /// Load configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, SiwaError> {
        serde_yaml::from_str(yaml).map_err(|e| SiwaError::ConfigError(e.to_string()))
    }

    /// Get domain patterns for validation
    pub fn domain_patterns(&self) -> Vec<DomainPattern> {
        self.security
            .allowed_domains
            .iter()
            .map(|d| DomainPattern::parse(d))
            .collect()
    }

    /// Get allowed canister principals
    pub fn allowed_canister_principals(&self) -> Result<Vec<Principal>, SiwaError> {
        self.security
            .allowed_canisters
            .iter()
            .map(|id| Principal::from_text(id).map_err(|e| SiwaError::ConfigError(e.to_string())))
            .collect()
    }

    /// Get delegation target principals
    pub fn delegation_target_principals(&self) -> Result<Vec<Principal>, SiwaError> {
        self.security
            .delegation_targets
            .iter()
            .map(|id| Principal::from_text(id).map_err(|e| SiwaError::ConfigError(e.to_string())))
            .collect()
    }

    /// Convert to Settings
    pub fn to_settings(&self, domain: &str, uri: &str, salt: &str) -> Result<Settings, SiwaError> {
        Ok(Settings {
            domain: domain.to_string(),
            uri: uri.to_string(),
            salt: salt.to_string(),
            chain_id: self.avalanche.chain_id,
            session_expiration_time: self.security.session_expiration_seconds * 1_000_000_000,
            login_expiration_time: self.security.login_expiration_seconds * 1_000_000_000,
            allowed_domains: self.security.allowed_domains.clone(),
            allowed_canisters: self.allowed_canister_principals()?,
            delegation_targets: self.delegation_target_principals()?,
            rate_limits: RateLimitSettings {
                max_logins_per_address: self.security.rate_limits.max_logins_per_address,
                max_logins_total: self.security.rate_limits.max_logins_total,
                window_seconds: self.security.rate_limits.window_seconds,
            },
            debug: self.debug,
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), SiwaError> {
        // Validate chain_id
        if self.avalanche.chain_id != 43113 && self.avalanche.chain_id != 43114 {
            return Err(SiwaError::ConfigError(
                "chain_id must be 43113 (Fuji) or 43114 (Mainnet)".into(),
            ));
        }

        // Validate session expiration (1 min to 30 days, consistent with Settings::validate)
        if self.security.session_expiration_seconds < 60
            || self.security.session_expiration_seconds > 30 * 24 * 60 * 60
        {
            return Err(SiwaError::ConfigError(
                "session_expiration_seconds must be between 60 and 2592000 (30 days)".into(),
            ));
        }

        // Validate login expiration (30 sec to 10 min)
        if self.security.login_expiration_seconds < 30
            || self.security.login_expiration_seconds > 600
        {
            return Err(SiwaError::ConfigError(
                "login_expiration_seconds must be between 30 and 600".into(),
            ));
        }

        Ok(())
    }

    /// Check if a domain is allowed
    pub fn is_domain_allowed(&self, domain: &str) -> bool {
        let domain_lower = domain.to_lowercase();
        self.domain_patterns()
            .iter()
            .any(|p| p.matches(&domain_lower))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG: &str = r#"
avalanche:
  chain_id: 43113
  rpc_url: "https://api.avax-test.network/ext/bc/C/rpc"
ic:
  network: "local"
  canisters:
    ic_siwa_provider: null
security:
  allowed_domains:
    - "localhost"
    - "*.example.com"
  allowed_canisters: []
  delegation_targets: []
  rate_limits:
    max_logins_per_address: 10
    max_logins_total: 1000
    window_seconds: 3600
  session_expiration_seconds: 1800
  login_expiration_seconds: 300
library:
  timeout_ms: 30000
"#;

    #[test]
    fn test_config_from_yaml() {
        let config = Config::from_yaml(TEST_CONFIG).expect("should parse valid YAML");
        assert_eq!(config.avalanche.chain_id, 43113);
        assert_eq!(config.ic.network, "local");
        assert_eq!(config.security.session_expiration_seconds, 1800);
        assert_eq!(config.security.allowed_domains.len(), 2);
    }

    #[test]
    fn test_config_domain_matching() {
        let config = Config::from_yaml(TEST_CONFIG).unwrap();

        assert!(config.is_domain_allowed("localhost"));
        assert!(config.is_domain_allowed("sub.example.com"));
        assert!(config.is_domain_allowed("example.com"));
        assert!(!config.is_domain_allowed("other.com"));
    }

    #[test]
    fn test_config_validation() {
        let config = Config::from_yaml(TEST_CONFIG).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_to_settings() {
        let config = Config::from_yaml(TEST_CONFIG).unwrap();
        let settings = config
            .to_settings("localhost", "http://localhost:5173", "test-salt")
            .expect("should create settings");

        assert_eq!(settings.domain, "localhost");
        assert_eq!(settings.chain_id, 43113);
        assert_eq!(settings.session_expiration_time, 1800 * 1_000_000_000);
        assert!(settings.delegation_targets.is_empty());
        assert_eq!(settings.rate_limits.max_logins_per_address, 10);
    }

    #[test]
    fn test_config_rate_limits_defaults() {
        let minimal_config = r#"
avalanche:
  chain_id: 43113
ic:
  network: "local"
security:
  allowed_domains: []
"#;
        let config = Config::from_yaml(minimal_config).expect("should parse minimal config");
        assert_eq!(config.security.rate_limits.max_logins_per_address, 10);
        assert_eq!(config.security.rate_limits.max_logins_total, 1000);
        assert_eq!(config.security.rate_limits.window_seconds, 3600);
    }
}
