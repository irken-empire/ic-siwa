//! Canister state management for IC-SIWA Provider
//!
//! Uses stable memory for persistent data (identity mappings, settings) and
//! in-memory storage for transient data (sessions, rate limiter, signature map).
//!
//! Persistent data survives canister upgrades. Transient data is lost on upgrade,
//! which is acceptable since users simply need to re-authenticate.

use candid::{CandidType, Principal};
use ic_certified_map::{labeled_hash, Hash};
use ic_siwa::{RateLimiter, Settings, SignatureMap};
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    BTreeMap as StableBTreeMap, Cell as StableCell, DefaultMemoryImpl, Storable,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap};

// --- Stable memory layout ---

/// Memory ID for the address -> principal mapping
const MEMORY_ID_ADDRESS_TO_PRINCIPAL: MemoryId = MemoryId::new(0);
/// Memory ID for the principal -> address mapping
const MEMORY_ID_PRINCIPAL_TO_ADDRESS: MemoryId = MemoryId::new(1);
/// Memory ID for the settings cell
const MEMORY_ID_SETTINGS: MemoryId = MemoryId::new(2);

/// Virtual memory type alias
type Memory = VirtualMemory<DefaultMemoryImpl>;

// --- Storable wrapper for Principal ---

/// Wrapper for `candid::Principal` to implement `Storable` for stable structures.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StorablePrincipal(Principal);

impl Storable for StorablePrincipal {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.0.as_slice().to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.as_slice().to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(Principal::from_slice(&bytes))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 29,
        is_fixed_size: false,
    };
}

// --- Thread-local stable structures ---

thread_local! {
    /// Memory manager for stable memory regions
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    /// Persistent: address (lowercase) -> principal mapping
    static ADDRESS_TO_PRINCIPAL: RefCell<StableBTreeMap<String, StorablePrincipal, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MEMORY_ID_ADDRESS_TO_PRINCIPAL))
        ));

    /// Persistent: principal -> address mapping
    static PRINCIPAL_TO_ADDRESS: RefCell<StableBTreeMap<StorablePrincipal, String, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MEMORY_ID_PRINCIPAL_TO_ADDRESS))
        ));

    /// Persistent: settings stored as Candid-encoded bytes
    static SETTINGS_CELL: RefCell<StableCell<Vec<u8>, Memory>> =
        RefCell::new(StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MEMORY_ID_SETTINGS)),
            vec![]
        ));

    /// Transient state (lost on upgrade -- sessions, rate limiter, signature map)
    static STATE: RefCell<TransientState> = RefCell::new(TransientState::default());
}

/// Label for the signature tree in certified data
pub const LABEL_SIG: &[u8] = b"sig";

/// Check if debug logging is enabled in settings.
/// Returns false if settings are not yet loaded (during init).
pub fn is_debug_enabled() -> bool {
    with_state(|state| state.settings_cache.as_ref().is_some_and(|s| s.debug))
}

/// Debug logging macro that only emits output when `settings.debug` is true.
/// Avoids string formatting costs when debug is disabled.
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if $crate::state::is_debug_enabled() {
            ic_cdk::println!($($arg)*);
        }
    };
}
pub(crate) use debug_log;

// --- Capacity limits for transient state HashMaps ---

/// Maximum number of pending login sessions (one per address attempting to log in)
const MAX_LOGIN_SESSIONS: usize = 10_000;
/// Maximum number of authenticated sessions
const MAX_AUTH_SESSIONS: usize = 10_000;
/// Maximum number of prepared delegations awaiting retrieval
const MAX_PREPARED_DELEGATIONS: usize = 10_000;
/// Maximum number of concurrent auth sessions per address
const MAX_SESSIONS_PER_ADDRESS: usize = 5;

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
    /// The caller principal that initiated this login via prepare_login.
    /// Used to bind login completion to the originating caller in multi-tenant
    /// deployments (when `allowed_canisters` is configured).
    pub initiator: Principal,
}

/// Completed authentication session
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AuthSession {
    /// The Avalanche address
    pub address: String,
    /// The derived ICP principal
    pub principal: Principal,
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

/// Entry in the prepared delegation expiration queue (min-heap by expiration)
#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedDelegationExpiry {
    expires_at: u64,
    key: PreparedDelegationKey,
}

impl Ord for PreparedDelegationExpiry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse so BinaryHeap (max-heap) yields earliest expiration first
        other.expires_at.cmp(&self.expires_at)
    }
}

impl PartialOrd for PreparedDelegationExpiry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Counter for periodic drain of stale prepared delegation queue entries
static PREPARED_DRAIN_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Drain stale queue entries every N store calls
const PREPARED_DRAIN_INTERVAL: u64 = 50;

/// Transient canister state (lost on upgrade, that's acceptable)
#[derive(Default)]
pub struct TransientState {
    /// Cached settings (avoids deserializing from stable memory on every call)
    pub settings_cache: Option<Settings>,
    /// Active login sessions keyed by address
    pub login_sessions: HashMap<String, LoginSession>,
    /// Authenticated sessions keyed by session key hash
    pub auth_sessions: HashMap<String, AuthSession>,
    /// Rate limiter for login attempts
    pub rate_limiter: RateLimiter,
    /// Signature map for certified delegations
    pub signature_map: SignatureMap,
    /// Prepared delegations for certified retrieval
    /// Key: "{seed_hash_hex}:{session_key_hash}"
    pub prepared_delegations: HashMap<PreparedDelegationKey, PreparedDelegation>,
    /// Min-heap of prepared delegation expirations for O(k) cleanup
    prepared_delegation_queue: BinaryHeap<PreparedDelegationExpiry>,
}

impl TransientState {
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

/// Read transient state immutably
pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&TransientState) -> R,
{
    STATE.with(|state| f(&state.borrow()))
}

/// Mutate transient state
pub fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut TransientState) -> R,
{
    STATE.with(|state| f(&mut state.borrow_mut()))
}

// --- Settings (persistent via StableCell) ---

/// Store settings to stable memory and update the in-memory cache
pub fn store_settings(settings: &Settings) {
    let bytes = candid::encode_one(settings).expect("Failed to encode settings");
    SETTINGS_CELL.with(|cell| {
        cell.borrow_mut().set(bytes);
    });
    // Update the in-memory cache
    with_state_mut(|state| {
        state.settings_cache = Some(settings.clone());
    });
}

/// Access current settings via a closure (zero-copy, no heap allocation)
pub fn with_settings<R>(f: impl FnOnce(&Settings) -> R) -> R {
    with_state(|state| {
        f(state
            .settings_cache
            .as_ref()
            .expect("Canister not initialized - settings not set"))
    })
}

/// Check if settings have been initialized
pub fn has_settings() -> bool {
    with_state(|state| state.settings_cache.is_some())
}

/// Load settings from stable memory into the in-memory cache.
/// Called during init/upgrade to populate the cache.
fn load_settings_cache() {
    let settings = SETTINGS_CELL.with(|cell| {
        let bytes = cell.borrow().get().clone();
        if bytes.is_empty() {
            None
        } else {
            candid::decode_one(&bytes).ok()
        }
    });
    if let Some(s) = settings {
        with_state_mut(|state| {
            state.settings_cache = Some(s);
        });
    }
}

// --- Initialization ---

/// Initialize canister state with settings
///
/// Stores settings to stable memory and resets transient state.
/// Identity mappings in stable memory are NOT cleared -- they persist across upgrades.
pub fn init_state(settings: Settings) {
    // Store settings to stable memory
    store_settings(&settings);

    // Reset transient state (sessions, rate limiter, signature map)
    with_state_mut(|state| {
        state.rate_limiter = RateLimiter::new(settings.rate_limits.clone());
        state.login_sessions.clear();
        state.auth_sessions.clear();
        state.signature_map = SignatureMap::default();
        state.prepared_delegations.clear();
        state.prepared_delegation_queue.clear();
    });

    // Update certified data with empty signature map
    update_certified_data();
}

/// Initialize only transient state after an upgrade where settings are already in stable memory
///
/// Called when post_upgrade receives no new InitArgs but settings exist in stable memory.
pub fn init_transient_state() {
    // Load settings from stable memory into the in-memory cache first
    load_settings_cache();
    let rate_limits = with_settings(|s| s.rate_limits.clone());
    with_state_mut(|state| {
        state.rate_limiter = RateLimiter::new(rate_limits);
        state.login_sessions.clear();
        state.auth_sessions.clear();
        state.signature_map = SignatureMap::default();
        state.prepared_delegations.clear();
        state.prepared_delegation_queue.clear();
    });
    update_certified_data();
}

// --- Certified data ---

/// Update the canister's certified data with the current signature map root hash
///
/// This must be called after any modification to the signature map.
pub fn update_certified_data() {
    with_state(|state| {
        let root_hash = compute_root_hash(&state.signature_map);
        ic_cdk::api::certified_data_set(root_hash);
    });
}

/// Compute the root hash for certified data
fn compute_root_hash(signature_map: &SignatureMap) -> Hash {
    labeled_hash(LABEL_SIG, &signature_map.root_hash())
}

// --- Delegation storage ---

/// Counter for periodic signature map queue compaction
static DELEGATION_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Drain stale queue entries every N store_delegation calls
const DRAIN_STALE_INTERVAL: u64 = 50;

/// Store a delegation hash in the signature map
///
/// This adds the delegation to the certified data so it can be verified
/// by the IC when used in queries.
///
/// # Arguments
/// * `seed_hash` - Hash of the seed (derived from address + salt)
/// * `delegation_hash` - Hash of the delegation
/// * `delegation_expires_at` - When the delegation expires (nanoseconds)
pub fn store_delegation(seed_hash: Hash, delegation_hash: Hash, delegation_expires_at: u64) {
    let now = ic_cdk::api::time();
    with_state_mut(|state| {
        // Prune up to 20 expired entries to bound memory growth
        state.signature_map.prune_expired(now, 20);

        // Periodically drain orphaned queue entries to bound memory
        let count = DELEGATION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % DRAIN_STALE_INTERVAL == 0 {
            state.signature_map.drain_stale();
        }

        state
            .signature_map
            .put(seed_hash, delegation_hash, delegation_expires_at);
        debug_log!(
            "[STORE_DELEGATION] Stored in signature map. seed_hash: {}, delegation_hash: {}, delegation_expires_at: {}, now: {}, map_len: {}",
            hex::encode(seed_hash),
            hex::encode(delegation_hash),
            delegation_expires_at,
            now,
            state.signature_map.len()
        );
    });
    update_certified_data();
    debug_log!("[STORE_DELEGATION] Certified data updated");
}

/// Create a certified signature for a delegation
///
/// This creates the full CBOR-encoded signature with certificate and witness tree,
/// suitable for use as an IC canister signature.
pub fn create_certified_delegation_signature(
    seed_hash: Hash,
    delegation_hash: Hash,
) -> Option<Vec<u8>> {
    debug_log!(
        "[CERTIFIED_SIG] Called with seed_hash: {}, delegation_hash: {}",
        hex::encode(seed_hash),
        hex::encode(delegation_hash)
    );

    let certificate = ic_cdk::api::data_certificate();
    if certificate.is_none() {
        debug_log!("[CERTIFIED_SIG] No data certificate available - not in a query call?");
        return None;
    }
    let certificate = certificate.unwrap();
    debug_log!(
        "[CERTIFIED_SIG] Got data certificate, len: {}",
        certificate.len()
    );

    with_state(|state| {
        let now = ic_cdk::api::time();
        let map_len = state.signature_map.len();
        debug_log!(
            "[CERTIFIED_SIG] Checking expiration. now: {}, signature_map_len: {}",
            now,
            map_len
        );

        if state
            .signature_map
            .is_expired(now, seed_hash, delegation_hash)
        {
            debug_log!(
                "[CERTIFIED_SIG] Delegation expired or not found. seed_hash: {}, delegation_hash: {}, now: {}",
                hex::encode(seed_hash),
                hex::encode(delegation_hash),
                now
            );
            return None;
        }

        debug_log!("[CERTIFIED_SIG] Delegation is valid, getting witness");

        let witness = state.signature_map.witness(seed_hash, delegation_hash);
        if witness.is_none() {
            debug_log!(
                "[CERTIFIED_SIG] Failed to get witness. seed_hash: {}, delegation_hash: {}",
                hex::encode(seed_hash),
                hex::encode(delegation_hash)
            );
            return None;
        }
        let witness = witness.unwrap();

        debug_log!("[CERTIFIED_SIG] Got witness, creating labeled tree");

        let tree = ic_certified_map::labeled(LABEL_SIG, witness);

        match ic_siwa::create_certified_signature(certificate.clone(), tree) {
            Ok(sig) => {
                debug_log!(
                    "[CERTIFIED_SIG] Successfully created certified signature, len: {}",
                    sig.len()
                );
                Some(sig)
            }
            Err(e) => {
                debug_log!(
                    "[CERTIFIED_SIG] Failed to create certified signature: {:?}",
                    e
                );
                None
            }
        }
    })
}

// --- Login sessions ---

/// Store a new login session
///
/// Enforces capacity limit by cleaning up expired sessions first.
/// Returns error if at capacity after cleanup.
pub fn store_login_session(session: LoginSession) -> Result<(), String> {
    let now = ic_cdk::api::time();
    with_state_mut(|state| {
        state.cleanup_expired_logins();

        // Allow overwriting pending login sessions so users can retry after
        // cancelling or failing a wallet signing prompt. Rate limiting already
        // prevents abuse. Log overwrites for security monitoring.
        let key = session.address.to_lowercase();
        if let Some(existing) = state.login_sessions.get(&key) {
            if existing.expires_at > now {
                debug_log!(
                    "[SECURITY] Overwriting pending login session for address {}",
                    session.address
                );
            }
        }

        if state.login_sessions.len() >= MAX_LOGIN_SESSIONS
            && !state.login_sessions.contains_key(&key)
        {
            return Err("Too many pending login sessions. Please try again later.".to_string());
        }
        state.login_sessions.insert(key, session);
        Ok(())
    })
}

/// Get a login session by address
///
/// Checks the requested entry's expiration inline (O(1)) rather than
/// scanning all sessions. Full cleanup runs periodically via the rate
/// limiter's counter.
pub fn get_login_session(address: &str) -> Option<LoginSession> {
    let now = ic_cdk::api::time();
    with_state(|state| {
        state
            .login_sessions
            .get(&address.to_lowercase())
            .filter(|s| s.expires_at > now)
            .cloned()
    })
}

/// Remove a login session (after successful login)
pub fn remove_login_session(address: &str) {
    with_state_mut(|state| {
        state.login_sessions.remove(&address.to_lowercase());
    });
}

// --- Auth sessions ---

/// Store an authenticated session
///
/// Enforces capacity limit by cleaning up expired sessions first.
/// Returns error if at capacity after cleanup.
pub fn store_auth_session(key_hash: String, session: AuthSession) -> Result<(), String> {
    with_state_mut(|state| {
        state.cleanup_expired_auth();

        // Enforce global limit
        if state.auth_sessions.len() >= MAX_AUTH_SESSIONS
            && !state.auth_sessions.contains_key(&key_hash)
        {
            return Err("Too many active sessions. Please try again later.".to_string());
        }

        // Enforce per-address limit: evict oldest session for this address if at limit
        let address_lower = session.address.to_lowercase();
        let address_sessions: Vec<(String, u64)> = state
            .auth_sessions
            .iter()
            .filter(|(_, s)| s.address.to_lowercase() == address_lower)
            .map(|(k, s)| (k.clone(), s.created_at))
            .collect();

        if address_sessions.len() >= MAX_SESSIONS_PER_ADDRESS {
            // Remove the oldest session for this address
            if let Some((oldest_key, _)) = address_sessions
                .iter()
                .min_by_key(|(_, created_at)| created_at)
            {
                state.auth_sessions.remove(oldest_key);
            }
        }

        state.auth_sessions.insert(key_hash, session.clone());
        Ok(())
    })?;
    // Store identity mappings in stable memory (persists across upgrades)
    store_identity_mapping(&session.address, session.principal);
    Ok(())
}

/// Get an auth session by session key hash
///
/// Checks the requested entry's expiration inline (O(1)) rather than
/// scanning all sessions. Full cleanup runs periodically via the rate
/// limiter's counter.
pub fn get_auth_session(key_hash: &str) -> Option<AuthSession> {
    let now = ic_cdk::api::time();
    with_state(|state| {
        state
            .auth_sessions
            .get(key_hash)
            .filter(|s| s.expires_at > now)
            .cloned()
    })
}

/// Remove an auth session by session key hash
pub fn remove_auth_session(key_hash: &str) -> bool {
    with_state_mut(|state| state.auth_sessions.remove(key_hash).is_some())
}

/// Remove all auth sessions for an address
///
/// Returns the number of sessions removed.
pub fn remove_all_sessions_for_address(address: &str) -> usize {
    let address_lower = address.to_lowercase();
    with_state_mut(|state| {
        let before = state.auth_sessions.len();
        state
            .auth_sessions
            .retain(|_, s| s.address.to_lowercase() != address_lower);
        before - state.auth_sessions.len()
    })
}

// --- Identity mappings (persistent via StableBTreeMap) ---

/// Store an address <-> principal mapping in stable memory
fn store_identity_mapping(address: &str, principal: Principal) {
    let address_lower = address.to_lowercase();
    let storable_principal = StorablePrincipal(principal);

    ADDRESS_TO_PRINCIPAL.with(|map| {
        map.borrow_mut()
            .insert(address_lower.clone(), storable_principal.clone());
    });
    PRINCIPAL_TO_ADDRESS.with(|map| {
        map.borrow_mut().insert(storable_principal, address_lower);
    });
}

/// Get principal for address (from stable memory)
pub fn get_principal_for_address(address: &str) -> Option<Principal> {
    ADDRESS_TO_PRINCIPAL.with(|map| map.borrow().get(&address.to_lowercase()).map(|sp| sp.0))
}

/// Get address for principal (from stable memory)
pub fn get_address_for_principal(principal: &Principal) -> Option<String> {
    PRINCIPAL_TO_ADDRESS.with(|map| map.borrow().get(&StorablePrincipal(*principal)))
}

/// Maximum number of identity mappings to purge per call to stay within
/// IC instruction limits.
const PURGE_BATCH_SIZE: usize = 1000;

/// Purge identity mappings from stable memory in bounded batches.
///
/// Removes up to [`PURGE_BATCH_SIZE`] mappings per call to avoid exceeding
/// the IC instruction limit. Call repeatedly until the return value is 0
/// to purge all mappings. Mappings will be re-populated on next login for
/// each address.
///
/// Returns the number of mappings removed in this call.
pub fn purge_identity_mappings() -> u64 {
    // Collect a batch of addresses from the forward map
    let addresses: Vec<String> = ADDRESS_TO_PRINCIPAL.with(|map| {
        map.borrow()
            .iter()
            .take(PURGE_BATCH_SIZE)
            .map(|entry| entry.key().clone())
            .collect()
    });

    let count = addresses.len() as u64;

    // Look up and remove each mapping from both directions
    ADDRESS_TO_PRINCIPAL.with(|fwd| {
        let mut fwd = fwd.borrow_mut();
        PRINCIPAL_TO_ADDRESS.with(|rev| {
            let mut rev = rev.borrow_mut();
            for addr in &addresses {
                if let Some(principal) = fwd.remove(addr) {
                    rev.remove(&principal);
                }
            }
        });
    });

    count
}

// --- Rate limiting ---

/// Counter for periodic cleanup
static CLEANUP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const CLEANUP_INTERVAL: u64 = 100;

/// Check rate limit for an address and record the attempt if allowed
pub fn check_rate_limit(address: &str) -> Result<(), String> {
    let now_ns = ic_cdk::api::time();

    // Periodically cleanup expired entries to prevent memory growth
    let count = CLEANUP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if count % CLEANUP_INTERVAL == 0 {
        cleanup_rate_limits();
        // Also clean up expired sessions periodically
        with_state_mut(|state| {
            state.cleanup_expired_logins();
            state.cleanup_expired_auth();
        });
    }

    with_state_mut(
        |state| match state.rate_limiter.check_and_record(address, now_ns) {
            Ok(()) => Ok(()),
            Err(e) => {
                debug_log!(
                    "[RATE_LIMIT] Blocked address={} remaining_global={} error={}",
                    address,
                    state.rate_limiter.remaining_global(now_ns),
                    e
                );
                Err(e.to_string())
            }
        },
    )
}

/// Cleanup expired rate limit entries (call periodically)
pub fn cleanup_rate_limits() {
    let now_ns = ic_cdk::api::time();
    with_state_mut(|state| {
        state.rate_limiter.cleanup_expired(now_ns);
    });
}

// --- Prepared delegations ---

/// Store a prepared delegation for later retrieval
///
/// Enforces capacity limit by pruning expired entries first (O(k) where k
/// is the number of expired entries, not O(n) for the full map).
/// Returns error if at capacity after cleanup.
pub fn store_prepared_delegation(
    seed_hash: &[u8; 32],
    session_key_hash: &str,
    delegation: PreparedDelegation,
) -> Result<(), String> {
    let key = format!("{}:{}", hex::encode(seed_hash), session_key_hash);
    with_state_mut(|state| {
        let now = ic_cdk::api::time();

        // Prune up to 20 expired entries from the front of the min-heap
        prune_prepared_delegations(state, now, 20);

        // Periodically drain orphaned queue entries to bound memory
        let count = PREPARED_DRAIN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % PREPARED_DRAIN_INTERVAL == 0 {
            drain_stale_prepared_delegations(state);
        }

        if state.prepared_delegations.len() >= MAX_PREPARED_DELEGATIONS
            && !state.prepared_delegations.contains_key(&key)
        {
            return Err("Too many prepared delegations. Please try again later.".to_string());
        }

        let expires_at = delegation.final_expiration;
        state.prepared_delegations.insert(key.clone(), delegation);
        state
            .prepared_delegation_queue
            .push(PreparedDelegationExpiry { expires_at, key });
        Ok(())
    })
}

/// Get a prepared delegation by seed hash and session key hash
pub fn get_prepared_delegation(
    seed_hash: &[u8; 32],
    session_key_hash: &str,
) -> Option<PreparedDelegation> {
    let key = format!("{}:{}", hex::encode(seed_hash), session_key_hash);
    with_state(|state| state.prepared_delegations.get(&key).cloned())
}

/// Cleanup expired prepared delegations (O(k) not O(n))
pub fn cleanup_expired_prepared_delegations() {
    let now = ic_cdk::api::time();
    with_state_mut(|state| {
        prune_prepared_delegations(state, now, 20);
    });
}

/// Prune up to `max_to_prune` expired entries from the prepared delegation queue.
fn prune_prepared_delegations(state: &mut TransientState, now: u64, max_to_prune: usize) {
    let mut pruned = 0;
    while pruned < max_to_prune {
        match state.prepared_delegation_queue.peek() {
            Some(entry) if entry.expires_at <= now => {}
            _ => break,
        }
        let Some(entry) = state.prepared_delegation_queue.pop() else {
            break;
        };
        // Only remove from the map if the entry is actually expired
        // (it may have been overwritten with a newer expiration)
        if let Some(stored) = state.prepared_delegations.get(&entry.key) {
            if stored.final_expiration <= now {
                state.prepared_delegations.remove(&entry.key);
                pruned += 1;
            }
        }
    }
}

/// Drain orphaned queue entries whose keys no longer exist in the map.
/// This prevents the queue from growing unboundedly when delegations are
/// overwritten (the old queue entry becomes orphaned).
fn drain_stale_prepared_delegations(state: &mut TransientState) {
    let old_queue = std::mem::take(&mut state.prepared_delegation_queue);
    state.prepared_delegation_queue = old_queue
        .into_iter()
        .filter(|entry| state.prepared_delegations.contains_key(&entry.key))
        .collect();
}
