//! Shared utilities for delegation handling
//!
//! This module provides common functions used by both `siwa_prepare_delegation`
//! and `siwa_get_delegation` to ensure consistent behavior.

use crate::state::{get_auth_session, get_settings};
use ic_certified_map::Hash;
use ic_siwa::hash::hash_bytes;
use ic_siwa::siwa::hash_session_key;
use ic_siwa::{create_delegation_hash, generate_seed, DelegationInfo};
use std::collections::HashMap;

/// Key for looking up prepared delegations
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreparedDelegationKey {
    /// Hash of the seed (derived from address + salt)
    pub seed_hash: [u8; 32],
    /// Hash of the session key
    pub session_key_hash: String,
}

/// Value stored for prepared delegations
#[derive(Clone, Debug)]
pub struct PreparedDelegationValue {
    /// The computed final expiration that was used when storing the delegation
    pub final_expiration: u64,
    /// The delegation hash that was stored in the signature map
    pub delegation_hash: [u8; 32],
    /// Optional delegation targets
    pub targets: Option<Vec<candid::Principal>>,
}

thread_local! {
    /// Storage for prepared delegation expirations
    /// This allows get_delegation to look up the exact expiration that was used
    /// in prepare_delegation, avoiding hash mismatches due to time differences.
    static PREPARED_DELEGATIONS: std::cell::RefCell<HashMap<PreparedDelegationKey, PreparedDelegationValue>>
        = std::cell::RefCell::new(HashMap::new());
}

/// Store a prepared delegation for later retrieval
pub fn store_prepared_delegation(key: PreparedDelegationKey, value: PreparedDelegationValue) {
    PREPARED_DELEGATIONS.with(|map| {
        map.borrow_mut().insert(key, value);
    });
}

/// Retrieve a prepared delegation
pub fn get_prepared_delegation(key: &PreparedDelegationKey) -> Option<PreparedDelegationValue> {
    PREPARED_DELEGATIONS.with(|map| map.borrow().get(key).cloned())
}

/// Remove a prepared delegation after use (optional cleanup)
#[allow(dead_code)]
pub fn remove_prepared_delegation(key: &PreparedDelegationKey) {
    PREPARED_DELEGATIONS.with(|map| {
        map.borrow_mut().remove(key);
    });
}

/// Validate a session and return common data needed for delegation operations
///
/// # Arguments
/// * `address` - The Avalanche address
/// * `session_key` - The session public key
///
/// # Returns
/// * `Ok((key_hash, session_expires_at))` - Session key hash and session expiration
/// * `Err(String)` - Error message if validation fails
pub fn validate_session(address: &str, session_key: &[u8]) -> Result<(String, u64), String> {
    // Look up the auth session
    let key_hash = hash_session_key(session_key);
    let auth_session = get_auth_session(&key_hash)
        .ok_or_else(|| format!("No authenticated session found for address {}", address))?;

    // Verify the address matches
    if auth_session.address.to_lowercase() != address.to_lowercase() {
        return Err("Address mismatch".to_string());
    }

    // Verify the session key matches
    if auth_session.session_key != session_key {
        return Err("Session key mismatch".to_string());
    }

    // Check if the auth session has expired
    let now = ic_cdk::api::time();
    if now > auth_session.expires_at {
        return Err("Session has expired".to_string());
    }

    Ok((key_hash, auth_session.expires_at))
}

/// Compute the final expiration for a delegation
///
/// This caps the requested expiration to:
/// 1. The session expiration time
/// 2. The configured maximum session expiration time from now
///
/// # Arguments
/// * `requested_expiration` - The expiration requested by the caller
/// * `session_expires_at` - When the session expires
///
/// # Returns
/// The final capped expiration time
pub fn compute_final_expiration(requested_expiration: u64, session_expires_at: u64) -> u64 {
    let settings = get_settings();
    let now = ic_cdk::api::time();

    // Cap the requested expiration to the session expiration
    let capped_expiration = requested_expiration.min(session_expires_at);

    // Also cap to the configured session expiration time from now
    let max_expiration = now + settings.session_expiration_time;
    capped_expiration.min(max_expiration)
}

/// Get delegation targets from settings
pub fn get_delegation_targets() -> Option<Vec<candid::Principal>> {
    let settings = get_settings();
    if settings.delegation_targets.is_empty() {
        None
    } else {
        Some(settings.delegation_targets.clone())
    }
}

/// Compute the seed hash for an address
pub fn compute_seed_hash(address: &str) -> Hash {
    let settings = get_settings();
    let seed = generate_seed(&settings.salt, address);
    hash_bytes(seed)
}

/// Create a delegation hash from the given parameters
pub fn compute_delegation_hash(
    session_key: &[u8],
    expiration: u64,
    targets: Option<&[candid::Principal]>,
) -> Hash {
    let delegation_info = DelegationInfo {
        pubkey: session_key,
        expiration,
        targets,
    };
    create_delegation_hash(&delegation_info)
}

/// Create the key for looking up prepared delegations
pub fn create_prepared_delegation_key(
    seed_hash: Hash,
    session_key_hash: &str,
) -> PreparedDelegationKey {
    PreparedDelegationKey {
        seed_hash,
        session_key_hash: session_key_hash.to_string(),
    }
}

/// Cleanup expired prepared delegations
/// This should be called periodically to prevent memory growth
pub fn cleanup_expired_prepared_delegations() {
    let now = ic_cdk::api::time();
    PREPARED_DELEGATIONS.with(|map| {
        map.borrow_mut()
            .retain(|_, value| value.final_expiration > now);
    });
}
