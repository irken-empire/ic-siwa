//! # IC-SIWA - Sign in with Avalanche for the Internet Computer
//!
//! This library provides the core functionality for authenticating users
//! with their Avalanche wallets and issuing ICP principals.
//!
//! ## Overview
//!
//! IC-SIWA adapts the Sign-In with Ethereum (SIWE) pattern for Avalanche C-Chain,
//! enabling cross-chain authentication between Avalanche and the Internet Computer.
//! Since Avalanche C-Chain uses the same ECDSA secp256k1 signatures as Ethereum,
//! existing wallet infrastructure works seamlessly.
//!
//! ## Features
//!
//! - **EIP-4361 Compatible**: Uses the standard Sign-In with Ethereum message format
//! - **ECDSA secp256k1**: Standard Avalanche/Ethereum signature verification
//! - **Principal Derivation**: Deterministic ICP principal derivation from wallet address
//! - **Domain Validation**: Whitelist support with exact and wildcard patterns
//! - **Security Guards**: Canister ID whitelists for inter-canister calls
//! - **YAML Configuration**: Load settings from YAML config files
//!
//! ## Authentication Flow
//!
//! 1. **Prepare Login**: Generate a SIWA message for the user to sign
//! 2. **Login**: Verify the signed message and derive ICP principal
//! 3. **Get Delegation**: Retrieve the delegation chain for the authenticated session
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use ic_siwa::{SiwaMessage, Settings, siwa::{validate_address, derive_principal}};
//!
//! // Create settings
//! let settings = Settings::new("example.com", "https://example.com", "secret-salt")
//!     .with_chain_id(43114); // Avalanche Mainnet
//!
//! // Validate address format
//! validate_address("0x1234...abcd")?;
//!
//! // Generate message for signing
//! let message = SiwaMessage::new(&settings, "0x1234...abcd", "unique-nonce");
//! let message_string = message.to_message();
//!
//! // User signs message_string with their wallet...
//!
//! // Verify signature and recover address
//! let recovered_address = message.verify_signature(&signature)?;
//!
//! // Derive ICP principal from address
//! let principal = derive_principal(&recovered_address, &settings.salt)?;
//! ```
//!
//! ## Security Features
//!
//! ### Domain Whitelisting
//!
//! ```rust,ignore
//! use ic_siwa::types::DomainValidator;
//!
//! let validator = DomainValidator::new(&[
//!     "example.com".to_string(),
//!     "*.example.com".to_string(),
//! ]);
//!
//! assert!(validator.is_allowed("example.com"));
//! assert!(validator.is_allowed("sub.example.com"));
//! assert!(!validator.is_allowed("other.com"));
//! ```
//!
//! ### Canister ID Whitelisting
//!
//! ```rust,ignore
//! use ic_siwa::{CanisterGuard, SecurityMode};
//! use candid::Principal;
//!
//! let guard = CanisterGuard::new(vec![allowed_canister_id])
//!     .with_mode(SecurityMode::Production);
//!
//! guard.validate_caller()?; // Returns error if caller not in whitelist
//! ```
//!
//! ## Chain IDs
//!
//! - Avalanche C-Chain Mainnet: `43114`
//! - Avalanche Fuji Testnet: `43113`
//!
//! ## Modules
//!
//! - [`config`]: YAML configuration loading
//! - [`delegation`]: Delegation creation and verification
//! - [`error`]: Error types
//! - [`hash`]: Cryptographic hash utilities (Keccak256, SHA256)
//! - [`security`]: Access control guards
//! - [`settings`]: Runtime settings
//! - [`siwa`]: SIWA message handling and signature verification
//! - [`types`]: Common types and domain validation

pub mod config;
pub mod delegation;
pub mod error;
pub mod hash;
pub mod rate_limit;
pub mod security;
pub mod settings;
pub mod signature_map;
pub mod siwa;
pub mod time;
pub mod types;

// Re-exports for convenience
pub use config::Config;
pub use delegation::{
    cbor_serialize, create_certified_signature, create_delegation_hash,
    create_user_canister_pubkey, generate_seed, Delegation, DelegationInfo,
};
pub use error::SiwaError;
pub use rate_limit::RateLimiter;
pub use security::{CanisterGuard, ControllerGuard, SecurityMode};
pub use settings::{RateLimitSettings, Settings};
pub use signature_map::SignatureMap;
pub use siwa::SiwaMessage;
pub use types::*;

/// Chain ID constants
pub mod chain_ids {
    /// Avalanche C-Chain Mainnet
    pub const AVALANCHE_MAINNET: u64 = 43114;
    /// Avalanche Fuji Testnet
    pub const AVALANCHE_FUJI: u64 = 43113;
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
