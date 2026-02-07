//! SIWA Get Delegation Service
//!
//! Returns a signed delegation for authenticated principals.

use crate::service::delegation_utils::{
    compute_seed_hash, get_prepared_delegation, validate_session,
};
use crate::state::create_certified_delegation_signature;
use candid::{CandidType, Principal};
use ic_siwa::siwa::hash_session_key;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

/// A delegation that grants a session key the ability to act on behalf of a principal
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Delegation {
    /// The session public key that is being delegated to
    pub pubkey: ByteBuf,
    /// Expiration timestamp in nanoseconds
    pub expiration: u64,
    /// Optional targets (canister IDs) the delegation is valid for
    pub targets: Option<Vec<Principal>>,
}

/// A signed delegation with the signature
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SignedDelegation {
    /// The delegation being signed
    pub delegation: Delegation,
    /// The signature over the delegation hash
    pub signature: ByteBuf,
}

/// The full delegation chain returned to the client
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DelegationChain {
    /// The delegations in the chain
    pub delegations: Vec<SignedDelegation>,
    /// The public key of the signer (canister's derived public key)
    pub user_key: ByteBuf,
}

/// Get a delegation for an authenticated session
///
/// # Arguments
/// * `address` - The Avalanche address to get delegation for
/// * `session_key` - The session public key
/// * `expiration` - Requested expiration timestamp (used for validation, not lookup)
///
/// # Returns
/// * `Ok(SignedDelegation)` - The signed delegation
/// * `Err(String)` - Error if not authenticated or expired
pub fn get_delegation(
    address: String,
    session_key: Vec<u8>,
    _expiration: u64,
) -> Result<SignedDelegation, String> {
    // Validate the session (this also checks expiration)
    let (_key_hash, _session_expires_at) = validate_session(&address, &session_key)?;

    // Compute the seed hash for this address
    let seed_hash = compute_seed_hash(&address);

    // Get the session key hash for looking up the prepared delegation
    let session_key_hash = hash_session_key(&session_key);

    crate::state::debug_log!(
        "[GET_DELEGATION] address: {}, seed_hash: {}, session_key_hash: {}, session_key_len: {}",
        address,
        hex::encode(seed_hash),
        session_key_hash,
        session_key.len()
    );

    // Look up the prepared delegation to get the exact expiration and hash that was stored
    let prepared = get_prepared_delegation(&seed_hash, &session_key_hash).ok_or_else(|| {
        format!(
            "Delegation not found in signature map - please call siwa_prepare_delegation first. \
             address: {}, seed_hash: {}, session_key_hash: {}",
            address,
            hex::encode(seed_hash),
            session_key_hash
        )
    })?;

    // Use the exact values from the prepared delegation
    let final_expiration = prepared.final_expiration;
    let delegation_hash = prepared.delegation_hash;
    let targets = prepared.targets;

    crate::state::debug_log!(
        "[GET_DELEGATION] Found prepared delegation. delegation_hash: {}, final_expiration: {}",
        hex::encode(delegation_hash),
        final_expiration
    );

    // Create the certified signature (includes certificate + witness tree, CBOR-encoded)
    let signature =
        create_certified_delegation_signature(seed_hash, delegation_hash).ok_or_else(|| {
            format!(
                "Failed to create certified signature - delegation may have expired or been pruned. \
                 seed_hash: {}, delegation_hash: {}, expiration: {}",
                hex::encode(seed_hash),
                hex::encode(delegation_hash),
                final_expiration
            )
        })?;

    // Create the delegation with the exact values used during prepare
    let delegation = Delegation {
        pubkey: ByteBuf::from(session_key),
        expiration: final_expiration,
        targets,
    };

    Ok(SignedDelegation {
        delegation,
        signature: ByteBuf::from(signature),
    })
}
