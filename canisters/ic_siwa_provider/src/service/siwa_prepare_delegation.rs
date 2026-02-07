//! SIWA Prepare Delegation Service
//!
//! Prepares a delegation by storing it in the signature map for certified responses.

use crate::service::delegation_utils::{
    cleanup_expired_prepared_delegations, compute_delegation_hash, compute_final_expiration,
    compute_seed_hash, get_delegation_targets, store_prepared_delegation, validate_session,
};
use crate::state::store_delegation;
use ic_siwa::siwa::hash_session_key;

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
    // Periodically cleanup expired prepared delegations
    cleanup_expired_prepared_delegations();

    // Validate the session and get session data
    let (_key_hash, session_expires_at) = validate_session(&address, &session_key)?;

    // Compute the final expiration (capped to session and settings limits)
    let final_expiration = compute_final_expiration(expiration, session_expires_at);

    // Get targets from settings
    let targets = get_delegation_targets();

    // Compute the seed hash and delegation hash
    let seed_hash = compute_seed_hash(&address);
    let delegation_hash =
        compute_delegation_hash(&session_key, final_expiration, targets.as_deref());

    crate::state::debug_log!(
        "[PREPARE_DELEGATION] address: {}, seed_hash: {}, delegation_hash: {}, final_expiration: {}, session_key_len: {}",
        address,
        hex::encode(seed_hash),
        hex::encode(delegation_hash),
        final_expiration,
        session_key.len()
    );

    // Store the delegation in the signature map
    store_delegation(seed_hash, delegation_hash);

    // Store the prepared delegation info so get_delegation can retrieve the exact
    // expiration value, avoiding hash mismatches due to time differences
    let session_key_hash = hash_session_key(&session_key);
    store_prepared_delegation(
        &seed_hash,
        &session_key_hash,
        final_expiration,
        delegation_hash,
        targets,
    )?;

    crate::state::debug_log!(
        "[PREPARE_DELEGATION] Stored delegation. session_key_hash: {}",
        session_key_hash
    );

    Ok(())
}
