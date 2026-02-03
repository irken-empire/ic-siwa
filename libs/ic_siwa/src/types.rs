//! Common types for IC-SIWA

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Avalanche address (C-Chain, Ethereum compatible)
pub type AvalancheAddress = String;

/// Session key bytes
pub type SessionKey = Vec<u8>;

/// Signature bytes (hex encoded or raw)
pub type Signature = String;

/// Nonce for preventing replay attacks
pub type Nonce = String;

/// Login session information
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    /// Avalanche address
    pub address: AvalancheAddress,
    /// ICP principal derived from address
    pub principal: Principal,
    /// Session key
    pub session_key: SessionKey,
    /// Expiration timestamp (nanoseconds)
    pub expiration: u64,
    /// Creation timestamp (nanoseconds)
    pub created_at: u64,
}

/// Prepared login response
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PreparedLogin {
    /// SIWA message to sign
    pub message: String,
    /// Nonce used in message
    pub nonce: Nonce,
    /// Expiration timestamp for this login attempt
    pub expiration: u64,
}

/// Login response
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct LoginResponse {
    /// ICP principal for the authenticated user
    pub principal: Principal,
    /// Session expiration timestamp
    pub expiration: u64,
}

/// Signed delegation
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SignedDelegation {
    /// Delegation bytes
    pub delegation: Vec<u8>,
    /// Signature bytes
    pub signature: Vec<u8>,
}

/// Domain pattern for whitelist matching
#[derive(Clone, Debug, PartialEq)]
pub enum DomainPattern {
    /// Exact match (e.g., "example.com")
    Exact(String),
    /// Wildcard match (e.g., "*.example.com")
    Wildcard(String),
}

impl DomainPattern {
    /// Parse a domain pattern string
    pub fn parse(pattern: &str) -> Self {
        let pattern_lower = pattern.to_lowercase();
        if let Some(suffix) = pattern_lower.strip_prefix("*.") {
            DomainPattern::Wildcard(suffix.to_string())
        } else {
            DomainPattern::Exact(pattern_lower)
        }
    }

    /// Check if a domain matches this pattern (case-insensitive)
    pub fn matches(&self, domain: &str) -> bool {
        let domain_lower = domain.to_lowercase();
        match self {
            DomainPattern::Exact(pattern) => domain_lower == *pattern,
            DomainPattern::Wildcard(pattern) => {
                domain_lower == *pattern || domain_lower.ends_with(&format!(".{}", pattern))
            }
        }
    }

    /// Get the pattern as a string (for error messages)
    pub fn as_str(&self) -> &str {
        match self {
            DomainPattern::Exact(s) => s,
            DomainPattern::Wildcard(s) => s,
        }
    }
}

/// Domain validator for whitelist checking
#[derive(Clone, Debug)]
pub struct DomainValidator {
    patterns: Vec<DomainPattern>,
}

impl DomainValidator {
    /// Create a new validator from a list of pattern strings
    pub fn new(patterns: &[String]) -> Self {
        Self {
            patterns: patterns.iter().map(|p| DomainPattern::parse(p)).collect(),
        }
    }

    /// Check if a domain is allowed
    pub fn is_allowed(&self, domain: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(domain))
    }

    /// Extract domain from an Origin header value
    /// e.g., "https://example.com:8080" -> "example.com"
    pub fn extract_domain_from_origin(origin: &str) -> Option<String> {
        // Remove protocol prefix
        let without_protocol = origin
            .strip_prefix("https://")
            .or_else(|| origin.strip_prefix("http://"))
            .unwrap_or(origin);

        // Remove port and path
        let domain = without_protocol.split(':').next()?.split('/').next()?;

        if domain.is_empty() {
            None
        } else {
            Some(domain.to_lowercase())
        }
    }

    /// Validate an Origin header against the whitelist
    pub fn validate_origin(&self, origin: &str) -> Result<String, String> {
        let domain = Self::extract_domain_from_origin(origin)
            .ok_or_else(|| format!("Invalid origin format: {}", origin))?;

        if self.is_allowed(&domain) {
            Ok(domain)
        } else {
            Err(format!("Domain '{}' is not in the allowed list", domain))
        }
    }

    /// Check if the validator has any patterns
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_pattern_exact() {
        let pattern = DomainPattern::parse("example.com");
        assert!(pattern.matches("example.com"));
        assert!(!pattern.matches("sub.example.com"));
        assert!(!pattern.matches("other.com"));
    }

    #[test]
    fn test_domain_pattern_exact_case_insensitive() {
        let pattern = DomainPattern::parse("Example.COM");
        assert!(pattern.matches("example.com"));
        assert!(pattern.matches("EXAMPLE.COM"));
        assert!(pattern.matches("Example.Com"));
    }

    #[test]
    fn test_domain_pattern_wildcard() {
        let pattern = DomainPattern::parse("*.example.com");
        assert!(pattern.matches("example.com"));
        assert!(pattern.matches("sub.example.com"));
        assert!(pattern.matches("a.b.example.com"));
        assert!(!pattern.matches("other.com"));
    }

    #[test]
    fn test_domain_pattern_wildcard_case_insensitive() {
        let pattern = DomainPattern::parse("*.Example.COM");
        assert!(pattern.matches("SUB.example.com"));
        assert!(pattern.matches("sub.EXAMPLE.COM"));
    }

    #[test]
    fn test_domain_validator_multiple_patterns() {
        let validator = DomainValidator::new(&[
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            "*.example.com".to_string(),
        ]);

        assert!(validator.is_allowed("localhost"));
        assert!(validator.is_allowed("127.0.0.1"));
        assert!(validator.is_allowed("example.com"));
        assert!(validator.is_allowed("sub.example.com"));
        assert!(!validator.is_allowed("other.com"));
    }

    #[test]
    fn test_extract_domain_from_origin() {
        assert_eq!(
            DomainValidator::extract_domain_from_origin("https://example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            DomainValidator::extract_domain_from_origin("http://example.com:8080"),
            Some("example.com".to_string())
        );
        assert_eq!(
            DomainValidator::extract_domain_from_origin("https://sub.example.com/path"),
            Some("sub.example.com".to_string())
        );
        assert_eq!(
            DomainValidator::extract_domain_from_origin("example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            DomainValidator::extract_domain_from_origin("https://EXAMPLE.COM"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_validate_origin_success() {
        let validator = DomainValidator::new(&["*.example.com".to_string()]);

        let result = validator.validate_origin("https://sub.example.com");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sub.example.com");
    }

    #[test]
    fn test_validate_origin_rejected() {
        let validator = DomainValidator::new(&["example.com".to_string()]);

        let result = validator.validate_origin("https://other.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in the allowed list"));
    }

    #[test]
    fn test_validate_origin_invalid_format() {
        let validator = DomainValidator::new(&["example.com".to_string()]);

        let result = validator.validate_origin("https://");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid origin format"));
    }
}
