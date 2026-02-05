//! SIWA Prepare Delegation Service
//!
//! Prepares a delegation by storing it in the signature map for certified responses.

use crate::state::{get_auth_session, get_settings, store_delegation};
use ic_siwa::hash::hash_bytes;
use ic_siwa::siwa::hash_session_key;
use ic_siwa::{create_delegation_hash, generate_seed, DelegationInfo};

/// Prepare a delegation for an authenticated session
///
/// This must be called before `siwa_get_delegation` to store the delegation
/// in the signature map for certified responses.
///
/// # Arguments
/// * `address` - The Avalanche address
/// * `session_key` - The session public key from login
/// * `expiration` - Requested expiration timestamp (may be capped)
///
/// # Returns
/// * `Ok(())` - Delegation prepared successfully
/// * `Err(String)` - Error if not authenticated or expired
pub fn prepare_delegation(
    address: String,
    session_key: Vec<u8>,
    expiration: u64,
) -> Result<(), String> {
    // Get settings
    let settings = get_settings();

    // Look up the auth session
    let key_hash = hash_session_key(&session_key);
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

    // Cap the requested expiration to the session expiration
    let capped_expiration = expiration.min(auth_session.expires_at);

    // Also cap to the configured session expiration time from now
    let max_expiration = now + settings.session_expiration_time;
    let final_expiration = capped_expiration.min(max_expiration);

    // Get targets from settings
    let targets = if settings.delegation_targets.is_empty() {
        None
    } else {
        Some(settings.delegation_targets.clone())
    };

    // Generate the seed and compute hashes
    let seed = generate_seed(&settings.salt, &address);
    let seed_hash = hash_bytes(seed);

    // Create the delegation info for hashing
    let delegation_info = DelegationInfo {
        pubkey: &session_key,
        expiration: final_expiration,
        targets: targets.as_deref(),
    };
    let delegation_hash = create_delegation_hash(&delegation_info);

    // Store the delegation in the signature map
    store_delegation(seed_hash, delegation_hash);

    Ok(())
}
