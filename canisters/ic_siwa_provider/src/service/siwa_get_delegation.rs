//! SIWA Get Delegation Service
//!
//! Returns a signed delegation for authenticated principals.

use crate::state::{get_auth_session, get_settings};
use candid::{CandidType, Principal};
use ic_siwa::hash::sha256;
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

    // Create the delegation
    let delegation = Delegation {
        pubkey: ByteBuf::from(session_key.clone()),
        expiration: final_expiration,
        targets: None, // No target restrictions for now
    };

    // Create a hash of the delegation for signing
    // The hash format follows the IC delegation hash standard
    let delegation_hash = create_delegation_hash(&delegation)?;

    // Sign the delegation
    // Note: In production, this would use ic_cdk::api::management_canister::main::sign_with_ecdsa
    // For now, we create a placeholder signature that will be replaced with threshold signing
    let signature = sign_delegation(&delegation_hash, &settings.salt)?;

    Ok(SignedDelegation {
        delegation,
        signature: ByteBuf::from(signature),
    })
}

/// Create a hash of a delegation following IC conventions
fn create_delegation_hash(delegation: &Delegation) -> Result<[u8; 32], String> {
    // Serialize the delegation in a canonical format for hashing
    // IC uses a specific domain separator for delegations
    let domain_separator = b"\x1Aic-request-auth-delegation";

    let mut data = domain_separator.to_vec();

    // Add pubkey
    data.extend_from_slice(&delegation.pubkey);

    // Add expiration as big-endian u64
    data.extend_from_slice(&delegation.expiration.to_be_bytes());

    // Add targets if present
    if let Some(ref targets) = delegation.targets {
        for target in targets {
            data.extend_from_slice(target.as_slice());
        }
    }

    Ok(sha256(&data))
}

/// Sign a delegation hash
/// Note: This is a placeholder. Production implementation would use
/// ic_cdk::api::management_canister::main::sign_with_ecdsa
fn sign_delegation(delegation_hash: &[u8; 32], salt: &str) -> Result<Vec<u8>, String> {
    // For now, create a deterministic signature based on the hash and salt
    // This will be replaced with proper threshold ECDSA signing
    let mut sign_data = delegation_hash.to_vec();
    sign_data.extend_from_slice(salt.as_bytes());
    let signature_hash = sha256(&sign_data);

    // Return a placeholder signature (64 bytes for ECDSA)
    // The actual implementation will call the management canister's sign_with_ecdsa
    let mut signature = signature_hash.to_vec();
    signature.extend_from_slice(&signature_hash);

    Ok(signature)
}
