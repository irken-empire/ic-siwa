//! Shared utilities for delegation handling
//!
//! This module provides common functions used by both `siwa_prepare_delegation`
//! and `siwa_get_delegation` to ensure consistent behavior.

use crate::state::{
    cleanup_expired_prepared_delegations as state_cleanup_expired_prepared_delegations,
    get_auth_session, get_prepared_delegation as state_get_prepared_delegation, get_settings,
    store_prepared_delegation as state_store_prepared_delegation, PreparedDelegation,
};
use ic_certified_map::Hash;
use ic_siwa::hash::hash_bytes;
use ic_siwa::siwa::hash_session_key;
use ic_siwa::{create_delegation_hash, generate_seed, DelegationInfo};

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

/// Store a prepared delegation for later retrieval
///
/// This stores the delegation metadata in the canister's persistent state,
/// making it accessible to query calls.
pub fn store_prepared_delegation(
    seed_hash: &Hash,
    session_key_hash: &str,
    final_expiration: u64,
    delegation_hash: Hash,
    targets: Option<Vec<candid::Principal>>,
) {
    state_store_prepared_delegation(
        seed_hash,
        session_key_hash,
        PreparedDelegation {
            final_expiration,
            delegation_hash,
            targets,
        },
    );
}

/// Retrieve a prepared delegation
///
/// Returns the delegation metadata that was stored by `siwa_prepare_delegation`.
pub fn get_prepared_delegation(
    seed_hash: &Hash,
    session_key_hash: &str,
) -> Option<PreparedDelegation> {
    state_get_prepared_delegation(seed_hash, session_key_hash)
}

/// Cleanup expired prepared delegations
/// This should be called periodically to prevent memory growth
pub fn cleanup_expired_prepared_delegations() {
    state_cleanup_expired_prepared_delegations();
}
