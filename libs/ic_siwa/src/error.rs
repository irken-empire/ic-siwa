//! Error types for IC-SIWA

use thiserror::Error;

/// Main error type for IC-SIWA operations
#[derive(Error, Debug)]
pub enum SiwaError {
    /// Invalid Avalanche address format
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Invalid signature format or verification failed
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Message has expired
    #[error("Message expired")]
    MessageExpired,

    /// Invalid nonce
    #[error("Invalid nonce: {0}")]
    InvalidNonce(String),

    /// Domain not allowed
    #[error("Domain not allowed: {0}")]
    DomainNotAllowed(String),

    /// Canister not allowed
    #[error("Canister not allowed: {0}")]
    CanisterNotAllowed(String),

    /// Session not found
    #[error("Session not found")]
    SessionNotFound,

    /// Delegation error
    #[error("Delegation error: {0}")]
    DelegationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<SiwaError> for String {
    fn from(error: SiwaError) -> Self {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_address() {
        let err = SiwaError::InvalidAddress("missing 0x prefix".to_string());
        assert_eq!(err.to_string(), "Invalid address: missing 0x prefix");
    }

    #[test]
    fn test_error_display_invalid_signature() {
        let err = SiwaError::InvalidSignature("wrong length".to_string());
        assert_eq!(err.to_string(), "Invalid signature: wrong length");
    }

    #[test]
    fn test_error_display_message_expired() {
        let err = SiwaError::MessageExpired;
        assert_eq!(err.to_string(), "Message expired");
    }

    #[test]
    fn test_error_display_invalid_nonce() {
        let err = SiwaError::InvalidNonce("already used".to_string());
        assert_eq!(err.to_string(), "Invalid nonce: already used");
    }

    #[test]
    fn test_error_display_domain_not_allowed() {
        let err = SiwaError::DomainNotAllowed("evil.com".to_string());
        assert_eq!(err.to_string(), "Domain not allowed: evil.com");
    }

    #[test]
    fn test_error_display_canister_not_allowed() {
        let err = SiwaError::CanisterNotAllowed("xyz-canister".to_string());
        assert_eq!(err.to_string(), "Canister not allowed: xyz-canister");
    }

    #[test]
    fn test_error_display_session_not_found() {
        let err = SiwaError::SessionNotFound;
        assert_eq!(err.to_string(), "Session not found");
    }

    #[test]
    fn test_error_display_delegation_error() {
        let err = SiwaError::DelegationError("expired".to_string());
        assert_eq!(err.to_string(), "Delegation error: expired");
    }

    #[test]
    fn test_error_display_config_error() {
        let err = SiwaError::ConfigError("invalid yaml".to_string());
        assert_eq!(err.to_string(), "Configuration error: invalid yaml");
    }

    #[test]
    fn test_error_display_internal_error() {
        let err = SiwaError::InternalError("unexpected state".to_string());
        assert_eq!(err.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn test_error_into_string() {
        let err = SiwaError::InvalidAddress("test".to_string());
        let s: String = err.into();
        assert_eq!(s, "Invalid address: test");
    }
}
