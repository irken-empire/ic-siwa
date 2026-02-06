//! Canister state management for IC-SIWA Provider
//!
//! Stores settings, active login sessions, address-principal mappings, rate limiter,
//! and the signature map for certified delegations.

use candid::{CandidType, Principal};
use ic_certified_map::{labeled_hash, Hash};
use ic_siwa::{RateLimiter, Settings, SignatureMap};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

/// Label for the signature tree in certified data
pub const LABEL_SIG: &[u8] = b"sig";

/// Active login session (pending signature verification)
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct LoginSession {
    /// The Avalanche address attempting to log in
    pub address: String,
    /// The nonce for this session
    pub nonce: String,
    /// The full SIWA message to be signed
    pub message: String,
    /// Timestamp when this session was created (nanoseconds)
    pub created_at: u64,
    /// Timestamp when this session expires (nanoseconds)
    pub expires_at: u64,
}

/// Completed authentication session
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AuthSession {
    /// The Avalanche address
    pub address: String,
    /// The derived ICP principal
    pub principal: Principal,
    /// Session key for delegation
    pub session_key: Vec<u8>,
    /// Timestamp when this session was created (nanoseconds)
    pub created_at: u64,
    /// Timestamp when this session expires (nanoseconds)
    pub expires_at: u64,
}

/// Prepared delegation metadata stored for later retrieval
#[derive(Clone, Debug)]
pub struct PreparedDelegation {
    /// The computed final expiration that was used when storing the delegation
    pub final_expiration: u64,
    /// The delegation hash that was stored in the signature map
    pub delegation_hash: [u8; 32],
    /// Optional delegation targets
    pub targets: Option<Vec<Principal>>,
}

/// Key for looking up prepared delegations (seed_hash + session_key_hash)
pub type PreparedDelegationKey = String;

/// Global canister state
#[derive(Default)]
pub struct State {
    /// SIWA settings
    pub settings: Option<Settings>,
    /// Active login sessions keyed by address
    pub login_sessions: HashMap<String, LoginSession>,
    /// Authenticated sessions keyed by session key hash
    pub auth_sessions: HashMap<String, AuthSession>,
    /// Address to principal mapping
    pub address_to_principal: HashMap<String, Principal>,
    /// Principal to address mapping
    pub principal_to_address: HashMap<Principal, String>,
    /// Rate limiter for login attempts
    pub rate_limiter: RateLimiter,
    /// Signature map for certified delegations
    pub signature_map: SignatureMap,
    /// Prepared delegations for certified retrieval
    /// Key: "{seed_hash_hex}:{session_key_hash}"
    pub prepared_delegations: HashMap<PreparedDelegationKey, PreparedDelegation>,
}

thread_local! {
    /// Thread-local state storage
    static STATE: RefCell<State> = RefCell::new(State::default());
}

impl State {
    /// Clean up expired login sessions
    pub fn cleanup_expired_logins(&mut self) {
        let now = ic_cdk::api::time();
        self.login_sessions
            .retain(|_, session| session.expires_at > now);
    }

    /// Clean up expired auth sessions
    pub fn cleanup_expired_auth(&mut self) {
        let now = ic_cdk::api::time();
        self.auth_sessions
            .retain(|_, session| session.expires_at > now);
    }
}

/// Read state immutably
pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    STATE.with(|state| f(&state.borrow()))
}

/// Mutate state
pub fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    STATE.with(|state| f(&mut state.borrow_mut()))
}

/// Initialize state with settings
pub fn init_state(settings: Settings) {
    with_state_mut(|state| {
        // Initialize rate limiter with settings
        state.rate_limiter = RateLimiter::new(settings.rate_limits.clone());
        state.settings = Some(settings);
        // Clear all session and mapping data on init/upgrade
        state.login_sessions.clear();
        state.auth_sessions.clear();
        state.address_to_principal.clear();
        state.principal_to_address.clear();
        // Reset signature map
        state.signature_map = SignatureMap::default();
    });
    // Update certified data with empty signature map
    update_certified_data();
}

/// Update the canister's certified data with the current signature map root hash
///
/// This must be called after any modification to the signature map.
pub fn update_certified_data() {
    with_state(|state| {
        let root_hash = compute_root_hash(&state.signature_map);
        ic_cdk::api::certified_data_set(&root_hash);
    });
}

/// Compute the root hash for certified data
fn compute_root_hash(signature_map: &SignatureMap) -> Hash {
    // Create a labeled hash for the signature tree
    labeled_hash(LABEL_SIG, &signature_map.root_hash())
}

/// Get current settings (panics if not initialized)
pub fn get_settings() -> Settings {
    with_state(|state| {
        state
            .settings
            .clone()
            .expect("Canister not initialized - settings not set")
    })
}

/// Store a delegation hash in the signature map
///
/// This adds the delegation to the certified data so it can be verified
/// by the IC when used in queries.
///
/// # Arguments
/// * `seed_hash` - Hash of the seed (derived from address + salt)
/// * `delegation_hash` - Hash of the delegation
pub fn store_delegation(seed_hash: Hash, delegation_hash: Hash) {
    let now = ic_cdk::api::time();
    with_state_mut(|state| {
        state.signature_map.put(seed_hash, delegation_hash, now);
        ic_cdk::println!(
            "[STORE_DELEGATION] Stored in signature map. seed_hash: {}, delegation_hash: {}, now: {}, map_len: {}",
            hex::encode(seed_hash),
            hex::encode(delegation_hash),
            now,
            state.signature_map.len()
        );
    });
    update_certified_data();
    ic_cdk::println!("[STORE_DELEGATION] Certified data updated");
}

/// Create a certified signature for a delegation
///
/// This creates the full CBOR-encoded signature with certificate and witness tree,
/// suitable for use as an IC canister signature.
///
/// # Arguments
/// * `seed_hash` - Hash of the seed
/// * `delegation_hash` - Hash of the delegation
///
/// # Returns
/// The CBOR-encoded certified signature bytes, or None if delegation not found
pub fn create_certified_delegation_signature(
    seed_hash: Hash,
    delegation_hash: Hash,
) -> Option<Vec<u8>> {
    ic_cdk::println!(
        "[CERTIFIED_SIG] Called with seed_hash: {}, delegation_hash: {}",
        hex::encode(seed_hash),
        hex::encode(delegation_hash)
    );

    // Get the data certificate from the IC (only available in query calls)
    let certificate = ic_cdk::api::data_certificate();
    if certificate.is_none() {
        ic_cdk::println!("[CERTIFIED_SIG] No data certificate available - not in a query call?");
        return None;
    }
    let certificate = certificate.unwrap();
    ic_cdk::println!(
        "[CERTIFIED_SIG] Got data certificate, len: {}",
        certificate.len()
    );

    with_state(|state| {
        // Check if expired first
        let now = ic_cdk::api::time();
        let map_len = state.signature_map.len();
        ic_cdk::println!(
            "[CERTIFIED_SIG] Checking expiration. now: {}, signature_map_len: {}",
            now,
            map_len
        );

        if state
            .signature_map
            .is_expired(now, seed_hash, delegation_hash)
        {
            ic_cdk::println!(
                "[CERTIFIED_SIG] Delegation expired or not found. seed_hash: {}, delegation_hash: {}, now: {}",
                hex::encode(seed_hash),
                hex::encode(delegation_hash),
                now
            );
            return None;
        }

        ic_cdk::println!("[CERTIFIED_SIG] Delegation is valid, getting witness");

        // Get the witness from the signature map
        let witness = state.signature_map.witness(seed_hash, delegation_hash);
        if witness.is_none() {
            ic_cdk::println!(
                "[CERTIFIED_SIG] Failed to get witness. seed_hash: {}, delegation_hash: {}",
                hex::encode(seed_hash),
                hex::encode(delegation_hash)
            );
            return None;
        }
        let witness = witness.unwrap();

        ic_cdk::println!("[CERTIFIED_SIG] Got witness, creating labeled tree");

        // Create the labeled tree with the signature label
        let tree = ic_certified_map::labeled(LABEL_SIG, witness);

        // Create the certified signature (CBOR-encoded with self-describing tag)
        match ic_siwa::create_certified_signature(certificate.clone(), tree) {
            Ok(sig) => {
                ic_cdk::println!(
                    "[CERTIFIED_SIG] Successfully created certified signature, len: {}",
                    sig.len()
                );
                Some(sig)
            }
            Err(e) => {
                ic_cdk::println!(
                    "[CERTIFIED_SIG] Failed to create certified signature: {:?}",
                    e
                );
                None
            }
        }
    })
}

/// Get settings reference if initialized
pub fn try_get_settings() -> Option<Settings> {
    with_state(|state| state.settings.clone())
}

/// Store a new login session
pub fn store_login_session(session: LoginSession) {
    with_state_mut(|state| {
        // Clean up expired sessions first
        state.cleanup_expired_logins();
        // Store the new session
        state
            .login_sessions
            .insert(session.address.to_lowercase(), session);
    });
}

/// Get a login session by address
pub fn get_login_session(address: &str) -> Option<LoginSession> {
    with_state_mut(|state| {
        state.cleanup_expired_logins();
        state.login_sessions.get(&address.to_lowercase()).cloned()
    })
}

/// Remove a login session (after successful login)
pub fn remove_login_session(address: &str) {
    with_state_mut(|state| {
        state.login_sessions.remove(&address.to_lowercase());
    });
}

/// Store an authenticated session
pub fn store_auth_session(key_hash: String, session: AuthSession) {
    with_state_mut(|state| {
        state.cleanup_expired_auth();
        state.auth_sessions.insert(key_hash, session.clone());
        // Also store the address-principal mappings
        state
            .address_to_principal
            .insert(session.address.to_lowercase(), session.principal);
        state
            .principal_to_address
            .insert(session.principal, session.address);
    });
}

/// Get an auth session by session key hash
pub fn get_auth_session(key_hash: &str) -> Option<AuthSession> {
    with_state_mut(|state| {
        state.cleanup_expired_auth();
        state.auth_sessions.get(key_hash).cloned()
    })
}

/// Get principal for address
pub fn get_principal_for_address(address: &str) -> Option<Principal> {
    with_state(|state| {
        state
            .address_to_principal
            .get(&address.to_lowercase())
            .copied()
    })
}

/// Get address for principal
pub fn get_address_for_principal(principal: &Principal) -> Option<String> {
    with_state(|state| state.principal_to_address.get(principal).cloned())
}

/// Counter for periodic cleanup
static CLEANUP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const CLEANUP_INTERVAL: u64 = 100; // Cleanup every 100 requests

/// Check rate limit for an address and record the attempt if allowed
pub fn check_rate_limit(address: &str) -> Result<(), String> {
    let now_ns = ic_cdk::api::time();

    // Periodically cleanup expired entries to prevent memory growth
    let count = CLEANUP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if count.is_multiple_of(CLEANUP_INTERVAL) {
        cleanup_rate_limits();
    }

    with_state_mut(|state| {
        match state.rate_limiter.check_and_record(address, now_ns) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Log rate limit hits for monitoring
                ic_cdk::println!(
                    "[RATE_LIMIT] Blocked address={} remaining_global={} error={}",
                    address,
                    state.rate_limiter.remaining_global(now_ns),
                    e
                );
                Err(e.to_string())
            }
        }
    })
}

/// Cleanup expired rate limit entries (call periodically)
pub fn cleanup_rate_limits() {
    let now_ns = ic_cdk::api::time();
    with_state_mut(|state| {
        state.rate_limiter.cleanup_expired(now_ns);
    });
}

/// Store a prepared delegation for later retrieval
///
/// # Arguments
/// * `seed_hash` - Hash of the seed (hex-encoded for the key)
/// * `session_key_hash` - Hash of the session key
/// * `delegation` - The prepared delegation metadata
pub fn store_prepared_delegation(
    seed_hash: &[u8; 32],
    session_key_hash: &str,
    delegation: PreparedDelegation,
) {
    let key = format!("{}:{}", hex::encode(seed_hash), session_key_hash);
    with_state_mut(|state| {
        state.prepared_delegations.insert(key, delegation);
    });
}

/// Get a prepared delegation by seed hash and session key hash
///
/// # Arguments
/// * `seed_hash` - Hash of the seed
/// * `session_key_hash` - Hash of the session key
///
/// # Returns
/// The prepared delegation if found
pub fn get_prepared_delegation(
    seed_hash: &[u8; 32],
    session_key_hash: &str,
) -> Option<PreparedDelegation> {
    let key = format!("{}:{}", hex::encode(seed_hash), session_key_hash);
    with_state(|state| state.prepared_delegations.get(&key).cloned())
}

/// Cleanup expired prepared delegations
pub fn cleanup_expired_prepared_delegations() {
    let now = ic_cdk::api::time();
    with_state_mut(|state| {
        state
            .prepared_delegations
            .retain(|_, v| v.final_expiration > now);
    });
}
