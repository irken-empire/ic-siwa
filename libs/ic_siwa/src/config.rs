//! Configuration loading from YAML files

use crate::error::SiwaError;
use crate::settings::Settings;
use crate::types::DomainPattern;
use candid::Principal;
use serde::{Deserialize, Serialize};

/// Configuration file structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Domain configuration
    pub domains: DomainsConfig,
    /// Avalanche chain configuration
    pub avalanche: AvalancheConfig,
    /// Internet Computer configuration
    pub ic: IcConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Library configuration
    #[serde(default)]
    pub library: LibraryConfig,
}

/// Domain whitelist configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DomainsConfig {
    /// List of allowed domain patterns
    pub allowed: Vec<String>,
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
    /// Allowed canister IDs for inter-canister calls
    #[serde(default)]
    pub allowed_canisters: Vec<String>,
    /// Session expiration in seconds
    #[serde(default = "default_session_expiration")]
    pub session_expiration_seconds: u64,
    /// Login message expiration in seconds
    #[serde(default = "default_login_expiration")]
    pub login_expiration_seconds: u64,
}

fn default_session_expiration() -> u64 {
    1800 // 30 minutes
}

fn default_login_expiration() -> u64 {
    300 // 5 minutes
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
        self.domains
            .allowed
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

    /// Convert to Settings
    pub fn to_settings(&self, domain: &str, uri: &str, salt: &str) -> Result<Settings, SiwaError> {
        Ok(Settings {
            domain: domain.to_string(),
            uri: uri.to_string(),
            salt: salt.to_string(),
            chain_id: self.avalanche.chain_id,
            session_expiration_time: self.security.session_expiration_seconds * 1_000_000_000,
            login_expiration_time: self.security.login_expiration_seconds * 1_000_000_000,
            allowed_domains: self.domains.allowed.clone(),
            allowed_canisters: self.allowed_canister_principals()?,
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), SiwaError> {
        // Validate chain_id
        if self.avalanche.chain_id == 0 {
            return Err(SiwaError::ConfigError("chain_id must be non-zero".into()));
        }

        // Validate session expiration (1 min to 7 days)
        if self.security.session_expiration_seconds < 60
            || self.security.session_expiration_seconds > 604800
        {
            return Err(SiwaError::ConfigError(
                "session_expiration_seconds must be between 60 and 604800".into(),
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
domains:
  allowed:
    - "localhost"
    - "*.example.com"
avalanche:
  chain_id: 43113
  rpc_url: "https://api.avax-test.network/ext/bc/C/rpc"
ic:
  network: "local"
  canisters:
    ic_siwa_provider: null
security:
  allowed_canisters: []
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
        assert_eq!(config.domains.allowed.len(), 2);
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
    }
}
