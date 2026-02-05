//! SIWA Get Delegation Service
//!
//! Returns a signed delegation for authenticated principals.

use crate::state::{create_certified_delegation_signature, get_auth_session, get_settings};
use candid::{CandidType, Principal};
use ic_siwa::hash::hash_bytes;
use ic_siwa::siwa::hash_session_key;
use ic_siwa::{create_delegation_hash, generate_seed, DelegationInfo};
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
/// * `expiration` - Requested expiration timestamp (may be capped)
///
/// # Returns
/// * `Ok(SignedDelegation)` - The signed delegation
/// * `Err(String)` - Error if not authenticated or expired
pub fn get_delegation(
    address: String,
    session_key: Vec<u8>,
    expiration: u64,
) -> Result<SignedDelegation, String> {
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

    // Create the delegation with optional targets from settings
    // If delegation_targets is configured, the delegation will only work for those canisters
    // This prevents the delegation from being used to call arbitrary canisters
    let targets = if settings.delegation_targets.is_empty() {
        None
    } else {
        Some(settings.delegation_targets.clone())
    };

    let delegation = Delegation {
        pubkey: ByteBuf::from(session_key.clone()),
        expiration: final_expiration,
        targets: targets.clone(),
    };

    // Generate the seed and compute hashes for signature map lookup
    let seed = generate_seed(&settings.salt, &address);
    let seed_hash = hash_bytes(&seed);

    // Create the delegation info for hashing (must match what was stored during prepare_delegation)
    let delegation_info = DelegationInfo {
        pubkey: &session_key,
        expiration: final_expiration,
        targets: targets.as_deref(),
    };
    let delegation_hash = create_delegation_hash(&delegation_info);

    // Create the certified signature (includes certificate + witness tree, CBOR-encoded)
    let signature =
        create_certified_delegation_signature(seed_hash, delegation_hash).ok_or_else(|| {
            "Delegation not found in signature map - please call siwa_prepare_delegation first"
                .to_string()
        })?;

    Ok(SignedDelegation {
        delegation,
        signature: ByteBuf::from(signature),
    })
}
